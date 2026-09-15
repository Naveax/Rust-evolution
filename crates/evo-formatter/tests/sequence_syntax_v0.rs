use evo_formatter::format_source;
use evo_lexer::lex;

#[test]
fn formats_contextual_lookup_block() {
    let source = "items=seq Item()\nappend items,Item(value=1)\nlookup items,0 as item\nprint item.value\nelse\nprint 0\nend\n";
    let tokens = lex(source).expect("sequence source should lex");
    assert_eq!(
        format_source(source, &tokens),
        "items = seq Item()\nappend items, Item(value = 1)\nlookup items, 0 as item\n    print item.value\nelse\n    print 0\nend\n"
    );
}
