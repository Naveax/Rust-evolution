use evo_codegen_rust::generate_lowered_rust;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    if let Err(error) = run_cli() {
        eprintln!("error: {error}");
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
            let generated = load_program(&source_path)?;
            print!("{generated}");
            Ok(())
        }
        "build" => {
            let output = args
                .next()
                .map_or_else(|| default_output_path(&source_path), PathBuf::from);
            reject_extra_args(args)?;
            let generated = load_program(&source_path)?;
            compile_rust(&generated, &output)?;
            println!("{}", output.display());
            Ok(())
        }
        "run" => {
            reject_extra_args(args)?;
            let generated = load_program(&source_path)?;
            run_generated(&generated)
        }
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: evo <check|emit-rust|build|run> <file.evo> [build-output]".to_owned()
}

fn reject_extra_args(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    if args.next().is_some() {
        Err(usage())
    } else {
        Ok(())
    }
}

fn load_program(path: &Path) -> Result<String, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let tokens = lex(&source).map_err(|error| error.to_string())?;
    let syntax = parse(&tokens).map_err(|error| error.to_string())?;
    let program = lower(&syntax).map_err(|error| error.to_string())?;
    Ok(generate_lowered_rust(&program))
}

fn compile_rust(generated: &str, output: &Path) -> Result<(), String> {
    let work_dir = unique_temp_dir("compile")?;
    fs::create_dir_all(&work_dir)
        .map_err(|error| format!("failed to create {}: {error}", work_dir.display()))?;
    let source_path = work_dir.join("main.rs");
    fs::write(&source_path, generated)
        .map_err(|error| format!("failed to write generated Rust: {error}"))?;

    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    let rustc = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    let status = Command::new(rustc)
        .arg(&source_path)
        .arg("--edition=2024")
        .arg("-C")
        .arg("opt-level=3")
        .arg("-C")
        .arg("codegen-units=1")
        .arg("-o")
        .arg(output)
        .status()
        .map_err(|error| format!("failed to execute rustc: {error}"))?;

    let _ = fs::remove_dir_all(&work_dir);
    if status.success() {
        Ok(())
    } else {
        Err(format!("rustc failed with {status}"))
    }
}

fn run_generated(generated: &str) -> Result<(), String> {
    let work_dir = unique_temp_dir("run")?;
    fs::create_dir_all(&work_dir)
        .map_err(|error| format!("failed to create {}: {error}", work_dir.display()))?;
    let binary = work_dir.join(format!("program{}", env::consts::EXE_SUFFIX));
    compile_rust(generated, &binary)?;
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
