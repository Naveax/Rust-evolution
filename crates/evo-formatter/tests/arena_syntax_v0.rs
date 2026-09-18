use evo_formatter::format_source;
use evo_lexer::lex;

#[test]
fn formats_contextual_arena_insert_lookup_remove_blocks_idempotently() {
    let source = "record Node\nnext handle Node\nend\nitems=arena Node()\ninsert items,Node(next=h)as h\nlookup items,h as item\nprint item.next\nelse\nprint 0\nend\nremove items,h as removed\nprint removed.next\nelse\nprint 0\nend\n";
    let expected = "record Node\n    next handle Node\nend\nitems = arena Node()\ninsert items, Node(next = h) as h\nlookup items, h as item\n    print item.next\nelse\n    print 0\nend\nremove items, h as removed\n    print removed.next\nelse\n    print 0\nend\n";
    let tokens = lex(source).expect("arena source should lex");
    let once = format_source(source, &tokens);
    assert_eq!(once, expected);
    let twice_tokens = lex(&once).expect("formatted arena source should lex");
    assert_eq!(format_source(&once, &twice_tokens), expected);
}
