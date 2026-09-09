include!("build_latency_baseline.rs");

use std::fmt::Write as _;

const LINK_RESEARCH_SAMPLES: usize = 7;
const LINK_RESEARCH_CASES: [&str; 2] = ["enums-v0", "logical-operators-v0"];
const MATERIAL_LINK_SHARE: f64 = 0.10;
const MATERIAL_LINK_MS: f64 = 8.0;
const IMMATERIAL_LINK_SHARE: f64 = 0.05;
const IMMATERIAL_LINK_MS: f64 = 5.0;

struct LinkCaseResult {
    name: String,
    full: Vec<Duration>,
    object: Vec<Duration>,
    instrumented_full: Vec<Duration>,
    link_child: Vec<Duration>,
    linker_invocations: Vec<u64>,
    full_bytes: u64,
    object_bytes: u64,
    instrumented_bytes: u64,
    generated_rust: Vec<u8>,
    link_args_probe: Vec<u8>,
    wrapper_link_args: String,
}

fn resolve_from_path(name: &str) -> PathBuf {
    let path = env::var_os("PATH").expect("PATH should exist on the controlled runner");
    for directory in env::split_paths(&path) {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("{name} should resolve from PATH on the controlled runner");
}

fn linker_wrapper_source() -> &'static str {
    r#"use std::env;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::{self, Command};
use std::time::Instant;

fn main() {
    let real = env::var_os("EVO_TEST_REAL_LINKER").expect("real linker must be set");
    let timings = env::var_os("EVO_TEST_LINK_TIMINGS").expect("timing path must be set");
    let args_path = env::var_os("EVO_TEST_LINK_ARGS").expect("args path must be set");
    let args = env::args_os().skip(1).collect::<Vec<_>>();

    let start = Instant::now();
    let status = Command::new(real)
        .args(&args)
        .status()
        .expect("real linker driver must execute");
    let elapsed_ns = start.elapsed().as_nanos();

    let mut timing_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(timings)
        .expect("link timing file must open");
    writeln!(timing_file, "{elapsed_ns}").expect("link timing must write");

    let mut args_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(args_path)
        .expect("link args file must open");
    writeln!(args_file, "{:?}", args).expect("link args must write");

    process::exit(status.code().unwrap_or(1));
}
"#
}

fn compile_linker_wrapper(dir: &Path, rustc: &OsStr) -> PathBuf {
    let source = dir.join("linker-wrapper.rs");
    let wrapper_dir = dir.join("linker-shadow");
    fs::create_dir_all(&wrapper_dir).expect("linker wrapper directory should be created");
    let binary = wrapper_dir.join(format!("cc{}", env::consts::EXE_SUFFIX));
    fs::write(&source, linker_wrapper_source()).expect("linker wrapper source should be written");

    let output = Command::new(rustc)
        .arg(&source)
        .arg("--edition=2024")
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("linker wrapper rustc should run");
    assert_success(&output, "linker wrapper compile");
    binary
}

fn production_rustc_command(rustc: &OsStr, generated: &Path) -> Command {
    let mut command = Command::new(rustc);
    command
        .arg(generated)
        .arg(format!("--edition={RUST_EDITION}"))
        .arg("--error-format=short")
        .arg("-C")
        .arg(format!("opt-level={RUST_OPT_LEVEL}"))
        .arg("-C")
        .arg(format!("codegen-units={RUST_CODEGEN_UNITS}"));
    command
}

fn prefixed_path(directory: &Path) -> OsString {
    let current = env::var_os("PATH").expect("PATH should exist");
    env::join_paths(std::iter::once(directory.to_path_buf()).chain(env::split_paths(&current)))
        .expect("instrumented PATH should join")
}

fn read_link_child_sample(path: &Path) -> (Duration, u64) {
    let raw = fs::read_to_string(path).expect("linker wrapper timing should exist");
    let values = raw
        .lines()
        .map(|line| {
            line.trim()
                .parse::<u64>()
                .expect("linker wrapper timing should be integer nanoseconds")
        })
        .collect::<Vec<_>>();
    assert!(
        !values.is_empty(),
        "default rustc link should invoke PATH-resolved cc on controlled Ubuntu"
    );
    let total_ns = values.iter().copied().sum::<u64>();
    (Duration::from_nanos(total_ns), values.len() as u64)
}

