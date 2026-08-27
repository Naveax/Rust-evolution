pub use evo_lowering::LowerError;

#[path = "../src/record_environment.rs"]
mod record_environment;
#[path = "../src/record_ownership.rs"]
mod record_ownership;
#[path = "../src/record_resolution.rs"]
mod record_resolution;
#[path = "../src/record_signatures.rs"]
mod record_signatures;

use evo_lexer::lex;
use evo_parser::parse;

#[test]
fn record_environment_builds_for_valid_nominal_schema() {
    let source = "record Point\nx int\ny bool\nend\n";
    let tokens = lex(source).expect("record source should lex");
    let program = parse(&tokens).expect("record source should parse");
    let environment = record_environment::collect_record_environment(&program)
        .expect("acyclic nominal record declaration should validate");
    let point = environment
        .schema("Point")
        .expect("Point schema should exist");
    assert_eq!(point.span.line, 1);
}
