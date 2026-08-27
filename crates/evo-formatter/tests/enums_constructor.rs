use evo_formatter::format_source;
use evo_lexer::lex;

fn format(source: &str) -> String {
    let tokens = lex(source).expect("enum constructor formatter source should lex");
    format_source(source, &tokens)
}

#[test]
fn formats_qualified_variant_constructors_and_is_idempotent() {
    let source = concat!(
        "enum MaybeInt\n",
        "None\n",
        "Some int\n",
        "end\n",
        "empty=MaybeInt.None( )\n",
        "value=MaybeInt.Some( 41 )\n",
        "probe=MaybeInt.Some(1,2)\n",
    );
    let expected = concat!(
        "enum MaybeInt\n",
        "    None\n",
        "    Some int\n",
        "end\n",
        "empty = MaybeInt.None()\n",
        "value = MaybeInt.Some(41)\n",
        "probe = MaybeInt.Some(1, 2)\n",
    );

    let once = format(source);
    assert_eq!(once, expected);
    assert_eq!(format(&once), once);
}
