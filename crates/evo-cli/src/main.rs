use evo_codegen_rust::{GeneratedRust, generate_lowered_rust_with_map};
use evo_diagnostics::render_error;
use evo_formatter::format_source;
use evo_lexer::lex_recovering;
use evo_lowering::lower;
use evo_parser::parse_recovering;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
struct LoadedProgram {
    source: String,
    generated: GeneratedRust,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustcShortError {
    generated_path: String,
    generated_line: usize,
    generated_column: usize,
    message: String,
}

fn main() {
    if let Err(error) = run_cli() {
        if error.starts_with("error: ") {
            eprintln!("{error}");
        } else {
            eprintln!("error: {error}");
        }
        process::exit(1);
    }
}

fn run_cli() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(usage)?;
    let source_path = args.next().ok_or_else(usage).map(PathBuf::from)?;

    match command.as_str() {
        "check" => {
            reject_extra_args(args)?;
            let _ = load_program(&source_path)?;
            println!("ok");
            Ok(())
        }
        "emit-rust" => {
            reject_extra_args(args)?;
            let program = load_program(&source_path)?;
            print!("{}", program.generated.source);
            Ok(())
        }
        "fmt" => {
            let check = match args.next() {
                None => false,
                Some(value) if value == "--check" => true,
                Some(_) => return Err(usage()),
            };
            reject_extra_args(args)?;
            format_file(&source_path, check)
        }
        "build" => {
            let output = args
                .next()
                .map_or_else(|| default_output_path(&source_path), PathBuf::from);
            reject_extra_args(args)?;
            let program = load_program(&source_path)?;
            compile_rust(&program, &source_path, &output)?;
            println!("{}", output.display());
            Ok(())
        }
        "run" => {
            reject_extra_args(args)?;
            let program = load_program(&source_path)?;
            run_generated(&program, &source_path)
        }
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: evo <check|emit-rust|fmt|build|run> <file.evo> [build-output|--check]".to_owned()
}

fn reject_extra_args(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    if args.next().is_some() {
        Err(usage())
    } else {
        Ok(())
    }
}

fn format_file(path: &Path, check: bool) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let tokens =
        lex_recovering(&source).map_err(|errors| render_lex_errors(path, &source, &errors))?;
    let _ =
        parse_recovering(&tokens).map_err(|errors| render_parse_errors(path, &source, &errors))?;
    let formatted = format_source(&source, &tokens);

    if check {
        if formatted == source {
            println!("ok");
            Ok(())
        } else {
            Err(format!("{} is not formatted", path.display()))
        }
    } else {
        if formatted != source {
            fs::write(path, formatted)
                .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
        }
        println!("{}", path.display());
        Ok(())
    }
}

fn load_program(path: &Path) -> Result<LoadedProgram, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let tokens =
        lex_recovering(&source).map_err(|errors| render_lex_errors(path, &source, &errors))?;
    let syntax =
        parse_recovering(&tokens).map_err(|errors| render_parse_errors(path, &source, &errors))?;
    let program =
        lower(&syntax).map_err(|error| render_error(path, &source, &error.message, error.span))?;
    let generated = generate_lowered_rust_with_map(&program);
    Ok(LoadedProgram { source, generated })
}

