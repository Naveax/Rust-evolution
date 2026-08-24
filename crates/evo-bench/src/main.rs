use evo_bench::{SampleStats, Verdict, compare_samples, measurement_is_stable, summarize};
use evo_codegen_rust::generate_rust;
use evo_lexer::lex;
use evo_parser::parse;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
struct CaseConfig {
    name: String,
    warmup: usize,
    samples: usize,
    timeout: Duration,
    max_relative_mad: f64,
}

#[derive(Debug)]
struct Execution {
    duration_ns: u128,
    output: Output,
    timed_out: bool,
}

#[derive(Debug)]
struct Correctness {
    passed: bool,
    reference: Execution,
    evolution: Execution,
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
    verdict: Verdict,
}

#[derive(Debug)]
struct RunReport {
    config: CaseConfig,
    rustc_verbose: String,
    target: String,
    correctness: Correctness,
    measurement: Option<Measurement>,
    normalized_llvm_ir_equal: bool,
    reference_binary_bytes: u64,
    evolution_binary_bytes: u64,
}

fn main() -> ExitCode {
    match run_cli() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
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
                output_dir = Some(PathBuf::from(args.next().ok_or_else(|| {
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

impl CaseConfig {
    fn load(case_dir: &Path) -> Result<Self, String> {
        let path = case_dir.join("case.conf");
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

        let mut name = None;
        let mut warmup = None;
        let mut samples = None;
        let mut timeout_ms = None;
        let mut max_relative_mad = None;

        for (index, raw_line) in text.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line.split_once('=').ok_or_else(|| {
                format!("{}:{}: expected key=value", path.display(), index + 1)
            })?;
            let key = key.trim();
            let value = value.trim();
            match key {
                "name" => name = Some(value.to_owned()),
                "warmup" => warmup = Some(parse_usize(&path, index, key, value)?),
                "samples" => samples = Some(parse_usize(&path, index, key, value)?),
                "timeout_ms" => timeout_ms = Some(parse_u64(&path, index, key, value)?),
                "max_relative_mad" => {
                    max_relative_mad = Some(parse_f64(&path, index, key, value)?)
                }
                _ => {
                    return Err(format!(
                        "{}:{}: unknown config key {key:?}",
                        path.display(),
                        index + 1
                    ));
                }
            }
        }

        let config = Self {
            name: required(name, "name", &path)?,
            warmup: required(warmup, "warmup", &path)?,
            samples: required(samples, "samples", &path)?,
            timeout: Duration::from_millis(required(timeout_ms, "timeout_ms", &path)?),
            max_relative_mad: required(max_relative_mad, "max_relative_mad", &path)?,
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("benchmark name must not be empty".to_owned());
        }
        if self.samples < 3 {
            return Err("samples must be at least 3".to_owned());
        }
        if self.timeout.is_zero() {
            return Err("timeout_ms must be greater than zero".to_owned());
        }
        if !self.max_relative_mad.is_finite() || self.max_relative_mad < 0.0 {
            return Err("max_relative_mad must be a finite non-negative number".to_owned());
        }
        Ok(())
    }
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
    let tokens = lex(&evolution_source).map_err(|error| error.to_string())?;
    let program = parse(&tokens).map_err(|error| error.to_string())?;
    let generated_rust = generate_rust(&program);
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
    let rustc_verbose = command_text(&rustc, [OsString::from("-Vv")])?;
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
        reference_binary_bytes,
        evolution_binary_bytes,
    })
}

fn check_correctness(
    reference_binary: &Path,
    evolution_binary: &Path,
    stdin: &[u8],
    expected_stdout: &[u8],
    expected_stderr: &[u8],
    timeout: Duration,
) -> Result<Correctness, String> {
    let reference = execute(reference_binary, stdin, timeout)?;
    let evolution = execute(evolution_binary, stdin, timeout)?;

    let mut problems = Vec::new();
    if reference.timed_out {
        problems.push("reference timed out");
    }
    if evolution.timed_out {
        problems.push("evolution timed out");
    }
    if reference.output.status != evolution.output.status {
        problems.push("exit status differs between reference and evolution");
    }
    if reference.output.stdout != evolution.output.stdout {
        problems.push("stdout differs between reference and evolution");
    }
    if reference.output.stderr != evolution.output.stderr {
        problems.push("stderr differs between reference and evolution");
    }
    if reference.output.stdout != expected_stdout {
        problems.push("reference stdout differs from expected.stdout");
    }
    if evolution.output.stdout != expected_stdout {
        problems.push("evolution stdout differs from expected.stdout");
    }
    if reference.output.stderr != expected_stderr {
        problems.push("reference stderr differs from expected.stderr");
    }
    if evolution.output.stderr != expected_stderr {
        problems.push("evolution stderr differs from expected.stderr");
    }
    if !reference.output.status.success() {
        problems.push("reference exited unsuccessfully");
    }
    if !evolution.output.status.success() {
        problems.push("evolution exited unsuccessfully");
    }

    Ok(Correctness {
        passed: problems.is_empty(),
        reference,
        evolution,
        reason: if problems.is_empty() {
            "outputs, stderr and exit status match expected results".to_owned()
        } else {
            problems.join("; ")
        },
    })
}

fn measure(
    reference_binary: &Path,
    evolution_binary: &Path,
    stdin: &[u8],
    expected_stdout: &[u8],
    expected_stderr: &[u8],
    config: &CaseConfig,
) -> Result<Measurement, String> {
    for index in 0..config.warmup {
        if index % 2 == 0 {
            verify_timed_execution(execute(reference_binary, stdin, config.timeout)?, expected_stdout, expected_stderr)?;
            verify_timed_execution(execute(evolution_binary, stdin, config.timeout)?, expected_stdout, expected_stderr)?;
        } else {
            verify_timed_execution(execute(evolution_binary, stdin, config.timeout)?, expected_stdout, expected_stderr)?;
            verify_timed_execution(execute(reference_binary, stdin, config.timeout)?, expected_stdout, expected_stderr)?;
        }
    }

    let mut reference_samples_ns = Vec::with_capacity(config.samples);
    let mut evolution_samples_ns = Vec::with_capacity(config.samples);

    for index in 0..config.samples {
        if index % 2 == 0 {
            reference_samples_ns.push(measured_duration(
                execute(reference_binary, stdin, config.timeout)?,
                expected_stdout,
                expected_stderr,
            )?);
            evolution_samples_ns.push(measured_duration(
                execute(evolution_binary, stdin, config.timeout)?,
                expected_stdout,
                expected_stderr,
            )?);
        } else {
            evolution_samples_ns.push(measured_duration(
                execute(evolution_binary, stdin, config.timeout)?,
                expected_stdout,
                expected_stderr,
            )?);
            reference_samples_ns.push(measured_duration(
                execute(reference_binary, stdin, config.timeout)?,
                expected_stdout,
                expected_stderr,
            )?);
        }
    }

    let reference_stats = summarize(&reference_samples_ns)
        .ok_or_else(|| "reference samples unexpectedly empty".to_owned())?;
    let evolution_stats = summarize(&evolution_samples_ns)
        .ok_or_else(|| "evolution samples unexpectedly empty".to_owned())?;
    let stable = measurement_is_stable(
        reference_stats,
        evolution_stats,
        config.max_relative_mad,
    );
    let comparison = compare_samples(
        &reference_samples_ns,
        &evolution_samples_ns,
        true,
        stable,
    )
    .map_err(|error| format!("failed to compare samples: {error:?}"))?;

    Ok(Measurement {
        reference_samples_ns,
        evolution_samples_ns,
        reference_stats,
        evolution_stats,
        stable,
        ratio: comparison.performance_ratio,
        verdict: comparison.verdict,
    })
}

fn execute(binary: &Path, stdin: &[u8], timeout: Duration) -> Result<Execution, String> {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to execute {}: {error}", binary.display()))?;

    if let Some(mut child_stdin) = child.stdin.take() {
        child_stdin
            .write_all(stdin)
            .map_err(|error| format!("failed to write benchmark stdin: {error}"))?;
    }

    wait_for_child(child, timeout)
}

fn wait_for_child(mut child: Child, timeout: Duration) -> Result<Execution, String> {
    let started = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|error| format!("failed while waiting for benchmark process: {error}"))?
            .is_some()
        {
            let duration_ns = started.elapsed().as_nanos();
            let output = child
                .wait_with_output()
                .map_err(|error| format!("failed to collect benchmark output: {error}"))?;
            return Ok(Execution {
                duration_ns,
                output,
                timed_out: false,
            });
        }

        if started.elapsed() >= timeout {
            child
                .kill()
                .map_err(|error| format!("failed to kill timed-out benchmark: {error}"))?;
            let duration_ns = started.elapsed().as_nanos();
            let output = child
                .wait_with_output()
                .map_err(|error| format!("failed to collect timed-out benchmark output: {error}"))?;
            return Ok(Execution {
                duration_ns,
                output,
                timed_out: true,
            });
        }

        thread::sleep(Duration::from_millis(1));
    }
}