fn case_classification(result: &LinkCaseResult) -> &'static str {
    let full = stats(&result.full);
    let link = stats(&result.link_child);
    let share = link.median_ms / full.median_ms;
    if link.median_ms >= MATERIAL_LINK_MS && share >= MATERIAL_LINK_SHARE {
        "MATERIAL"
    } else if link.median_ms < IMMATERIAL_LINK_MS || share < IMMATERIAL_LINK_SHARE {
        "IMMATERIAL"
    } else {
        "MID-RANGE"
    }
}

fn aggregate_classification(results: &[LinkCaseResult]) -> &'static str {
    if results
        .iter()
        .all(|result| case_classification(result) == "MATERIAL")
    {
        "FOLLOW-UP-CANDIDATE"
    } else if results
        .iter()
        .all(|result| case_classification(result) == "IMMATERIAL")
    {
        "DEFER-LINKER-WORK"
    } else {
        "EXPAND-CORPUS"
    }
}

fn write_link_research_report(
    out_dir: &Path,
    results: &[LinkCaseResult],
    rustc_version: &str,
    linker_version: &[u8],
) {
    fs::create_dir_all(out_dir).expect("link research output directory should be created");
    let git_sha = env::var("EVO_GIT_SHA")
        .or_else(|_| env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "local".to_owned());
    let host = rustc_host(rustc_version);
    let aggregate = aggregate_classification(results);

    let mut markdown = format!(
        "# Link-time attribution research v0\n\n\
- git_sha: `{git_sha}`\n\
- platform: `{}-{}`\n\
- rustc_host: `{host}`\n\
- samples_per_arm: `{LINK_RESEARCH_SAMPLES}`\n\
- cases: `{}`\n\
- production_flags: `--edition={RUST_EDITION} --error-format=short -C opt-level={RUST_OPT_LEVEL} -C codegen-units={RUST_CODEGEN_UNITS}`\n\
- materiality_guide: `link >= {MATERIAL_LINK_MS:.1} ms AND >= {:.0}% of normal full rustc on both initial cases`\n\
- clearly_immaterial_guide: `link < {IMMATERIAL_LINK_MS:.1} ms OR < {:.0}% of normal full rustc on both initial cases`\n\
- aggregate_classification: `{aggregate}`\n\
- correctness: `PASS`\n\n\
| Case | Full median ms | Object median ms | Full-object rough ms | Instrumented full median ms | Link child median ms | Link/full | Wrapper overhead rough ms | Link invocations/sample | Full/Object/Instrumented bytes | Classification |\n\
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- | --- |\n",
        env::consts::OS,
        env::consts::ARCH,
        results.len(),
        MATERIAL_LINK_SHARE * 100.0,
        IMMATERIAL_LINK_SHARE * 100.0,
    );

    for result in results {
        let full = stats(&result.full);
        let object = stats(&result.object);
        let instrumented = stats(&result.instrumented_full);
        let link = stats(&result.link_child);
        let full_minus_object = full.median_ms - object.median_ms;
        let wrapper_overhead = instrumented.median_ms - full.median_ms;
        let invocation_range = format!(
            "{}-{}",
            result.linker_invocations.iter().copied().min().unwrap_or(0),
            result.linker_invocations.iter().copied().max().unwrap_or(0)
        );
        writeln!(
            markdown,
            "| {} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3}% | {:.3} | {} | {}/{}/{} | {} |",
            result.name,
            full.median_ms,
            object.median_ms,
            full_minus_object,
            instrumented.median_ms,
            link.median_ms,
            link.median_ms / full.median_ms * 100.0,
            wrapper_overhead,
            invocation_range,
            result.full_bytes,
            result.object_bytes,
            result.instrumented_bytes,
            case_classification(result),
        )
        .expect("link research Markdown should write");
    }

    markdown.push_str(
        "\n`full - object` is retained only as a rough supporting attribution signal. It is subtraction of separately sampled compiler modes and is not causal profiling. The primary linker signal is child wall time recorded inside a transparent PATH-shadow `cc` wrapper that forwards rustc's exact linker argv to the real PATH-resolved `cc`. The instrumented full-build arm exposes wrapper/process/logging overhead. `--print link-args` raw output is retained per case as debugging evidence and is not parsed as a stable format contract. No linker or production rustc setting is changed by this research harness.\n",
    );
    fs::write(out_dir.join("report.md"), markdown)
        .expect("link research Markdown report should be written");

    let mut json = format!(
        "{{\n  \"git_sha\": \"{git_sha}\",\n  \"platform\": \"{}-{}\",\n  \"rustc_host\": \"{host}\",\n  \"samples_per_arm\": {LINK_RESEARCH_SAMPLES},\n  \"aggregate_classification\": \"{aggregate}\",\n  \"correctness\": \"PASS\",\n  \"cases\": [\n",
        env::consts::OS,
        env::consts::ARCH,
    );
    for (index, result) in results.iter().enumerate() {
        let full = stats(&result.full);
        let object = stats(&result.object);
        let instrumented = stats(&result.instrumented_full);
        let link = stats(&result.link_child);
        let comma = if index + 1 == results.len() { "" } else { "," };
        write!(
            json,
            "    {{\n      \"name\": \"{}\",\n      \"classification\": \"{}\",\n      \"full_samples_ms\": [{}],\n      \"object_samples_ms\": [{}],\n      \"instrumented_full_samples_ms\": [{}],\n      \"link_child_samples_ms\": [{}],\n      \"full_median_ms\": {:.3},\n      \"object_median_ms\": {:.3},\n      \"instrumented_full_median_ms\": {:.3},\n      \"link_child_median_ms\": {:.3},\n      \"full_minus_object_rough_ms\": {:.3},\n      \"link_share_of_full\": {:.9},\n      \"wrapper_overhead_rough_ms\": {:.3},\n      \"linker_invocations\": [{}],\n      \"full_bytes\": {},\n      \"object_bytes\": {},\n      \"instrumented_bytes\": {}\n    }}{comma}\n",
            result.name,
            case_classification(result),
            samples_json(&result.full),
            samples_json(&result.object),
            samples_json(&result.instrumented_full),
            samples_json(&result.link_child),
            full.median_ms,
            object.median_ms,
            instrumented.median_ms,
            link.median_ms,
            full.median_ms - object.median_ms,
            link.median_ms / full.median_ms,
            instrumented.median_ms - full.median_ms,
            result
                .linker_invocations
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            result.full_bytes,
            result.object_bytes,
            result.instrumented_bytes,
        )
        .expect("link research JSON should write");
    }
    json.push_str("  ]\n}\n");
    fs::write(out_dir.join("report.json"), json)
        .expect("link research JSON report should be written");

    let mut csv = String::from("case,arm,index,elapsed_ms,linker_invocations\n");
    for result in results {
        for (arm, samples) in [
            ("full", &result.full),
            ("object", &result.object),
            ("instrumented-full", &result.instrumented_full),
            ("link-child", &result.link_child),
        ] {
            for (index, sample) in samples.iter().enumerate() {
                let invocations = if arm == "link-child" || arm == "instrumented-full" {
                    result.linker_invocations[index]
                } else {
                    0
                };
                writeln!(
                    csv,
                    "{},{arm},{},{:.3},{invocations}",
                    result.name,
                    index + 1,
                    sample.as_secs_f64() * 1_000.0
                )
                .expect("link research CSV should write");
            }
        }
    }
    fs::write(out_dir.join("raw-samples.csv"), csv)
        .expect("link research CSV report should be written");
    fs::write(out_dir.join("rustc-vV.txt"), rustc_version)
        .expect("rustc version evidence should be written");
    fs::write(out_dir.join("link-driver-version.txt"), linker_version)
        .expect("linker version evidence should be written");

    for result in results {
        let case_dir = out_dir.join(&result.name);
        fs::create_dir_all(&case_dir).expect("case evidence directory should be created");
        fs::write(case_dir.join("generated.rs"), &result.generated_rust)
            .expect("generated Rust evidence should be written");
        fs::write(
            case_dir.join("rustc-print-link-args.txt"),
            &result.link_args_probe,
        )
        .expect("rustc link args evidence should be written");
        fs::write(
            case_dir.join("wrapper-link-args.txt"),
            &result.wrapper_link_args,
        )
        .expect("wrapper link args evidence should be written");
    }
}

