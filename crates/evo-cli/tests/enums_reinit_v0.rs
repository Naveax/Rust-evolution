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
fn native_enum_can_be_reinitialized_after_move() {
    let dir = temp_dir("enums-reinit");
    fs::create_dir_all(&dir).expect("temporary directory should be created");
    let source = dir.join("reinit.evo");
    let binary = dir.join(format!("reinit{}", env::consts::EXE_SUFFIX));
    fs::write(
        &source,
        concat!(
            "enum Flag\n",
            "Off\n",
            "On\n",
            "end\n",
            "fn read(value Flag) int\n",
            "match value\n",
            "case Flag.Off\n",
            "return 0\n",
            "case Flag.On\n",
            "return 1\n",
            "end\n",
            "end\n",
            "value = Flag.Off()\n",
            "first = read(value)\n",
            "value = Flag.On()\n",
            "print first + read(value)\n",
        ),
    )
    .expect("enum reinitialization source should be written");

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
        "reinitialized enum program should produce a native binary"
    );

    let run = Command::new(&binary)
        .output()
        .expect("compiled enum binary should run");
    let run_stderr = String::from_utf8_lossy(&run.stderr);
    assert!(run.status.success(), "{run_stderr}");
    assert_eq!(
        String::from_utf8(run.stdout).expect("enum stdout should be UTF-8"),
        "1\n"
    );

    let _ = fs::remove_dir_all(&dir);
}
