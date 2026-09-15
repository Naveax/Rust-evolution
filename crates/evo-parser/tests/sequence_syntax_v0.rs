use evo_lexer::lex;
use evo_parser::{ExprKind, StmtKind, TypeName, parse};

fn parse_source(source: &str) -> evo_parser::Program {
    parse(&lex(source).expect("lexing should succeed")).expect("parsing should succeed")
}

#[test]
fn parses_contextual_sequence_surface() {
    let program = parse_source(
        "record Item\nvalue int\nend\nfn keep(items seq Item) seq Item\nreturn items\nend\nitems = seq Item()\nappend items, Item(value = 7)\nlookup items, 0 as item\nprint item.value\nelse\nprint 0\nend\n",
    );
    assert_eq!(
        program.functions[0].parameters[0].type_name,
        TypeName::Sequence(Box::new(TypeName::Named("Item".to_owned())))
    );
    assert_eq!(
        program.functions[0].return_type,
        TypeName::Sequence(Box::new(TypeName::Named("Item".to_owned())))
    );
    let StmtKind::Bind { expr, .. } = &program.statements[0].kind else {
        panic!("expected sequence binding");
    };
    assert!(matches!(expr.kind, ExprKind::SequenceNew { .. }));
    assert!(matches!(
        program.statements[1].kind,
        StmtKind::SequenceAppend { .. }
    ));
    let StmtKind::SequenceLookup {
        binding,
        then_body,
        else_body,
        ..
    } = &program.statements[2].kind
    else {
        panic!("expected checked lookup");
    };
    assert_eq!(binding, "item");
    assert_eq!(then_body.len(), 1);
    assert_eq!(else_body.len(), 1);
}

#[test]
fn contextual_words_remain_identifiers_outside_exact_positions() {
    let program = parse_source(
        "fn seq(value int) int\nreturn value\nend\nseq = 1\nappend = seq(seq)\nlookup = append\nas = lookup\nprint as\n",
    );
    assert_eq!(program.statements.len(), 5);
}

#[test]
fn checked_lookup_requires_explicit_failure_branch() {
    let error =
        parse(&lex("items = seq int()\nlookup items, 0 as value\nprint value\nend\n").unwrap())
            .expect_err("lookup must expose failure");
    assert!(error.message.contains("explicit 'else'"));
}

#[test]
fn rejects_nested_sequence_constructor_surface() {
    let error =
        parse(&lex("items = seq seq int()\n").unwrap()).expect_err("nested sequence is outside v0");
    assert!(error.message.contains("nested sequence"));
}

#[test]
fn removal_surface_stays_absent_in_v0() {
    let error = parse(&lex("items = seq int()\nremove items, 0\n").unwrap())
        .expect_err("v0 must not expose removal syntax");
    assert!(error.message.contains("expected '=' after binding name"));
}

#[test]
fn reference_element_types_stay_outside_v0() {
    let error = parse(&lex("fn bad(items seq &Item) int\nreturn 1\nend\n").unwrap())
        .expect_err("reference sequence elements are outside v0");
    assert!(error.message.contains("function parameters") || error.message.contains("type"));
}
