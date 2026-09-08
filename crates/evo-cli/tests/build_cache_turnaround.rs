use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const COLD_SAMPLES: usize = 5;
const WARM_SAMPLES: usize = 9;
const ACCEPTED_UNCACHED_WARM_BASELINE_MS: f64 = 96.986;

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    env::temp_dir().join(format!(
        "evo-build-cache-turnaround-{label}-{}-{nanos}",
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

fn build_command(
    source: &Path,
    output: &Path,
    cache_dir: &Path,
    wrapper: &Path,
    rustc: &OsStr,
    counter: &Path,
) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_evo"));
    command
        .arg("build")
        .arg(source)
        .arg(output)
        .env("EVO_CACHE_DIR", cache_dir)
        .env("RUSTC", wrapper)
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

fn compile_count(counter: &Path) -> u64 {
    fs::read_to_string(counter)
        .expect("counter should exist")
        .trim()
        .parse()
        .expect("counter should contain an integer")
}

fn assert_binary_output(binary: &Path, stdin: &[u8], expected_stdout: &[u8]) {
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

fn median_ms(samples: &[Duration]) -> f64 {
    let mut values = samples
        .iter()
        .map(|sample| sample.as_secs_f64() * 1_000.0)
        .collect::<Vec<_>>();
    values.sort_by(|left, right| left.total_cmp(right));
    values[values.len() / 2]
}

fn samples_json(samples: &[Duration]) -> String {
    samples
        .iter()
        .map(|sample| format!("{:.3}", sample.as_secs_f64() * 1_000.0))
        .collect::<Vec<_>>()
        .join(", ")
}

struct ReportInput<'a> {
    cold: &'a [Duration],
    warm: &'a [Duration],
    cold_compile_count: u64,
    warm_compile_count: u64,
    generated_rust: &'a [u8],
    fixture_source: &'a [u8],
    fixture_stdin: &'a [u8],
    expected_stdout: &'a [u8],
    rustc_version: &'a str,
}

fn write_report(out_dir: &Path, input: &ReportInput<'_>) {
    fs::create_dir_all(out_dir).expect("evidence directory should be created");
    let cold_median = median_ms(input.cold);
    let warm_median = median_ms(input.warm);
    let speedup = cold_median / warm_median;
    let baseline_speedup = ACCEPTED_UNCACHED_WARM_BASELINE_MS / warm_median;
    let git_sha = env::var("EVO_GIT_SHA")
        .or_else(|_| env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "local".to_owned());

    let markdown = format!(
        "# Verified build cache turnaround v0\n\n\
- git_sha: `{git_sha}`\n\
- platform: `{}-{}`\n\
- fixture: `benchmarks/cases/enums-v0/evolution.evo`\n\
- cold_median_ms: `{cold_median:.3}`\n\
- warm_cached_median_ms: `{warm_median:.3}`\n\
- cold_to_warm_speedup: `{speedup:.3}x`\n\
- accepted_uncached_warm_baseline_ms: `{ACCEPTED_UNCACHED_WARM_BASELINE_MS:.3}`\n\
- accepted_baseline_to_cached_speedup: `{baseline_speedup:.3}x`\n\
- cold_rustc_compile_count: `{}`\n\
- warm_rustc_compile_count: `{}`\n\
- correctness: `PASS`\n\n\
Hard acceptance is exact warm rustc compile count zero plus correct native output. Timing is supporting evidence.\n",
        env::consts::OS,
        env::consts::ARCH,
        input.cold_compile_count,
        input.warm_compile_count,
    );
    fs::write(out_dir.join("report.md"), markdown).expect("Markdown report should write");

    let json = format!(
        "{{\n  \"git_sha\": \"{git_sha}\",\n  \"platform\": \"{}-{}\",\n  \"fixture\": \"benchmarks/cases/enums-v0/evolution.evo\",\n  \"cold_samples_ms\": [{}],\n  \"warm_cached_samples_ms\": [{}],\n  \"cold_median_ms\": {cold_median:.3},\n  \"warm_cached_median_ms\": {warm_median:.3},\n  \"cold_to_warm_speedup\": {speedup:.6},\n  \"accepted_uncached_warm_baseline_ms\": {ACCEPTED_UNCACHED_WARM_BASELINE_MS:.3},\n  \"accepted_baseline_to_cached_speedup\": {baseline_speedup:.6},\n  \"cold_rustc_compile_count\": {},\n  \"warm_rustc_compile_count\": {},\n  \"correctness\": \"PASS\"\n}}\n",
        env::consts::OS,
        env::consts::ARCH,
        samples_json(input.cold),
        samples_json(input.warm),
        input.cold_compile_count,
        input.warm_compile_count,
    );
    fs::write(out_dir.join("report.json"), json).expect("JSON report should write");

    let mut csv = String::from("kind,index,elapsed_ms\n");
    for (index, sample) in input.cold.iter().enumerate() {
        csv.push_str(&format!(
            "cold,{},{}\n",
            index + 1,
            sample.as_secs_f64() * 1_000.0
        ));
    }
    for (index, sample) in input.warm.iter().enumerate() {
        csv.push_str(&format!(
            "warm-cached,{},{}\n",
            index + 1,
            sample.as_secs_f64() * 1_000.0
        ));
    }
    fs::write(out_dir.join("raw-samples.csv"), csv).expect("CSV report should write");
    fs::write(out_dir.join("generated.rs"), input.generated_rust)
        .expect("generated Rust evidence should write");
    fs::write(out_dir.join("evolution.evo"), input.fixture_source)
        .expect("fixture source evidence should write");
    fs::write(out_dir.join("stdin.bin"), input.fixture_stdin)
        .expect("fixture stdin evidence should write");
    fs::write(out_dir.join("expected.stdout"), input.expected_stdout)
        .expect("expected output evidence should write");
    fs::write(out_dir.join("rustc-vV.txt"), input.rustc_version)
        .expect("rustc identity should write");
}

#[test]
#[ignore = "run explicitly on the controlled Ubuntu CI evidence runner"]
fn verified_build_cache_reports_zero_warm_compiles_and_speedup() {
    let dir = temp_dir("evidence");
    fs::create_dir_all(&dir).expect("evidence test directory should be created");

    let fixture = fixture_dir();
    let fixture_source =
        fs::read(fixture.join("evolution.evo")).expect("fixture source should read");
    let fixture_stdin = fs::read(fixture.join("stdin.bin")).expect("fixture stdin should read");
    let expected_stdout =
        fs::read(fixture.join("expected.stdout")).expect("expected stdout should read");
    let reference_rust =
        fs::read(fixture.join("reference.rs")).expect("fixture reference Rust should read");

    let source = dir.join("program.evo");
    fs::write(&source, &fixture_source).expect("fixture source should be written");
    let rustc = real_rustc();
    let wrapper = compile_rustc_wrapper(&dir, &rustc);

    let rustc_version_output = Command::new(&rustc)
        .arg("-vV")
        .output()
        .expect("rustc -vV should execute");
    assert_success(&rustc_version_output, "rustc -vV");
    let rustc_version = String::from_utf8_lossy(&rustc_version_output.stdout).to_string();

    let generated = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("emit-rust")
        .arg(&source)
        .output()
        .expect("emit-rust should execute");
    assert_success(&generated, "emit-rust");
    assert_eq!(
        generated.stdout, reference_rust,
        "cache evidence must retain accepted Enums generated Rust"
    );

    let mut cold_samples = Vec::with_capacity(COLD_SAMPLES);
    let mut cold_compile_count = 0;
    for index in 0..COLD_SAMPLES {
        let sample_dir = dir.join(format!("cold-{index}"));
        let cache_dir = sample_dir.join("cache");
        let counter = sample_dir.join("rustc-count.txt");
        let output = sample_dir.join(format!("program{}", env::consts::EXE_SUFFIX));
        fs::create_dir_all(&sample_dir).expect("cold sample dir should exist");
        fs::write(&counter, "0").expect("cold counter should initialize");

        let (build, elapsed) = timed_output(&mut build_command(
            &source, &output, &cache_dir, &wrapper, &rustc, &counter,
        ));
        assert_success(&build, "cold build");
        assert_binary_output(&output, &fixture_stdin, &expected_stdout);
        assert_eq!(compile_count(&counter), 1, "cold sample must compile once");
        cold_compile_count += 1;
        cold_samples.push(elapsed);
    }

    let warm_dir = dir.join("warm");
    let warm_cache = warm_dir.join("cache");
    let warm_counter = warm_dir.join("rustc-count.txt");
    let warm_output = warm_dir.join(format!("program{}", env::consts::EXE_SUFFIX));
    fs::create_dir_all(&warm_dir).expect("warm sample dir should exist");
    fs::write(&warm_counter, "0").expect("warm counter should initialize");

    let prime = build_command(
        &source,
        &warm_output,
        &warm_cache,
        &wrapper,
        &rustc,
        &warm_counter,
    )
    .output()
    .expect("warm prime should execute");
    assert_success(&prime, "warm prime");
    assert_binary_output(&warm_output, &fixture_stdin, &expected_stdout);
    assert_eq!(
        compile_count(&warm_counter),
        1,
        "warm prime must compile once"
    );

    let mut warm_samples = Vec::with_capacity(WARM_SAMPLES);
    for _ in 0..WARM_SAMPLES {
        let before = compile_count(&warm_counter);
        let (build, elapsed) = timed_output(&mut build_command(
            &source,
            &warm_output,
            &warm_cache,
            &wrapper,
            &rustc,
            &warm_counter,
        ));
        assert_success(&build, "warm cached build");
        assert_binary_output(&warm_output, &fixture_stdin, &expected_stdout);
        assert_eq!(
            compile_count(&warm_counter),
            before,
            "verified warm cache hit must not compile"
        );
        warm_samples.push(elapsed);
    }

    let warm_compile_count = compile_count(&warm_counter) - 1;
    assert_eq!(
        warm_compile_count, 0,
        "warm samples must compile zero times"
    );

    let cold_median = median_ms(&cold_samples);
    let warm_median = median_ms(&warm_samples);
    assert!(
        warm_median < cold_median,
        "cached warm median {warm_median:.3} ms must be below cold median {cold_median:.3} ms"
    );
    assert!(
        warm_median < ACCEPTED_UNCACHED_WARM_BASELINE_MS,
        "cached warm median {warm_median:.3} ms must be below accepted uncached baseline {ACCEPTED_UNCACHED_WARM_BASELINE_MS:.3} ms"
    );

    let out_dir = env::var_os("EVO_BUILD_CACHE_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("report"));
    write_report(
        &out_dir,
        &ReportInput {
            cold: &cold_samples,
            warm: &warm_samples,
            cold_compile_count,
            warm_compile_count,
            generated_rust: &generated.stdout,
            fixture_source: &fixture_source,
            fixture_stdin: &fixture_stdin,
            expected_stdout: &expected_stdout,
            rustc_version: &rustc_version,
        },
    );

    println!("cold_build_median_ms={cold_median:.3}");
    println!("warm_cached_build_median_ms={warm_median:.3}");
    println!("warm_rustc_compile_count={warm_compile_count}");

    let _ = fs::remove_dir_all(dir);
}
