use std::env;
use std::fs;
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn evolution_source_compiles_and_runs_natively() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    let dir = env::temp_dir().join(format!("evo-e2e-{}-{nanos}", process::id()));
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
fn check_rejects_invalid_source() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    let dir = env::temp_dir().join(format!("evo-invalid-{}-{nanos}", process::id()));
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("invalid.evo");
    fs::write(&source, "x 1\n").expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo should run");

    let _ = fs::remove_dir_all(&dir);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected '='"));
}
