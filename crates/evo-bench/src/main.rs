mod compiler;
mod config;
mod execution;

use compiler::{
    compare_binary_bytes, compare_normalized_ir, compile_binary, compile_llvm_ir,
    parse_host_target, rustc_program, rustc_verbose,
};
use config::{CaseConfig, safe_file_name};
use evo_bench::{
    SampleStats, Verdict, classify_with_binary_parity, compare_samples, measurement_is_stable,
    summarize,
};
use evo_codegen_rust::generate_lowered_rust;
use evo_diagnostics::render_error;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse_recovering;
use execution::{Execution, execute_with_timeout, measure_blocking};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SCHEMA_VERSION: u32 = 2;
const MEASUREMENT_MODE: &str = "process-wall-clock";
const TIMING_OUTPUT_POLICY: &str =
    "correctness captures stdout/stderr; timed samples redirect both streams to null symmetrically";
const BINARY_PARITY_BASIS: &str = "byte-identical-binary-parity";
const TIMING_BASIS: &str = "timing-median-ratio";

#[derive(Debug)]
struct Correctness {
    passed: bool,
    reason: String,
}

#[derive(Debug)]
struct Measurement {
    reference_samples_ns: Vec<u128>,
    evolution_samples_ns: Vec<u128>,
    reference_stats: SampleStats,
    evolution_stats: SampleStats,
    stable: bool,
    ratio: f64,
    timing_verdict: Verdict,
    verdict: Verdict,
    verdict_basis: &'static str,
}

#[derive(Debug)]
struct RunReport {
    config: CaseConfig,
    rustc_verbose: String,
    target: String,
    correctness: Correctness,
    measurement: Option<Measurement>,
    normalized_llvm_ir_equal: bool,
    binary_equal: bool,
    reference_binary_bytes: u64,
    evolution_binary_bytes: u64,
}

fn main() -> ExitCode {
    match run_cli() {
        Ok(code) => code,
        Err(error) => {
            if error.starts_with("error: ") {
                eprintln!("{error}");
            } else {
                eprintln!("error: {error}");
            }
            ExitCode::FAILURE
        }
    }
}

fn run_cli() -> Result<ExitCode, String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(usage)?;
    if command != "run" {
        return Err(usage());
    }

    let case_dir = PathBuf::from(args.next().ok_or_else(usage)?);
    let mut output_dir = None;
    let mut report_only = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                output_dir =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        "--out requires a directory argument".to_owned()
                    })?));
            }
            "--report-only" => report_only = true,
            _ => return Err(format!("unknown argument {arg:?}\n{}", usage())),
        }
    }

    let config = CaseConfig::load(&case_dir)?;
    let output_dir = output_dir
        .unwrap_or_else(|| PathBuf::from("target/evo-bench").join(safe_file_name(&config.name)));
    let report = run_case(&case_dir, &output_dir, config)?;
    write_reports(&output_dir, &report)?;
    print_summary(&report);

    if report_only {
        return Ok(ExitCode::SUCCESS);
    }

    match report
        .measurement
        .as_ref()
        .map_or(Verdict::Fail, |measurement| measurement.verdict)
    {
        Verdict::Pass => Ok(ExitCode::SUCCESS),
        Verdict::Fail => Ok(ExitCode::FAILURE),
        Verdict::Inconclusive => Ok(ExitCode::from(2)),
    }
}

fn usage() -> String {
    "usage: evo-bench run <case-dir> [--out <dir>] [--report-only]".to_owned()
}

