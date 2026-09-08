use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const FRONTEND_SAMPLES: usize = 9;
const BUILD_SAMPLES: usize = 5;
const RUST_EDITION: &str = "2024";
const RUST_OPT_LEVEL: &str = "3";
const RUST_CODEGEN_UNITS: &str = "1";
const EDIT_FROM: &str = "sum = sum + value";
const EDIT_TO: &str = "sum = value + sum";

#[derive(Debug, Clone, Copy)]
struct SampleStats {
    median_ms: f64,
    min_ms: f64,
    max_ms: f64,
    p95_ms: f64,
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    env::temp_dir().join(format!(
        "evo-build-latency-{label}-{}-{nanos}",
        process::id()
    ))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture_dir() -> PathBuf {
    repo_root().join("benchmarks/cases/enums-v0")
}

fn real_rustc() -> OsString {
    env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"))
}

fn rustc_wrapper_source() -> &'static str {
    r#"use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write as _;
use std::process::{self, Command};

fn increment_compile_count() {
    let path = env::var_os("EVO_TEST_RUSTC_COUNT").expect("counter path must be set");
    let current = fs::read_to_string(&path)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0);
    fs::write(path, (current + 1).to_string()).expect("counter must be writable");
}

fn main() {
    let real = env::var_os("EVO_TEST_REAL_RUSTC").expect("real rustc must be set");
    let args = env::args_os().skip(1).collect::<Vec<OsString>>();
    let version_probe = args.len() == 1 && args[0] == OsStr::new("-vV");

    if !version_probe {
        increment_compile_count();
    }

    let output = Command::new(real)
        .args(&args)
        .output()
        .expect("real rustc must execute");

    std::io::stdout()
        .write_all(&output.stdout)
        .expect("rustc stdout must forward");
    std::io::stderr()
        .write_all(&output.stderr)
        .expect("rustc stderr must forward");
    process::exit(output.status.code().unwrap_or(1));
}
"#
}

fn compile_rustc_wrapper(dir: &Path, rustc: &OsStr) -> PathBuf {
    let source = dir.join("rustc-wrapper.rs");
    let binary = dir.join(format!("rustc-wrapper{}", env::consts::EXE_SUFFIX));
    fs::write(&source, rustc_wrapper_source()).expect("wrapper source should be written");

    let output = Command::new(rustc)
        .arg(&source)
        .arg("--edition=2024")
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("wrapper rustc should run");
    assert!(
        output.status.success(),
        "wrapper compile failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    binary
}

fn compile_count(counter: &Path) -> u64 {
    fs::read_to_string(counter)
        .expect("counter should exist")
        .trim()
        .parse()
        .expect("counter should contain an integer")
}

fn build_command(
    source: &Path,
    output: &Path,
    wrapper: &Path,
    rustc: &OsStr,
    counter: &Path,
) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_evo"));
    command
        .arg("build")
        .arg(source)
        .arg(output)
        .env("RUSTC", wrapper)
        .env("EVO_TEST_REAL_RUSTC", rustc)
        .env("EVO_TEST_RUSTC_COUNT", counter);
    command
}

fn evo_command(subcommand: &str, source: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_evo"));
    command.arg(subcommand).arg(source);
    command
}

fn timed_output(command: &mut Command) -> (Output, Duration) {
    let start = Instant::now();
    let output = command.output().expect("command should execute");
    (output, start.elapsed())
}

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_binary(binary: &Path, stdin: &[u8], expected_stdout: &[u8]) {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("built binary should execute");
    child
        .stdin
        .take()
        .expect("stdin pipe should exist")
        .write_all(stdin)
        .expect("fixture stdin should write");
    let output = child
        .wait_with_output()
        .expect("built binary should finish");
    assert_success(&output, "built binary");
    assert_eq!(output.stdout, expected_stdout);
}

fn stats(samples: &[Duration]) -> SampleStats {
    assert!(!samples.is_empty(), "samples must not be empty");
    let mut values = samples
        .iter()
        .map(|sample| sample.as_secs_f64() * 1_000.0)
        .collect::<Vec<_>>();
    values.sort_by(|left, right| left.total_cmp(right));

    let middle = values.len() / 2;
    let median_ms = if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    };
    let p95_rank = ((values.len() as f64) * 0.95).ceil() as usize;

    SampleStats {
        median_ms,
        min_ms: values[0],
        max_ms: values[values.len() - 1],
        p95_ms: values[p95_rank.saturating_sub(1)],
    }
}

