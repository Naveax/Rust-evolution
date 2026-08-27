use evo_lexer::{TokenKind, lex, lex_recovering};

#[test]
fn tokenizes_record_keyword_named_constructor_and_field_access() {
    let source = concat!(
        "record Point\n",
        "x int\n",
        "y int\n",
        "end\n",
        "p = Point(x = 2, y = 3)\n",
        "print p.x\n"
    );
    let tokens = lex(source).expect("records source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(kinds.contains(&TokenKind::Record));
    assert!(kinds.contains(&TokenKind::Dot));
    assert!(kinds.contains(&TokenKind::Comma));
    assert!(kinds.contains(&TokenKind::TypeInt));
}

#[test]
fn record_keyword_prefix_remains_an_identifier() {
    let tokens = lex("recording = 1\n").expect("identifier should lex");
    assert!(matches!(
        &tokens[0].kind,
        TokenKind::Identifier(name) if name == "recording"
    ));
}

#[test]
fn recovering_lexer_matches_fail_fast_for_records_source() {
    let source = "record Point\nx int\nend\np = Point(x = 1)\nprint p.x\n";
    assert_eq!(
        lex_recovering(source).expect("recovery lexing should succeed"),
        lex(source).expect("fail-fast lexing should succeed")
    );
}
