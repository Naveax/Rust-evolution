use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static BUILD_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
const CANONICAL_SOURCE_FILE_NAME: &str = "benchmark.rs";

pub(crate) fn rustc_program() -> OsString {
    env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"))
}

pub(crate) fn rustc_verbose(rustc: &OsStr) -> Result<String, String> {
    command_text(rustc, [OsString::from("-Vv")])
}

pub(crate) fn parse_host_target(rustc_verbose: &str) -> Result<String, String> {
    rustc_verbose
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .ok_or_else(|| "rustc -Vv output did not contain a host target".to_owned())
}

pub(crate) fn compile_binary(rustc: &OsStr, source: &Path, output: &Path) -> Result<(), String> {
    let work_dir = unique_build_dir(output, "bin")?;
    fs::create_dir_all(&work_dir)
        .map_err(|error| format!("failed to create {}: {error}", work_dir.display()))?;

    let absolute_output = absolute_path(output)?;
    let result = (|| {
        stage_canonical_source(source, &work_dir, "binary comparison")?;
        let mut command = rustc_base_command(rustc, Path::new(CANONICAL_SOURCE_FILE_NAME));
        command
            .current_dir(&work_dir)
            .arg("-o")
            .arg(&absolute_output);
        run_compile_command(command, "binary")
    })();

    let _ = fs::remove_dir_all(&work_dir);
    result
}

pub(crate) fn compile_llvm_ir(rustc: &OsStr, source: &Path, output: &Path) -> Result<(), String> {
    let work_dir = unique_build_dir(output, "ir")?;
    fs::create_dir_all(&work_dir)
        .map_err(|error| format!("failed to create {}: {error}", work_dir.display()))?;

    let result = (|| {
        stage_canonical_source(source, &work_dir, "LLVM comparison")?;
        let mut command = rustc_base_command(rustc, Path::new(CANONICAL_SOURCE_FILE_NAME));
        command
            .current_dir(&work_dir)
            .arg("--emit=llvm-ir")
            .arg("--out-dir")
            .arg(".");
        run_compile_command(command, "LLVM IR")?;
        aggregate_llvm_ir(&work_dir, output)
    })();

    let _ = fs::remove_dir_all(&work_dir);
    result
}

pub(crate) fn compare_binary_bytes(reference: &Path, evolution: &Path) -> Result<bool, String> {
    let reference_bytes = fs::read(reference)
        .map_err(|error| format!("failed to read {}: {error}", reference.display()))?;
    let evolution_bytes = fs::read(evolution)
        .map_err(|error| format!("failed to read {}: {error}", evolution.display()))?;
    Ok(reference_bytes == evolution_bytes)
}

pub(crate) fn compare_normalized_ir(reference: &Path, evolution: &Path) -> Result<bool, String> {
    let reference_text = fs::read_to_string(reference)
        .map_err(|error| format!("failed to read {}: {error}", reference.display()))?;
    let evolution_text = fs::read_to_string(evolution)
        .map_err(|error| format!("failed to read {}: {error}", evolution.display()))?;
    Ok(normalize_llvm_ir(&reference_text) == normalize_llvm_ir(&evolution_text))
}

fn stage_canonical_source(source: &Path, work_dir: &Path, purpose: &str) -> Result<(), String> {
    let canonical_source = work_dir.join(CANONICAL_SOURCE_FILE_NAME);
    fs::copy(source, &canonical_source).map_err(|error| {
        format!(
            "failed to stage {} as {} for {purpose}: {error}",
            source.display(),
            canonical_source.display()
        )
    })?;
    Ok(())
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        env::current_dir()
            .map(|current_dir| current_dir.join(path))
            .map_err(|error| format!("failed to resolve {}: {error}", path.display()))
    }
}

fn unique_build_dir(output: &Path, kind: &str) -> Result<PathBuf, String> {
    let parent = output.parent().ok_or_else(|| {
        format!(
            "compiler output path has no parent directory: {}",
            output.display()
        )
    })?;
    let stem = output
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("artifact");
    let counter = BUILD_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(parent.join(format!(
        ".evo-bench-{kind}-{stem}-{}-{counter}",
        std::process::id()
    )))
}

fn aggregate_llvm_ir(dir: &Path, output: &Path) -> Result<(), String> {
    let mut modules = Vec::new();
    let mut artifacts = Vec::new();

    for entry in fs::read_dir(dir)
        .map_err(|error| format!("failed to inspect {}: {error}", dir.display()))?
    {
        let entry = entry.map_err(|error| {
            format!(
                "failed to inspect an artifact in {}: {error}",
                dir.display()
            )
        })?;
        let path = entry.path();
        let display_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        artifacts.push(display_name);

        if path.extension().is_some_and(|extension| extension == "ll") {
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            modules.push(normalize_llvm_ir(&text));
        }
    }

    artifacts.sort();
    if modules.is_empty() {
        return Err(format!(
            "rustc reported successful LLVM IR emission but no .ll file was produced in {}; artifacts: [{}]",
            dir.display(),
            artifacts.join(", ")
        ));
    }

    modules.sort();
    let mut aggregate = String::new();
    for module in modules {
        aggregate.push_str("; ---- normalized rustc LLVM module ----\n");
        aggregate.push_str(&module);
        if !module.ends_with('\n') {
            aggregate.push('\n');
        }
    }

    fs::write(output, aggregate)
        .map_err(|error| format!("failed to write {}: {error}", output.display()))
}

