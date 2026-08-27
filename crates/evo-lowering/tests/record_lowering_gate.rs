use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;

fn lower_record_source(source: &str) -> Result<evo_lowering::Program, evo_lowering::LowerError> {
    let tokens = lex(source).expect("record gate source should lex");
    let program = parse(&tokens).expect("record gate source should parse");
    lower(&program)
}

#[test]
fn production_lowering_reports_schema_errors_before_value_lowering() {
    let duplicate = lower_record_source("record Point\nx int\nx bool\nend\n")
        .expect_err("duplicate record field should fail");
    assert!(duplicate.message.contains("duplicate field"));
    assert_eq!(duplicate.span.line, 3);

    let recursive = lower_record_source("record Node\nnext Node\nend\n")
        .expect_err("recursive by-value record should fail");
    assert!(recursive.message.contains("recursive by-value"));
    assert_eq!(recursive.span.line, 2);
}

#[test]
fn valid_schema_attaches_to_production_lowered_program() {
    let program = lower_record_source("record Point\nx int\nend\n")
        .expect("validated record schema should lower into production IR");
    assert_eq!(program.records.len(), 1);
    assert_eq!(program.records[0].name, "Point");
    assert_eq!(program.records[0].fields.len(), 1);
    assert_eq!(program.records[0].fields[0].name, "x");
    assert_eq!(program.records[0].span.line, 1);
    assert_eq!(program.records[0].fields[0].span.line, 2);
}
