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

fn build_and_run(label: &str, source_text: &str) -> String {
    let dir = temp_dir(label);
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("program.evo");
    let binary = dir.join(format!("program{}", env::consts::EXE_SUFFIX));
    fs::write(&source, source_text).expect("record source should be written");

    let build = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("build")
        .arg(&source)
        .arg(&binary)
        .output()
        .expect("evo build should run");
    let build_stderr = String::from_utf8_lossy(&build.stderr);
    assert!(build.status.success(), "{build_stderr}");
    assert!(
        binary.exists(),
        "record program should produce a native binary"
    );

    let run = Command::new(&binary)
        .output()
        .expect("compiled record binary should run");
    let run_stderr = String::from_utf8_lossy(&run.stderr);
    assert!(run.status.success(), "{run_stderr}");
    let stdout = String::from_utf8(run.stdout).expect("record stdout should be UTF-8");

    let _ = fs::remove_dir_all(&dir);
    stdout
}

#[test]
fn native_nested_record_and_chained_scalar_access_work() {
    let source = concat!(
        "record Point\n",
        "x int\n",
        "end\n",
        "record Wrapper\n",
        "point Point\n",
        "end\n",
        "fn read(wrapper Wrapper) int\n",
        "return wrapper.point.x\n",
        "end\n",
        "wrapper = Wrapper(point = Point(x = 42))\n",
        "print read(wrapper)\n",
    );

    assert_eq!(build_and_run("records-nested", source), "42\n");
}

#[test]
fn native_zero_field_record_roundtrips_through_record_return() {
    let source = concat!(
        "record Marker\n",
        "end\n",
        "fn pass(value Marker) Marker\n",
        "return value\n",
        "end\n",
        "fn consume(value Marker) int\n",
        "return 7\n",
        "end\n",
        "marker = pass(Marker())\n",
        "print consume(marker)\n",
    );

    assert_eq!(build_and_run("records-zero-field", source), "7\n");
}

#[test]
fn native_record_can_be_reinitialized_after_move() {
    let source = concat!(
        "record Point\n",
        "x int\n",
        "end\n",
        "fn take(point Point) int\n",
        "return point.x\n",
        "end\n",
        "point = Point(x = 1)\n",
        "first = take(point)\n",
        "point = Point(x = 41)\n",
        "print first + take(point)\n",
    );

    assert_eq!(build_and_run("records-reinit", source), "42\n");
}

#[test]
fn rejected_moved_record_build_produces_no_native_binary() {
    let dir = temp_dir("records-rejected-build");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("moved.evo");
    let binary = dir.join(format!("moved{}", env::consts::EXE_SUFFIX));
    fs::write(
        &source,
        "record Marker\nend\nfn bad(value Marker) Marker\nother = value\nreturn value\nend\n",
    )
    .expect("move-error source should be written");

    let build = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("build")
        .arg(&source)
        .arg(&binary)
        .output()
        .expect("evo build should run");
    let stderr = String::from_utf8_lossy(&build.stderr);
    let binary_exists = binary.exists();
    let _ = fs::remove_dir_all(&dir);

    assert!(!build.status.success());
    assert!(
        !binary_exists,
        "rejected record program must not produce a binary"
    );
    assert!(stderr.contains("use of moved record local"), "{stderr}");
    assert!(stderr.contains("5 | return value"), "{stderr}");
    assert!(!stderr.contains("rustc failed"), "{stderr}");
    assert!(!stderr.contains("main.rs"), "{stderr}");
}
