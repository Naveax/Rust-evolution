use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    env::temp_dir().join(format!("evo-fast-run-{label}-{}-{nanos}", process::id()))
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

fn evo_command(
    source: &Path,
    cache_dir: &Path,
    wrapper: &Path,
    rustc: &OsStr,
    counter: &Path,
    fingerprint_salt: Option<&str>,
    no_cache: bool,
) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_evo"));
    command
        .arg("run")
        .arg(source)
        .env("EVO_CACHE_DIR", cache_dir)
        .env("RUSTC", wrapper)
        .env("EVO_TEST_REAL_RUSTC", rustc)
        .env("EVO_TEST_RUSTC_COUNT", counter);
    if let Some(salt) = fingerprint_salt {
        command.env("EVO_TEST_FINGERPRINT_SALT", salt);
    }
    if no_cache {
        command.arg("--no-cache");
    }
    command
}

fn run_evo(
    source: &Path,
    cache_dir: &Path,
    wrapper: &Path,
    rustc: &OsStr,
    counter: &Path,
    fingerprint_salt: Option<&str>,
    no_cache: bool,
) -> Output {
    evo_command(
        source,
        cache_dir,
        wrapper,
        rustc,
        counter,
        fingerprint_salt,
        no_cache,
    )
    .output()
    .expect("evo run should execute")
}

fn run_evo_with_input(
    source: &Path,
    cache_dir: &Path,
    wrapper: &Path,
    rustc: &OsStr,
    counter: &Path,
    fingerprint_salt: Option<&str>,
    input: &str,
) -> Output {
    let mut command = evo_command(
        source,
        cache_dir,
        wrapper,
        rustc,
        counter,
        fingerprint_salt,
        false,
    );
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("evo run should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(input.as_bytes())
        .expect("stdin should be writable");
    child.wait_with_output().expect("evo run should finish")
}

fn compile_count(counter: &Path) -> u64 {
    fs::read_to_string(counter)
        .expect("counter should exist")
        .trim()
        .parse()
        .expect("counter should contain an integer")
}

fn assert_success_with_stdout(output: &Output, expected: &str) {
    assert!(
        output.status.success(),
        "evo failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), expected);
}

fn first_cache_entry(cache_dir: &Path) -> PathBuf {
    fs::read_dir(cache_dir.join("run-cache-v0"))
        .expect("run cache should exist")
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with("entry-"))
        .expect("at least one cache entry should exist")
        .path()
}

#[test]
fn unchanged_run_reuses_verified_binary_and_invalidates_safely() {
    let dir = temp_dir("cache");
    let cache_dir = dir.join("cache");
    let counter = dir.join("rustc-count.txt");
    let source = dir.join("program.evo");
    fs::create_dir_all(&dir).expect("test directory should be created");
    fs::write(&counter, "0").expect("counter should be initialized");
    fs::write(&source, "print 2\n").expect("source should be written");

    let rustc = real_rustc();
    let wrapper = compile_rustc_wrapper(&dir, &rustc);

    let first = run_evo(
        &source, &cache_dir, &wrapper, &rustc, &counter, None, false,
    );
    assert_success_with_stdout(&first, "2");
    assert_eq!(compile_count(&counter), 1, "cold run must compile once");

    let second = run_evo(
        &source, &cache_dir, &wrapper, &rustc, &counter, None, false,
    );
    assert_success_with_stdout(&second, "2");
    assert_eq!(
        compile_count(&counter),
        1,
        "unchanged warm run must not invoke rustc compilation"
    );

    let entry = first_cache_entry(&cache_dir);
    fs::write(entry.join("compiler.txt"), "corrupt")
        .expect("cache metadata should be corruptible for the regression test");
    let after_corruption = run_evo(
        &source, &cache_dir, &wrapper, &rustc, &counter, None, false,
    );
    assert_success_with_stdout(&after_corruption, "2");
    assert_eq!(
        compile_count(&counter),
        2,
        "corrupted metadata must fail closed to recompilation"
    );

    fs::write(&source, "print @\n").expect("invalid source should be written");
    let invalid = run_evo(
        &source, &cache_dir, &wrapper, &rustc, &counter, None, false,
    );
    assert!(!invalid.status.success(), "invalid frontend source must fail");
    assert!(
        String::from_utf8_lossy(&invalid.stderr).contains("unexpected character"),
        "{}",
        String::from_utf8_lossy(&invalid.stderr)
    );
    assert_eq!(
        compile_count(&counter),
        2,
        "frontend validation must run before any cached executable is considered"
    );

    fs::write(&source, "print 2\n").expect("original source should be restored");
    let restored = run_evo(
        &source, &cache_dir, &wrapper, &rustc, &counter, None, false,
    );
    assert_success_with_stdout(&restored, "2");
    assert_eq!(
        compile_count(&counter),
        2,
        "restoring exact validated source should reuse the valid replacement entry"
    );

    fs::write(&source, "print 3\n").expect("changed source should be written");
    let source_changed = run_evo(
        &source, &cache_dir, &wrapper, &rustc, &counter, None, false,
    );
    assert_success_with_stdout(&source_changed, "3");
    assert_eq!(
        compile_count(&counter),
        3,
        "source changes must invalidate the compile cache"
    );

    let fingerprint_changed = run_evo(
        &source,
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        Some("toolchain-b"),
        false,
    );
    assert_success_with_stdout(&fingerprint_changed, "3");
    assert_eq!(
        compile_count(&counter),
        4,
        "compiler fingerprint changes must invalidate the cache"
    );

    let fingerprint_warm = run_evo(
        &source,
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        Some("toolchain-b"),
        false,
    );
    assert_success_with_stdout(&fingerprint_warm, "3");
    assert_eq!(
        compile_count(&counter),
        4,
        "unchanged source under the new compiler fingerprint must warm-hit"
    );

    let bypassed = run_evo(
        &source,
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        Some("toolchain-b"),
        true,
    );
    assert_success_with_stdout(&bypassed, "3");
    assert_eq!(
        compile_count(&counter),
        5,
        "--no-cache must force a real compilation"
    );

    fs::write(&source, "value = input_int\nprint value\n")
        .expect("input program should be written");
    let input_cold = run_evo_with_input(
        &source,
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        Some("toolchain-b"),
        "41\n",
    );
    assert_success_with_stdout(&input_cold, "41");
    assert_eq!(compile_count(&counter), 6);

    let input_warm = run_evo_with_input(
        &source,
        &cache_dir,
        &wrapper,
        &rustc,
        &counter,
        Some("toolchain-b"),
        "42\n",
    );
    assert_success_with_stdout(&input_warm, "42");
    assert_eq!(
        compile_count(&counter),
        6,
        "cached executable must keep runtime stdin behavior without recompiling"
    );

    let _ = fs::remove_dir_all(dir);
}