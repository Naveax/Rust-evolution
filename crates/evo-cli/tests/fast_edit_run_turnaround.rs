use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const COLD_SAMPLES: usize = 5;
const WARM_SAMPLES: usize = 9;

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    env::temp_dir().join(format!("evo-turnaround-{label}-{}-{nanos}", process::id()))
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

fn evo_command(
    source: &Path,
    cache_dir: &Path,
    wrapper: &Path,
    rustc: &OsStr,
    counter: &Path,
) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_evo"));
    command
        .arg("run")
        .arg(source)
        .env("EVO_CACHE_DIR", cache_dir)
        .env("RUSTC", wrapper)
        .env("EVO_TEST_REAL_RUSTC", rustc)
        .env("EVO_TEST_RUSTC_COUNT", counter);
    command
}

fn timed_run(
    source: &Path,
    cache_dir: &Path,
    wrapper: &Path,
    rustc: &OsStr,
    counter: &Path,
) -> (Output, Duration) {
    let start = Instant::now();
    let output = evo_command(source, cache_dir, wrapper, rustc, counter)
        .output()
        .expect("evo run should execute");
    (output, start.elapsed())
}

fn run(
    source: &Path,
    cache_dir: &Path,
    wrapper: &Path,
    rustc: &OsStr,
    counter: &Path,
) -> Output {
    evo_command(source, cache_dir, wrapper, rustc, counter)
        .output()
        .expect("evo run should execute")
}

fn compile_count(counter: &Path) -> u64 {
    fs::read_to_string(counter)
        .expect("counter should exist")
        .trim()
        .parse()
        .expect("counter should contain an integer")
}

fn assert_output(output: &Output, expected: &str) {
    assert!(
        output.status.success(),
        "evo failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), expected);
}

fn median_ms(samples: &[Duration]) -> f64 {
    let mut values = samples
        .iter()
        .map(|sample| sample.as_secs_f64() * 1_000.0)
        .collect::<Vec<_>>();
    values.sort_by(|left, right| left.total_cmp(right));
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

fn samples_json(samples: &[Duration]) -> String {
    samples
        .iter()
        .map(|sample| format!("{:.3}", sample.as_secs_f64() * 1_000.0))
        .collect::<Vec<_>>()
        .join(", ")
}

fn samples_csv(kind: &str, samples: &[Duration], compile_delta: u64, reason: &str) -> String {
    samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            format!(
                "{kind},{},{:.3},{compile_delta},{reason}\n",
                index + 1,
                sample.as_secs_f64() * 1_000.0
            )
        })
        .collect()
}

fn write_report(
    out_dir: &Path,
    cold_samples: &[Duration],
    warm_samples: &[Duration],
    cold_compile_count: u64,
    warm_compile_count: u64,
) {
    fs::create_dir_all(out_dir).expect("turnaround output directory should be created");

    let cold_median_ms = median_ms(cold_samples);
    let warm_median_ms = median_ms(warm_samples);
    let warm_speedup = cold_median_ms / warm_median_ms;
    let verdict = if warm_compile_count == 0 && warm_median_ms < cold_median_ms {
        "PASS"
    } else {
        "FAIL"
    };
    let git_sha = env::var("GITHUB_SHA").unwrap_or_else(|_| "local".to_owned());

    let markdown = format!(
        "# Fast edit-run turnaround evidence\n\n\
- git_sha: `{git_sha}`\n\
- platform: `{}-{}`\n\
- cold_samples: `{}`\n\
- warm_samples: `{}`\n\
- cold_run_ms (median): `{cold_median_ms:.3}`\n\
- warm_run_ms (median): `{warm_median_ms:.3}`\n\
- warm_speedup: `{warm_speedup:.3}x`\n\
- cold rustc compile invocations: `{cold_compile_count}`\n\
- warm rustc compile invocations: `{warm_compile_count}`\n\
- cold reason: `cache-miss:no-entry`\n\
- warm reason: `verified-cache-hit`\n\
- verdict: **{verdict}**\n\n\
Cold samples each use a fresh empty `EVO_CACHE_DIR`. Warm samples reuse one verified entry after an untimed priming run. The generated program output is checked on every sample.\n",
        env::consts::OS,
        env::consts::ARCH,
        cold_samples.len(),
        warm_samples.len(),
    );
    fs::write(out_dir.join("report.md"), markdown).expect("Markdown report should be written");

    let json = format!(
        "{{\n  \"git_sha\": \"{git_sha}\",\n  \"platform\": \"{}-{}\",\n  \"cold_samples_ms\": [{}],\n  \"warm_samples_ms\": [{}],\n  \"cold_run_ms\": {cold_median_ms:.3},\n  \"warm_run_ms\": {warm_median_ms:.3},\n  \"warm_speedup\": {warm_speedup:.6},\n  \"cold_rustc_compile_count\": {cold_compile_count},\n  \"warm_rustc_compile_count\": {warm_compile_count},\n  \"cold_reason\": \"cache-miss:no-entry\",\n  \"warm_reason\": \"verified-cache-hit\",\n  \"verdict\": \"{verdict}\"\n}}\n",
        env::consts::OS,
        env::consts::ARCH,
        samples_json(cold_samples),
        samples_json(warm_samples),
    );
    fs::write(out_dir.join("report.json"), json).expect("JSON report should be written");

    let mut csv = String::from("kind,index,elapsed_ms,rustc_compile_delta,reason\n");
    csv.push_str(&samples_csv("cold", cold_samples, 1, "cache-miss:no-entry"));
    csv.push_str(&samples_csv("warm", warm_samples, 0, "verified-cache-hit"));
    fs::write(out_dir.join("raw-samples.csv"), csv).expect("CSV report should be written");
}

