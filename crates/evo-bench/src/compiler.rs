use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::Path;
use std::process::Command;

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
    let mut command = rustc_base_command(rustc, source);
    command.arg("-o").arg(output);
    run_compile_command(command, "binary")
}

pub(crate) fn compile_llvm_ir(rustc: &OsStr, source: &Path, output: &Path) -> Result<(), String> {
    let mut command = rustc_base_command(rustc, source);
    command.arg("--emit=llvm-ir").arg("-o").arg(output);
    run_compile_command(command, "LLVM IR")
}

pub(crate) fn compare_normalized_ir(reference: &Path, evolution: &Path) -> Result<bool, String> {
    let reference_text = fs::read_to_string(reference)
        .map_err(|error| format!("failed to read {}: {error}", reference.display()))?;
    let evolution_text = fs::read_to_string(evolution)
        .map_err(|error| format!("failed to read {}: {error}", evolution.display()))?;
    Ok(normalize_llvm_ir(&reference_text) == normalize_llvm_ir(&evolution_text))
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
    use super::{normalize_llvm_ir, normalize_rust_symbol_hashes};

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
}
