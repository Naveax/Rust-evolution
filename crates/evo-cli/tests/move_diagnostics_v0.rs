use std::env;
use std::fs;
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    env::temp_dir().join(format!("evo-move-diagnostics-{label}-{}-{nanos}", process::id()))
}

fn check(label: &str, source_text: &str) -> (bool, String, String) {
    let dir = temp_dir(label);
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("case.evo");
    fs::write(&source, source_text).expect("move-diagnostic source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo check should run");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let path = source.display().to_string();
    let _ = fs::remove_dir_all(&dir);
    (success, stderr, path)
}

fn assert_location(stderr: &str, path: &str, line: usize) {
    let location = format!(" --> {path}:{line}:");
    assert!(stderr.contains(&location), "missing {location:?} in:\n{stderr}");
}

#[test]
fn direct_record_reuse_shows_the_original_move_site() {
    let (success, stderr, path) = check(
        "record-direct",
        "record Marker\nend\nfn bad(value Marker) Marker\nother = value\nreturn value\nend\n",
    );

    assert!(!success);
    assert!(stderr.contains("use of moved record local \"value\""), "{stderr}");
    assert_location(&stderr, &path, 5);
    assert_location(&stderr, &path, 4);
    assert!(stderr.contains("5 | return value"), "{stderr}");
    assert!(stderr.contains("note: value was moved here"), "{stderr}");
    assert!(stderr.contains("4 | other = value"), "{stderr}");
    assert!(!stderr.contains("main.rs"), "{stderr}");
}

#[test]
fn direct_enum_reuse_shows_the_original_move_site() {
    let (success, stderr, path) = check(
        "enum-direct",
        "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nfirst = value\nsecond = value\n",
    );

    assert!(!success);
    assert!(stderr.contains("use of moved enum local \"value\""), "{stderr}");
    assert_location(&stderr, &path, 7);
    assert_location(&stderr, &path, 6);
    assert!(stderr.contains("note: value was moved here"), "{stderr}");
    assert!(stderr.contains("6 | first = value"), "{stderr}");
}

#[test]
fn function_argument_move_identifies_the_consuming_argument_expression() {
    let (success, stderr, path) = check(
        "function-argument",
        "enum Flag\nOff\nOn\nend\nfn take(value Flag) int\nreturn 0\nend\nvalue = Flag.On()\nfirst = take(value)\nsecond = take(value)\n",
    );

    assert!(!success);
    assert_location(&stderr, &path, 10);
    assert_location(&stderr, &path, 9);
    assert!(
        stderr.contains("note: value was moved into this function argument"),
        "{stderr}"
    );
    assert!(stderr.contains("9 | first = take(value)"), "{stderr}");
}

#[test]
fn owned_match_reuse_identifies_the_scrutinee_move() {
    let (success, stderr, path) = check(
        "match-scrutinee",
        "enum MaybeInt\nNone\nSome int\nend\nvalue = MaybeInt.Some(1)\nmatch value\ncase MaybeInt.None\nprint 0\ncase MaybeInt.Some(x)\nprint x\nend\nagain = value\n",
    );

    assert!(!success);
    assert_location(&stderr, &path, 12);
    assert_location(&stderr, &path, 6);
    assert!(stderr.contains("note: value was moved into this match"), "{stderr}");
    assert!(stderr.contains("6 | match value"), "{stderr}");
}

#[test]
fn one_continuing_branch_reports_control_flow_move_provenance() {
    let (success, stderr, path) = check(
        "branch-one",
        "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nif true\nfirst = value\nelse\nprint 0\nend\nsecond = value\n",
    );

    assert!(!success);
    assert_location(&stderr, &path, 11);
    assert_location(&stderr, &path, 7);
    assert!(
        stderr.contains("note: a continuing control-flow path moved the value here"),
        "{stderr}"
    );
    assert!(stderr.contains("7 | first = value"), "{stderr}");
}

#[test]
fn two_continuing_branch_moves_choose_source_order_deterministically() {
    let (success, stderr, path) = check(
        "branch-two",
        "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nif true\nfirst = value\nelse\nother = value\nend\nagain = value\n",
    );

    assert!(!success);
    assert_location(&stderr, &path, 11);
    assert_location(&stderr, &path, 7);
    assert!(stderr.contains("7 | first = value"), "{stderr}");
    assert!(!stderr.contains("note: value was moved here\n --> "), "{stderr}");
    assert!(
        stderr.contains("note: a continuing control-flow path moved the value here"),
        "{stderr}"
    );
}

#[test]
fn terminal_branch_move_does_not_poison_the_continuing_path() {
    let (success, stderr, _) = check(
        "terminal-branch",
        "enum Flag\nOff\nOn\nend\nfn use(value Flag) int\nif true\nfirst = value\nreturn 1\nelse\nprint 0\nend\nsecond = value\nreturn 0\nend\n",
    );

    assert!(success, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
}

#[test]
fn reinitialization_clears_stale_provenance_and_later_move_wins() {
    let (success, stderr, path) = check(
        "reinitialize",
        "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nfirst = value\nvalue = Flag.Off()\nsecond = value\nthird = value\n",
    );

    assert!(!success);
    assert_location(&stderr, &path, 9);
    assert_location(&stderr, &path, 8);
    assert!(stderr.contains("8 | second = value"), "{stderr}");
    assert!(!stderr.contains("6 | first = value\n"), "{stderr}");
}

#[test]
fn repeat_loop_carried_move_points_to_the_body_move() {
    let (success, stderr, path) = check(
        "repeat",
        "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nrepeat 2\nfirst = value\nend\n",
    );

    assert!(!success);
    assert!(stderr.contains("later iteration"), "{stderr}");
    assert_location(&stderr, &path, 6);
    assert_location(&stderr, &path, 7);
    assert!(stderr.contains("note: repeat body moves the value here"), "{stderr}");
    assert!(stderr.contains("7 | first = value"), "{stderr}");
}

#[test]
fn child_scope_and_type_errors_do_not_inherit_move_notes() {
    let (scope_success, scope_stderr, _) = check(
        "child-scope",
        "enum Flag\nOff\nOn\nend\nif true\ntemp = Flag.On()\nfirst = temp\nend\nsecond = temp\n",
    );
    assert!(!scope_success);
    assert!(!scope_stderr.contains("note:"), "{scope_stderr}");

    let (type_success, type_stderr, _) = check(
        "type-mismatch",
        "enum A\nOne\nend\nenum B\nOne\nend\nvalue = A.One()\nvalue = B.One()\n",
    );
    assert!(!type_success);
    assert!(type_stderr.contains("different value type"), "{type_stderr}");
    assert!(!type_stderr.contains("note:"), "{type_stderr}");
}