fn run_case(case_dir: &Path, output_dir: &Path, config: CaseConfig) -> Result<RunReport, String> {
    prepare_output_dir(output_dir)?;

    let evolution_source_path = case_dir.join("evolution.evo");
    let reference_source_path = case_dir.join("reference.rs");
    let expected_stdout = read_required(case_dir.join("expected.stdout"))?;
    let expected_stderr = read_optional(case_dir.join("expected.stderr"))?.unwrap_or_default();
    let stdin = read_optional(case_dir.join("stdin.bin"))?.unwrap_or_default();

    let evolution_source = fs::read_to_string(&evolution_source_path).map_err(|error| {
        format!(
            "failed to read {}: {error}",
            evolution_source_path.display()
        )
    })?;
    let tokens = lex(&evolution_source).map_err(|error| {
        render_error(
            &evolution_source_path,
            &evolution_source,
            &error.message,
            error.span,
        )
    })?;
    let syntax = parse_recovering(&tokens).map_err(|errors| {
        render_parse_errors(&evolution_source_path, &evolution_source, &errors)
    })?;
    let program = lower(&syntax).map_err(|error| {
        render_error(
            &evolution_source_path,
            &evolution_source,
            &error.message,
            error.span,
        )
    })?;
    let generated_rust = generate_lowered_rust(&program);
    let generated_path = output_dir.join("generated.rs");
    fs::write(&generated_path, &generated_rust)
        .map_err(|error| format!("failed to write {}: {error}", generated_path.display()))?;

    if !reference_source_path.is_file() {
        return Err(format!(
            "reference source does not exist: {}",
            reference_source_path.display()
        ));
    }

    let rustc = rustc_program();
    let rustc_verbose = rustc_verbose(&rustc)?;
    let target = parse_host_target(&rustc_verbose)?;

    let reference_binary = output_dir.join(executable_name("reference"));
    let evolution_binary = output_dir.join(executable_name("evolution"));
    let reference_ir = output_dir.join("reference.ll");
    let evolution_ir = output_dir.join("evolution.ll");

    compile_binary(&rustc, &reference_source_path, &reference_binary)?;
    compile_binary(&rustc, &generated_path, &evolution_binary)?;
    compile_llvm_ir(&rustc, &reference_source_path, &reference_ir)?;
    compile_llvm_ir(&rustc, &generated_path, &evolution_ir)?;

    let correctness = check_correctness(
        &reference_binary,
        &evolution_binary,
        &stdin,
        &expected_stdout,
        &expected_stderr,
        config.timeout,
    )?;

    let normalized_llvm_ir_equal = compare_normalized_ir(&reference_ir, &evolution_ir)?;
    let binary_equal = compare_binary_bytes(&reference_binary, &evolution_binary)?;
    let reference_binary_bytes = file_len(&reference_binary)?;
    let evolution_binary_bytes = file_len(&evolution_binary)?;

    let measurement = if correctness.passed {
        Some(measure(
            &reference_binary,
            &evolution_binary,
            &stdin,
            &expected_stdout,
            &expected_stderr,
            &config,
            binary_equal,
        )?)
    } else {
        None
    };

    Ok(RunReport {
        config,
        rustc_verbose,
        target,
        correctness,
        measurement,
        normalized_llvm_ir_equal,
        binary_equal,
        reference_binary_bytes,
        evolution_binary_bytes,
    })
}

