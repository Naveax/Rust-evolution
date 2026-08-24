use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn arithmetic_smoke_runs_end_to_end_and_writes_reports() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let case_dir = manifest_dir.join("../../benchmarks/cases/arithmetic-smoke");
    let output_dir = unique_temp_dir("arithmetic-smoke");

    let output = Command::new(env!("CARGO_BIN_EXE_evo-bench"))
        .arg("run")
        .arg(&case_dir)
        .arg("--out")
        .arg(&output_dir)
        .arg("--report-only")
        .output()
        .expect("evo-bench should execute");

    assert!(
        output.status.success(),
        "evo-bench failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report =
        fs::read_to_string(output_dir.join("report.json")).expect("report.json should be written");
    assert!(report.contains("\"correctness\": true"));
    assert!(report.contains("\"performance_ratio\":"));
    assert!(output_dir.join("report.md").is_file());
    assert!(output_dir.join("raw-samples.csv").is_file());
    assert!(output_dir.join("generated.rs").is_file());
    assert!(output_dir.join("reference.ll").is_file());
    assert!(output_dir.join("evolution.ll").is_file());

    let _ = fs::remove_dir_all(output_dir);
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rust-evolution-evo-bench-test-{label}-{}-{nanos}",
        std::process::id()
    ))
}