fn render_lex_errors(path: &Path, source: &str, errors: &[evo_lexer::LexError]) -> String {
    errors
        .iter()
        .map(|error| render_error(path, source, &error.message, error.span))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_parse_errors(path: &Path, source: &str, errors: &[evo_parser::ParseError]) -> String {
    errors
        .iter()
        .map(|error| render_error(path, source, &error.message, error.span))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn compile_rust(program: &LoadedProgram, source_path: &Path, output: &Path) -> Result<(), String> {
    let work_dir = unique_temp_dir("compile")?;
    fs::create_dir_all(&work_dir)
        .map_err(|error| format!("failed to create {}: {error}", work_dir.display()))?;
    let generated_path = work_dir.join("main.rs");
    fs::write(&generated_path, &program.generated.source)
        .map_err(|error| format!("failed to write generated Rust: {error}"))?;

    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    let rustc = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    let result = Command::new(rustc)
        .arg(&generated_path)
        .arg("--edition=2024")
        .arg("--error-format=short")
        .arg("-C")
        .arg("opt-level=3")
        .arg("-C")
        .arg("codegen-units=1")
        .arg("-o")
        .arg(output)
        .output()
        .map_err(|error| format!("failed to execute rustc: {error}"));

    let compile_result = match result {
        Ok(result) if result.status.success() => {
            forward_rustc_output(&result.stdout, &result.stderr)?;
            Ok(())
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            Err(render_rustc_failure(program, source_path, &stderr))
        }
        Err(error) => Err(error),
    };

    let _ = fs::remove_dir_all(&work_dir);
    compile_result
}

fn forward_rustc_output(stdout: &[u8], stderr: &[u8]) -> Result<(), String> {
    if !stdout.is_empty() {
        io::stdout()
            .write_all(stdout)
            .map_err(|error| format!("failed to forward rustc stdout: {error}"))?;
    }
    if !stderr.is_empty() {
        io::stderr()
            .write_all(stderr)
            .map_err(|error| format!("failed to forward rustc stderr: {error}"))?;
    }
    Ok(())
}

fn render_rustc_failure(program: &LoadedProgram, source_path: &Path, stderr: &str) -> String {
    if let Some(diagnostic) = parse_rustc_short_error(stderr)
        && diagnostic.generated_path.ends_with("main.rs")
        && let Some(source_span) = program
            .generated
            .source_span_for_line(diagnostic.generated_line)
    {
        return render_error(
            source_path,
            &program.source,
            &diagnostic.message,
            source_span,
        );
    }

    let stderr = stderr.trim();
    if stderr.is_empty() {
        "rustc failed without diagnostics".to_owned()
    } else {
        format!("rustc failed:\n{stderr}")
    }
}

fn parse_rustc_short_error(stderr: &str) -> Option<RustcShortError> {
    stderr.lines().find_map(parse_rustc_short_error_line)
}

fn parse_rustc_short_error_line(line: &str) -> Option<RustcShortError> {
    let error_marker = line.find(": error")?;
    let location = &line[..error_marker];
    let diagnostic = &line[error_marker + 2..];

    let mut location_parts = location.rsplitn(3, ':');
    let generated_column = location_parts.next()?.parse().ok()?;
    let generated_line = location_parts.next()?.parse().ok()?;
    let generated_path = location_parts.next()?.to_owned();

    let message = diagnostic
        .split_once(": ")
        .map_or(diagnostic, |(_, message)| message)
        .trim();
    if message.is_empty() {
        return None;
    }

    Some(RustcShortError {
        generated_path,
        generated_line,
        generated_column,
        message: message.to_owned(),
    })
}

fn run_generated(program: &LoadedProgram, source_path: &Path) -> Result<(), String> {
    let work_dir = unique_temp_dir("run")?;
    fs::create_dir_all(&work_dir)
        .map_err(|error| format!("failed to create {}: {error}", work_dir.display()))?;
    let binary = work_dir.join(format!("program{}", env::consts::EXE_SUFFIX));
    let compile_result = compile_rust(program, source_path, &binary);
    if let Err(error) = compile_result {
        let _ = fs::remove_dir_all(&work_dir);
        return Err(error);
    }

    let status = Command::new(&binary)
        .status()
        .map_err(|error| format!("failed to execute generated binary: {error}"))?;
    let _ = fs::remove_dir_all(&work_dir);
    if status.success() {
        Ok(())
    } else {
        Err(format!("generated program exited with {status}"))
    }
}

fn unique_temp_dir(label: &str) -> Result<PathBuf, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?
        .as_nanos();
    Ok(env::temp_dir().join(format!("rust-evolution-{label}-{}-{nanos}", process::id())))
}

fn default_output_path(source: &Path) -> PathBuf {
    let mut output = source.to_path_buf();
    if env::consts::EXE_SUFFIX.is_empty() {
        output.set_extension("");
    } else {
        output.set_extension(env::consts::EXE_SUFFIX.trim_start_matches('.'));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{LoadedProgram, parse_rustc_short_error, render_rustc_failure};
    use evo_codegen_rust::GeneratedRust;
    use std::path::Path;

    #[test]
    fn parses_unix_rustc_short_error() {
        let parsed = parse_rustc_short_error(
            "/tmp/rust-evolution/main.rs:3:15: error[E0308]: mismatched types\n",
        )
        .expect("diagnostic should parse");

        assert_eq!(parsed.generated_path, "/tmp/rust-evolution/main.rs");
        assert_eq!(parsed.generated_line, 3);
        assert_eq!(parsed.generated_column, 15);
        assert_eq!(parsed.message, "mismatched types");
    }

    #[test]
    fn parses_windows_drive_colon_from_the_right() {
        let parsed = parse_rustc_short_error(
            r"C:\Users\runner\Temp\rust-evolution\main.rs:12:7: error[E0308]: mismatched types",
        )
        .expect("diagnostic should parse");

        assert_eq!(
            parsed.generated_path,
            r"C:\Users\runner\Temp\rust-evolution\main.rs"
        );
        assert_eq!(parsed.generated_line, 12);
        assert_eq!(parsed.generated_column, 7);
        assert_eq!(parsed.message, "mismatched types");
    }

    #[test]
    fn ignores_non_error_short_diagnostics() {
        assert!(parse_rustc_short_error("main.rs:2:1: warning: unused variable: `x`\n").is_none());
    }

    #[test]
    fn unmapped_rustc_error_preserves_raw_fallback() {
        let program = LoadedProgram {
            source: "x = 1\n".to_owned(),
            generated: GeneratedRust {
                source: "fn main() {}\n".to_owned(),
                mappings: Vec::new(),
            },
        };
        let stderr = "/tmp/generated/main.rs:1:1: error: internal generated error\n";
        let rendered = render_rustc_failure(&program, Path::new("sample.evo"), stderr);

        assert!(rendered.starts_with("rustc failed:\n"));
        assert!(rendered.contains("main.rs:1:1"));
        assert!(rendered.contains("internal generated error"));
    }
}