fn verify_timed_execution(
    execution: Execution,
    expected_stdout: &[u8],
    expected_stderr: &[u8],
) -> Result<(), String> {
    if execution.timed_out {
        return Err("benchmark execution timed out during measurement".to_owned());
    }
    if !execution.output.status.success() {
        return Err(format!(
            "benchmark execution failed during measurement: {}",
            execution.output.status
        ));
    }
    if execution.output.stdout != expected_stdout || execution.output.stderr != expected_stderr {
        return Err("benchmark output changed during measurement".to_owned());
    }
    Ok(())
}

fn measured_duration(
    execution: Execution,
    expected_stdout: &[u8],
    expected_stderr: &[u8],
) -> Result<u128, String> {
    let duration_ns = execution.duration_ns;
    verify_timed_execution(execution, expected_stdout, expected_stderr)?;
    Ok(duration_ns)
}

fn compile_binary(rustc: &OsStr, source: &Path, output: &Path) -> Result<(), String> {
    let mut command = rustc_base_command(rustc, source);
    command.arg("-o").arg(output);
    run_compile_command(command, "binary")
}

fn compile_llvm_ir(rustc: &OsStr, source: &Path, output: &Path) -> Result<(), String> {
    let mut command = rustc_base_command(rustc, source);
    command.arg("--emit=llvm-ir").arg("-o").arg(output);
    run_compile_command(command, "LLVM IR")
}

