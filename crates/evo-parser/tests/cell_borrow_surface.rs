use evo_lexer::lex;
use evo_parser::{ExprKind, StmtKind, TypeName, parse};

fn parse_source(source: &str) -> evo_parser::Program {
    parse(&lex(source).expect("cell/borrow source should lex"))
        .expect("cell/borrow source should parse")
}

#[test]
fn parses_explicit_owned_cell_type_and_constructor() {
    let program = parse_source(concat!(
        "record Item\n",
        "value int\n",
        "end\n",
        "fn keep(state cell Item) cell Item\n",
        "return state\n",
        "end\n",
        "state = cell Item(value = 1)\n",
    ));

    let expected = TypeName::Cell(Box::new(TypeName::Named("Item".to_owned())));
    assert_eq!(program.functions[0].parameters[0].type_name, expected);
    assert_eq!(program.functions[0].return_type, expected);

    let StmtKind::Bind { expr, .. } = &program.statements[0].kind else {
        panic!("expected cell binding");
    };
    assert!(matches!(expr.kind, ExprKind::CellNew(_)));
}

#[test]
fn parses_lexical_dynamic_borrow_blocks_and_bounded_replace() {
    let program = parse_source(concat!(
        "record Item\n",
        "value int\n",
        "end\n",
        "state = cell Item(value = 1)\n",
        "borrow state as view\n",
        "print view.value\n",
        "end\n",
        "borrow_mut state as edit\n",
        "replace edit, Item(value = 2)\n",
        "end\n",
        "try_borrow state as view\n",
        "print view.value\n",
        "else\n",
        "print 0\n",
        "end\n",
        "try_borrow_mut state as edit\n",
        "replace edit, Item(value = 3)\n",
        "else\n",
        "print 0\n",
        "end\n",
    ));

    assert!(matches!(
        program.statements[1].kind,
        StmtKind::Borrow { .. }
    ));
    let StmtKind::BorrowMut { body, .. } = &program.statements[2].kind else {
        panic!("expected exclusive borrow block");
    };
    assert!(matches!(body[0].kind, StmtKind::Replace { .. }));
    assert!(matches!(
        program.statements[3].kind,
        StmtKind::TryBorrow { .. }
    ));
    let StmtKind::TryBorrowMut { then_body, .. } = &program.statements[4].kind else {
        panic!("expected checked exclusive borrow block");
    };
    assert!(matches!(then_body[0].kind, StmtKind::Replace { .. }));
}

#[test]
fn cell_and_borrow_words_remain_contextual() {
    let program = parse_source(concat!(
        "record cell\n",
        "value int\n",
        "end\n",
        "fn cell(x int) int\n",
        "return x\n",
        "end\n",
        "fn keep(x cell) cell\n",
        "return x\n",
        "end\n",
        "cell = 7\n",
        "borrow = cell\n",
        "borrow_mut = borrow\n",
        "try_borrow = borrow_mut\n",
        "try_borrow_mut = try_borrow\n",
        "replace = try_borrow_mut\n",
        "print cell(replace)\n",
    ));

    assert_eq!(
        program.functions[1].parameters[0].type_name,
        TypeName::Named("cell".to_owned())
    );
    assert_eq!(
        program.functions[1].return_type,
        TypeName::Named("cell".to_owned())
    );
    let StmtKind::Print(expr) = &program.statements[6].kind else {
        panic!("expected ordinary cell call");
    };
    assert!(matches!(expr.kind, ExprKind::Call { ref name, .. } if name == "cell"));
}

#[test]
fn rejects_unsupported_cell_storage_and_nested_container_shapes() {
    for (source, expected) in [
        (
            "record Item\nvalue int\nend\nrecord Holder\nstate cell Item\nend\n",
            "cell record fields",
        ),
        (
            "record Item\nvalue int\nend\nenum Maybe\nSome cell Item\nend\n",
            "cell enum payloads",
        ),
        (
            "record Item\nvalue int\nend\nfn bad(xs seq cell Item) int\nreturn 0\nend\n",
            "cell sequence elements",
        ),
        (
            "record Item\nvalue int\nend\nfn bad(xs arena cell Item) int\nreturn 0\nend\n",
            "cell arena payloads",
        ),
        (
            "fn bad(state cell int) int\nreturn 0\nend\n",
            "cell types require a nominal record type",
        ),
    ] {
        let error = parse(&lex(source).expect("rejection source should lex"))
            .expect_err("unsupported cell shape must fail closed");
        assert!(error.message.contains(expected), "{}", error.message);
    }
}

#[test]
fn checked_dynamic_borrow_requires_explicit_else() {
    let source = concat!(
        "record Item\n",
        "value int\n",
        "end\n",
        "state = cell Item(value = 1)\n",
        "try_borrow state as view\n",
        "print view.value\n",
        "end\n",
    );
    let error = parse(&lex(source).unwrap()).expect_err("checked borrow must expose failure");
    assert!(error.message.contains("explicit 'else'"));
}