#[test]
#[ignore = "run explicitly on the controlled Ubuntu link-attribution runner"]
fn link_time_research_attributes_object_and_real_link_driver_cost() {
    assert_eq!(
        env::consts::OS,
        "linux",
        "link attribution slice is Ubuntu-only"
    );

    let dir = temp_dir("link-time-research");
    fs::create_dir_all(&dir).expect("link research directory should be created");
    let rustc = real_rustc();
    let version = rustc_version(&rustc);
    assert_eq!(
        rustc_host(&version),
        "x86_64-unknown-linux-gnu",
        "controlled link attribution expects the Ubuntu host target"
    );
    let real_cc = resolve_from_path("cc");
    let cc_version = Command::new(&real_cc)
        .arg("--version")
        .output()
        .expect("real cc version probe should run");
    assert_success(&cc_version, "real cc version probe");
    let wrapper = compile_linker_wrapper(&dir, &rustc);
    let wrapper_dir = wrapper
        .parent()
        .expect("wrapper should have a parent directory")
        .to_path_buf();
    let instrumented_path = prefixed_path(&wrapper_dir);
    let mut results = Vec::with_capacity(LINK_RESEARCH_CASES.len());

    for case_name in LINK_RESEARCH_CASES {
        let fixture = repo_root().join("benchmarks/cases").join(case_name);
        let source_bytes =
            fs::read(fixture.join("evolution.evo")).expect("fixture Evolution source should read");
        let fixture_stdin = fs::read(fixture.join("stdin.bin")).expect("fixture stdin should read");
        let expected_stdout =
            fs::read(fixture.join("expected.stdout")).expect("expected stdout should read");
        let case_dir = dir.join(case_name);
        fs::create_dir_all(&case_dir).expect("case directory should be created");
        let evo_path = case_dir.join("program.evo");
        fs::write(&evo_path, source_bytes).expect("fixture Evolution source should stage");
        let emitted = evo_command("emit-rust", &evo_path)
            .output()
            .expect("emit-rust should execute");
        assert_success(&emitted, "emit-rust for link research");
        assert!(
            !emitted.stdout.is_empty(),
            "generated Rust should not be empty"
        );
        let generated = case_dir.join("main.rs");
        fs::write(&generated, &emitted.stdout).expect("generated Rust should stage");

        let probe_binary = case_dir.join(format!("probe{}", env::consts::EXE_SUFFIX));
        let probe = production_rustc_command(&rustc, &generated)
            .arg("--print")
            .arg("link-args")
            .arg("-o")
            .arg(&probe_binary)
            .output()
            .expect("rustc link-args probe should execute");
        assert_success(&probe, "rustc --print link-args probe");
        run_binary(&probe_binary, &fixture_stdin, &expected_stdout);
        let mut link_args_probe = probe.stdout;
        link_args_probe.extend_from_slice(&probe.stderr);

        let prime_full = case_dir.join(format!("prime-full{}", env::consts::EXE_SUFFIX));
        let prime = production_rustc_command(&rustc, &generated)
            .arg("-o")
            .arg(&prime_full)
            .output()
            .expect("full prime should execute");
        assert_success(&prime, "full prime");
        run_binary(&prime_full, &fixture_stdin, &expected_stdout);

        let prime_object = case_dir.join("prime.o");
        let prime = production_rustc_command(&rustc, &generated)
            .arg("--emit=obj")
            .arg("-o")
            .arg(&prime_object)
            .output()
            .expect("object prime should execute");
        assert_success(&prime, "object prime");

        let mut full_samples = Vec::with_capacity(LINK_RESEARCH_SAMPLES);
        let mut object_samples = Vec::with_capacity(LINK_RESEARCH_SAMPLES);
        let mut instrumented_samples = Vec::with_capacity(LINK_RESEARCH_SAMPLES);
        let mut link_child_samples = Vec::with_capacity(LINK_RESEARCH_SAMPLES);
        let mut linker_invocations = Vec::with_capacity(LINK_RESEARCH_SAMPLES);
        let mut wrapper_link_args = String::new();
        let mut final_full = PathBuf::new();
        let mut final_object = PathBuf::new();
        let mut final_instrumented = PathBuf::new();

        for index in 0..LINK_RESEARCH_SAMPLES {
            let full_binary = case_dir.join(format!("full-{index}{}", env::consts::EXE_SUFFIX));
            let object = case_dir.join(format!("object-{index}.o"));
            let instrumented_binary =
                case_dir.join(format!("instrumented-{index}{}", env::consts::EXE_SUFFIX));
            let timing_file = case_dir.join(format!("link-timing-{index}.txt"));
            let args_file = case_dir.join(format!("link-args-{index}.txt"));

            let (full, elapsed) = timed_output(
                production_rustc_command(&rustc, &generated)
                    .arg("-o")
                    .arg(&full_binary),
            );
            assert_success(&full, "normal full rustc");
            run_binary(&full_binary, &fixture_stdin, &expected_stdout);
            full_samples.push(elapsed);

            let (object_output, elapsed) = timed_output(
                production_rustc_command(&rustc, &generated)
                    .arg("--emit=obj")
                    .arg("-o")
                    .arg(&object),
            );
            assert_success(&object_output, "rustc object emission");
            object_samples.push(elapsed);

            let (instrumented, elapsed) = timed_output(
                production_rustc_command(&rustc, &generated)
                    .arg("-o")
                    .arg(&instrumented_binary)
                    .env("PATH", &instrumented_path)
                    .env("EVO_TEST_REAL_LINKER", &real_cc)
                    .env("EVO_TEST_LINK_TIMINGS", &timing_file)
                    .env("EVO_TEST_LINK_ARGS", &args_file),
            );
            assert_success(&instrumented, "instrumented full rustc");
            run_binary(&instrumented_binary, &fixture_stdin, &expected_stdout);
            instrumented_samples.push(elapsed);
            let (link_elapsed, invocation_count) = read_link_child_sample(&timing_file);
            link_child_samples.push(link_elapsed);
            linker_invocations.push(invocation_count);
            writeln!(
                wrapper_link_args,
                "# sample {}\n{}",
                index + 1,
                fs::read_to_string(&args_file).expect("wrapper link args should read")
            )
            .expect("wrapper args aggregate should write");

            final_full = full_binary;
            final_object = object;
            final_instrumented = instrumented_binary;
        }

        results.push(LinkCaseResult {
            name: case_name.to_owned(),
            full: full_samples,
            object: object_samples,
            instrumented_full: instrumented_samples,
            link_child: link_child_samples,
            linker_invocations,
            full_bytes: fs::metadata(&final_full)
                .expect("full binary metadata should read")
                .len(),
            object_bytes: fs::metadata(&final_object)
                .expect("object metadata should read")
                .len(),
            instrumented_bytes: fs::metadata(&final_instrumented)
                .expect("instrumented binary metadata should read")
                .len(),
            generated_rust: emitted.stdout,
            link_args_probe,
            wrapper_link_args,
        });
    }

    let out_dir = env::var_os("EVO_LINK_TIME_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("report"));
    let mut linker_version = cc_version.stdout;
    linker_version.extend_from_slice(&cc_version.stderr);
    write_link_research_report(&out_dir, &results, &version, &linker_version);

    println!(
        "link_time_research_classification={}",
        aggregate_classification(&results)
    );
    for result in &results {
        let full = stats(&result.full);
        let object = stats(&result.object);
        let instrumented = stats(&result.instrumented_full);
        let link = stats(&result.link_child);
        println!(
            "link_time_case={} classification={} full_ms={:.3} object_ms={:.3} instrumented_ms={:.3} link_ms={:.3} link_share={:.6}",
            result.name,
            case_classification(result),
            full.median_ms,
            object.median_ms,
            instrumented.median_ms,
            link.median_ms,
            link.median_ms / full.median_ms,
        );
    }

    let _ = fs::remove_dir_all(dir);
}
