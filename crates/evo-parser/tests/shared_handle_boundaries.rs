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

#[test]
fn rejects_weak_owner_scalar_types_in_v0() {
    for source in [
        "fn f(value weak int) int\nreturn 1\nend\n",
        "fn f(value weak bool) int\nreturn 1\nend\n",
        "fn f(value weak string) int\nreturn 1\nend\n",
    ] {
        let error = parse_error(source);
        assert!(error.message.contains("weak-owner"));
        assert!(error.message.contains("nominal record type"));
    }
}

#[test]
fn rejects_weak_owner_storage_shapes_outside_local_function_slice() {
    for source in [
        "record Item\nvalue int\nend\nrecord Holder\nedge weak Item\nend\n",
        "record Item\nvalue int\nend\nenum MaybeItem\nSome weak Item\nend\n",
        "record Item\nvalue int\nend\nfn f(items seq weak Item) int\nreturn 1\nend\n",
        "record Item\nvalue int\nend\nfn f(items arena weak Item) int\nreturn 1\nend\n",
    ] {
        let error = parse_error(source);
        assert!(error.message.contains("weak-owner"));
    }
}

#[test]
fn checked_upgrade_requires_explicit_else_branch() {
    let source = concat!(
        "record Item\nvalue int\nend\n",
        "owner = share Item(value = 1)\n",
        "edge = downgrade owner\n",
        "upgrade edge as live\n",
        "print live.value\n",
        "end\n",
    );
    let error = parse_error(source);
    assert!(error.message.contains("explicit 'else'"));
}
