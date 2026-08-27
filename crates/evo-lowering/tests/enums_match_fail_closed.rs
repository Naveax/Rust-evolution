use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;

#[test]
fn parsed_match_fails_closed_before_semantic_lowering_or_codegen() {
    let source = "value = 1\nmatch value\ncase Maybe.None\nprint 0\nend\n";
    let tokens = lex(source).expect("match fail-closed source should lex");
    let syntax = parse(&tokens).expect("match fail-closed source should parse");
    let error = lower(&syntax).expect_err("parsed match must stop before semantic lowering/codegen");

    assert!(error.message.contains("match statements are parsed"));
    assert!(error.message.contains("semantic lowering/codegen"));
    assert_eq!(error.span.line, 2);
}
