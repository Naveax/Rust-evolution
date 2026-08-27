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

fn runtime_record_source() -> &'static str {
    "record Point\nx int\ny int\nend\nfn sum(point Point) int\nreturn point.x + point.y\nend\npoint = Point(y = 2, x = 40)\nprint sum(point)\n"
}

#[test]
fn check_accepts_valid_records_after_production_lowering_lands() {
    let dir = temp_dir("records-check");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("point.evo");
    fs::write(&source, record_source()).expect("record source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo check should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = fs::remove_dir_all(&dir);

    assert!(output.status.success(), "{stderr}");
    assert_eq!(stdout, "ok\n");
    assert!(stderr.is_empty(), "{stderr}");
}

#[test]
fn emit_rust_uses_static_record_structs_without_runtime_scaffolding() {
    let dir = temp_dir("records-emit-rust");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("point.evo");
    fs::write(&source, runtime_record_source()).expect("record source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("emit-rust")
        .arg(&source)
        .output()
        .expect("evo emit-rust should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = fs::remove_dir_all(&dir);

    assert!(output.status.success(), "{stderr}");
    assert!(stdout.contains("struct __EvoRecord_Point {"), "{stdout}");
    assert!(stdout.contains("__evo_field_x: i64,"), "{stdout}");
    assert!(stdout.contains("__evo_field_y: i64,"), "{stdout}");
    assert!(
        stdout.contains(
            "__EvoRecord_Point { __evo_field_x: 40, __evo_field_y: 2 }"
        ),
        "{stdout}"
    );
    assert!(!stdout.contains(".clone()"), "{stdout}");
    assert!(!stdout.contains("Box<"), "{stdout}");
    assert!(!stdout.contains("HashMap"), "{stdout}");
}

#[test]
fn build_compiles_static_record_program_and_binary_runs() {
    let dir = temp_dir("records-build");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("point.evo");
    let binary = dir.join(format!("point{}", env::consts::EXE_SUFFIX));
    fs::write(&source, runtime_record_source()).expect("record source should be written");

    let build = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("build")
        .arg(&source)
        .arg(&binary)
        .output()
        .expect("evo build should run");

    let build_stderr = String::from_utf8_lossy(&build.stderr);
    assert!(build.status.success(), "{build_stderr}");
    assert!(binary.exists(), "record program should compile to a native binary");

    let run = Command::new(&binary)
        .output()
        .expect("compiled record binary should run");
    let run_stdout = String::from_utf8_lossy(&run.stdout);
    let run_stderr = String::from_utf8_lossy(&run.stderr);
    let _ = fs::remove_dir_all(&dir);

    assert!(run.status.success(), "{run_stderr}");
    assert_eq!(run_stdout, "42\n");
}

#[test]
fn check_reports_record_schema_error_before_codegen() {
    let dir = temp_dir("records-schema");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("invalid-record.evo");
    fs::write(&source, "record Point\nx int\nx bool\nend\n")
        .expect("invalid record source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo check should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let location = format!(" --> {}:3:1", source.display());
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert!(stderr.contains("duplicate field"), "{stderr}");
    assert!(stderr.contains(&location), "{stderr}");
    assert!(stderr.contains("3 | x bool"), "{stderr}");
    assert!(!stderr.contains("main.rs"), "{stderr}");
}

#[test]
fn check_reports_recursive_record_layout_at_field_span() {
    let dir = temp_dir("records-recursive");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("recursive-record.evo");
    fs::write(&source, "record Node\nnext Node\nend\n")
        .expect("recursive record source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo check should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let location = format!(" --> {}:2:1", source.display());
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert!(
        stderr.contains("recursive by-value record layout"),
        "{stderr}"
    );
    assert!(stderr.contains("Node -> Node"), "{stderr}");
    assert!(stderr.contains(&location), "{stderr}");
    assert!(stderr.contains("2 | next Node"), "{stderr}");
    assert!(!stderr.contains("main.rs"), "{stderr}");
}

#[test]
fn moved_record_diagnostic_stays_at_evolution_source_before_rustc() {
    let dir = temp_dir("records-move-error");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("moved-record.evo");
    fs::write(
        &source,
        "record Marker\nend\nfn bad(value Marker) Marker\nother = value\nreturn value\nend\n",
    )
    .expect("move-error source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo check should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert!(stderr.contains("use of moved record local"), "{stderr}");
    assert!(stderr.contains("5 | return value"), "{stderr}");
    assert!(!stderr.contains("rustc failed"), "{stderr}");
    assert!(!stderr.contains("main.rs"), "{stderr}");
}
