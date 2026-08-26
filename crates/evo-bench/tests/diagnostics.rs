use std::env;
use std::fs;
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    env::temp_dir().join(format!("evo-bench-{label}-{}-{nanos}", process::id()))
}

fn write_case(dir: &std::path::Path, evolution_source: &str) -> std::path::PathBuf {
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
    fs::write(&evolution, evolution_source).expect("Evolution source should be written");
    fs::write(dir.join("reference.rs"), "fn main() {}\n").expect("reference should be written");
    fs::write(dir.join("expected.stdout"), "").expect("expected stdout should be written");
    evolution
}

#[test]
fn invalid_evolution_source_uses_recovered_source_diagnostics() {
    let dir = temp_dir("parser-diagnostic");
    fs::create_dir_all(&dir).expect("temporary directory should be created");

    let evolution = write_case(&dir, "x 1\ny 2\n");
    let out = dir.join("out");

    let output = Command::new(env!("CARGO_BIN_EXE_evo-bench"))
        .arg("run")
        .arg(&dir)
        .arg("--out")
        .arg(&out)
        .arg("--report-only")
        .env("RUSTC", "evo-rustc-must-not-run")
        .output()
        .expect("evo-bench should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let first_location = format!(" --> {}:1:3", evolution.display());
    let second_location = format!(" --> {}:2:3", evolution.display());
    let generated_exists = out.join("generated.rs").exists();
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert_eq!(stderr.matches("error: expected '='").count(), 2, "{stderr}");
    assert!(stderr.contains(&first_location), "{stderr}");
    assert!(stderr.contains(&second_location), "{stderr}");
    assert!(stderr.contains("1 | x 1"), "{stderr}");
    assert!(stderr.contains("2 | y 2"), "{stderr}");
    assert!(!stderr.contains("failed to execute rustc"), "{stderr}");
    assert!(
        !generated_exists,
        "parser failure must not emit generated Rust"
    );
}

#[test]
fn lexical_errors_are_aggregated_before_parser_and_rustc() {
    let dir = temp_dir("lexer-diagnostic");
    fs::create_dir_all(&dir).expect("temporary directory should be created");

    let evolution = write_case(&dir, "print @\nprint $\n");
    let out = dir.join("out");

    let output = Command::new(env!("CARGO_BIN_EXE_evo-bench"))
        .arg("run")
        .arg(&dir)
        .arg("--out")
        .arg(&out)
        .arg("--report-only")
        .env("RUSTC", "evo-rustc-must-not-run")
        .output()
        .expect("evo-bench should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let first_location = format!(" --> {}:1:7", evolution.display());
    let second_location = format!(" --> {}:2:7", evolution.display());
    let generated_exists = out.join("generated.rs").exists();
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success());
    assert_eq!(stderr.matches("error: unexpected character").count(), 2, "{stderr}");
    assert!(stderr.contains(&first_location), "{stderr}");
    assert!(stderr.contains(&second_location), "{stderr}");
    assert!(stderr.contains("1 | print @"), "{stderr}");
    assert!(stderr.contains("2 | print $"), "{stderr}");
    assert!(!stderr.contains("expected expression"), "{stderr}");
    assert!(!stderr.contains("failed to execute rustc"), "{stderr}");
    assert!(
        !generated_exists,
        "lexer failure must not emit generated Rust"
    );
}
