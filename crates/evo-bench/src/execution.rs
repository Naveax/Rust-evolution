use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub(crate) struct Execution {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    pub(crate) timed_out: bool,
}

pub(crate) fn execute_with_timeout(
    binary: &Path,
    stdin: &[u8],
    timeout: Duration,
) -> Result<Execution, String> {
    let capture = CaptureWorkspace::new("correctness", stdin)?;
    let stdin_file = File::open(&capture.stdin_path)
        .map_err(|error| format!("failed to open benchmark stdin: {error}"))?;
    let stdout_file = File::create(&capture.stdout_path)
        .map_err(|error| format!("failed to create benchmark stdout capture: {error}"))?;
    let stderr_file = File::create(&capture.stderr_path)
        .map_err(|error| format!("failed to create benchmark stderr capture: {error}"))?;

    let started = Instant::now();
    let mut child = Command::new(binary)
        .stdin(Stdio::from(stdin_file))
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .map_err(|error| format!("failed to execute {}: {error}", binary.display()))?;

    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed while waiting for benchmark process: {error}"))?
        {
            return capture.finish(status, false);
        }

        if started.elapsed() >= timeout {
            match child.kill() {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {}
                Err(error) => {
                    return Err(format!("failed to kill timed-out benchmark: {error}"));
                }
            }
            let status = child
                .wait()
                .map_err(|error| format!("failed to reap timed-out benchmark: {error}"))?;
            return capture.finish(status, true);
        }

        thread::sleep(Duration::from_millis(1));
    }
}

/// Measures process wall-clock execution without timeout polling.
///
/// Correctness is validated before this function is used. During timing, stdout and stderr are
/// redirected to the platform null device so parent-side capture does not contaminate the sample.
/// The process still performs its writes; only capture/storage overhead is removed symmetrically.
pub(crate) fn measure_blocking(binary: &Path, stdin: &[u8]) -> Result<u128, String> {
    let workspace = MeasurementWorkspace::new(stdin)?;
    let stdin_file = File::open(&workspace.stdin_path)
        .map_err(|error| format!("failed to open benchmark stdin: {error}"))?;

    let started = Instant::now();
    let status = Command::new(binary)
        .stdin(Stdio::from(stdin_file))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| format!("failed to execute {}: {error}", binary.display()))?;
    let duration_ns = started.elapsed().as_nanos();

    workspace.cleanup();

    if status.success() {
        Ok(duration_ns)
    } else {
        Err(format!(
            "benchmark execution failed during measurement: {status}"
        ))
    }
}

struct CaptureWorkspace {
    dir: PathBuf,
    stdin_path: PathBuf,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

impl CaptureWorkspace {
    fn new(label: &str, stdin: &[u8]) -> Result<Self, String> {
        let dir = unique_temp_dir(label);
        fs::create_dir_all(&dir)
            .map_err(|error| format!("failed to create {}: {error}", dir.display()))?;
        let stdin_path = dir.join("stdin.bin");
        let stdout_path = dir.join("stdout.bin");
        let stderr_path = dir.join("stderr.bin");
        fs::write(&stdin_path, stdin)
            .map_err(|error| format!("failed to write benchmark stdin: {error}"))?;
        Ok(Self {
            dir,
            stdin_path,
            stdout_path,
            stderr_path,
        })
    }

    fn finish(self, status: ExitStatus, timed_out: bool) -> Result<Execution, String> {
        let stdout = fs::read(&self.stdout_path)
            .map_err(|error| format!("failed to read benchmark stdout: {error}"))?;
        let stderr = fs::read(&self.stderr_path)
            .map_err(|error| format!("failed to read benchmark stderr: {error}"))?;
        let execution = Execution {
            status,
            stdout,
            stderr,
            timed_out,
        };
        self.cleanup();
        Ok(execution)
    }

    fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

struct MeasurementWorkspace {
    dir: PathBuf,
    stdin_path: PathBuf,
}

impl MeasurementWorkspace {
    fn new(stdin: &[u8]) -> Result<Self, String> {
        let dir = unique_temp_dir("measurement");
        fs::create_dir_all(&dir)
            .map_err(|error| format!("failed to create {}: {error}", dir.display()))?;
        let stdin_path = dir.join("stdin.bin");
        fs::write(&stdin_path, stdin)
            .map_err(|error| format!("failed to write benchmark stdin: {error}"))?;
        Ok(Self { dir, stdin_path })
    }

    fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!(
        "rust-evolution-evo-bench-{label}-{}-{counter}",
        std::process::id()
    ))
}
