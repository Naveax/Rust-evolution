use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    env::temp_dir().join(format!("evo-build-cache-{label}-{}-{nanos}", process::id()))
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

    if version_probe && let Ok(salt) = env::var("EVO_TEST_FINGERPRINT_SALT") {
        println!("evo-test-fingerprint-salt: {salt}");
    }

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
    output: Option<&Path>,
    cache_dir: &Path,
    wrapper: &Path,
    rustc: &OsStr,
    counter: &Path,
    fingerprint_salt: Option<&str>,
    no_cache: bool,
) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_evo"));
    command
        .arg("build")
        .arg(source)
        .env("EVO_CACHE_DIR", cache_dir)
        .env("RUSTC", wrapper)
        .env("EVO_TEST_REAL_RUSTC", rustc)
        .env("EVO_TEST_RUSTC_COUNT", counter);
    if let Some(output) = output {
        command.arg(output);
    }
    if no_cache {
        command.arg("--no-cache");
    }
    if let Some(salt) = fingerprint_salt {
        command.env("EVO_TEST_FINGERPRINT_SALT", salt);
    }
    command
}

fn run_build(
    source: &Path,
    output: Option<&Path>,
    cache_dir: &Path,
    wrapper: &Path,
    rustc: &OsStr,
    counter: &Path,
    fingerprint_salt: Option<&str>,
    no_cache: bool,
) -> Output {
    build_command(
        source,
        output,
        cache_dir,
        wrapper,
        rustc,
        counter,
        fingerprint_salt,
        no_cache,
    )
    .output()
    .expect("evo build should execute")
}

fn compile_count(counter: &Path) -> u64 {
    fs::read_to_string(counter)
        .expect("counter should exist")
        .trim()
        .parse()
        .expect("counter should contain an integer")
}

fn assert_build_success(output: &Output) {
    assert!(
        output.status.success(),
        "evo build failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_binary_stdout(binary: &Path, expected: &str) {
    let output = Command::new(binary)
        .output()
        .expect("built binary should execute");
    assert!(
        output.status.success(),
        "built binary failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), expected);
}

fn first_cache_entry(cache_dir: &Path) -> PathBuf {
    fs::read_dir(cache_dir.join("build-cache-v0"))
        .expect("build cache should exist")
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with("entry-"))
        .expect("at least one build-cache entry should exist")
        .path()
}

fn default_output_path(source: &Path) -> PathBuf {
    let mut output = source.to_path_buf();
    if env::consts::EXE_SUFFIX.is_empty() {
        output.set_extension("");
    } else {
        output.set_extension(env::consts::EXE_SUFFIX.trim_start_matches('.'));
    }
    output
}

#[test]
fn unchanged_build_reuses_verified_artifact_and_invalidates_safely() {
    let dir = temp_dir("behavior");
    let cache_dir = dir.join("cache");
    let counter = dir.join("rustc-count.txt");
    let source = dir.join("program.evo");
    let output_a = dir
        .join("out-a")
        .join(format!("program{}", env::consts::EXE_SUFFIX));
    let output_b = dir
        .join("out-b")
        .join(format!("program{}", env::consts::EXE_SUFFIX));
    fs::create_dir_all(&dir).expect("test directory should be created");
    fs::write(&counter, "0").expect("counter should be initialized");
    fs::write(&source, "print 2\n").expect("source should be written");

    let rustc = real_rustc();
    let wrapper = compile_rustc_wrapper(&dir, &rustc);

    let cold = run_build(
        &source,
        Some(&output_a),
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        None,
        false,
    );
    assert_build_success(&cold);
    assert_binary_stdout(&output_a, "2");
    assert_eq!(compile_count(&counter), 1, "cold build must compile once");

    fs::write(&output_a, b"not-a-binary").expect("existing output should be replaceable");
    let warm = run_build(
        &source,
        Some(&output_a),
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        None,
        false,
    );
    assert_build_success(&warm);
    assert_binary_stdout(&output_a, "2");
    assert_eq!(
        compile_count(&counter),
        1,
        "unchanged warm build must materialize without compiling"
    );

    let different_output = run_build(
        &source,
        Some(&output_b),
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        None,
        false,
    );
    assert_build_success(&different_output);
    assert_binary_stdout(&output_b, "2");
    assert_eq!(
        compile_count(&counter),
        1,
        "verified cached artifact must support a different requested output path"
    );

    let entry = first_cache_entry(&cache_dir);
    fs::write(entry.join("compiler.txt"), "corrupt")
        .expect("cache identity should be corruptible for the regression test");
    let after_corruption = run_build(
        &source,
        Some(&output_b),
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        None,
        false,
    );
    assert_build_success(&after_corruption);
    assert_binary_stdout(&output_b, "2");
    assert_eq!(
        compile_count(&counter),
        2,
        "corrupt cache identity must fail closed to recompilation"
    );

    fs::write(&source, "print @\n").expect("invalid source should be written");
    let invalid = run_build(
        &source,
        Some(&output_b),
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        None,
        false,
    );
    assert!(
        !invalid.status.success(),
        "invalid frontend source must fail"
    );
    assert!(
        String::from_utf8_lossy(&invalid.stderr).contains("unexpected character"),
        "{}",
        String::from_utf8_lossy(&invalid.stderr)
    );
    assert_eq!(
        compile_count(&counter),
        2,
        "frontend validation must run before build-cache reuse"
    );

    fs::write(&source, "print 3\n").expect("changed source should be written");
    let changed = run_build(
        &source,
        Some(&output_b),
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        None,
        false,
    );
    assert_build_success(&changed);
    assert_binary_stdout(&output_b, "3");
    assert_eq!(compile_count(&counter), 3, "source change must miss cache");

    let fingerprint_changed = run_build(
        &source,
        Some(&output_b),
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        Some("toolchain-b"),
        false,
    );
    assert_build_success(&fingerprint_changed);
    assert_binary_stdout(&output_b, "3");
    assert_eq!(
        compile_count(&counter),
        4,
        "compiler fingerprint change must miss cache"
    );

    let fingerprint_warm = run_build(
        &source,
        Some(&output_b),
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        Some("toolchain-b"),
        false,
    );
    assert_build_success(&fingerprint_warm);
    assert_eq!(
        compile_count(&counter),
        4,
        "unchanged build under new fingerprint must warm-hit"
    );

    let bypass = run_build(
        &source,
        Some(&output_b),
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        Some("toolchain-b"),
        true,
    );
    assert_build_success(&bypass);
    assert_binary_stdout(&output_b, "3");
    assert_eq!(compile_count(&counter), 5, "--no-cache must compile");

    let default_bypass = run_build(
        &source,
        None,
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        Some("toolchain-b"),
        true,
    );
    assert_build_success(&default_bypass);
    let default_output = default_output_path(&source);
    assert_binary_stdout(&default_output, "3");
    assert_eq!(
        compile_count(&counter),
        6,
        "--no-cache without explicit output must use default output and compile"
    );

    let unavailable_cache = dir.join("cache-is-a-file");
    fs::write(&unavailable_cache, b"not-a-directory").expect("cache sentinel should write");
    let fallback = run_build(
        &source,
        Some(&output_a),
        &unavailable_cache,
        &wrapper,
        &rustc,
        &counter,
        Some("toolchain-b"),
        false,
    );
    assert_build_success(&fallback);
    assert_binary_stdout(&output_a, "3");
    assert_eq!(
        compile_count(&counter),
        7,
        "unavailable cache must fall back to normal compilation"
    );

    let _ = fs::remove_dir_all(dir);
}
