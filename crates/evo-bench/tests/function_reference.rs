use evo_codegen_rust::generate_lowered_rust;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;
use std::fs;
use std::path::PathBuf;

#[test]
fn function_call_reference_matches_generated_rust_exactly() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let case_dir = manifest_dir.join("../../benchmarks/cases/function-call-v0");

    let evolution_source = fs::read_to_string(case_dir.join("evolution.evo"))
        .expect("function benchmark Evolution source should be readable");
    let reference = fs::read_to_string(case_dir.join("reference.rs"))
        .expect("function benchmark Rust reference should be readable");

    let tokens = lex(&evolution_source).expect("function benchmark should lex");
    let syntax = parse(&tokens).expect("function benchmark should parse");
    let lowered = lower(&syntax).expect("function benchmark should lower");
    let generated = generate_lowered_rust(&lowered);

    assert_eq!(
        reference, generated,
        "function-call-v0 reference.rs must mirror generated static Rust exactly"
    );
}
