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
    fs::write(&source, source_text).expect("enum source should be written");

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
        "enum program should produce a native binary"
    );

    let run = Command::new(&binary)
        .output()
        .expect("compiled enum binary should run");
    let run_stderr = String::from_utf8_lossy(&run.stderr);
    assert!(run.status.success(), "{run_stderr}");
    let stdout = String::from_utf8(run.stdout).expect("enum stdout should be UTF-8");

    let _ = fs::remove_dir_all(&dir);
    stdout
}

#[test]
fn native_unit_only_enum_matches_each_variant() {
    let source = concat!(
        "enum Switch\n",
        "Off\n",
        "On\n",
        "end\n",
        "fn read(value Switch) int\n",
        "match value\n",
        "case Switch.Off\n",
        "return 0\n",
        "case Switch.On\n",
        "return 1\n",
        "end\n",
        "end\n",
        "print read(Switch.Off())\n",
        "print read(Switch.On())\n",
    );

    assert_eq!(build_and_run("enums-unit-only", source), "0\n1\n");
}

#[test]
fn native_record_payload_is_bound_and_read_in_arm() {
    let source = concat!(
        "record Item\n",
        "value int\n",
        "end\n",
        "enum MaybeItem\n",
        "None\n",
        "Some Item\n",
        "end\n",
        "fn read(value MaybeItem) int\n",
        "match value\n",
        "case MaybeItem.None\n",
        "return 0\n",
        "case MaybeItem.Some(item)\n",
        "return item.value\n",
        "end\n",
        "end\n",
        "print read(MaybeItem.Some(Item(value = 42)))\n",
    );

    assert_eq!(build_and_run("enums-record-payload", source), "42\n");
}

#[test]
fn native_enum_parameter_and_return_roundtrip() {
    let source = concat!(
        "enum Flag\n",
        "Off\n",
        "On\n",
        "end\n",
        "fn pass(value Flag) Flag\n",
        "return value\n",
        "end\n",
        "fn read(value Flag) int\n",
        "match value\n",
        "case Flag.Off\n",
        "return 0\n",
        "case Flag.On\n",
        "return 42\n",
        "end\n",
        "end\n",
        "print read(pass(Flag.On()))\n",
    );

    assert_eq!(build_and_run("enums-return-roundtrip", source), "42\n");
}

#[test]
fn native_nested_if_and_match_preserve_control_flow() {
    let source = concat!(
        "enum Inner\n",
        "Zero\n",
        "Value int\n",
        "end\n",
        "enum Outer\n",
        "Empty\n",
        "Wrapped Inner\n",
        "end\n",
        "fn decode(value Outer) int\n",
        "if true\n",
        "match value\n",
        "case Outer.Empty\n",
        "return 0\n",
        "case Outer.Wrapped(inner)\n",
        "match inner\n",
        "case Inner.Zero\n",
        "return 1\n",
        "case Inner.Value(x)\n",
        "return x\n",
        "end\n",
        "end\n",
        "else\n",
        "return 9\n",
        "end\n",
        "end\n",
        "print decode(Outer.Wrapped(Inner.Value(42)))\n",
    );

    assert_eq!(build_and_run("enums-nested-control", source), "42\n");
}

#[test]
fn invalid_match_build_never_reaches_rustc_or_produces_binary() {
    let dir = temp_dir("enums-invalid-match-build");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("invalid-match.evo");
    let binary = dir.join(format!("invalid-match{}", env::consts::EXE_SUFFIX));
    fs::write(
        &source,
        "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nmatch value\ncase Flag.On\nprint 1\nend\n",
    )
    .expect("invalid enum source should be written");

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
        "invalid enum match must not produce a binary"
    );
    assert!(stderr.contains("missing variant(s): Off"), "{stderr}");
    assert!(!stderr.contains("rustc failed"), "{stderr}");
    assert!(!stderr.contains("main.rs"), "{stderr}");
}
