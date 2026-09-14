use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;

fn lower_error(source: &str) -> evo_lowering::LowerError {
    let tokens = lex(source).expect("shared-owner diagnostic source should lex");
    let syntax = parse(&tokens).expect("shared-owner diagnostic source should parse");
    lower(&syntax).expect_err("diagnostic source should be rejected before Rust codegen")
}

const ITEM: &str = "record Item\nvalue int\nend\n";

#[test]
fn moved_shared_handle_reuse_is_source_native_and_points_at_reuse() {
    let source = format!(
        "{ITEM}owner = share Item(value = 1)\nmoved = owner\nprint owner.value\n"
    );
    let error = lower_error(&source);
    assert!(error.message.contains("moved shared handle"));
    assert_eq!(error.span.line, 6);
}

#[test]
fn dup_on_owned_record_has_distinct_shared_owner_diagnostic() {
    let source = format!("{ITEM}item = Item(value = 1)\nalias = dup item\n");
    let error = lower_error(&source);
    assert!(error.message.contains("dup"));
    assert!(error.message.contains("shared"));
    assert!(error.message.contains("Item"));
    assert_eq!(error.span.line, 5);
}

#[test]
fn share_on_scalar_has_distinct_owned_record_diagnostic() {
    let source = "value = 1\nowner = share value\n";
    let error = lower_error(source);
    assert!(error.message.contains("share requires an owned record"));
    assert!(error.message.contains("int"));
    assert_eq!(error.span.line, 2);
}

#[test]
fn live_payload_reference_conflict_names_shared_handle_and_reference() {
    let source = format!(
        "{ITEM}owner = share Item(value = 1)\nr = &owner\nmoved = owner\nprint r.value\n"
    );
    let error = lower_error(&source);
    assert!(error.message.contains("cannot move shared handle local"));
    assert!(error.message.contains("immutable reference"));
    assert_eq!(error.span.line, 6);
}
