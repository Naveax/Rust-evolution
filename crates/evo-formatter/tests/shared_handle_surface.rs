use evo_formatter::format_source;
use evo_lexer::lex;

fn format(source: &str) -> String {
    let tokens = lex(source).expect("shared-handle formatter source should lex");
    format_source(source, &tokens)
}

#[test]
fn preserves_contextual_shared_handle_surface_idempotently() {
    let source = concat!(
        "fn forward(item shared Item)shared Item\n",
        "return dup item\n",
        "end\n",
        "owner=share Item(value=1)\n",
        "alias=dup owner\n",
    );
    let expected = concat!(
        "fn forward(item shared Item) shared Item\n",
        "    return dup item\n",
        "end\n",
        "owner = share Item(value = 1)\n",
        "alias = dup owner\n",
    );
    let once = format(source);
    assert_eq!(once, expected);
    assert_eq!(format(&once), once);
}

#[test]
fn preserves_existing_function_call_spelling_for_contextual_words() {
    let source = "print share(dup(shared))\n";
    assert_eq!(format(source), source);
}

#[test]
fn formats_weak_surface_and_checked_upgrade_idempotently() {
    let source = concat!(
        "fn forward(edge weak Item)weak Item\n",
        "return edge\n",
        "end\n",
        "owner=share Item(value=1)\n",
        "edge=downgrade owner\n",
        "upgrade edge as live\n",
        "print live.value\n",
        "else\n",
        "print 0\n",
        "end\n",
    );
    let expected = concat!(
        "fn forward(edge weak Item) weak Item\n",
        "    return edge\n",
        "end\n",
        "owner = share Item(value = 1)\n",
        "edge = downgrade owner\n",
        "upgrade edge as live\n",
        "    print live.value\n",
        "else\n",
        "    print 0\n",
        "end\n",
    );
    let once = format(source);
    assert_eq!(once, expected);
    assert_eq!(format(&once), once);
}

#[test]
fn ordinary_upgrade_call_does_not_open_formatter_indentation() {
    let source = "print upgrade(edge)\nprint 1\n";
    assert_eq!(format(source), source);
}
