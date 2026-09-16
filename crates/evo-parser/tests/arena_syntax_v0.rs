use evo_lexer::lex;
use evo_parser::{ExprKind, RecordFieldType, StmtKind, TypeName, parse};

fn parse_source(source: &str) -> evo_parser::Program {
    parse(&lex(source).expect("arena source should lex")).expect("arena source should parse")
}

#[test]
fn parses_contextual_arena_surface_and_handle_contracts() {
    let program = parse_source(
        "record Node\nnext handle Node\nend\nfn keep(h handle Node) handle Node\nreturn h\nend\nitems = arena Node()\ninsert items, Node(next = keep) as h\nlookup items, h as item\nprint item.next\nelse\nprint 0\nend\nremove items, h as removed\nprint removed.next\nelse\nprint 0\nend\n",
    );
    assert_eq!(
        program.records[0].fields[0].type_name,
        RecordFieldType::Handle("Node".to_owned())
    );
    assert_eq!(
        program.functions[0].parameters[0].type_name,
        TypeName::Handle(Box::new(TypeName::Named("Node".to_owned())))
    );
    assert_eq!(
        program.functions[0].return_type,
        TypeName::Handle(Box::new(TypeName::Named("Node".to_owned())))
    );
    let StmtKind::Bind { expr, .. } = &program.statements[0].kind else {
        panic!("expected arena binding");
    };
    assert!(matches!(expr.kind, ExprKind::ArenaNew { .. }));
    assert!(matches!(
        program.statements[1].kind,
        StmtKind::ArenaInsert { .. }
    ));
    assert!(matches!(
        program.statements[2].kind,
        StmtKind::SequenceLookup { .. }
    ));
    assert!(matches!(
        program.statements[3].kind,
        StmtKind::ArenaRemove { .. }
    ));
}

#[test]
fn sequence_can_store_typed_handles_without_general_generic_syntax() {
    let program = parse_source(
        "record Node\nvalue int\nend\nfn edges(xs seq handle Node) seq handle Node\nreturn xs\nend\n",
    );
    let expected = TypeName::Sequence(Box::new(TypeName::Handle(Box::new(TypeName::Named(
        "Node".to_owned(),
    )))));
    assert_eq!(program.functions[0].parameters[0].type_name, expected);
}

#[test]
fn contextual_arena_words_remain_ordinary_identifiers_elsewhere() {
    let program = parse_source(
        "fn arena(value int) int\nreturn value\nend\narena = 1\nhandle = arena(arena)\ninsert = handle\nremove = insert\nprint remove\n",
    );
    assert_eq!(program.statements.len(), 5);
}

#[test]
fn checked_remove_requires_explicit_failure_branch() {
    let error =
        parse(&lex("items = arena int()\nremove items, h as value\nprint value\nend\n").unwrap())
            .expect_err("remove must expose failure");
    assert!(error.message.contains("explicit 'else'"));
}

#[test]
fn rejects_nested_or_reference_arena_payloads() {
    let nested =
        parse(&lex("items = arena seq int()\n").unwrap()).expect_err("nested payload must fail");
    assert!(nested.message.contains("nested"));
    let borrowed = parse(&lex("fn bad(items arena &Node) int\nreturn 1\nend\n").unwrap())
        .expect_err("reference payload must fail");
    assert!(borrowed.message.contains("type") || borrowed.message.contains("payload"));
}
