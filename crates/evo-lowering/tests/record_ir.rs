#[path = "../src/record_ir.rs"]
mod record_ir;

use evo_lexer::lex;
use evo_parser::parse;
use record_ir::{RecordType, lower_record_schemas};

fn parse_source(source: &str) -> evo_parser::Program {
    let tokens = lex(source).expect("record IR source should lex");
    parse(&tokens).expect("record IR source should parse")
}

#[test]
fn lowers_record_declarations_in_source_order_with_spans() {
    let program =
        parse_source("record Point\nx int\ny bool\nend\nrecord Label\ntext string\nend\n");
    let records = lower_record_schemas(&program);

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].name, "Point");
    assert_eq!(records[0].span.line, 1);
    assert_eq!(records[0].fields.len(), 2);
    assert_eq!(records[0].fields[0].name, "x");
    assert_eq!(records[0].fields[0].value_type, RecordType::Integer);
    assert_eq!(records[0].fields[0].span.line, 2);
    assert_eq!(records[0].fields[1].name, "y");
    assert_eq!(records[0].fields[1].value_type, RecordType::Bool);
    assert_eq!(records[1].fields[0].value_type, RecordType::String);
}

#[test]
fn preserves_nominal_named_field_identity_for_forward_reference() {
    let program = parse_source("record Wrapper\npoint Point\nend\nrecord Point\nx int\nend\n");
    let records = lower_record_schemas(&program);

    assert_eq!(
        records[0].fields[0].value_type,
        RecordType::Named("Point".to_owned())
    );
}

#[test]
fn empty_program_has_no_record_ir() {
    let records = lower_record_schemas(&parse_source("print 1\n"));
    assert!(records.is_empty());
}