fn samples_json(samples: &[Duration]) -> String {
    samples
        .iter()
        .map(|sample| format!("{:.3}", sample.as_secs_f64() * 1_000.0))
        .collect::<Vec<_>>()
        .join(", ")
}

fn csv_rows(kind: &str, samples: &[Duration], rustc_delta: u64, notes: &str) -> String {
    samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            format!(
                "{kind},{},{:.3},{rustc_delta},{notes}\n",
                index + 1,
                sample.as_secs_f64() * 1_000.0
            )
        })
        .collect()
}

fn rustc_version(rustc: &OsStr) -> String {
    let output = Command::new(rustc)
        .arg("-vV")
        .output()
        .expect("rustc -vV should execute");
    assert_success(&output, "rustc -vV");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn rustc_host(version: &str) -> &str {
    version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .unwrap_or("unknown")
}

struct ReportInput<'a> {
    check: &'a [Duration],
    emit: &'a [Duration],
    cold: &'a [Duration],
    warm: &'a [Duration],
    edit: &'a [Duration],
    direct: &'a [Duration],
    cold_compile_count: u64,
    warm_compile_count: u64,
    edit_compile_count: u64,
    direct_compile_count: u64,
    generated_rust: &'a [u8],
    fixture_source: &'a [u8],
    fixture_stdin: &'a [u8],
    expected_stdout: &'a [u8],
    rustc_version: &'a str,
}

