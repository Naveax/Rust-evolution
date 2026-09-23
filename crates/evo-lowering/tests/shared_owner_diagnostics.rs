use evo_diagnostics::render_error;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;
use std::path::Path;

fn lower_error(source: &str) -> evo_lowering::LowerError {
    let tokens = lex(source).expect("shared-owner diagnostic source should lex");
    let syntax = parse(&tokens).expect("shared-owner diagnostic source should parse");
    lower(&syntax).expect_err("diagnostic source should be rejected before Rust codegen")
}

const ITEM: &str = "record Item\nvalue int\nend\n";

#[test]
fn moved_shared_handle_reuse_is_source_native_and_points_at_reuse() {
    let source = format!("{ITEM}owner = share Item(value = 1)\nmoved = owner\nprint owner.value\n");
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
    let source =
        format!("{ITEM}owner = share Item(value = 1)\nr = &owner\nmoved = owner\nprint r.value\n");
    let error = lower_error(&source);
    assert!(error.message.contains("cannot move shared handle local"));
    assert!(error.message.contains("immutable reference"));
    assert_eq!(error.span.line, 6);
}

#[test]
fn live_payload_reference_conflict_renders_reference_origin_note() {
    let source =
        format!("{ITEM}owner = share Item(value = 1)\nr = &owner\nmoved = owner\nprint r.value\n");
    let error = lower_error(&source);
    let rendered = render_error(
        Path::new("shared-owner.evo"),
        &source,
        &error.message,
        error.span,
    );
    assert!(rendered.contains("note: immutable reference \"r\" was created here"));
    assert!(rendered.contains(" --> shared-owner.evo:5:5"));
}

#[test]
fn downgrade_requires_a_shared_owner() {
    let source = format!("{ITEM}item = Item(value = 1)\nedge = downgrade item\n");
    let error = lower_error(&source);
    assert!(error.message.contains("downgrade requires a shared owner"));
    assert!(error.message.contains("Item"));
}

#[test]
fn moved_weak_handle_reuse_is_source_native() {
    let source = format!(
        "{ITEM}owner = share Item(value = 1)\nedge = downgrade owner\nmoved = edge\nupgrade edge as live\nprint live.value\nelse\nprint 0\nend\n"
    );
    let error = lower_error(&source);
    assert!(error.message.contains("moved weak handle"));
}

#[test]
fn upgrade_requires_a_weak_owner() {
    let source = format!(
        "{ITEM}owner = share Item(value = 1)\nupgrade owner as live\nprint live.value\nelse\nprint 0\nend\n"
    );
    let error = lower_error(&source);
    assert!(error.message.contains("upgrade requires a weak owner"));
    assert!(error.message.contains("shared Item"));
}

#[test]
fn upgrade_success_binding_must_be_fresh_and_not_reassigned() {
    let collision = format!(
        "{ITEM}owner = share Item(value = 1)\nedge = downgrade owner\nlive = 1\nupgrade edge as live\nprint 1\nelse\nprint 0\nend\n"
    );
    let error = lower_error(&collision);
    assert!(
        error
            .message
            .contains("conflicts with an already-visible local")
    );

    let reassignment = format!(
        "{ITEM}owner = share Item(value = 1)\nedge = downgrade owner\nupgrade edge as live\nlive = dup live\nprint live.value\nelse\nprint 0\nend\n"
    );
    let error = lower_error(&reassignment);
    assert!(
        error
            .message
            .contains("reassigning weak upgrade success binding")
    );
}

#[test]
fn weak_payload_access_does_not_implicitly_upgrade() {
    let source =
        format!("{ITEM}owner = share Item(value = 1)\nedge = downgrade owner\nprint edge.value\n");
    let error = lower_error(&source);
    assert!(
        error
            .message
            .contains("field access requires a record value")
    );
}

#[test]
fn weak_success_binding_is_not_visible_after_checked_upgrade() {
    let source = format!(
        "{ITEM}owner = share Item(value = 1)\nedge = downgrade owner\nupgrade edge as live\nprint live.value\nelse\nprint 0\nend\nprint live.value\n"
    );
    let error = lower_error(&source);
    assert!(error.message.contains("outside its scope"));
}
