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

fn record_source() -> &'static str {
    "record Point\nx int\ny int\nend\nprint 1\n"
}

#[test]
fn check_rejects_records_at_evolution_source_until_semantic_lowering_lands() {
    let dir = temp_dir("records-check");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("point.evo");
    fs::write(&source, record_source()).expect("record source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo check should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let location = format!(" --> {}:1:1", source.display());
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert!(stderr.contains("Records v0 semantic lowering"), "{stderr}");
    assert!(stderr.contains(&location), "{stderr}");
    assert!(stderr.contains("1 | record Point"), "{stderr}");
    assert!(!stderr.contains("main.rs"), "{stderr}");
}

#[test]
fn build_stops_before_rustc_for_parsed_records() {
    let dir = temp_dir("records-build");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("point.evo");
    let binary = dir.join(format!("point{}", env::consts::EXE_SUFFIX));
    fs::write(&source, record_source()).expect("record source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("build")
        .arg(&source)
        .arg(&binary)
        .output()
        .expect("evo build should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let binary_exists = binary.exists();
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert!(stderr.contains("Records v0 semantic lowering"), "{stderr}");
    assert!(!stderr.contains("rustc failed"), "{stderr}");
    assert!(!binary_exists, "record source must not reach native compilation");
}