fn write_report(out_dir: &Path, input: &ReportInput<'_>) {
    fs::create_dir_all(out_dir).expect("build latency output directory should be created");

    let check = stats(input.check);
    let emit = stats(input.emit);
    let cold = stats(input.cold);
    let warm = stats(input.warm);
    let edit = stats(input.edit);
    let direct = stats(input.direct);
    let cold_outside_rustc_ms = cold.median_ms - direct.median_ms;
    let warm_outside_rustc_ms = warm.median_ms - direct.median_ms;
    let edit_outside_rustc_ms = edit.median_ms - direct.median_ms;
    let git_sha = env::var("EVO_GIT_SHA")
        .or_else(|_| env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "local".to_owned());
    let host = rustc_host(input.rustc_version);
    let flags = format!(
        "--edition={RUST_EDITION} --error-format=short -C opt-level={RUST_OPT_LEVEL} -C codegen-units={RUST_CODEGEN_UNITS}"
    );

    let markdown = format!(
        "# Build latency baseline v0\n\n\
- git_sha: `{git_sha}`\n\
- platform: `{}-{}`\n\
- rustc_host: `{host}`\n\
- rustc_flags: `{flags}`\n\
- fixture: `benchmarks/cases/enums-v0/evolution.evo`\n\
- frontend_check_ms median/min/max/p95: `{:.3}/{:.3}/{:.3}/{:.3}`\n\
- emit_rust_ms median/min/max/p95: `{:.3}/{:.3}/{:.3}/{:.3}`\n\
- cold_build_ms median/min/max/p95: `{:.3}/{:.3}/{:.3}/{:.3}`\n\
- warm_unchanged_build_ms median/min/max/p95: `{:.3}/{:.3}/{:.3}/{:.3}`\n\
- edit_build_ms median/min/max/p95: `{:.3}/{:.3}/{:.3}/{:.3}`\n\
- direct_rustc_ms median/min/max/p95: `{:.3}/{:.3}/{:.3}/{:.3}`\n\
- cold_build_minus_direct_rustc_ms: `{cold_outside_rustc_ms:.3}`\n\
- warm_build_minus_direct_rustc_ms: `{warm_outside_rustc_ms:.3}`\n\
- edit_build_minus_direct_rustc_ms: `{edit_outside_rustc_ms:.3}`\n\
- cold measured rustc compile invocations: `{}`\n\
- warm measured rustc compile invocations: `{}`\n\
- edit measured rustc compile invocations: `{}`\n\
- direct rustc measured compilations: `{}`\n\n\
`build - direct rustc` is retained as a rough attribution signal only; subtraction of separately sampled medians is not causal profiling. All measured native compilation classes use the same rustc-counting wrapper, so instrumentation process overhead is present on both sides. Cold samples use fresh output paths. Warm samples reuse one primed source/output pair. Each edit sample primes a baseline output, changes one generated-Rust-affecting but result-preserving source line, then rebuilds. Direct rustc compiles the exact emitted Rust with the same successful-build flags.\n",
        env::consts::OS,
        env::consts::ARCH,
        check.median_ms,
        check.min_ms,
        check.max_ms,
        check.p95_ms,
        emit.median_ms,
        emit.min_ms,
        emit.max_ms,
        emit.p95_ms,
        cold.median_ms,
        cold.min_ms,
        cold.max_ms,
        cold.p95_ms,
        warm.median_ms,
        warm.min_ms,
        warm.max_ms,
        warm.p95_ms,
        edit.median_ms,
        edit.min_ms,
        edit.max_ms,
        edit.p95_ms,
        direct.median_ms,
        direct.min_ms,
        direct.max_ms,
        direct.p95_ms,
        input.cold_compile_count,
        input.warm_compile_count,
        input.edit_compile_count,
        input.direct_compile_count,
    );
    fs::write(out_dir.join("report.md"), markdown).expect("Markdown report should be written");

    let json = format!(
        "{{\n  \"git_sha\": \"{git_sha}\",\n  \"platform\": \"{}-{}\",\n  \"rustc_host\": \"{host}\",\n  \"rustc_flags\": \"{flags}\",\n  \"fixture\": \"benchmarks/cases/enums-v0/evolution.evo\",\n  \"check_samples_ms\": [{}],\n  \"emit_samples_ms\": [{}],\n  \"cold_build_samples_ms\": [{}],\n  \"warm_build_samples_ms\": [{}],\n  \"edit_build_samples_ms\": [{}],\n  \"direct_rustc_samples_ms\": [{}],\n  \"check_median_ms\": {:.3},\n  \"emit_median_ms\": {:.3},\n  \"cold_build_median_ms\": {:.3},\n  \"warm_build_median_ms\": {:.3},\n  \"edit_build_median_ms\": {:.3},\n  \"direct_rustc_median_ms\": {:.3},\n  \"cold_build_minus_direct_rustc_ms\": {cold_outside_rustc_ms:.3},\n  \"warm_build_minus_direct_rustc_ms\": {warm_outside_rustc_ms:.3},\n  \"edit_build_minus_direct_rustc_ms\": {edit_outside_rustc_ms:.3},\n  \"cold_rustc_compile_count\": {},\n  \"warm_rustc_compile_count\": {},\n  \"edit_rustc_compile_count\": {},\n  \"direct_rustc_compile_count\": {}\n}}\n",
        env::consts::OS,
        env::consts::ARCH,
        samples_json(input.check),
        samples_json(input.emit),
        samples_json(input.cold),
        samples_json(input.warm),
        samples_json(input.edit),
        samples_json(input.direct),
        check.median_ms,
        emit.median_ms,
        cold.median_ms,
        warm.median_ms,
        edit.median_ms,
        direct.median_ms,
        input.cold_compile_count,
        input.warm_compile_count,
        input.edit_compile_count,
        input.direct_compile_count,
    );
    fs::write(out_dir.join("report.json"), json).expect("JSON report should be written");

    let mut csv = String::from("kind,index,elapsed_ms,rustc_compile_delta,notes\n");
    csv.push_str(&csv_rows("check", input.check, 0, "frontend-only"));
    csv.push_str(&csv_rows("emit-rust", input.emit, 0, "frontend-codegen"));
    csv.push_str(&csv_rows("cold-build", input.cold, 1, "fresh-output"));
    csv.push_str(&csv_rows("warm-build", input.warm, 1, "same-source-output"));
    csv.push_str(&csv_rows(
        "edit-build",
        input.edit,
        1,
        "generated-rust-changing-edit",
    ));
    csv.push_str(&csv_rows(
        "direct-rustc",
        input.direct,
        1,
        "exact-emitted-rust",
    ));
    fs::write(out_dir.join("raw-samples.csv"), csv).expect("CSV report should be written");

    fs::write(out_dir.join("generated.rs"), input.generated_rust)
        .expect("generated Rust evidence should be written");
    fs::write(out_dir.join("evolution.evo"), input.fixture_source)
        .expect("fixture source evidence should be written");
    fs::write(out_dir.join("stdin.bin"), input.fixture_stdin)
        .expect("fixture stdin evidence should be written");
    fs::write(out_dir.join("expected.stdout"), input.expected_stdout)
        .expect("fixture expected output evidence should be written");
    fs::write(out_dir.join("rustc-vV.txt"), input.rustc_version)
        .expect("rustc version evidence should be written");
}