fn rustc_base_command(rustc: &OsStr, source: &Path) -> Command {
    let mut command = Command::new(rustc);
    command
        .arg(source)
        .arg("--crate-name")
        .arg("evo_benchmark_case")
        .arg("--edition=2024")
        .arg("-C")
        .arg("opt-level=3")
        .arg("-C")
        .arg("codegen-units=1")
        .arg("-C")
        .arg("lto=thin")
        .arg("-C")
        .arg("debuginfo=0");
    command
}

fn run_compile_command(mut command: Command, artifact: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("failed to execute rustc for {artifact}: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "rustc failed while building {artifact}: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn command_text<I, S>(program: &OsStr, args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("failed to execute {program:?}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "command {program:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| format!("command output was not UTF-8: {error}"))
}

fn normalize_llvm_ir(input: &str) -> String {
    let mut output = String::new();
    for line in input.lines() {
        if line.starts_with("; ModuleID =") || line.starts_with("source_filename =") {
            continue;
        }
        output.push_str(&normalize_rust_symbol_hashes(line));
        output.push('\n');
    }
    output
}

fn normalize_rust_symbol_hashes(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut output = String::with_capacity(line.len());
    let mut chars = line.char_indices().peekable();

    while let Some((byte_index, ch)) = chars.next() {
        if ch == 'h'
            && byte_index + 17 <= bytes.len()
            && bytes[byte_index + 1..byte_index + 17]
                .iter()
                .all(u8::is_ascii_hexdigit)
        {
            output.push_str("h<RUST_HASH>");
            while matches!(chars.peek(), Some((next, _)) if *next < byte_index + 17) {
                let _ = chars.next();
            }
        } else {
            output.push(ch);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{
        CANONICAL_SOURCE_FILE_NAME, aggregate_llvm_ir, normalize_llvm_ir,
        normalize_rust_symbol_hashes,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn uses_canonical_source_name_for_compiler_comparisons() {
        assert_eq!(CANONICAL_SOURCE_FILE_NAME, "benchmark.rs");
    }

    #[test]
    fn normalizes_rust_symbol_hashes() {
        assert_eq!(
            normalize_rust_symbol_hashes("_ZN4test17h0123456789abcdefE"),
            "_ZN4test17h<RUST_HASH>E"
        );
    }

    #[test]
    fn normalized_ir_drops_source_identity() {
        let left = "; ModuleID = 'left'\nsource_filename = \"left.rs\"\ndefine void @_ZN4test17h0123456789abcdefE() {}\n";
        let right = "; ModuleID = 'right'\nsource_filename = \"right.rs\"\ndefine void @_ZN4test17hfedcba9876543210E() {}\n";
        assert_eq!(normalize_llvm_ir(left), normalize_llvm_ir(right));
    }

    #[test]
    fn preserves_non_ascii_text_while_normalizing() {
        assert_eq!(
            normalize_rust_symbol_hashes("; açıklama h0123456789abcdef"),
            "; açıklama h<RUST_HASH>"
        );
    }

    #[test]
    fn aggregates_multiple_llvm_modules_deterministically() {
        let dir = test_temp_dir("multi-ir");
        fs::create_dir_all(&dir).expect("temp directory should be created");
        fs::write(
            dir.join("z.ll"),
            "; ModuleID = 'z'\nsource_filename = \"z.rs\"\ndefine void @z() {}\n",
        )
        .expect("first test IR should be written");
        fs::write(
            dir.join("a.ll"),
            "; ModuleID = 'a'\nsource_filename = \"a.rs\"\ndefine void @a() {}\n",
        )
        .expect("second test IR should be written");
        fs::write(dir.join("crate.d"), "deps\n").expect("side artifact should be written");
        let output = dir.join("aggregate.ll");

        aggregate_llvm_ir(&dir, &output).expect("multiple IR modules should aggregate");
        let aggregate = fs::read_to_string(&output).expect("aggregate should be readable");
        let a = aggregate
            .find("define void @a() {}")
            .expect("a module missing");
        let z = aggregate
            .find("define void @z() {}")
            .expect("z module missing");
        assert!(a < z, "normalized modules should be sorted by content");
        assert!(!aggregate.contains("ModuleID"));
        assert!(!aggregate.contains("source_filename"));
        let _ = fs::remove_dir_all(dir);
    }

    fn test_temp_dir(label: &str) -> PathBuf {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "rust-evolution-compiler-test-{label}-{}-{counter}",
            std::process::id()
        ))
    }
}
