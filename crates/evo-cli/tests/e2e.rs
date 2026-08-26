use std::env;
use std::fs;
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    env::temp_dir().join(format!("evo-{label}-{}-{nanos}", process::id()))
}

#[test]
fn evolution_source_compiles_and_runs_natively() {
    let dir = temp_dir("e2e");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("basic.evo");
    fs::write(&source, "x = 1\ny = 1\nprint x + y\n").expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("run")
        .arg(&source)
        .output()
        .expect("evo should run");

    let _ = fs::remove_dir_all(&dir);
    assert!(
        output.status.success(),
        "evo failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "2");
}

#[test]
fn check_rejects_invalid_source_with_parser_context() {
    let dir = temp_dir("invalid-parser");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("invalid.evo");
    fs::write(&source, "x 1\n").expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let location = format!(" --> {}:1:3", source.display());
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert!(stderr.contains("error: expected '='"), "{stderr}");
    assert!(stderr.contains(&location), "{stderr}");
    assert!(stderr.contains("1 | x 1"), "{stderr}");
    assert!(stderr.contains("  |   ^"), "{stderr}");
}

#[test]
fn check_reports_multiple_parser_errors_in_one_run() {
    let dir = temp_dir("multi-parser");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("multiple-invalid.evo");
    fs::write(&source, "x 1\ny 2\n").expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let first_location = format!(" --> {}:1:3", source.display());
    let second_location = format!(" --> {}:2:3", source.display());
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert_eq!(stderr.matches("error: expected '='").count(), 2, "{stderr}");
    assert!(stderr.contains(&first_location), "{stderr}");
    assert!(stderr.contains(&second_location), "{stderr}");
    assert!(stderr.contains("1 | x 1"), "{stderr}");
    assert!(stderr.contains("2 | y 2"), "{stderr}");
}

#[test]
fn build_stops_before_rustc_when_parser_errors_exist() {
    let dir = temp_dir("parser-before-rustc");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("invalid-build.evo");
    let binary = dir.join("must-not-exist");
    fs::write(&source, "x 1\ny 2\n").expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("build")
        .arg(&source)
        .arg(&binary)
        .env("RUSTC", "evo-rustc-must-not-run")
        .output()
        .expect("evo build should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert_eq!(stderr.matches("error: expected '='").count(), 2, "{stderr}");
    assert!(!stderr.contains("failed to execute rustc"), "{stderr}");
    assert!(!binary.exists(), "parser failure must not produce a binary");
}

#[test]
fn check_renders_lowering_error_at_evolution_source() {
    let dir = temp_dir("invalid-lowering");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("use-before-definition.evo");
    fs::write(&source, "x = x + 1\n").expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let location = format!(" --> {}:1:5", source.display());
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert!(stderr.contains("before definition"), "{stderr}");
    assert!(stderr.contains(&location), "{stderr}");
    assert!(stderr.contains("1 | x = x + 1"), "{stderr}");
    assert!(stderr.contains("  |     ^"), "{stderr}");
}

#[test]
fn build_remaps_rustc_compile_error_to_evolution_source() {
    let dir = temp_dir("rustc-build-remap");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("division-by-zero.evo");
    let binary = dir.join("division-by-zero-output");
    fs::write(&source, "print 1 / 0\n").expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("build")
        .arg(&source)
        .arg(&binary)
        .output()
        .expect("evo build should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let location = format!(" --> {}:1:1", source.display());
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success(), "build unexpectedly succeeded");
    assert!(stderr.starts_with("error: "), "{stderr}");
    assert!(stderr.contains(&location), "{stderr}");
    assert!(stderr.contains("1 | print 1 / 0"), "{stderr}");
    assert!(stderr.contains("  | ^"), "{stderr}");
    assert!(!stderr.contains("main.rs:"), "{stderr}");
    assert!(!stderr.contains("rustc failed:"), "{stderr}");
}

#[test]
fn run_uses_the_same_rustc_error_remap_path() {
    let dir = temp_dir("rustc-run-remap");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("division-by-zero.evo");
    fs::write(&source, "print 1 / 0\n").expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("run")
        .arg(&source)
        .output()
        .expect("evo run should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let location = format!(" --> {}:1:1", source.display());
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success(), "run unexpectedly succeeded");
    assert!(stderr.starts_with("error: "), "{stderr}");
    assert!(stderr.contains(&location), "{stderr}");
    assert!(stderr.contains("1 | print 1 / 0"), "{stderr}");
    assert!(!stderr.contains("main.rs:"), "{stderr}");
    assert!(!stderr.contains("rustc failed:"), "{stderr}");
}
