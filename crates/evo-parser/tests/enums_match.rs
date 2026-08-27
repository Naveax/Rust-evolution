use evo_lexer::lex;
use evo_parser::{MatchPattern, StmtKind, parse, parse_recovering};

fn parse_source(source: &str) -> evo_parser::Program {
    let tokens = lex(source).expect("match parser source should lex");
    parse(&tokens).expect("match parser source should parse")
}

#[test]
fn parses_statement_match_with_unit_and_payload_binding_arms() {
    let program = parse_source(
        "value = MaybeInt.Some(41)\nmatch value\ncase MaybeInt.Some(x)\nprint x\ncase MaybeInt.None\nprint 0\nend\n",
    );

    let StmtKind::Match { value, arms } = &program.statements[1].kind else {
        panic!("expected match statement");
    };
    assert_eq!(value.span.line, 2);
    assert_eq!(arms.len(), 2);

    assert_eq!(
        arms[0].pattern,
        MatchPattern {
            enum_name: "MaybeInt".to_owned(),
            variant_name: "Some".to_owned(),
            binding: Some("x".to_owned()),
            span: arms[0].pattern.span,
        }
    );
    assert_eq!(arms[0].pattern.span.line, 3);
    assert_eq!(arms[0].body.len(), 1);

    assert_eq!(arms[1].pattern.enum_name, "MaybeInt");
    assert_eq!(arms[1].pattern.variant_name, "None");
    assert_eq!(arms[1].pattern.binding, None);
    assert_eq!(arms[1].pattern.span.line, 5);
    assert_eq!(arms[1].body.len(), 1);
}

#[test]
fn nested_match_if_and_repeat_keep_their_own_end_boundaries() {
    let program = parse_source(
        "if true\nmatch value\ncase Maybe.Some(x)\nrepeat 1\nprint x\nend\ncase Maybe.None\nif false\nprint 0\nend\nend\nend\n",
    );

    let StmtKind::If { then_body, .. } = &program.statements[0].kind else {
        panic!("expected outer if");
    };
    let StmtKind::Match { arms, .. } = &then_body[0].kind else {
        panic!("expected nested match");
    };
    assert_eq!(arms.len(), 2);
    assert!(matches!(arms[0].body[0].kind, StmtKind::Repeat { .. }));
    assert!(matches!(arms[1].body[0].kind, StmtKind::If { .. }));
}

#[test]
fn rejects_stray_case_and_malformed_case_patterns_source_natively() {
    let stray = lex("case Maybe.None\nprint 1\n").expect("source should lex");
    let error = parse(&stray).expect_err("stray case must fail");
    assert!(error.message.contains("unexpected 'case'"));
    assert_eq!(error.span.line, 1);

    let missing_enum = lex("match value\ncase None\nprint 0\nend\n").expect("source should lex");
    let error = parse(&missing_enum).expect_err("unqualified case must fail");
    assert!(error.message.contains("expected '.'"));
    assert_eq!(error.span.line, 2);

    let empty_payload =
        lex("match value\ncase Maybe.Some()\nprint 1\nend\n").expect("source should lex");
    let error = parse(&empty_payload).expect_err("empty payload pattern must fail");
    assert!(error.message.contains("payload binding"));
    assert_eq!(error.span.line, 2);
}

#[test]
fn diagnoses_missing_match_expression_case_and_end() {
    let missing_expression = lex("match\ncase Maybe.None\nprint 0\nend\n").expect("source should lex");
    let error = parse(&missing_expression).expect_err("match expression is required");
    assert!(error.message.contains("expression after 'match'"));
    assert_eq!(error.span.line, 1);

    let missing_case = lex("match value\nend\n").expect("source should lex");
    let error = parse(&missing_case).expect_err("match needs a case arm");
    assert!(error.message.contains("at least one 'case'"));
    assert_eq!(error.span.line, 2);

    let missing_end =
        lex("match value\ncase Maybe.None\nprint 0\n").expect("source should lex");
    let error = parse(&missing_end).expect_err("match needs a closing end");
    assert!(error.message.contains("missing 'end' for match"));
}

#[test]
fn recovering_parser_keeps_case_boundaries_bounded() {
    let source = concat!(
        "case Maybe.None\n",
        "print 1\n",
        "match value\n",
        "case Maybe.Some()\n",
        "print 2\n",
        "case Maybe.None\n",
        "print 0\n",
        "end\n",
        "print 3\n",
    );
    let tokens = lex(source).expect("source should lex");
    let errors = parse_recovering(&tokens).expect_err("recovery should report malformed cases");
    assert!(errors.len() <= 8);
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("unexpected 'case'"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("payload binding"))
    );
}