fn render_parse_errors(path: &Path, source: &str, errors: &[evo_parser::ParseError]) -> String {
    errors
        .iter()
        .map(|error| render_error(path, source, &error.message, error.span))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn check_correctness(
    reference_binary: &Path,
    evolution_binary: &Path,
    stdin: &[u8],
    expected_stdout: &[u8],
    expected_stderr: &[u8],
    timeout: std::time::Duration,
) -> Result<Correctness, String> {
    let reference = execute_with_timeout(reference_binary, stdin, timeout)?;
    let evolution = execute_with_timeout(evolution_binary, stdin, timeout)?;

    let mut problems = Vec::new();
    collect_execution_problems(
        "reference",
        &reference,
        expected_stdout,
        expected_stderr,
        &mut problems,
    );
    collect_execution_problems(
        "evolution",
        &evolution,
        expected_stdout,
        expected_stderr,
        &mut problems,
    );

    if reference.status != evolution.status {
        problems.push("exit status differs between reference and evolution".to_owned());
    }
    if reference.stdout != evolution.stdout {
        problems.push("stdout differs between reference and evolution".to_owned());
    }
    if reference.stderr != evolution.stderr {
        problems.push("stderr differs between reference and evolution".to_owned());
    }

    Ok(Correctness {
        passed: problems.is_empty(),
        reason: if problems.is_empty() {
            "outputs, stderr and exit status match expected results".to_owned()
        } else {
            problems.join("; ")
        },
    })
}

fn collect_execution_problems(
    label: &str,
    execution: &Execution,
    expected_stdout: &[u8],
    expected_stderr: &[u8],
    problems: &mut Vec<String>,
) {
    if execution.timed_out {
        problems.push(format!("{label} timed out"));
    }
    if execution.stdout != expected_stdout {
        problems.push(format!("{label} stdout differs from expected.stdout"));
    }
    if execution.stderr != expected_stderr {
        problems.push(format!("{label} stderr differs from expected.stderr"));
    }
    if !execution.status.success() {
        problems.push(format!("{label} exited unsuccessfully"));
    }
}

fn measure(
    reference_binary: &Path,
    evolution_binary: &Path,
    stdin: &[u8],
    expected_stdout: &[u8],
    expected_stderr: &[u8],
    config: &CaseConfig,
    binary_equal: bool,
) -> Result<Measurement, String> {
    for index in 0..config.warmup {
        if index % 2 == 0 {
            verify_execution(
                execute_with_timeout(reference_binary, stdin, config.timeout)?,
                expected_stdout,
                expected_stderr,
            )?;
            verify_execution(
                execute_with_timeout(evolution_binary, stdin, config.timeout)?,
                expected_stdout,
                expected_stderr,
            )?;
        } else {
            verify_execution(
                execute_with_timeout(evolution_binary, stdin, config.timeout)?,
                expected_stdout,
                expected_stderr,
            )?;
            verify_execution(
                execute_with_timeout(reference_binary, stdin, config.timeout)?,
                expected_stdout,
                expected_stderr,
            )?;
        }
    }

    let mut reference_samples_ns = Vec::with_capacity(config.samples);
    let mut evolution_samples_ns = Vec::with_capacity(config.samples);

    for index in 0..config.samples {
        if index % 2 == 0 {
            reference_samples_ns.push(measure_blocking(reference_binary, stdin)?);
            evolution_samples_ns.push(measure_blocking(evolution_binary, stdin)?);
        } else {
            evolution_samples_ns.push(measure_blocking(evolution_binary, stdin)?);
            reference_samples_ns.push(measure_blocking(reference_binary, stdin)?);
        }
    }

    let reference_stats = summarize(&reference_samples_ns)
        .ok_or_else(|| "reference samples unexpectedly empty".to_owned())?;
    let evolution_stats = summarize(&evolution_samples_ns)
        .ok_or_else(|| "evolution samples unexpectedly empty".to_owned())?;
    let stable = measurement_is_stable(reference_stats, evolution_stats, config.max_relative_mad);
    let comparison = compare_samples(&reference_samples_ns, &evolution_samples_ns, true, stable)
        .map_err(|error| format!("failed to compare samples: {error:?}"))?;
    let verdict =
        classify_with_binary_parity(true, binary_equal, stable, comparison.performance_ratio);
    let verdict_basis = if binary_equal {
        BINARY_PARITY_BASIS
    } else {
        TIMING_BASIS
    };

    Ok(Measurement {
        reference_samples_ns,
        evolution_samples_ns,
        reference_stats,
        evolution_stats,
        stable,
        ratio: comparison.performance_ratio,
        timing_verdict: comparison.verdict,
        verdict,
        verdict_basis,
    })
}

fn verify_execution(
    execution: Execution,
    expected_stdout: &[u8],
    expected_stderr: &[u8],
) -> Result<(), String> {
    if execution.timed_out {
        return Err("benchmark execution timed out during warmup".to_owned());
    }
    if !execution.status.success() {
        return Err(format!(
            "benchmark execution failed during warmup: {}",
            execution.status
        ));
    }
    if execution.stdout != expected_stdout || execution.stderr != expected_stderr {
        return Err("benchmark output changed during warmup".to_owned());
    }
    Ok(())
}

fn write_reports(output_dir: &Path, report: &RunReport) -> Result<(), String> {
    let json_path = output_dir.join("report.json");
    let markdown_path = output_dir.join("report.md");
    let raw_path = output_dir.join("raw-samples.csv");

    fs::write(&json_path, render_json(report))
        .map_err(|error| format!("failed to write {}: {error}", json_path.display()))?;
    fs::write(&markdown_path, render_markdown(report))
        .map_err(|error| format!("failed to write {}: {error}", markdown_path.display()))?;
    fs::write(&raw_path, render_raw_samples(report))
        .map_err(|error| format!("failed to write {}: {error}", raw_path.display()))?;
    Ok(())
}

fn render_json(report: &RunReport) -> String {
    let measurement = report.measurement.as_ref();
    let verdict = measurement.map_or("FAIL".to_owned(), |value| value.verdict.to_string());
    let timing_verdict =
        measurement.map_or("FAIL".to_owned(), |value| value.timing_verdict.to_string());
    let verdict_basis = measurement.map_or("correctness", |value| value.verdict_basis);
    let stable = measurement.is_some_and(|value| value.stable);
    let ratio = measurement.map_or("null".to_owned(), |value| format!("{:.9}", value.ratio));
    let reference_stats = measurement.map(|value| value.reference_stats);
    let evolution_stats = measurement.map(|value| value.evolution_stats);

    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": {schema_version},\n",
            "  \"benchmark_name\": {name},\n",
            "  \"target\": {target},\n",
            "  \"rustc_verbose\": {rustc},\n",
            "  \"warmup\": {warmup},\n",
            "  \"samples\": {samples},\n",
            "  \"timeout_ms\": {timeout_ms},\n",
            "  \"max_relative_mad\": {max_relative_mad:.9},\n",
            "  \"measurement_mode\": {measurement_mode},\n",
            "  \"timing_output_policy\": {timing_output_policy},\n",
            "  \"correctness\": {correctness},\n",
            "  \"correctness_reason\": {reason},\n",
            "  \"stable_measurement\": {stable},\n",
            "  \"performance_ratio\": {ratio},\n",
            "  \"timing_verdict\": {timing_verdict},\n",
            "  \"verdict\": {verdict},\n",
            "  \"verdict_basis\": {verdict_basis},\n",
            "  \"normalized_llvm_ir_equal\": {ir_equal},\n",
            "  \"binary_equal\": {binary_equal},\n",
            "  \"reference_binary_bytes\": {reference_binary_bytes},\n",
            "  \"evolution_binary_bytes\": {evolution_binary_bytes},\n",
            "  \"reference_stats\": {reference_stats},\n",
            "  \"evolution_stats\": {evolution_stats}\n",
            "}}\n"
        ),
        schema_version = SCHEMA_VERSION,
        name = json_string(&report.config.name),
        target = json_string(&report.target),
        rustc = json_string(&report.rustc_verbose),
        warmup = report.config.warmup,
        samples = report.config.samples,
        timeout_ms = report.config.timeout.as_millis(),
        max_relative_mad = report.config.max_relative_mad,
        measurement_mode = json_string(MEASUREMENT_MODE),
        timing_output_policy = json_string(TIMING_OUTPUT_POLICY),
        correctness = report.correctness.passed,
        reason = json_string(&report.correctness.reason),
        stable = stable,
        ratio = ratio,
        timing_verdict = json_string(&timing_verdict),
        verdict = json_string(&verdict),
        verdict_basis = json_string(verdict_basis),
        ir_equal = report.normalized_llvm_ir_equal,
        binary_equal = report.binary_equal,
        reference_binary_bytes = report.reference_binary_bytes,
        evolution_binary_bytes = report.evolution_binary_bytes,
        reference_stats = stats_json(reference_stats),
        evolution_stats = stats_json(evolution_stats),
    )
}

