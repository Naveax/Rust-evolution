use std::env;
use std::fs;
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    env::temp_dir().join(format!("evo-block-locals-{label}-{}-{nanos}", process::id()))
}

fn normalized_stdout(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

#[test]
fn block_locals_compile_and_run_natively() {
    let dir = temp_dir("native");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("block-locals.evo");
    let program = concat!(
        "fn choose(flag bool, x int) int\n",
        "if flag\n",
        "local = x + 1\n",
        "return local\n",
        "else\n",
        "local = x - 1\n",
        "return local\n",
        "end\n",
        "end\n",
        "x = 1\n",
        "if true\n",
        "inside = 2\n",
        "print inside\n",
        "x = x + 1\n",
        "end\n",
        "if false\n",
        "ignored = 99\n",
        "else\n",
        "inside = 3\n",
        "print inside\n",
        "end\n",
        "repeat 2\n",
        "temp = x + 1\n",
        "temp = temp + 1\n",
        "print temp\n",
        "x = x + 1\n",
        "end\n",
        "if true\n",
        "parent = 5\n",
        "if true\n",
        "child = parent + 1\n",
        "print child\n",
        "end\n",
        "print parent\n",
        "end\n",
        "print x\n",
        "print choose(true, 10)\n",
    );
    fs::write(&source, program).expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("run")
        .arg(&source)
        .output()
        .expect("evo run should execute");

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = normalized_stdout(&output.stdout);
    let _ = fs::remove_dir_all(&dir);

    assert!(output.status.success(), "evo run failed: {stderr}");
    assert_eq!(stdout, "2\n3\n4\n5\n6\n5\n4\n11\n");
}

#[test]
fn block_local_use_after_scope_is_a_source_native_error() {
    let dir = temp_dir("use-after-scope");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("use-after-scope.evo");
    fs::write(&source, "if true\ninside = 1\nend\nprint inside\n")
        .expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo check should execute");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let location = format!(" --> {}:4:7", source.display());
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert!(stderr.contains("outside its scope"), "{stderr}");
    assert!(stderr.contains(&location), "{stderr}");
    assert!(stderr.contains("4 | print inside"), "{stderr}");
    assert!(stderr.contains("  |       ^"), "{stderr}");
}

#[test]
fn functions_still_cannot_capture_top_level_locals() {
    let dir = temp_dir("top-level-capture");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("top-level-capture.evo");
    fs::write(
        &source,
        "fn read() int\nreturn top\nend\ntop = 1\nprint read()\n",
    )
    .expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo check should execute");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let location = format!(" --> {}:2:8", source.display());
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert!(stderr.contains("before definition or outside its scope"), "{stderr}");
    assert!(stderr.contains(&location), "{stderr}");
    assert!(stderr.contains("2 | return top"), "{stderr}");
}
