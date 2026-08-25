use std::env;
use std::fs;
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn invalid_evolution_source_uses_shared_source_diagnostic() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    let dir = env::temp_dir().join(format!("evo-bench-diagnostic-{}-{nanos}", process::id()));
    fs::create_dir_all(&dir).expect("temporary directory should be created");

    let evolution = dir.join("evolution.evo");
    fs::write(
        dir.join("case.conf"),
        concat!(
            "name=invalid-frontend\n",
            "warmup=0\n",
            "samples=3\n",
            "timeout_ms=1000\n",
            "max_relative_mad=1.0\n"
        ),
    )
    .expect("config should be written");
    fs::write(&evolution, "x 1\n").expect("Evolution source should be written");
    fs::write(dir.join("reference.rs"), "fn main() {}\n").expect("reference should be written");
    fs::write(dir.join("expected.stdout"), "").expect("expected stdout should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_evo-bench"))
        .arg("run")
        .arg(&dir)
        .arg("--out")
        .arg(dir.join("out"))
        .arg("--report-only")
        .output()
        .expect("evo-bench should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let location = format!(" --> {}:1:3", evolution.display());
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert!(stderr.contains("error: expected '='"), "{stderr}");
    assert!(stderr.contains(&location), "{stderr}");
    assert!(stderr.contains("1 | x 1"), "{stderr}");
    assert!(stderr.contains("  |   ^"), "{stderr}");
}
