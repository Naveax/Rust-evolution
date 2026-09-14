use evo_lexer::lex;
use evo_parser::parse;

fn parse_error(source: &str) -> evo_parser::ParseError {
    let tokens = lex(source).expect("shared-owner boundary source should lex");
    parse(&tokens).expect_err("shared-owner boundary source should be rejected")
}

#[test]
fn rejects_shared_owner_scalar_types_in_v0() {
    for source in [
        "fn f(value shared int) int\nreturn 1\nend\n",
        "fn f(value shared bool) int\nreturn 1\nend\n",
        "fn f(value shared string) int\nreturn 1\nend\n",
    ] {
        let error = parse_error(source);
        assert!(error.message.contains("nominal record type"));
    }
}

#[test]
fn rejects_nested_shared_owner_types() {
    let source = concat!(
        "record Item\nvalue int\nend\n",
        "fn f(value shared shared Item) int\nreturn 1\nend\n",
    );
    parse_error(source);
}

#[test]
fn field_assignment_is_not_a_mutation_escape_hatch_for_shared_ownership() {
    let source = concat!(
        "record Item\nvalue int\nend\n",
        "owner = share Item(value = 1)\n",
        "owner.value = 2\n",
    );
    parse_error(source);
}

#[test]
fn contextual_words_stay_ordinary_identifiers_outside_shared_forms() {
    let source = concat!(
        "record shared\nvalue int\nend\n",
        "fn share(dup int) int\nreturn dup\nend\n",
        "value = shared(value = 1)\n",
        "print share(value.value)\n",
    );
    let tokens = lex(source).expect("compatibility source should lex");
    parse(&tokens).expect("shared/share/dup identifier compatibility must remain intact");
}
