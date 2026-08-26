use evo_codegen_rust::{GeneratedRust, generate_lowered_rust_with_map};
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;
use std::fs;
use std::path::PathBuf;

fn lower_source(source: &str) -> evo_lowering::Program {
    let tokens = lex(source).expect("lexing should succeed");
    let syntax = parse(&tokens).expect("parsing should succeed");
    lower(&syntax).expect("lowering should succeed")
}

fn generated_line_containing(generated: &GeneratedRust, needle: &str) -> usize {
    generated
        .source
        .lines()
        .position(|line| line.contains(needle))
        .map(|index| index + 1)
        .unwrap_or_else(|| panic!("generated Rust did not contain {needle:?}"))
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}

#[test]
fn block_locals_lower_to_plain_lexical_rust_and_keep_source_mappings() {
    let source = concat!(
        "x = 1\n",
        "if true\n",
        "temp = x + 1\n",
        "temp = temp + 1\n",
        "x = x + temp\n",
        "else\n",
        "temp = x + 2\n",
        "x = x + temp\n",
        "end\n",
        "print x\n",
    );
    let program = lower_source(source);
    let generated = generate_lowered_rust_with_map(&program);

    assert!(generated.source.contains("let mut __evo_x = 1;"));
    assert!(
        generated
            .source
            .contains("let mut __evo_temp = (__evo_x + 1);")
    );
    assert!(generated.source.contains("__evo_temp = (__evo_temp + 1);"));
    assert!(generated.source.contains("let __evo_temp = (__evo_x + 2);"));
    assert!(generated.source.contains("__evo_x = (__evo_x + __evo_temp);"));

    for forbidden in ["HashMap", "Box<", "Rc<", "RefCell", "dyn "] {
        assert!(
            !generated.source.contains(forbidden),
            "block-local codegen unexpectedly contains {forbidden:?}:\n{}",
            generated.source
        );
    }

    let then_declaration = generated_line_containing(&generated, "let mut __evo_temp");
    let then_reassignment = generated_line_containing(&generated, "__evo_temp = (__evo_temp + 1)");
    let else_declaration = generated_line_containing(&generated, "let __evo_temp = (__evo_x + 2)");

    assert_eq!(
        generated
            .source_span_for_line(then_declaration)
            .map(|span| span.line),
        Some(3)
    );
    assert_eq!(
        generated
            .source_span_for_line(then_reassignment)
            .map(|span| span.line),
        Some(4)
    );
    assert_eq!(
        generated
            .source_span_for_line(else_declaration)
            .map(|span| span.line),
        Some(7)
    );
}

#[test]
fn block_locals_benchmark_reference_matches_generated_rust() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let case_dir = manifest_dir.join("../../benchmarks/cases/block-locals-v0");
    let evolution = fs::read_to_string(case_dir.join("evolution.evo"))
        .expect("benchmark Evolution source should exist");
    let reference = fs::read_to_string(case_dir.join("reference.rs"))
        .expect("benchmark reference Rust should exist");

    let program = lower_source(&evolution);
    let generated = generate_lowered_rust_with_map(&program);

    assert_eq!(
        normalize_newlines(&generated.source),
        normalize_newlines(&reference),
        "block-locals benchmark reference must stay locked to ordinary generated Rust"
    );
}
