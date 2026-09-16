use evo_codegen_rust::generate_lowered_rust;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn generational_arena_keeps_independent_control_and_parity_locked_timed_reference() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let case_dir = manifest_dir.join("../../benchmarks/cases/generational-arena-v0");
    let evolution_source = fs::read_to_string(case_dir.join("evolution.evo"))
        .expect("arena benchmark Evolution source should be readable");
    let reference = fs::read_to_string(case_dir.join("reference.rs"))
        .expect("arena benchmark timed Rust reference should be readable");
    let independent = fs::read_to_string(case_dir.join("independent_reference.rs"))
        .expect("arena benchmark independent Rust control should be readable");

    let tokens = lex(&evolution_source).expect("arena benchmark should lex");
    let syntax = parse(&tokens).expect("arena benchmark should parse");
    let lowered = lower(&syntax).expect("arena benchmark should lower");
    let generated = generate_lowered_rust(&lowered);

    assert!(generated.contains("struct __EvoHandle<T>"));
    assert!(generated.contains("struct __EvoArena<T>"));
    assert!(generated.contains("fn __evo_arena_insert<T>"));
    assert!(generated.contains("fn __evo_arena_get<T>"));
    assert!(generated.contains("fn __evo_arena_remove<T>"));
    assert!(generated.contains("slot.generation == u64::MAX"));
    assert!(generated.contains("checked_add(1).unwrap_or(0)"));

    for forbidden in [
        "unsafe",
        "RefCell",
        "Mutex",
        "RwLock",
        "HashMap",
        "Rc::clone",
        "Arc<",
    ] {
        assert!(
            !generated.contains(forbidden),
            "generated arena benchmark must not contain hidden runtime mechanism {forbidden:?}"
        );
    }

    let generated_prefix = generated
        .split_once("fn main() {")
        .expect("generated arena source should contain main")
        .0;
    let reference_prefix = reference
        .split_once("fn main() {")
        .expect("timed arena reference should contain main")
        .0;
    assert_eq!(
        normalize_newlines(reference_prefix),
        normalize_newlines(generated_prefix),
        "timed reference must lock the direct generated arena/runtime helper contract"
    );
    assert_ne!(
        normalize_newlines(&reference),
        normalize_newlines(&generated),
        "timed reference workload body should remain independently written"
    );

    assert!(independent.contains("struct RefArena<T>"));
    assert!(independent.contains("struct RefHandle<T>"));
    assert!(!independent.contains("__EvoArena"));
    assert!(!independent.contains("__EvoHandle"));
    assert_ne!(normalize_newlines(&independent), normalize_newlines(&generated));

    let metadata = manifest_dir
        .join("../../target/generational-arena-independent-reference.rmeta");
    let compile = Command::new("rustc")
        .arg("--edition=2024")
        .arg("--crate-name")
        .arg("evo_generational_arena_independent_reference")
        .arg("--emit=metadata")
        .arg(case_dir.join("independent_reference.rs"))
        .arg("-o")
        .arg(&metadata)
        .output()
        .expect("rustc should compile independent arena control");
    assert!(
        compile.status.success(),
        "independent arena control failed to compile: {}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let _ = fs::remove_file(metadata);
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}
