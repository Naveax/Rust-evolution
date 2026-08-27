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
    column: usize,
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
    let location = format!(" --> {}:{line}:{column}", source.display());
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
        9,
    );
}

#[test]
fn check_rejects_unknown_enum_variant_before_codegen() {
    assert_check_fails_before_rustc(
        "enum-unknown-variant",
        "unknown-variant.evo",
        "enum MaybeInt\nNone\nSome int\nend\nvalue = MaybeInt.Missing()\n",
        "unknown variant \"Missing\" for enum \"MaybeInt\"",
        5,
        9,
    );
}

#[test]
fn check_rejects_wrong_enum_payload_type_before_codegen() {
    assert_check_fails_before_rustc(
        "enum-payload-type",
        "payload-type.evo",
        "enum MaybeInt\nNone\nSome int\nend\nvalue = MaybeInt.Some(true)\n",
        "expects int, found bool",
        5,
        23,
    );
}

#[test]
fn check_rejects_non_exhaustive_enum_match_before_codegen() {
    assert_check_fails_before_rustc(
        "enum-match-exhaustive",
        "non-exhaustive-match.evo",
        "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nmatch value\ncase Flag.On\nprint 1\nend\n",
        "missing variant(s): Off",
        6,
        1,
    );
}

#[test]
fn check_rejects_invalid_match_payload_binding_before_codegen() {
    assert_check_fails_before_rustc(
        "enum-match-binding",
        "invalid-match-binding.evo",
        "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nmatch value\ncase Flag.On(value)\nprint value\ncase Flag.Off\nprint 0\nend\n",
        "cannot bind a payload",
        7,
        6,
    );
}

#[test]
fn check_rejects_non_enum_match_scrutinee_before_codegen() {
    assert_check_fails_before_rustc(
        "enum-match-scrutinee",
        "non-enum-scrutinee.evo",
        "enum Flag\nOff\nOn\nend\nmatch true\ncase Flag.Off\nprint 0\ncase Flag.On\nprint 1\nend\n",
        "scrutinee must have an enum type",
        5,
        7,
    );
}

#[test]
fn check_rejects_match_arm_from_wrong_enum_before_codegen() {
    assert_check_fails_before_rustc(
        "enum-match-wrong-arm",
        "wrong-enum-arm.evo",
        "enum Left\nOne\nend\nenum Right\nOther\nend\nvalue = Left.One()\nmatch value\ncase Right.Other\nprint 0\nend\n",
        "scrutinee has enum type \"Left\"",
        9,
        6,
    );
}

#[test]
fn check_rejects_match_payload_binding_scope_escape_before_codegen() {
    assert_check_fails_before_rustc(
        "enum-match-scope",
        "match-binding-scope.evo",
        "enum MaybeInt\nNone\nSome int\nend\nvalue = MaybeInt.Some(1)\nmatch value\ncase MaybeInt.None\nprint 0\ncase MaybeInt.Some(x)\nprint x\nend\nprint x\n",
        "outside its scope",
        12,
        7,
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
        1,
    );
}
