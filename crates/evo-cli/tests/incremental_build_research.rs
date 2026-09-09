use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SAMPLES: usize = 7;
const RUST_EDITION: &str = "2024";
const RUST_OPT_LEVEL: &str = "3";
const RUST_CODEGEN_UNITS: &str = "1";
const EDIT_FROM: &str = "sum = sum + value";
const EDIT_TO: &str = "sum = value + sum";
const ACCEPTED_76_EDIT_BASELINE_MS: f64 = 95.515;

#[derive(Debug, Clone, Copy)]
struct SampleStats {
    median_ms: f64,
    min_ms: f64,
    max_ms: f64,
    p95_ms: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct DirStats {
    files: u64,
    bytes: u64,
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    env::temp_dir().join(format!(
        "evo-incremental-research-{label}-{}-{nanos}",
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
    assert_success(&output, "wrapper compile");
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
        .arg("--no-cache")
        .env("RUSTC", wrapper)
        .env("EVO_TEST_REAL_RUSTC", rustc)
        .env("EVO_TEST_RUSTC_COUNT", counter);
    command
}

fn emit_command(source: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_evo"));
    command.arg("emit-rust").arg(source);
    command
}

fn direct_rustc_command(
    wrapper: &Path,
    rustc: &OsStr,
    counter: &Path,
    generated: &Path,
    output: &Path,
    incremental: Option<&Path>,
) -> Command {
    let mut command = Command::new(wrapper);
    command
        .arg(generated)
        .arg(format!("--edition={RUST_EDITION}"))
        .arg("--error-format=short")
        .arg("-C")
        .arg(format!("opt-level={RUST_OPT_LEVEL}"))
        .arg("-C")
        .arg(format!("codegen-units={RUST_CODEGEN_UNITS}"));
    if let Some(incremental) = incremental {
        command
            .arg("-C")
            .arg(format!("incremental={}", incremental.display()));
    }
    command
        .arg("-o")
        .arg(output)
        .env("EVO_TEST_REAL_RUSTC", rustc)
        .env("EVO_TEST_RUSTC_COUNT", counter);
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

fn u64_json(values: impl Iterator<Item = u64>) -> String {
    values.map(|value| value.to_string()).collect::<Vec<_>>().join(", ")
}

fn rustc_text(rustc: &OsStr, args: &[&str], label: &str) -> String {
    let output = Command::new(rustc)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("{label} should execute: {error}"));
    assert_success(&output, label);
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn rustc_host(version: &str) -> &str {
    version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .unwrap_or("unknown")
}

fn dir_stats(path: &Path) -> DirStats {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return DirStats::default();
    };
    if metadata.file_type().is_symlink() {
        return DirStats::default();
    }
    if metadata.is_file() {
        return DirStats {
            files: 1,
            bytes: metadata.len(),
        };
    }
    if !metadata.is_dir() {
        return DirStats::default();
    }

    let mut total = DirStats::default();
    for entry in fs::read_dir(path).expect("research state directory should be readable") {
        let entry = entry.expect("research state entry should be readable");
        let child = dir_stats(&entry.path());
        total.files += child.files;
        total.bytes += child.bytes;
    }
    total
}

struct ReportInput<'a> {
    evo_prime: &'a [Duration],
    evo_edit: &'a [Duration],
    stable_prime: &'a [Duration],
    stable_edit: &'a [Duration],
    incremental_prime: &'a [Duration],
    incremental_edit: &'a [Duration],
    incremental_state_after_prime: &'a [DirStats],
    incremental_state_after_edit: &'a [DirStats],
    baseline_generated: &'a [u8],
    edited_generated: &'a [u8],
    baseline_source: &'a [u8],
    edited_source: &'a [u8],
    fixture_stdin: &'a [u8],
    expected_stdout: &'a [u8],
    rustc_version: &'a str,
    rustc_c_help: &'a str,
}

fn push_csv_rows(
    csv: &mut String,
    arm: &str,
    phase: &str,
    samples: &[Duration],
    state: Option<&[DirStats]>,
    notes: &str,
) {
    for (index, sample) in samples.iter().enumerate() {
        let state = state.map_or(DirStats::default(), |states| states[index]);
        csv.push_str(&format!(
            "{arm},{phase},{},{:.3},1,{},{},{notes}\n",
            index + 1,
            sample.as_secs_f64() * 1_000.0,
            state.files,
            state.bytes,
        ));
    }
}

fn write_report(out_dir: &Path, input: &ReportInput<'_>) {
    fs::create_dir_all(out_dir).expect("incremental research output directory should be created");

    let evo_prime = stats(input.evo_prime);
    let evo_edit = stats(input.evo_edit);
    let stable_prime = stats(input.stable_prime);
    let stable_edit = stats(input.stable_edit);
    let incremental_prime = stats(input.incremental_prime);
    let incremental_edit = stats(input.incremental_edit);
    let stable_to_incremental_speedup = stable_edit.median_ms / incremental_edit.median_ms;
    let evo_to_incremental_speedup = evo_edit.median_ms / incremental_edit.median_ms;
    let accepted_76_to_incremental_speedup =
        ACCEPTED_76_EDIT_BASELINE_MS / incremental_edit.median_ms;
    let git_sha = env::var("EVO_GIT_SHA")
        .or_else(|_| env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "local".to_owned());
    let host = rustc_host(input.rustc_version);
    let base_flags = format!(
        "--edition={RUST_EDITION} --error-format=short -C opt-level={RUST_OPT_LEVEL} -C codegen-units={RUST_CODEGEN_UNITS}"
    );

    let markdown = format!(
        "# Changed-source incremental build research v0\n\n\
- git_sha: `{git_sha}`\n\
- platform: `{}-{}`\n\
- rustc_host: `{host}`\n\
- fixture: `benchmarks/cases/enums-v0/evolution.evo`\n\
- samples_per_phase: `{SAMPLES}`\n\
- accepted_76_edit_baseline_ms: `{ACCEPTED_76_EDIT_BASELINE_MS:.3}`\n\
- base_rustc_flags: `{base_flags}`\n\
- incremental_delta: `-C incremental=<per-sample-session-dir>`\n\n\
| Arm | Prime median/min/max/p95 ms | Edited median/min/max/p95 ms |\n\
| --- | ---: | ---: |\n\
| current `evo build --no-cache` fresh-workdir | {:.3}/{:.3}/{:.3}/{:.3} | {:.3}/{:.3}/{:.3}/{:.3} |\n\
| stable generated path, direct rustc, no incremental state | {:.3}/{:.3}/{:.3}/{:.3} | {:.3}/{:.3}/{:.3}/{:.3} |\n\
| stable generated path, direct rustc, persistent incremental state | {:.3}/{:.3}/{:.3}/{:.3} | {:.3}/{:.3}/{:.3}/{:.3} |\n\n\
- stable-no-incremental -> incremental edited speedup: `{stable_to_incremental_speedup:.3}x`\n\
- current-evo-edit -> incremental edited speedup: `{evo_to_incremental_speedup:.3}x`\n\
- accepted-#76-edit -> incremental edited speedup: `{accepted_76_to_incremental_speedup:.3}x`\n\n\
Every measured prime and edited build invokes rustc exactly once through the counting wrapper and every produced native binary must emit the committed expected stdout. The incremental arm keeps `codegen-units=1`; no production `evo build`, build-cache, run-cache, language, codegen or runtime behavior is changed by this research harness. Each sample gets a fresh incremental session directory, primes baseline generated Rust, then edits the same stable `main.rs` path and reuses only that sample's compiler state. Unfavorable timing samples are retained.\n",
        env::consts::OS,
        env::consts::ARCH,
        evo_prime.median_ms,
        evo_prime.min_ms,
        evo_prime.max_ms,
        evo_prime.p95_ms,
        evo_edit.median_ms,
        evo_edit.min_ms,
        evo_edit.max_ms,
        evo_edit.p95_ms,
        stable_prime.median_ms,
        stable_prime.min_ms,
        stable_prime.max_ms,
        stable_prime.p95_ms,
        stable_edit.median_ms,
        stable_edit.min_ms,
        stable_edit.max_ms,
        stable_edit.p95_ms,
        incremental_prime.median_ms,
        incremental_prime.min_ms,
        incremental_prime.max_ms,
        incremental_prime.p95_ms,
        incremental_edit.median_ms,
        incremental_edit.min_ms,
        incremental_edit.max_ms,
        incremental_edit.p95_ms,
    );
    fs::write(out_dir.join("report.md"), markdown).expect("Markdown report should be written");

    let json = format!(
        "{{\n  \"git_sha\": \"{git_sha}\",\n  \"platform\": \"{}-{}\",\n  \"rustc_host\": \"{host}\",\n  \"fixture\": \"benchmarks/cases/enums-v0/evolution.evo\",\n  \"samples_per_phase\": {SAMPLES},\n  \"accepted_76_edit_baseline_ms\": {ACCEPTED_76_EDIT_BASELINE_MS:.3},\n  \"evo_prime_samples_ms\": [{}],\n  \"evo_edit_samples_ms\": [{}],\n  \"stable_prime_samples_ms\": [{}],\n  \"stable_edit_samples_ms\": [{}],\n  \"incremental_prime_samples_ms\": [{}],\n  \"incremental_edit_samples_ms\": [{}],\n  \"evo_edit_median_ms\": {:.3},\n  \"stable_edit_median_ms\": {:.3},\n  \"incremental_edit_median_ms\": {:.3},\n  \"stable_to_incremental_speedup\": {:.6},\n  \"evo_to_incremental_speedup\": {:.6},\n  \"accepted_76_to_incremental_speedup\": {:.6},\n  \"incremental_state_after_prime_files\": [{}],\n  \"incremental_state_after_prime_bytes\": [{}],\n  \"incremental_state_after_edit_files\": [{}],\n  \"incremental_state_after_edit_bytes\": [{}]\n}}\n",
        env::consts::OS,
        env::consts::ARCH,
        samples_json(input.evo_prime),
        samples_json(input.evo_edit),
        samples_json(input.stable_prime),
        samples_json(input.stable_edit),
        samples_json(input.incremental_prime),
        samples_json(input.incremental_edit),
        evo_edit.median_ms,
        stable_edit.median_ms,
        incremental_edit.median_ms,
        stable_to_incremental_speedup,
        evo_to_incremental_speedup,
        accepted_76_to_incremental_speedup,
        u64_json(input.incremental_state_after_prime.iter().map(|state| state.files)),
        u64_json(input.incremental_state_after_prime.iter().map(|state| state.bytes)),
        u64_json(input.incremental_state_after_edit.iter().map(|state| state.files)),
        u64_json(input.incremental_state_after_edit.iter().map(|state| state.bytes)),
    );
    fs::write(out_dir.join("report.json"), json).expect("JSON report should be written");

    let mut csv = String::from(
        "arm,phase,index,elapsed_ms,rustc_compile_delta,state_files,state_bytes,notes\n",
    );
    push_csv_rows(
        &mut csv,
        "evo-fresh-workdir",
        "prime",
        input.evo_prime,
        None,
        "production-no-cache-path",
    );
    push_csv_rows(
        &mut csv,
        "evo-fresh-workdir",
        "edit",
        input.evo_edit,
        None,
        "generated-rust-changing-edit",
    );
    push_csv_rows(
        &mut csv,
        "stable-direct-no-incremental",
        "prime",
        input.stable_prime,
        None,
        "stable-main-rs",
    );
    push_csv_rows(
        &mut csv,
        "stable-direct-no-incremental",
        "edit",
        input.stable_edit,
        None,
        "stable-main-rs",
    );
    push_csv_rows(
        &mut csv,
        "stable-direct-incremental",
        "prime",
        input.incremental_prime,
        Some(input.incremental_state_after_prime),
        "fresh-session-stable-main-rs",
    );
    push_csv_rows(
        &mut csv,
        "stable-direct-incremental",
        "edit",
        input.incremental_edit,
        Some(input.incremental_state_after_edit),
        "reused-session-stable-main-rs",
    );
    fs::write(out_dir.join("raw-samples.csv"), csv).expect("CSV report should be written");

    fs::write(out_dir.join("baseline-generated.rs"), input.baseline_generated)
        .expect("baseline generated Rust should be written");
    fs::write(out_dir.join("edited-generated.rs"), input.edited_generated)
        .expect("edited generated Rust should be written");
    fs::write(out_dir.join("baseline.evo"), input.baseline_source)
        .expect("baseline Evolution source should be written");
    fs::write(out_dir.join("edited.evo"), input.edited_source)
        .expect("edited Evolution source should be written");
    fs::write(out_dir.join("stdin.bin"), input.fixture_stdin)
        .expect("fixture stdin evidence should be written");
    fs::write(out_dir.join("expected.stdout"), input.expected_stdout)
        .expect("fixture expected output evidence should be written");
    fs::write(out_dir.join("rustc-vV.txt"), input.rustc_version)
        .expect("rustc version evidence should be written");
    fs::write(out_dir.join("rustc-C-help.txt"), input.rustc_c_help)
        .expect("rustc -C help evidence should be written");
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
fn directory_stats_count_regular_files_recursively() {
    let dir = temp_dir("dir-stats");
    fs::create_dir_all(dir.join("nested")).expect("nested directory should be created");
    fs::write(dir.join("one"), b"abc").expect("first file should write");
    fs::write(dir.join("nested/two"), b"12345").expect("second file should write");
    assert_eq!(dir_stats(&dir), DirStats { files: 2, bytes: 8 });
    let _ = fs::remove_dir_all(dir);
}

#[test]
#[ignore = "run explicitly on the controlled Ubuntu CI evidence runner"]
fn incremental_build_research_isolates_stable_path_and_persistent_rustc_state() {
    let dir = temp_dir("evidence");
    fs::create_dir_all(&dir).expect("test directory should be created");

    let fixture = fixture_dir();
    let fixture_source =
        fs::read(fixture.join("evolution.evo")).expect("fixture source should read");
    let fixture_stdin = fs::read(fixture.join("stdin.bin")).expect("fixture stdin should read");
    let expected_stdout =
        fs::read(fixture.join("expected.stdout")).expect("fixture expected stdout should read");

    let baseline_source =
        String::from_utf8(fixture_source.clone()).expect("fixture should be UTF-8");
    assert!(baseline_source.matches(EDIT_FROM).count() >= 2);
    let edited_source = baseline_source.replacen(EDIT_FROM, EDIT_TO, 1);
    assert_ne!(baseline_source, edited_source);

    let emission_dir = dir.join("emission");
    fs::create_dir_all(&emission_dir).expect("emission directory should be created");
    let emission_source = emission_dir.join("program.evo");
    fs::write(&emission_source, &baseline_source).expect("baseline source should write");
    let baseline_generated = emit_command(&emission_source)
        .output()
        .expect("baseline emit-rust should execute");
    assert_success(&baseline_generated, "baseline emit-rust");
    fs::write(&emission_source, &edited_source).expect("edited source should write");
    let edited_generated = emit_command(&emission_source)
        .output()
        .expect("edited emit-rust should execute");
    assert_success(&edited_generated, "edited emit-rust");
    assert_ne!(baseline_generated.stdout, edited_generated.stdout);

    let counter = dir.join("rustc-count.txt");
    fs::write(&counter, "0").expect("counter should be initialized");
    let rustc = real_rustc();
    let wrapper = compile_rustc_wrapper(&dir, &rustc);
    let rustc_version = rustc_text(&rustc, &["-vV"], "rustc -vV");
    let rustc_c_help = rustc_text(&rustc, &["-C", "help"], "rustc -C help");
    assert!(
        rustc_c_help.lines().any(|line| line.contains("incremental")),
        "pinned rustc must advertise the incremental codegen option"
    );

    let mut evo_prime_samples = Vec::with_capacity(SAMPLES);
    let mut evo_edit_samples = Vec::with_capacity(SAMPLES);
    for index in 0..SAMPLES {
        let sample_dir = dir.join(format!("evo-{index}"));
        fs::create_dir_all(&sample_dir).expect("evo sample directory should be created");
        let source = sample_dir.join("program.evo");
        let output = sample_dir.join(format!("program{}", env::consts::EXE_SUFFIX));
        fs::write(&source, &baseline_source).expect("evo baseline source should write");

        let before_prime = compile_count(&counter);
        let (prime, prime_elapsed) = timed_output(&mut build_command(
            &source, &output, &wrapper, &rustc, &counter,
        ));
        assert_success(&prime, "evo prime build");
        assert_eq!(compile_count(&counter) - before_prime, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        evo_prime_samples.push(prime_elapsed);

        fs::write(&source, &edited_source).expect("evo edited source should write");
        let before_edit = compile_count(&counter);
        let (edit, edit_elapsed) = timed_output(&mut build_command(
            &source, &output, &wrapper, &rustc, &counter,
        ));
        assert_success(&edit, "evo edited build");
        assert_eq!(compile_count(&counter) - before_edit, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        evo_edit_samples.push(edit_elapsed);
    }

    let mut stable_prime_samples = Vec::with_capacity(SAMPLES);
    let mut stable_edit_samples = Vec::with_capacity(SAMPLES);
    for index in 0..SAMPLES {
        let sample_dir = dir.join(format!("stable-{index}"));
        fs::create_dir_all(&sample_dir).expect("stable sample directory should be created");
        let generated = sample_dir.join("main.rs");
        let output = sample_dir.join(format!("program{}", env::consts::EXE_SUFFIX));
        fs::write(&generated, &baseline_generated.stdout)
            .expect("stable baseline generated Rust should write");

        let before_prime = compile_count(&counter);
        let (prime, prime_elapsed) = timed_output(&mut direct_rustc_command(
            &wrapper, &rustc, &counter, &generated, &output, None,
        ));
        assert_success(&prime, "stable direct prime");
        assert_eq!(compile_count(&counter) - before_prime, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        stable_prime_samples.push(prime_elapsed);

        fs::write(&generated, &edited_generated.stdout)
            .expect("stable edited generated Rust should write");
        let before_edit = compile_count(&counter);
        let (edit, edit_elapsed) = timed_output(&mut direct_rustc_command(
            &wrapper, &rustc, &counter, &generated, &output, None,
        ));
        assert_success(&edit, "stable direct edit");
        assert_eq!(compile_count(&counter) - before_edit, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        stable_edit_samples.push(edit_elapsed);
    }

    let mut incremental_prime_samples = Vec::with_capacity(SAMPLES);
    let mut incremental_edit_samples = Vec::with_capacity(SAMPLES);
    let mut incremental_state_after_prime = Vec::with_capacity(SAMPLES);
    let mut incremental_state_after_edit = Vec::with_capacity(SAMPLES);
    for index in 0..SAMPLES {
        let sample_dir = dir.join(format!("incremental-{index}"));
        fs::create_dir_all(&sample_dir).expect("incremental sample directory should be created");
        let generated = sample_dir.join("main.rs");
        let output = sample_dir.join(format!("program{}", env::consts::EXE_SUFFIX));
        let session = sample_dir.join("session");
        fs::write(&generated, &baseline_generated.stdout)
            .expect("incremental baseline generated Rust should write");

        let before_prime = compile_count(&counter);
        let (prime, prime_elapsed) = timed_output(&mut direct_rustc_command(
            &wrapper,
            &rustc,
            &counter,
            &generated,
            &output,
            Some(&session),
        ));
        assert_success(&prime, "incremental direct prime");
        assert_eq!(compile_count(&counter) - before_prime, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        let state_after_prime = dir_stats(&session);
        assert!(state_after_prime.files > 0, "incremental prime must persist state files");
        incremental_prime_samples.push(prime_elapsed);
        incremental_state_after_prime.push(state_after_prime);

        fs::write(&generated, &edited_generated.stdout)
            .expect("incremental edited generated Rust should write");
        let before_edit = compile_count(&counter);
        let (edit, edit_elapsed) = timed_output(&mut direct_rustc_command(
            &wrapper,
            &rustc,
            &counter,
            &generated,
            &output,
            Some(&session),
        ));
        assert_success(&edit, "incremental direct edit");
        assert_eq!(compile_count(&counter) - before_edit, 1);
        run_binary(&output, &fixture_stdin, &expected_stdout);
        incremental_edit_samples.push(edit_elapsed);
        incremental_state_after_edit.push(dir_stats(&session));
    }

    let out_dir = env::var_os("EVO_INCREMENTAL_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("incremental-build-research-report"));
    write_report(
        &out_dir,
        &ReportInput {
            evo_prime: &evo_prime_samples,
            evo_edit: &evo_edit_samples,
            stable_prime: &stable_prime_samples,
            stable_edit: &stable_edit_samples,
            incremental_prime: &incremental_prime_samples,
            incremental_edit: &incremental_edit_samples,
            incremental_state_after_prime: &incremental_state_after_prime,
            incremental_state_after_edit: &incremental_state_after_edit,
            baseline_generated: &baseline_generated.stdout,
            edited_generated: &edited_generated.stdout,
            baseline_source: baseline_source.as_bytes(),
            edited_source: edited_source.as_bytes(),
            fixture_stdin: &fixture_stdin,
            expected_stdout: &expected_stdout,
            rustc_version: &rustc_version,
            rustc_c_help: &rustc_c_help,
        },
    );

    let evo_edit = stats(&evo_edit_samples);
    let stable_edit = stats(&stable_edit_samples);
    let incremental_edit = stats(&incremental_edit_samples);
    println!("evo_edit_ms={:.3}", evo_edit.median_ms);
    println!("stable_edit_ms={:.3}", stable_edit.median_ms);
    println!("incremental_edit_ms={:.3}", incremental_edit.median_ms);
    println!(
        "stable_to_incremental_speedup={:.3}",
        stable_edit.median_ms / incremental_edit.median_ms
    );
    println!(
        "accepted_76_to_incremental_speedup={:.3}",
        ACCEPTED_76_EDIT_BASELINE_MS / incremental_edit.median_ms
    );

    let _ = fs::remove_dir_all(dir);
}
