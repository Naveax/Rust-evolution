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

#[test]
fn check_keeps_parsed_enum_declarations_fail_closed_before_codegen() {
    let dir = temp_dir("enums-check-gate");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("maybe-int.evo");
    fs::write(
        &source,
        "enum MaybeInt\nNone\nSome int\nend\nprint 1\n",
    )
    .expect("enum source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("evo check should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let location = format!(" --> {}:1:1", source.display());
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert!(stderr.contains("Enums v0 semantic lowering"), "{stderr}");
    assert!(stderr.contains(&location), "{stderr}");
    assert!(stderr.contains("1 | enum MaybeInt"), "{stderr}");
    assert!(!stderr.contains("main.rs"), "{stderr}");
    assert!(!stderr.contains("rustc failed"), "{stderr}");
}