fn rustc_base_command(rustc: &OsStr, source: &Path) -> Command {
    let mut command = Command::new(rustc);
    command
        .arg(source)
        .arg("--crate-name")
        .arg("evo_benchmark_case")
        .arg("--edition=2024")
        .arg("-C")
        .arg("opt-level=3")
        .arg("-C")
        .arg("codegen-units=1")
        .arg("-C")
        .arg("lto=thin")
        .arg("-C")
        .arg("debuginfo=0");
    command
}

fn run_compile_command(mut command: Command, artifact: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("failed to execute rustc for {artifact}: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "rustc failed while building {artifact}: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn compare_normalized_ir(reference: &Path, evolution: &Path) -> Result<bool, String> {
    let reference_text = fs::read_to_string(reference)
        .map_err(|error| format!("failed to read {}: {error}", reference.display()))?;
    let evolution_text = fs::read_to_string(evolution)
        .map_err(|error| format!("failed to read {}: {error}", evolution.display()))?;
    Ok(normalize_llvm_ir(&reference_text) == normalize_llvm_ir(&evolution_text))
}

fn normalize_llvm_ir(input: &str) -> String {
    let mut output = String::new();
    for line in input.lines() {
        if line.starts_with("; ModuleID =") || line.starts_with("source_filename =") {
            continue;
        }
        output.push_str(&normalize_rust_symbol_hashes(line));
        output.push('\n');
    }
    output
}

fn normalize_rust_symbol_hashes(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut output = String::with_capacity(line.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'h'
            && index + 17 <= bytes.len()
            && bytes[index + 1..index + 17]
                .iter()
                .all(u8::is_ascii_hexdigit)
        {
            output.push_str("h<RUST_HASH>");
            index += 17;
        } else {
            output.push(char::from(bytes[index]));
            index += 1;
        }
    }
    output
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
            "  \"correctness\": {correctness},\n",
            "  \"correctness_reason\": {reason},\n",
            "  \"stable_measurement\": {stable},\n",
            "  \"performance_ratio\": {ratio},\n",
            "  \"verdict\": {verdict},\n",
            "  \"normalized_llvm_ir_equal\": {ir_equal},\n",
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
        correctness = report.correctness.passed,
        reason = json_string(&report.correctness.reason),
        stable = stable,
        ratio = ratio,
        verdict = json_string(&verdict),
        ir_equal = report.normalized_llvm_ir_equal,
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
        "- Correctness: **{}**\n",
        if report.correctness.passed { "PASS" } else { "FAIL" }
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
        "- Binary size: reference {} B, evolution {} B\n",
        report.reference_binary_bytes, report.evolution_binary_bytes
    ));

    if let Some(measurement) = &report.measurement {
        text.push_str(&format!("- Verdict: **{}**\n", measurement.verdict));
        text.push_str(&format!(
            "- Performance ratio `T_evolution / T_reference`: **{:.9}**\n",
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
        text.push_str("- Verdict: **FAIL** (performance phase skipped because correctness failed)\n");
    }

    text.push_str("\n## Measurement policy\n\n");
    text.push_str("Correctness is checked before timing. Reference and Evolution executions alternate order. Unstable measurements are INCONCLUSIVE rather than PASS. The hard runtime contract remains `T_evolution <= T_reference_rust`.\n");
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
    println!("normalized LLVM IR equal: {}", report.normalized_llvm_ir_equal);
    if let Some(measurement) = &report.measurement {
        println!("reference median: {:.0} ns", measurement.reference_stats.median_ns);
        println!("evolution median: {:.0} ns", measurement.evolution_stats.median_ns);
        println!("ratio: {:.9}", measurement.ratio);
        println!("stable: {}", measurement.stable);
        println!("verdict: {}", measurement.verdict);
    } else {
        println!("verdict: FAIL (correctness)");
    }
}

fn prepare_output_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("failed to clean {}: {error}", path.display()))?;
    }
    fs::create_dir_all(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))
}