fn direct_rustc_command(
    wrapper: &Path,
    rustc: &OsStr,
    counter: &Path,
    generated: &Path,
    output: &Path,
) -> Command {
    let mut command = Command::new(wrapper);
    command
        .arg(generated)
        .arg(format!("--edition={RUST_EDITION}"))
        .arg("--error-format=short")
        .arg("-C")
        .arg(format!("opt-level={RUST_OPT_LEVEL}"))
        .arg("-C")
        .arg(format!("codegen-units={RUST_CODEGEN_UNITS}"))
        .arg("-o")
        .arg(output)
        .env("EVO_TEST_REAL_RUSTC", rustc)
        .env("EVO_TEST_RUSTC_COUNT", counter);
    command
}

#[test]
fn sample_stats_are_deterministic() {
    let samples = [
        Duration::from_millis(5),
        Duration::from_millis(1),
        Duration::from_millis(3),
        Duration::from_millis(2),
        Duration::from_millis(4),
    ];
    let summary = stats(&samples);
    assert_eq!(summary.median_ms, 3.0);
    assert_eq!(summary.min_ms, 1.0);
    assert_eq!(summary.max_ms, 5.0);
    assert_eq!(summary.p95_ms, 5.0);
}

#[test]
fn report_files_record_samples_and_compile_counts() {
    let dir = temp_dir("report-format");
    let samples = [Duration::from_millis(10), Duration::from_millis(20)];
    write_report(
        &dir,
        &ReportInput {
            check: &samples,
            emit: &samples,
            cold: &samples,
            warm: &samples,
            edit: &samples,
            direct: &samples,
            cold_compile_count: 2,
            warm_compile_count: 2,
            edit_compile_count: 2,
            direct_compile_count: 2,
            generated_rust: b"fn main() {}\n",
            fixture_source: b"print 7\n",
            fixture_stdin: b"",
            expected_stdout: b"7\n",
            rustc_version: "rustc 1.98.0\nhost: test-host\n",
        },
    );

    let json = fs::read_to_string(dir.join("report.json")).expect("JSON report should read");
    assert!(json.contains("\"cold_rustc_compile_count\": 2"));
    assert!(json.contains("\"direct_rustc_compile_count\": 2"));
    assert!(json.contains("\"rustc_host\": \"test-host\""));

    let csv = fs::read_to_string(dir.join("raw-samples.csv")).expect("CSV report should read");
    assert!(csv.contains("cold-build,1,10.000,1,fresh-output"));
    assert!(csv.contains("direct-rustc,2,20.000,1,exact-emitted-rust"));
    assert_eq!(
        fs::read(dir.join("generated.rs")).expect("generated Rust should read"),
        b"fn main() {}\n",
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rustc_wrapper_counts_compiles_but_not_version_probe() {
    let dir = temp_dir("wrapper-count");
    fs::create_dir_all(&dir).expect("test directory should be created");
    let counter = dir.join("rustc-count.txt");
    fs::write(&counter, "0").expect("counter should be initialized");
    let rustc = real_rustc();
    let wrapper = compile_rustc_wrapper(&dir, &rustc);

    let version = Command::new(&wrapper)
        .arg("-vV")
        .env("EVO_TEST_REAL_RUSTC", &rustc)
        .env("EVO_TEST_RUSTC_COUNT", &counter)
        .output()
        .expect("wrapper version probe should run");
    assert_success(&version, "wrapper version probe");
    assert_eq!(compile_count(&counter), 0);

    let source = dir.join("tiny.rs");
    let binary = dir.join(format!("tiny{}", env::consts::EXE_SUFFIX));
    fs::write(&source, "fn main() {}\n").expect("tiny source should be written");
    let compile = Command::new(&wrapper)
        .arg(&source)
        .arg("--edition=2024")
        .arg("-o")
        .arg(&binary)
        .env("EVO_TEST_REAL_RUSTC", &rustc)
        .env("EVO_TEST_RUSTC_COUNT", &counter)
        .output()
        .expect("wrapper compile should run");
    assert_success(&compile, "wrapper compile");
    assert_eq!(compile_count(&counter), 1);

    let _ = fs::remove_dir_all(dir);
}

#[test]
#[ignore = "run explicitly on the controlled Ubuntu CI evidence runner"]
fn build_latency_baseline_reports_cold_warm_edit_and_rustc_attribution() {
    let dir = temp_dir("evidence");
    fs::create_dir_all(&dir).expect("test directory should be created");

    let fixture = fixture_dir();
    let fixture_source =
        fs::read(fixture.join("evolution.evo")).expect("fixture source should read");
    let fixture_stdin = fs::read(fixture.join("stdin.bin")).expect("fixture stdin should read");
    let expected_stdout =
        fs::read(fixture.join("expected.stdout")).expect("fixture expected stdout should read");
    let reference_rust =
        fs::read(fixture.join("reference.rs")).expect("fixture reference Rust should read");

    let source = dir.join("program.evo");
    fs::write(&source, &fixture_source).expect("working fixture source should be written");
    let counter = dir.join("rustc-count.txt");
    fs::write(&counter, "0").expect("counter should be initialized");
    let rustc = real_rustc();
    let rustc_version = rustc_version(&rustc);
    let wrapper = compile_rustc_wrapper(&dir, &rustc);

    let check_prime = evo_command("check", &source)
        .output()
        .expect("check prime should execute");
    assert_success(&check_prime, "evo check prime");
    let emit_prime = evo_command("emit-rust", &source)
        .output()
        .expect("emit-rust prime should execute");
    assert_success(&emit_prime, "evo emit-rust prime");

    let mut check_samples = Vec::with_capacity(FRONTEND_SAMPLES);
    let mut emit_samples = Vec::with_capacity(FRONTEND_SAMPLES);
    for _ in 0..FRONTEND_SAMPLES {
        let (check, elapsed) = timed_output(&mut evo_command("check", &source));
        assert_success(&check, "evo check");
        assert_eq!(String::from_utf8_lossy(&check.stdout).trim(), "ok");
        check_samples.push(elapsed);

        let (emit, elapsed) = timed_output(&mut evo_command("emit-rust", &source));
        assert_success(&emit, "evo emit-rust");
        assert!(
            !emit.stdout.is_empty(),
            "emit-rust must produce Rust source"
        );
        emit_samples.push(elapsed);
    }

    let generated = evo_command("emit-rust", &source)
        .output()
        .expect("emit-rust should execute");
    assert_success(&generated, "evo emit-rust identity");
    assert_eq!(generated.stdout, reference_rust);
    let generated_path = dir.join("generated.rs");
    fs::write(&generated_path, &generated.stdout).expect("generated Rust should be written");

    let mut cold_samples = Vec::with_capacity(BUILD_SAMPLES);
    let mut cold_compile_count = 0;
    for index in 0..BUILD_SAMPLES {
        let output_path = dir
            .join(format!("cold-{index}"))
            .join(format!("program{}", env::consts::EXE_SUFFIX));
        let before = compile_count(&counter);
        let (build, elapsed) = timed_output(&mut build_command(
            &source,
            &output_path,
            &wrapper,
            &rustc,
            &counter,
        ));
        assert_success(&build, "cold evo build");
        let compile_delta = compile_count(&counter) - before;
        assert_eq!(compile_delta, 1);
        cold_compile_count += compile_delta;
        run_binary(&output_path, &fixture_stdin, &expected_stdout);
        cold_samples.push(elapsed);
    }

    let warm_output = dir
        .join("warm")
        .join(format!("program{}", env::consts::EXE_SUFFIX));
    let warm_prime = build_command(&source, &warm_output, &wrapper, &rustc, &counter)
        .output()
        .expect("warm prime should execute");
    assert_success(&warm_prime, "warm build prime");
    run_binary(&warm_output, &fixture_stdin, &expected_stdout);

    let mut warm_compile_count = 0;
    let mut warm_samples = Vec::with_capacity(BUILD_SAMPLES);
    for _ in 0..BUILD_SAMPLES {
        let before = compile_count(&counter);
        let (build, elapsed) = timed_output(&mut build_command(
            &source,
            &warm_output,
            &wrapper,
            &rustc,
            &counter,
        ));
        assert_success(&build, "warm evo build");
        let compile_delta = compile_count(&counter) - before;
        assert_eq!(compile_delta, 1);
        warm_compile_count += compile_delta;
        run_binary(&warm_output, &fixture_stdin, &expected_stdout);
        warm_samples.push(elapsed);
    }

    let baseline_source =
        String::from_utf8(fixture_source.clone()).expect("fixture should be UTF-8");
    assert!(baseline_source.matches(EDIT_FROM).count() >= 2);
    let edited_source = baseline_source.replacen(EDIT_FROM, EDIT_TO, 1);
    assert_ne!(baseline_source, edited_source);

    let mut edit_compile_count = 0;
    let mut edit_samples = Vec::with_capacity(BUILD_SAMPLES);
    for index in 0..BUILD_SAMPLES {
        let edit_dir = dir.join(format!("edit-{index}"));
        fs::create_dir_all(&edit_dir).expect("edit sample directory should be created");
        let edit_source_path = edit_dir.join("program.evo");
        let edit_output = edit_dir.join(format!("program{}", env::consts::EXE_SUFFIX));
        fs::write(&edit_source_path, &baseline_source).expect("baseline edit source should write");

        let prime = build_command(&edit_source_path, &edit_output, &wrapper, &rustc, &counter)
            .output()
            .expect("edit prime should execute");
        assert_success(&prime, "edit build prime");
        fs::write(&edit_source_path, &edited_source).expect("edited source should write");

        let before = compile_count(&counter);
        let (build, elapsed) = timed_output(&mut build_command(
            &edit_source_path,
            &edit_output,
            &wrapper,
            &rustc,
            &counter,
        ));
        assert_success(&build, "edited evo build");
        let compile_delta = compile_count(&counter) - before;
        assert_eq!(compile_delta, 1);
        edit_compile_count += compile_delta;
        run_binary(&edit_output, &fixture_stdin, &expected_stdout);
        edit_samples.push(elapsed);
    }

    let direct_prime_output = dir.join(format!("direct-prime{}", env::consts::EXE_SUFFIX));
    let direct_prime = direct_rustc_command(
        &wrapper,
        &rustc,
        &counter,
        &generated_path,
        &direct_prime_output,
    )
    .output()
    .expect("direct rustc prime should execute");
    assert_success(&direct_prime, "direct rustc prime");
    run_binary(&direct_prime_output, &fixture_stdin, &expected_stdout);

    let direct_compile_start = compile_count(&counter);
    let mut direct_samples = Vec::with_capacity(BUILD_SAMPLES);
    for index in 0..BUILD_SAMPLES {
        let output_path = dir.join(format!("direct-{index}{}", env::consts::EXE_SUFFIX));
        let (compile, elapsed) = timed_output(&mut direct_rustc_command(
            &wrapper,
            &rustc,
            &counter,
            &generated_path,
            &output_path,
        ));
        assert_success(&compile, "direct rustc compile");
        run_binary(&output_path, &fixture_stdin, &expected_stdout);
        direct_samples.push(elapsed);
    }
    let direct_compile_count = compile_count(&counter) - direct_compile_start;

    assert_eq!(cold_compile_count, BUILD_SAMPLES as u64);
    assert_eq!(warm_compile_count, BUILD_SAMPLES as u64);
    assert_eq!(edit_compile_count, BUILD_SAMPLES as u64);
    assert_eq!(direct_compile_count, BUILD_SAMPLES as u64);

    let out_dir = env::var_os("EVO_BUILD_LATENCY_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("build-latency-report"));
    write_report(
        &out_dir,
        &ReportInput {
            check: &check_samples,
            emit: &emit_samples,
            cold: &cold_samples,
            warm: &warm_samples,
            edit: &edit_samples,
            direct: &direct_samples,
            cold_compile_count,
            warm_compile_count,
            edit_compile_count,
            direct_compile_count,
            generated_rust: &generated.stdout,
            fixture_source: &fixture_source,
            fixture_stdin: &fixture_stdin,
            expected_stdout: &expected_stdout,
            rustc_version: &rustc_version,
        },
    );

    let cold = stats(&cold_samples);
    let warm = stats(&warm_samples);
    let edit = stats(&edit_samples);
    let direct = stats(&direct_samples);
    println!("cold_build_ms={:.3}", cold.median_ms);
    println!("warm_build_ms={:.3}", warm.median_ms);
    println!("edit_build_ms={:.3}", edit.median_ms);
    println!("direct_rustc_ms={:.3}", direct.median_ms);
    println!("cold_rustc_compile_count={cold_compile_count}");
    println!("warm_rustc_compile_count={warm_compile_count}");
    println!("edit_rustc_compile_count={edit_compile_count}");
    println!("direct_rustc_compile_count={direct_compile_count}");

    let _ = fs::remove_dir_all(dir);
}
