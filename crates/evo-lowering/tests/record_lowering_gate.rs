use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;

fn lower_record_source(source: &str) -> evo_lowering::LowerError {
    let tokens = lex(source).expect("record gate source should lex");
    let program = parse(&tokens).expect("record gate source should parse");
    lower(&program).expect_err("record lowering is still fail-closed")
}

#[test]
fn production_lowering_reports_schema_errors_before_feature_gate() {
    let duplicate = lower_record_source("record Point\nx int\nx bool\nend\n");
    assert!(duplicate.message.contains("duplicate field"));
    assert_eq!(duplicate.span.line, 3);

    let recursive = lower_record_source("record Node\nnext Node\nend\n");
    assert!(recursive.message.contains("recursive by-value"));
    assert_eq!(recursive.span.line, 2);
}

#[test]
fn valid_schema_remains_fail_closed_until_value_lowering_lands() {
    let error = lower_record_source("record Point\nx int\nend\n");
    assert!(error.message.contains("Records v0 semantic lowering"));
    assert_eq!(error.span.line, 1);
}