#[test]
#[ignore = "run explicitly on the controlled Ubuntu CI evidence runner"]
fn fast_edit_run_turnaround_reports_verified_warm_speedup() {
    let dir = temp_dir("evidence");
    let source = dir.join("program.evo");
    let counter = dir.join("rustc-count.txt");
    fs::create_dir_all(&dir).expect("test directory should be created");
    fs::write(&source, "print 7\n").expect("source should be written");
    fs::write(&counter, "0").expect("counter should be initialized");

    let rustc = real_rustc();
    let wrapper = compile_rustc_wrapper(&dir, &rustc);
    let mut cold_samples = Vec::with_capacity(COLD_SAMPLES);

    for index in 0..COLD_SAMPLES {
        let cache_dir = dir.join(format!("cold-cache-{index}"));
        let before = compile_count(&counter);
        let (output, elapsed) = timed_run(&source, &cache_dir, &wrapper, &rustc, &counter);
        assert_output(&output, "7");
        let after = compile_count(&counter);
        assert_eq!(after - before, 1, "each cold run must compile exactly once");
        cold_samples.push(elapsed);
    }

    let warm_cache = dir.join("warm-cache");
    let before_prime = compile_count(&counter);
    let prime = run(&source, &warm_cache, &wrapper, &rustc, &counter);
    assert_output(&prime, "7");
    assert_eq!(
        compile_count(&counter) - before_prime,
        1,
        "warm cache priming must compile exactly once"
    );

    let warm_compile_start = compile_count(&counter);
    let mut warm_samples = Vec::with_capacity(WARM_SAMPLES);
    for _ in 0..WARM_SAMPLES {
        let (output, elapsed) = timed_run(&source, &warm_cache, &wrapper, &rustc, &counter);
        assert_output(&output, "7");
        warm_samples.push(elapsed);
    }
    let warm_compile_count = compile_count(&counter) - warm_compile_start;
    let cold_compile_count = COLD_SAMPLES as u64;

    let out_dir = env::var_os("EVO_TURNAROUND_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("turnaround-report"));
    write_report(
        &out_dir,
        &cold_samples,
        &warm_samples,
        cold_compile_count,
        warm_compile_count,
    );

    let cold_median_ms = median_ms(&cold_samples);
    let warm_median_ms = median_ms(&warm_samples);
    println!("cold_run_ms={cold_median_ms:.3}");
    println!("warm_run_ms={warm_median_ms:.3}");
    println!("warm_speedup={:.3}x", cold_median_ms / warm_median_ms);
    println!("cold_rustc_compile_count={cold_compile_count}");
    println!("warm_rustc_compile_count={warm_compile_count}");

    assert_eq!(
        warm_compile_count, 0,
        "verified warm runs must perform zero rustc compilations"
    );
    assert!(
        warm_median_ms < cold_median_ms,
        "warm median {warm_median_ms:.3} ms must be below cold median {cold_median_ms:.3} ms"
    );

    let _ = fs::remove_dir_all(dir);
}
