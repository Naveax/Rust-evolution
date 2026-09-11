use evo_codegen_rust::generate_lowered_rust;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;
use std::fs;
use std::path::PathBuf;

#[test]
fn inferred_shared_borrow_reference_matches_generated_rust_exactly() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let case_dir = manifest_dir.join("../../benchmarks/cases/inferred-shared-borrow-v0");

    let evolution_source = fs::read_to_string(case_dir.join("evolution.evo"))
        .expect("shared-borrow benchmark Evolution source should be readable");
    let reference = fs::read_to_string(case_dir.join("reference.rs"))
        .expect("shared-borrow benchmark Rust reference should be readable");

    let tokens = lex(&evolution_source).expect("shared-borrow benchmark should lex");
    let syntax = parse(&tokens).expect("shared-borrow benchmark should parse");
    let lowered = lower(&syntax).expect("shared-borrow benchmark should lower");
    let generated = generate_lowered_rust(&lowered);

    assert_eq!(
        normalize_newlines(&reference),
        normalize_newlines(&generated),
        "inferred-shared-borrow-v0 reference.rs must mirror generated static Rust exactly"
    );
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}
