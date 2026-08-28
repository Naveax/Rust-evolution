use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;

fn lower_source(source: &str) -> evo_lowering::Program {
    let tokens = lex(source).expect("promotion source should lex");
    let syntax = parse(&tokens).expect("promotion source should parse");
    lower(&syntax).expect("promotion source should lower")
}

#[test]
fn valid_enum_program_is_promoted_into_lowered_program() {
    let program = lower_source(
        "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nmatch value\ncase Flag.Off\nprint 0\ncase Flag.On\nprint 1\nend\n",
    );

    assert!(program.has_enum_program());
    assert_eq!(program.enum_source_span().map(|span| span.line), Some(1));
    assert!(program.records.is_empty());
    assert!(program.functions.is_empty());
    assert!(program.statements.is_empty());
}

#[test]
fn existing_program_keeps_legacy_lowered_shape_without_enum_payload() {
    let program = lower_source("x = 1\nprint x\n");

    assert!(!program.has_enum_program());
    assert_eq!(program.enum_source_span(), None);
    assert!(program.records.is_empty());
    assert!(program.functions.is_empty());
    assert_eq!(program.statements.len(), 2);
}
