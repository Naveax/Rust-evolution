pub use evo_lowering::LowerError;

#[path = "../src/record_constructor.rs"]
mod record_constructor;
#[path = "../src/record_environment.rs"]
mod record_environment;

use evo_lexer::{Span, lex};
use evo_parser::parse;
use record_constructor::lower_constructor_fields;
use record_environment::{ConstructorFieldInput, SemanticType, collect_record_environment};

fn span(line: usize) -> Span {
    Span {
        start: 0,
        end: 1,
        line,
        column: 1,
    }
}

fn environment(source: &str) -> record_environment::RecordEnvironment {
    let tokens = lex(source).expect("constructor source should lex");
    let program = parse(&tokens).expect("constructor source should parse");
    collect_record_environment(&program).expect("record environment should build")
}

#[test]
fn constructor_fields_are_normalized_to_declaration_order() {
    let records = environment("record Point\nx int\ny bool\nend\n");
    assert_eq!(
        records
            .schema("Point")
            .expect("Point schema should exist")
            .span
            .line,
        1
    );
    let lowered = lower_constructor_fields(
        &records,
        "Point",
        vec![
            ConstructorFieldInput {
                name: "y".to_owned(),
                value_type: SemanticType::Bool,
                span: span(3),
            },
            ConstructorFieldInput {
                name: "x".to_owned(),
                value_type: SemanticType::Integer,
                span: span(2),
            },
        ],
        span(1),
    )
    .expect("valid named fields should normalize");

    assert_eq!(lowered.value_type, SemanticType::Record("Point".to_owned()));
    assert_eq!(lowered.fields[0].name, "x");
    assert_eq!(lowered.fields[1].name, "y");
    assert_eq!(lowered.fields[0].span.line, 2);
    assert_eq!(lowered.fields[1].span.line, 3);
}

#[test]
fn zero_field_constructor_normalizes_to_empty_field_list() {
    let records = environment("record Marker\nend\n");
    let lowered = lower_constructor_fields(&records, "Marker", Vec::new(), span(1))
        .expect("zero-field record should construct without synthetic fields");
    assert_eq!(
        lowered.value_type,
        SemanticType::Record("Marker".to_owned())
    );
    assert!(lowered.fields.is_empty());
}

#[test]
fn constructor_validation_errors_are_preserved_before_ordering() {
    let records = environment("record Point\nx int\ny bool\nend\n");
    let error = lower_constructor_fields(
        &records,
        "Point",
        vec![ConstructorFieldInput {
            name: "x".to_owned(),
            value_type: SemanticType::Integer,
            span: span(2),
        }],
        span(1),
    )
    .expect_err("missing fields must still fail at semantic validation");
    assert!(error.message.contains("missing field"));
}
