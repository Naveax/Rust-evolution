use std::env;
use std::fs;
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    env::temp_dir().join(format!("evo-fmt-{label}-{}-{nanos}", process::id()))
}

#[test]
fn fmt_formats_in_place_and_check_accepts_canonical_file() {
    let dir = temp_dir("in-place");
    fs::create_dir_all(&dir).expect("temp directory should be created");
    let source = dir.join("sample.evo");
    fs::write(&source, "repeat 2# outer\nx= -1\nprint(x+2)# value\nend")
        .expect("source should be written");

    let format = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("fmt")
        .arg(&source)
        .output()
        .expect("evo fmt should run");
    assert!(
        format.status.success(),
        "formatter failed: {}",
        String::from_utf8_lossy(&format.stderr)
    );

    let formatted = fs::read_to_string(&source).expect("formatted source should be readable");
    assert_eq!(
        formatted,
        concat!(
            "repeat 2  # outer\n",
            "    x = -1\n",
            "    print (x + 2)  # value\n",
            "end\n"
        )
    );

    let check = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("fmt")
        .arg(&source)
        .arg("--check")
        .output()
        .expect("evo fmt --check should run");
    let _ = fs::remove_dir_all(&dir);

    assert!(check.status.success());
    assert_eq!(String::from_utf8_lossy(&check.stdout).trim(), "ok");
}

#[test]
fn fmt_check_rejects_noncanonical_source_without_rewriting() {
    let dir = temp_dir("check-fail");
    fs::create_dir_all(&dir).expect("temp directory should be created");
    let source = dir.join("sample.evo");
    let original = "x=1\n";
    fs::write(&source, original).expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("fmt")
        .arg(&source)
        .arg("--check")
        .output()
        .expect("evo fmt --check should run");
    let after = fs::read_to_string(&source).expect("source should be readable");
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert_eq!(after, original);
    assert!(String::from_utf8_lossy(&output.stderr).contains("is not formatted"));
}

#[test]
fn fmt_invalid_syntax_does_not_rewrite_source() {
    let dir = temp_dir("invalid");
    fs::create_dir_all(&dir).expect("temp directory should be created");
    let source = dir.join("invalid.evo");
    let original = "x 1\ny 2\n";
    fs::write(&source, original).expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("fmt")
        .arg(&source)
        .output()
        .expect("evo fmt should run");
    let after = fs::read_to_string(&source).expect("source should be readable");
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert_eq!(after, original);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.matches("error: expected '='").count(), 2, "{stderr}");
}

#[test]
fn fmt_does_not_require_semantic_lowering_success() {
    let dir = temp_dir("syntax-only");
    fs::create_dir_all(&dir).expect("temp directory should be created");
    let source = dir.join("semantic-error.evo");
    fs::write(&source, "x=x+1").expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("fmt")
        .arg(&source)
        .output()
        .expect("evo fmt should run");
    let formatted = fs::read_to_string(&source).expect("source should be readable");
    let _ = fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "formatter should be syntax-only: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(formatted, "x = x + 1\n");
}
