use std::env;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn runtime_input_process_corpus() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("test work directory should be created");

    let source_path = work_dir.join("runtime-input.evo");
    let binary_path = work_dir.join(format!("runtime-input{}", env::consts::EXE_SUFFIX));
    fs::write(
        &source_path,
        concat!(
            "n = input_int\n",
            "sum = 0\n",
            "repeat n\n",
            "sum = sum + 1\n",
            "end\n",
            "print sum\n",
        ),
    )
    .expect("Evolution source should be written");

    let build = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("build")
        .arg(&source_path)
        .arg(&binary_path)
        .output()
        .expect("evo build should execute");
    let build_stdout = String::from_utf8_lossy(&build.stdout);
    let build_stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        build.status.success(),
        "evo build failed; stdout={build_stdout:?} stderr={build_stderr:?}"
    );

    for (input, expected_stdout) in [
        ("0\n", "0\n"),
        ("1\n", "1\n"),
        ("3\n", "3\n"),
        ("-3\n", "0\n"),
    ] {
        let output = run_with_input(&binary_path, input);
        assert!(
            output.status.success(),
            "generated program failed for input {input:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            expected_stdout.as_bytes(),
            "unexpected stdout for input {input:?}"
        );
        assert!(
            output.stderr.is_empty(),
            "unexpected stderr for input {input:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let invalid = run_with_input(&binary_path, "not-an-integer\n");
    assert!(
        !invalid.status.success(),
        "invalid integer input must fail the generated process"
    );
    let invalid_stderr = String::from_utf8_lossy(&invalid.stderr);
    assert!(
        invalid_stderr.contains("expected signed integer input"),
        "invalid-input failure contract missing from stderr: {invalid_stderr:?}"
    );

    let _ = fs::remove_dir_all(work_dir);
}

fn run_with_input(binary: &Path, input: &str) -> Output {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("generated binary should start");

    let mut stdin = child.stdin.take().expect("stdin pipe should exist");
    stdin
        .write_all(input.as_bytes())
        .expect("test input should be written");
    drop(stdin);

    child
        .wait_with_output()
        .expect("generated binary should finish")
}

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    env::temp_dir().join(format!(
        "rust-evolution-runtime-input-test-{}-{nanos}",
        process::id()
    ))
}
