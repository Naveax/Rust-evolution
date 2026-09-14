use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;

fn lower_source(source: &str) -> Result<evo_lowering::Program, evo_lowering::LowerError> {
    let tokens = lex(source).expect("shared-owner move/reinit source should lex");
    let syntax = parse(&tokens).expect("shared-owner move/reinit source should parse");
    lower(&syntax)
}

const ITEM: &str = "record Item\nvalue int\nend\n";

#[test]
fn moved_shared_owner_can_be_explicitly_reinitialized_with_same_type() {
    let source = format!(
        "{ITEM}owner = share Item(value = 1)\nmoved = owner\nowner = share Item(value = 2)\nprint moved.value\nprint owner.value\n"
    );
    lower_source(&source)
        .expect("same-type explicit reinitialization should restore a moved shared-owner local");
}

#[test]
fn dup_of_moved_shared_owner_is_rejected_without_hidden_recovery() {
    let source = format!(
        "{ITEM}owner = share Item(value = 1)\nmoved = owner\nalias = dup owner\nprint moved.value\n"
    );
    let error = lower_source(&source).expect_err("dup must not revive a moved shared-owner handle");
    assert!(error.message.contains("moved shared handle"));
}
