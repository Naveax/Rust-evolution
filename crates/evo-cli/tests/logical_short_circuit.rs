use std::env;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn logical_operators_short_circuit_at_process_level() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("test work directory should be created");

    let source_path = work_dir.join("logical-short-circuit.evo");
    let binary_path = work_dir.join(format!("logical-short-circuit{}", env::consts::EXE_SUFFIX));
    fs::write(
        &source_path,
        concat!(
            "if false and input_int > 0\n",
            "print 10\n",
            "else\n",
            "print 1\n",
            "end\n",
            "if true or input_int > 0\n",
            "print 2\n",
            "else\n",
            "print 20\n",
            "end\n",
            "if true and input_int > 0\n",
            "print 3\n",
            "else\n",
            "print 30\n",
            "end\n",
            "if false or input_int > 0\n",
            "print 4\n",
            "else\n",
            "print 40\n",
            "end\n",
        ),
    )
    .expect("Evolution source should be written");

    let build = Command::new(env!("CARGO_BIN_EXE_evo"))
        .arg("build")
        .arg(&source_path)
        .arg(&binary_path)
        .output()
        .expect("evo build should execute");
    assert!(
        build.status.success(),
        "evo build failed; stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    // Only the third and fourth conditions may evaluate input_int.
    // If either of the first two expressions evaluates its RHS eagerly,
    // the later conditions run out of input and the process fails.
    let output = run_with_input(&binary_path, "1\n1\n");
    assert!(
        output.status.success(),
        "generated program failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"1\n2\n3\n4\n");
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
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
        "rust-evolution-logical-short-circuit-test-{}-{nanos}",
        process::id()
    ))
}
