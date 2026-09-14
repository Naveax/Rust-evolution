use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;

fn lower_error(source: &str) -> evo_lowering::LowerError {
    let tokens = lex(source).expect("shared-owner conversion boundary source should lex");
    let syntax = parse(&tokens).expect("shared-owner conversion boundary source should parse");
    lower(&syntax).expect_err("implicit ownership-category conversion must be rejected")
}

const ITEM: &str = "record Item\nvalue int\nend\n";

#[test]
fn owned_record_does_not_implicitly_convert_to_shared_owner() {
    let source = format!(
        "{ITEM}fn consume(item shared Item) int\nreturn item.value\nend\nitem = Item(value = 1)\nprint consume(item)\n"
    );
    let error = lower_error(&source);
    assert!(error.message.contains("expects shared Item"));
    assert!(error.message.contains("found Item"));
}

#[test]
fn shared_owner_does_not_implicitly_convert_to_reference() {
    let source = format!(
        "{ITEM}fn read(item &Item) int\nreturn item.value\nend\nowner = share Item(value = 1)\nprint read(owner)\n"
    );
    let error = lower_error(&source);
    assert!(error.message.contains("expects &Item"));
    assert!(error.message.contains("shared Item"));
}

#[test]
fn reference_does_not_implicitly_convert_to_shared_owner() {
    let source = format!(
        "{ITEM}fn consume(item shared Item) int\nreturn item.value\nend\nowner = share Item(value = 1)\nr = &owner\nprint consume(r)\n"
    );
    let error = lower_error(&source);
    assert!(error.message.contains("expects shared Item"));
    assert!(error.message.contains("&Item"));
}
