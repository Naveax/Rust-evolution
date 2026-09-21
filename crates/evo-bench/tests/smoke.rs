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
        .env("EVO_GIT_SHA", "0123456789abcdef0123456789abcdef01234567")
        .env("EVO_REQUIRE_GIT_SHA", "1")
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
    assert!(report.contains("\"schema_version\": 3"));
    assert!(report.contains("\"git_sha\": \"0123456789abcdef0123456789abcdef01234567\""));
    assert!(report.contains("\"build_flags\":"));
    assert!(report.contains("\"host_os\":"));
    assert!(report.contains("\"host_arch\":"));
    assert!(report.contains("\"ci_provider\":"));
    assert!(report.contains("\"runner_name\":"));
    assert!(report.contains("\"runner_os\":"));
    assert!(report.contains("\"runner_arch\":"));
    assert!(report.contains("\"correctness\": true"));
    assert!(report.contains("\"performance_ratio\":"));

    let markdown =
        fs::read_to_string(output_dir.join("report.md")).expect("report.md should be written");
    assert!(markdown.contains("- Git SHA: `0123456789abcdef0123456789abcdef01234567`"));
    assert!(markdown.contains(
        "- Build flags: `--crate-name evo_benchmark_case --edition=2024 -C opt-level=3 -C codegen-units=1 -C lto=thin -C debuginfo=0`"
    ));

    assert!(output_dir.join("raw-samples.csv").is_file());
    assert!(output_dir.join("generated.rs").is_file());
    assert!(output_dir.join("reference.ll").is_file());
    assert!(output_dir.join("evolution.ll").is_file());

    let _ = fs::remove_dir_all(output_dir);
}

#[test]
fn required_provenance_rejects_missing_explicit_sha() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let case_dir = manifest_dir.join("../../benchmarks/cases/arithmetic-smoke");
    let output_dir = unique_temp_dir("missing-provenance");

    let output = Command::new(env!("CARGO_BIN_EXE_evo-bench"))
        .arg("run")
        .arg(&case_dir)
        .arg("--out")
        .arg(&output_dir)
        .arg("--report-only")
        .env_remove("EVO_GIT_SHA")
        .env("EVO_REQUIRE_GIT_SHA", "1")
        .env("RUSTC", "evo-rustc-must-not-run")
        .output()
        .expect("evo-bench should execute");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = fs::remove_dir_all(output_dir);

    assert!(!output.status.success());
    assert!(
        stderr.contains("exact benchmark provenance is required"),
        "{stderr}"
    );
    assert!(!stderr.contains("failed to execute rustc"), "{stderr}");
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
