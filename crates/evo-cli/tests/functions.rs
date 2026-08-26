use std::env;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn functions_v0_builds_and_runs_natively() {
    let work_dir = unique_temp_dir();
    fs::create_dir_all(&work_dir).expect("test work directory should be created");

    let source_path = work_dir.join("functions.evo");
    let binary_path = work_dir.join(format!("functions{}", env::consts::EXE_SUFFIX));
    fs::write(
        &source_path,
        concat!(
            "print banner()\n",
            "fn step(x int) int\n",
            "if x > 1 and not (x == 7)\n",
            "return x / 2\n",
            "else\n",
            "return x + 3\n",
            "end\n",
            "end\n",
            "fn choose(flag bool, a int, b int) int\n",
            "if flag\n",
            "return a\n",
            "else\n",
            "return b\n",
            "end\n",
            "end\n",
            "fn banner() string\n",
            "return \"functions-v0\"\n",
            "end\n",
            "n = input_int\n",
            "x = input_int\n",
            "sum = 0\n",
            "repeat n\n",
            "x = step(x)\n",
            "sum = sum + x\n",
            "end\n",
            "print choose(sum >= 0, sum, -sum)\n",
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
        "evo build failed; stdout={} stderr={}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    for (input, expected_total) in [
        ("0\n10\n", "0"),
        ("1\n10\n", "5"),
        ("5\n10\n", "14"),
        ("10\n7\n", "32"),
    ] {
        let output = run_with_input(&binary_path, input);
        assert!(
            output.status.success(),
            "generated program failed for input {input:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            format!("functions-v0\n{expected_total}\n")
        );
        assert!(
            output.stderr.is_empty(),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

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
        "rust-evolution-functions-test-{}-{nanos}",
        process::id()
    ))
}
