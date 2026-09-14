use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;

fn lower_source(source: &str) -> Result<evo_lowering::Program, evo_lowering::LowerError> {
    let tokens = lex(source).expect("shared-owner matrix source should lex");
    let syntax = parse(&tokens).expect("shared-owner matrix source should parse");
    lower(&syntax)
}

const ITEM: &str = "record Item\nvalue int\nend\n";

#[test]
fn repeat_duplicate_is_explicit_and_does_not_consume_source_owner() {
    let source = format!(
        "{ITEM}owner = share Item(value = 7)\nrepeat 3\nalias = dup owner\nprint alias.value\nend\nprint owner.value\n"
    );
    lower_source(&source)
        .expect("each repeat iteration may explicitly duplicate and drop its local alias");
}

#[test]
fn branch_duplicate_does_not_poison_source_owner_after_merge() {
    let source = format!(
        "{ITEM}owner = share Item(value = 7)\nif true\nalias = dup owner\nprint alias.value\nelse\nother = dup owner\nprint other.value\nend\nprint owner.value\n"
    );
    lower_source(&source).expect("branch-local duplicates must leave the source owner available");
}

#[test]
fn ordinary_move_inside_repeat_is_rejected_for_later_iterations() {
    let source = format!(
        "{ITEM}owner = share Item(value = 7)\nrepeat 2\nmoved = owner\nprint moved.value\nend\n"
    );
    let error = lower_source(&source)
        .expect_err("moving the source handle in repeat breaks a later iteration");
    assert!(error.message.contains("shared handle"));
    assert!(error.message.contains("later iteration"));
}

#[test]
fn allocation_and_handle_duplication_are_not_interchangeable() {
    let source = format!("{ITEM}owner = share Item(value = 7)\nsecond = share owner\n");
    let error =
        lower_source(&source).expect_err("share must not silently duplicate an existing handle");
    assert!(error.message.contains("share requires an owned record"));
    assert!(error.message.contains("shared Item"));
}

#[test]
fn shared_owner_reinitialization_is_same_type_only() {
    let same = format!(
        "{ITEM}owner = share Item(value = 1)\nowner = share Item(value = 2)\nprint owner.value\n"
    );
    lower_source(&same).expect("same shared-owner type may be explicitly reinitialized");

    let different = concat!(
        "record Item\nvalue int\nend\n",
        "record Other\nvalue int\nend\n",
        "owner = share Item(value = 1)\n",
        "owner = share Other(value = 2)\n",
    );
    let error =
        lower_source(different).expect_err("shared-owner locals must not change payload type");
    assert!(error.message.contains("different value type"));
}

#[test]
fn live_payload_reference_blocks_shared_owner_reinitialization() {
    let source = format!(
        "{ITEM}owner = share Item(value = 1)\nr = &owner\nowner = share Item(value = 2)\nprint r.value\n"
    );
    let error = lower_source(&source).expect_err(
        "reinitializing the source handle while its payload reference is live must fail",
    );
    assert!(
        error
            .message
            .contains("cannot reinitialize shared handle local")
    );
    assert!(error.message.contains("immutable reference"));
}

#[test]
fn source_owner_reinitialization_is_allowed_after_final_reference_use() {
    let source = format!(
        "{ITEM}owner = share Item(value = 1)\nr = &owner\nprint r.value\nowner = share Item(value = 2)\nprint owner.value\n"
    );
    lower_source(&source)
        .expect("final-use release must permit later source-handle reinitialization");
}
