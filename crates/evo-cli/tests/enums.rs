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

fn native_enum_source() -> &'static str {
    concat!(
        "enum MaybeInt\n",
        "None\n",
        "Some int\n",
        "end\n",
        "fn unwrap(value MaybeInt) int\n",
        "match value\n",
        "case MaybeInt.None\n",
        "return 0\n",
        "case MaybeInt.Some(x)\n",
        "return x\n",
        "end\n",
        "end\n",
        "value = MaybeInt.Some(42)\n",
        "print unwrap(value)\n",
    )
}

#[test]
fn check_accepts_valid_promoted_enum_program() {
    let dir = temp_dir("enums-check-native");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("maybe-int.evo");
    fs::write(&source, native_enum_source()).expect("enum source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo check should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let _ = fs::remove_dir_all(&dir);

    assert!(output.status.success(), "{stderr}");
    assert_eq!(stdout, "ok\n");
    assert!(!stderr.contains("rustc failed"), "{stderr}");
    assert!(!stderr.contains("main.rs"), "{stderr}");
}

#[test]
fn emit_rust_uses_static_enum_and_match_without_runtime_scaffolding() {
    let dir = temp_dir("enums-emit-rust");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("maybe-int.evo");
    fs::write(&source, native_enum_source()).expect("enum source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("emit-rust")
        .arg(&source)
        .output()
        .expect("evo emit-rust should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let rust = String::from_utf8(output.stdout).expect("generated Rust should be UTF-8");
    let _ = fs::remove_dir_all(&dir);

    assert!(output.status.success(), "{stderr}");
    assert!(rust.contains("enum __EvoEnum_MaybeInt {"), "{rust}");
    assert!(rust.contains("__EvoVariant_None,"), "{rust}");
    assert!(rust.contains("__EvoVariant_Some(i64),"), "{rust}");
    assert!(
        rust.contains("__EvoEnum_MaybeInt::__EvoVariant_Some(__evo_x) => {"),
        "{rust}"
    );
    assert!(!rust.contains(".clone("), "{rust}");
    assert!(!rust.contains("Box<"), "{rust}");
    assert!(!rust.contains("Rc<"), "{rust}");
    assert!(!rust.contains("Arc<"), "{rust}");
    assert!(!rust.contains("HashMap"), "{rust}");
}

#[test]
fn build_and_run_compile_static_enum_program_natively() {
    let dir = temp_dir("enums-native-build-run");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("maybe-int.evo");
    let binary = dir.join(format!("maybe-int{}", env::consts::EXE_SUFFIX));
    fs::write(&source, native_enum_source()).expect("enum source should be written");

    let build = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("build")
        .arg(&source)
        .arg(&binary)
        .output()
        .expect("evo build should run");
    let build_stderr = String::from_utf8_lossy(&build.stderr);
    assert!(build.status.success(), "{build_stderr}");
    assert!(binary.exists(), "enum build should produce a native binary");

    let binary_run = Command::new(&binary)
        .output()
        .expect("compiled enum binary should run");
    let binary_stderr = String::from_utf8_lossy(&binary_run.stderr);
    assert!(binary_run.status.success(), "{binary_stderr}");
    assert_eq!(binary_run.stdout, b"42\n");

    let cli_run = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("run")
        .arg(&source)
        .output()
        .expect("evo run should compile and run enum source");
    let cli_stderr = String::from_utf8_lossy(&cli_run.stderr);
    assert!(cli_run.status.success(), "{cli_stderr}");
    assert_eq!(cli_run.stdout, b"42\n");

    let _ = fs::remove_dir_all(&dir);
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
fn check_rejects_enum_reuse_after_move_before_codegen() {
    assert_check_fails_before_rustc(
        "enum-ownership-reuse",
        "reuse-after-move.evo",
        "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nfirst = value\nsecond = value\n",
        "moved enum local \"value\"",
        7,
        10,
    );
}

#[test]
fn check_rejects_enum_reuse_after_owned_match_before_codegen() {
    assert_check_fails_before_rustc(
        "enum-ownership-match-reuse",
        "reuse-after-match.evo",
        "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nmatch value\ncase Flag.Off\nprint 0\ncase Flag.On\nprint 1\nend\nagain = value\n",
        "moved enum local \"value\"",
        12,
        9,
    );
}

#[test]
fn check_rejects_record_payload_binding_reuse_before_codegen() {
    assert_check_fails_before_rustc(
        "enum-ownership-record-payload",
        "record-payload-reuse.evo",
        "record Item\nvalue int\nend\nenum Wrapped\nNone\nSome Item\nend\nfn use(value Wrapped) int\nmatch value\ncase Wrapped.None\nreturn 0\ncase Wrapped.Some(x)\nfirst = x\nsecond = x\nreturn 1\nend\nend\n",
        "moved record local \"x\"",
        14,
        10,
    );
}

#[test]
fn check_rejects_enum_return_after_move_before_codegen() {
    assert_check_fails_before_rustc(
        "enum-ownership-return",
        "return-after-move.evo",
        "enum Flag\nOff\nOn\nend\nfn consume(value Flag) Flag\nfirst = value\nreturn value\nend\n",
        "moved enum local \"value\"",
        7,
        8,
    );
}

#[test]
fn check_rejects_nonreusable_nominal_field_move_before_codegen() {
    assert_check_fails_before_rustc(
        "enum-ownership-field-move",
        "nominal-field-move.evo",
        "enum Flag\nOff\nOn\nend\nrecord Holder\nvalue Flag\nend\nholder = Holder(value = Flag.On())\nextracted = holder.value\n",
        "moving nominal field \"value\" out of an expression is not supported in Enums v0 ownership; no implicit clone is inserted",
        9,
        13,
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
