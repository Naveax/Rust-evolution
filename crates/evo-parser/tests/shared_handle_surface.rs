use evo_lexer::lex;
use evo_parser::{ExprKind, StmtKind, TypeName, parse};

fn parse_source(source: &str) -> evo_parser::Program {
    let tokens = lex(source).expect("shared-handle parser source should lex");
    parse(&tokens).expect("shared-handle parser source should parse")
}

#[test]
fn parses_contextual_shared_owner_signature_types() {
    let program = parse_source(concat!(
        "record Item\n",
        "value int\n",
        "end\n",
        "fn forward(item shared Item) shared Item\n",
        "return item\n",
        "end\n",
    ));
    let function = &program.functions[0];
    assert_eq!(
        function.parameters[0].type_name,
        TypeName::SharedOwner("Item".to_owned())
    );
    assert_eq!(
        function.return_type,
        TypeName::SharedOwner("Item".to_owned())
    );
}

#[test]
fn parses_contextual_share_and_dup_prefix_expressions() {
    let program = parse_source(concat!(
        "record Item\n",
        "value int\n",
        "end\n",
        "item = Item(value = 1)\n",
        "owner = share item\n",
        "alias = dup owner\n",
    ));
    let StmtKind::Bind { expr: owner, .. } = &program.statements[1].kind else {
        panic!("expected owner binding");
    };
    assert!(matches!(owner.kind, ExprKind::SharedAlloc(_)));
    let StmtKind::Bind { expr: alias, .. } = &program.statements[2].kind else {
        panic!("expected alias binding");
    };
    assert!(matches!(alias.kind, ExprKind::SharedDuplicate(_)));
}

#[test]
fn contextual_words_do_not_steal_existing_calls_or_names() {
    let program = parse_source(concat!(
        "record shared\n",
        "value int\n",
        "end\n",
        "fn share(x int) int\n",
        "return x\n",
        "end\n",
        "fn dup(x int) int\n",
        "return x\n",
        "end\n",
        "fn keep(x shared) shared\n",
        "return x\n",
        "end\n",
        "shared = 1\n",
        "print share(dup(shared))\n",
    ));
    assert_eq!(
        program.functions[2].parameters[0].type_name,
        TypeName::Named("shared".to_owned())
    );
    assert_eq!(
        program.functions[2].return_type,
        TypeName::Named("shared".to_owned())
    );
    let StmtKind::Print(expr) = &program.statements[1].kind else {
        panic!("expected print call");
    };
    assert!(matches!(expr.kind, ExprKind::Call { ref name, .. } if name == "share"));
}

#[test]
fn rejects_shared_owner_storage_in_record_and_enum_v0() {
    for source in [
        "record Item\nvalue int\nend\nrecord Holder\nitem shared Item\nend\n",
        "record Item\nvalue int\nend\nenum MaybeItem\nSome shared Item\nend\n",
    ] {
        let tokens = lex(source).expect("rejection source should lex");
        let error = parse(&tokens).expect_err("shared-owner storage must remain out of v0");
        assert!(error.message.contains("shared-owner"));
    }
}
