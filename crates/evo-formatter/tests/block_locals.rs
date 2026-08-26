use evo_formatter::format_source;
use evo_lexer::lex;

fn format(source: &str) -> String {
    let tokens = lex(source).expect("formatter test source should lex");
    format_source(source, &tokens)
}

#[test]
fn formats_block_local_bindings_and_is_idempotent() {
    let source = concat!(
        "if(true)\n",
        "inside=1\n",
        "repeat 2\n",
        "temp=inside+1\n",
        "print temp\n",
        "end\n",
        "else\n",
        "inside=2\n",
        "print inside\n",
        "end\n",
    );
    let expected = concat!(
        "if (true)\n",
        "    inside = 1\n",
        "    repeat 2\n",
        "        temp = inside + 1\n",
        "        print temp\n",
        "    end\n",
        "else\n",
        "    inside = 2\n",
        "    print inside\n",
        "end\n",
    );

    let once = format(source);
    assert_eq!(once, expected);
    assert_eq!(format(&once), once);
}
