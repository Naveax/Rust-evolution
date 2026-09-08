use std::env;
use std::fs;
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    env::temp_dir().join(format!("evo-suggestions-{label}-{}-{nanos}", process::id()))
}

fn check_failure(label: &str, source_text: &str) -> String {
    let dir = temp_dir(label);
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("input.evo");
    fs::write(&source, source_text).expect("suggestion source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo check should run");
    let stderr = String::from_utf8(output.stderr).expect("diagnostic output should be UTF-8");
    let _ = fs::remove_dir_all(&dir);

    assert!(
        !output.status.success(),
        "source unexpectedly succeeded: {source_text}"
    );
    assert!(!stderr.contains("main.rs"), "{stderr}");
    assert!(!stderr.contains("rustc failed"), "{stderr}");
    stderr
}

fn assert_help(label: &str, source: &str, primary: &str, candidate: &str) {
    let stderr = check_failure(label, source);
    assert!(stderr.contains(primary), "{stderr}");
    assert!(
        stderr.contains(&format!("help: did you mean {candidate:?}?")),
        "{stderr}"
    );
}

fn assert_no_help(label: &str, source: &str, primary: &str) {
    let stderr = check_failure(label, source);
    assert!(stderr.contains(primary), "{stderr}");
    assert!(!stderr.contains("help:"), "{stderr}");
}

#[test]
fn close_local_function_and_record_names_render_source_native_help() {
    assert_help(
        "local",
        "count = 1\nprint coutn\n",
        "use of local \"coutn\"",
        "count",
    );
    assert_help(
        "function",
        "fn count() int\nreturn 1\nend\nprint coutn()\n",
        "unknown function \"coutn\"",
        "count",
    );
    assert_help(
        "record-constructor",
        "record Point\ncount int\nend\nvalue = Piont(count = 1)\n",
        "unknown record constructor \"Piont\"",
        "Point",
    );
    assert_help(
        "record-field",
        "record Point\ncount int\nend\nvalue = Point(count = 1)\nprint value.coutn\n",
        "unknown field \"coutn\" on record \"Point\"",
        "count",
    );
}

#[test]
fn enum_constructor_variant_match_and_nominal_typos_render_help() {
    assert_help(
        "enum-constructor",
        "enum Flag\nOff\nOn\nend\nvalue = Flga.On()\n",
        "unknown enum constructor \"Flga\"",
        "Flag",
    );
    assert_help(
        "enum-variant",
        "enum State\nDisabled\nEnabled\nend\nvalue = State.Enabeld()\n",
        "unknown variant \"Enabeld\" for enum \"State\"",
        "Enabled",
    );
    assert_help(
        "match-variant",
        "enum State\nDisabled\nEnabled\nend\nvalue = State.Enabled()\nmatch value\ncase State.Disabled\nprint 0\ncase State.Enabeld\nprint 1\nend\n",
        "unknown variant \"Enabeld\" for enum \"State\" in match pattern",
        "Enabled",
    );
    assert_help(
        "nominal-signature",
        "enum MaybeInt\nNone\nSome int\nend\nfn use(value MabyInt) int\nreturn 0\nend\n",
        "unknown nominal type \"MabyInt\" in function signature",
        "MaybeInt",
    );
}

#[test]
fn ambiguous_distant_and_out_of_scope_names_stay_silent() {
    assert_no_help(
        "tie",
        "cat = 1\ncut = 2\nprint cot\n",
        "use of local \"cot\"",
    );
    assert_no_help(
        "distant",
        "count = 1\nprint missing\n",
        "use of local \"missing\"",
    );
    assert_no_help(
        "out-of-scope",
        "if true\ntemporary = 1\nend\nprint temporay\n",
        "use of local \"temporay\"",
    );
}

#[test]
fn wrong_namespace_does_not_offer_a_record_as_a_function() {
    assert_no_help(
        "wrong-namespace",
        "record Point\nend\nprint Piont()\n",
        "unknown function \"Piont\"",
    );
}