fn stats_json(stats: Option<SampleStats>) -> String {
    stats.map_or_else(
        || "null".to_owned(),
        |value| {
            format!(
                concat!(
                    "{{\"count\":{},\"min_ns\":{},\"max_ns\":{},",
                    "\"median_ns\":{:.3},\"p95_ns\":{},\"relative_mad\":{:.9}}}"
                ),
                value.count,
                value.min_ns,
                value.max_ns,
                value.median_ns,
                value.p95_ns,
                value.relative_mad
            )
        },
    )
}

fn render_markdown(report: &RunReport) -> String {
    let mut text = String::new();
    text.push_str("# Rust Evolution benchmark report\n\n");
    text.push_str(&format!("- Benchmark: `{}`\n", report.config.name));
    text.push_str(&format!("- Target: `{}`\n", report.target));
    text.push_str(&format!(
        "- Warmup/sample count: {}/{}\n",
        report.config.warmup, report.config.samples
    ));
    text.push_str(&format!(
        "- Correctness: **{}**\n",
        if report.correctness.passed {
            "PASS"
        } else {
            "FAIL"
        }
    ));
    text.push_str(&format!(
        "- Correctness detail: {}\n",
        report.correctness.reason
    ));
    text.push_str(&format!(
        "- Normalized LLVM IR equal: **{}**\n",
        report.normalized_llvm_ir_equal
    ));
    text.push_str(&format!(
        "- Exact binary equal: **{}**\n",
        report.binary_equal
    ));
    text.push_str(&format!(
        "- Binary size: reference {} B, evolution {} B\n",
        report.reference_binary_bytes, report.evolution_binary_bytes
    ));

    if let Some(measurement) = &report.measurement {
        text.push_str(&format!("- Verdict: **{}**\n", measurement.verdict));
        text.push_str(&format!(
            "- Verdict basis: `{}`\n",
            measurement.verdict_basis
        ));
        text.push_str(&format!(
            "- Timing-only verdict: **{}**\n",
            measurement.timing_verdict
        ));
        text.push_str(&format!(
            "- Observed performance ratio `T_evolution / T_reference`: **{:.9}**\n",
            measurement.ratio
        ));
        text.push_str(&format!(
            "- Stable measurement: **{}** (max relative MAD {:.4})\n\n",
            measurement.stable, report.config.max_relative_mad
        ));
        text.push_str("| Metric | Reference | Evolution |\n");
        text.push_str("| --- | ---: | ---: |\n");
        text.push_str(&format!(
            "| Median | {:.0} ns | {:.0} ns |\n",
            measurement.reference_stats.median_ns, measurement.evolution_stats.median_ns
        ));
        text.push_str(&format!(
            "| p95 | {} ns | {} ns |\n",
            measurement.reference_stats.p95_ns, measurement.evolution_stats.p95_ns
        ));
        text.push_str(&format!(
            "| Min | {} ns | {} ns |\n",
            measurement.reference_stats.min_ns, measurement.evolution_stats.min_ns
        ));
        text.push_str(&format!(
            "| Max | {} ns | {} ns |\n",
            measurement.reference_stats.max_ns, measurement.evolution_stats.max_ns
        ));
        text.push_str(&format!(
            "| Relative MAD | {:.6} | {:.6} |\n",
            measurement.reference_stats.relative_mad, measurement.evolution_stats.relative_mad
        ));
    } else {
        text.push_str(
            "- Verdict: **FAIL** (performance phase skipped because correctness failed)\n",
        );
    }

    text.push_str("\n## Measurement policy\n\n");
    text.push_str("Correctness is checked before timing. Reference and Evolution sources are staged under the same canonical `benchmark.rs` identity before rustc compilation. Exact byte-identical executables are deterministic runtime parity proof: observed wall-clock samples and their timing-only verdict remain reported, but scheduler noise cannot turn the same executable into a regression. When binaries differ, the hard timing gate remains unchanged: stable `T_evolution / T_reference <= 1.00` is PASS, a stable ratio above 1.00 is FAIL, and unstable measurements are INCONCLUSIVE. Correctness and warmup executions use timeout-controlled file capture. Timed samples use blocking process waits without polling; stdout/stderr are redirected to the platform null device on both sides, and execution order alternates by sample.\n");
    text
}

