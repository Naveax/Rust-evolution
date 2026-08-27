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

fn assert_check_fails_before_rustc(
    label: &str,
    filename: &str,
    source_text: &str,
    message: &str,
    line: usize,
) {
    let dir = temp_dir(label);
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join(filename);
    fs::write(&source, source_text).expect("enum source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo check should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let location = format!(" --> {}:{line}:1", source.display());
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert!(stderr.contains(message), "{stderr}");
    assert!(stderr.contains(&location), "{stderr}");
    assert!(!stderr.contains("main.rs"), "{stderr}");
    assert!(!stderr.contains("rustc failed"), "{stderr}");
}

#[test]
fn check_keeps_parsed_enum_declarations_fail_closed_before_codegen() {
    assert_check_fails_before_rustc(
        "enums-check-gate",
        "maybe-int.evo",
        "enum MaybeInt\nNone\nSome int\nend\nprint 1\n",
        "Enums v0 semantic lowering",
        1,
    );
}

#[test]
fn check_keeps_qualified_enum_constructors_fail_closed_before_codegen() {
    assert_check_fails_before_rustc(
        "enum-constructor-check-gate",
        "constructor.evo",
        "value = MaybeInt.Some(41)\n",
        "enum variant constructors are parsed",
        1,
    );
}

#[test]
fn check_keeps_match_statements_fail_closed_before_codegen() {
    assert_check_fails_before_rustc(
        "enum-match-check-gate",
        "match.evo",
        "value = 1\nmatch value\ncase MaybeInt.None\nprint 0\nend\n",
        "match statements are parsed",
        2,
    );
}
