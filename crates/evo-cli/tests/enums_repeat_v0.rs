use std::env;
use std::fs;
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    env::temp_dir().join(format!("evo-enums-repeat-{}-{nanos}", process::id()))
}

#[test]
fn native_repeat_composes_with_enum_construction_and_match() {
    let dir = temp_dir();
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("repeat-match.evo");
    let binary = dir.join(format!("repeat-match{}", env::consts::EXE_SUFFIX));
    fs::write(
        &source,
        concat!(
            "enum Flag\n",
            "Off\n",
            "On\n",
            "end\n",
            "repeat 2\n",
            "value = Flag.On()\n",
            "match value\n",
            "case Flag.Off\n",
            "print 0\n",
            "case Flag.On\n",
            "print 1\n",
            "end\n",
            "end\n",
        ),
    )
    .expect("enum repeat source should be written");

    let build = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("build")
        .arg(&source)
        .arg(&binary)
        .output()
        .expect("evo build should run");
    let build_stderr = String::from_utf8_lossy(&build.stderr);
    assert!(build.status.success(), "{build_stderr}");

    let run = Command::new(&binary)
        .output()
        .expect("compiled repeat enum binary should run");
    let run_stderr = String::from_utf8_lossy(&run.stderr);
    let stdout = run.stdout.clone();
    let _ = fs::remove_dir_all(&dir);

    assert!(run.status.success(), "{run_stderr}");
    assert_eq!(stdout, b"1\n1\n");
}