fn render_raw_samples(report: &RunReport) -> String {
    let mut text = String::from("sample,reference_ns,evolution_ns\n");
    if let Some(measurement) = &report.measurement {
        for (index, (reference, evolution)) in measurement
            .reference_samples_ns
            .iter()
            .zip(&measurement.evolution_samples_ns)
            .enumerate()
        {
            text.push_str(&format!("{index},{reference},{evolution}\n"));
        }
    }
    text
}

fn print_summary(report: &RunReport) {
    println!("benchmark: {}", report.config.name);
    println!("correctness: {}", report.correctness.passed);
    println!(
        "normalized LLVM IR equal: {}",
        report.normalized_llvm_ir_equal
    );
    println!("exact binary equal: {}", report.binary_equal);
    if let Some(measurement) = &report.measurement {
        println!(
            "reference median: {:.0} ns",
            measurement.reference_stats.median_ns
        );
        println!(
            "evolution median: {:.0} ns",
            measurement.evolution_stats.median_ns
        );
        println!("observed ratio: {:.9}", measurement.ratio);
        println!("stable: {}", measurement.stable);
        println!("timing verdict: {}", measurement.timing_verdict);
        println!("verdict basis: {}", measurement.verdict_basis);
        println!("verdict: {}", measurement.verdict);
    } else {
        println!("verdict: FAIL (correctness)");
    }
}

fn prepare_output_dir(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("output directory must not be empty".to_owned());
    }
    fs::create_dir_all(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))
}

fn read_required(path: PathBuf) -> Result<Vec<u8>, String> {
    fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn read_optional(path: PathBuf) -> Result<Option<Vec<u8>>, String> {
    if path.is_file() {
        fs::read(&path)
            .map(Some)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))
    } else {
        Ok(None)
    }
}

fn file_len(path: &Path) -> Result<u64, String> {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))
}

fn executable_name(stem: &str) -> String {
    format!("{stem}{}", env::consts::EXE_SUFFIX)
}

fn json_string(input: &str) -> String {
    let mut output = String::with_capacity(input.len() + 2);
    output.push('"');
    for ch in input.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch.is_control() => output.push_str(&format!("\\u{:04x}", u32::from(ch))),
            _ => output.push(ch),
        }
    }
    output.push('"');
    output
}

#[cfg(test)]
mod tests {
    use super::json_string;

    #[test]
    fn escapes_json_control_characters() {
        assert_eq!(json_string("a\n\"b\\c"), "\"a\\n\\\"b\\\\c\"");
    }
}