fn rustc_program() -> OsString {
    env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"))
}

fn command_text<I, S>(program: &OsStr, args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("failed to execute {program:?}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "command {program:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("command output was not UTF-8: {error}"))
}

fn parse_host_target(rustc_verbose: &str) -> Result<String, String> {
    rustc_verbose
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .ok_or_else(|| "rustc -Vv output did not contain a host target".to_owned())
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

fn safe_file_name(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn required<T>(value: Option<T>, key: &str, path: &Path) -> Result<T, String> {
    value.ok_or_else(|| format!("{}: missing required key {key:?}", path.display()))
}

fn parse_usize(path: &Path, index: usize, key: &str, value: &str) -> Result<usize, String> {
    value.parse::<usize>().map_err(|error| {
        format!(
            "{}:{}: invalid {key}: {error}",
            path.display(),
            index + 1
        )
    })
}

fn parse_u64(path: &Path, index: usize, key: &str, value: &str) -> Result<u64, String> {
    value.parse::<u64>().map_err(|error| {
        format!(
            "{}:{}: invalid {key}: {error}",
            path.display(),
            index + 1
        )
    })
}

fn parse_f64(path: &Path, index: usize, key: &str, value: &str) -> Result<f64, String> {
    value.parse::<f64>().map_err(|error| {
        format!(
            "{}:{}: invalid {key}: {error}",
            path.display(),
            index + 1
        )
    })
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
    use super::{normalize_llvm_ir, normalize_rust_symbol_hashes, safe_file_name};

    #[test]
    fn normalizes_rust_symbol_hashes() {
        assert_eq!(
            normalize_rust_symbol_hashes("_ZN4test17h0123456789abcdefE"),
            "_ZN4test17h<RUST_HASH>E"
        );
    }

    #[test]
    fn normalized_ir_drops_source_identity() {
        let left = "; ModuleID = 'left'\nsource_filename = \"left.rs\"\ndefine void @_ZN4test17h0123456789abcdefE() {}\n";
        let right = "; ModuleID = 'right'\nsource_filename = \"right.rs\"\ndefine void @_ZN4test17hfedcba9876543210E() {}\n";
        assert_eq!(normalize_llvm_ir(left), normalize_llvm_ir(right));
    }

    #[test]
    fn sanitizes_case_name_for_output_path() {
        assert_eq!(safe_file_name("arithmetic / smoke"), "arithmetic___smoke");
    }
}
