use evo_lexer::lex;
use evo_lowering::{ExprKind, ParameterPassingMode, StmtKind, ValueType, lower};
use evo_parser::parse;

fn lower_source(source: &str) -> Result<evo_lowering::Program, evo_lowering::LowerError> {
    let tokens = lex(source).expect("shared-owner source should lex");
    let syntax = parse(&tokens).expect("shared-owner source should parse");
    lower(&syntax)
}

const ITEM: &str = "record Item\nvalue int\nend\n";

#[test]
fn lowers_explicit_shared_allocation_and_duplication_as_distinct_operations() {
    let source = format!(
        "{ITEM}item = Item(value = 1)\nowner = share item\nalias = dup owner\nprint owner.value\nprint alias.value\n"
    );
    let program = lower_source(&source).expect("explicit shared-owner operations should lower");
    let StmtKind::Let { expr, .. } = &program.statements[1].kind else {
        panic!("expected owner binding");
    };
    assert!(matches!(expr.kind, ExprKind::SharedAlloc(_)));
    let StmtKind::Let { expr, .. } = &program.statements[2].kind else {
        panic!("expected alias binding");
    };
    assert!(matches!(expr.kind, ExprKind::SharedDuplicate(_)));
}

#[test]
fn shared_owner_function_parameters_and_returns_are_owned_moves() {
    let source = format!(
        "{ITEM}fn forward(item shared Item) shared Item\nreturn item\nend\nowner = share Item(value = 1)\nkept = forward(dup owner)\nprint owner.value\nprint kept.value\n"
    );
    let program = lower_source(&source).expect("duplicate then forward should retain original owner");
    let function = &program.functions[0];
    assert_eq!(
        function.parameters[0].value_type,
        ValueType::SharedOwner("Item".to_owned())
    );
    assert_eq!(
        function.return_type,
        ValueType::SharedOwner("Item".to_owned())
    );
    assert_eq!(function.parameters[0].passing_mode, ParameterPassingMode::Owned);
}

#[test]
fn ordinary_assignment_moves_shared_handle_and_reuse_is_rejected() {
    let source = format!(
        "{ITEM}owner = share Item(value = 1)\nmoved = owner\nprint owner.value\n"
    );
    let error = lower_source(&source).expect_err("ordinary assignment must move a shared handle");
    assert!(error.message.contains("moved shared handle"));
}

#[test]
fn by_value_call_moves_shared_handle_without_hidden_duplicate() {
    let source = format!(
        "{ITEM}fn consume(item shared Item) int\nreturn item.value\nend\nowner = share Item(value = 1)\nvalue = consume(owner)\nprint owner.value\n"
    );
    let error = lower_source(&source).expect_err("by-value call must consume shared handle");
    assert!(error.message.contains("moved shared handle"));
}

#[test]
fn duplicated_handle_moves_independently_from_original() {
    let source = format!(
        "{ITEM}owner = share Item(value = 1)\nalias = dup owner\nmoved = alias\nprint owner.value\nprint moved.value\n"
    );
    lower_source(&source).expect("moving one duplicated handle must not invalidate another");
}

#[test]
fn scalar_payload_reads_are_allowed_but_nominal_payload_moves_are_rejected() {
    let scalar = format!("{ITEM}owner = share Item(value = 1)\nprint owner.value\n");
    lower_source(&scalar).expect("scalar payload field should be inspectable");

    let nominal = concat!(
        "record Inner\nvalue int\nend\n",
        "record Outer\ninner Inner\nend\n",
        "owner = share Outer(inner = Inner(value = 1))\n",
        "moved = owner.inner\n",
    );
    let error = lower_source(nominal).expect_err("nominal payload must not move through shared owner");
    assert!(error.message.contains("record-valued field"));
    assert!(error.message.contains("no implicit clone"));
}

#[test]
fn dup_requires_shared_owner_and_share_requires_owned_record() {
    let non_shared = format!("{ITEM}item = Item(value = 1)\nalias = dup item\n");
    let error = lower_source(&non_shared).expect_err("dup on owned record must fail");
    assert!(error.message.contains("dup"));
    assert!(error.message.contains("shared"));

    let scalar = "value = 1\nowner = share value\n";
    let error = lower_source(scalar).expect_err("share on scalar must fail");
    assert!(error.message.contains("share"));
    assert!(error.message.contains("record"));
}

#[test]
fn live_payload_reference_blocks_only_its_source_shared_handle() {
    let blocked = format!(
        "{ITEM}owner = share Item(value = 1)\nr = &owner\nmoved = owner\nprint r.value\n"
    );
    let error = lower_source(&blocked).expect_err("source handle move with live payload reference must fail");
    assert!(error.message.contains("cannot move shared handle local"));
    assert!(error.message.contains("immutable reference"));

    let independent = format!(
        "{ITEM}owner = share Item(value = 1)\nalias = dup owner\nr = &owner\nmoved = alias\nprint r.value\nprint moved.value\n"
    );
    lower_source(&independent).expect("a different duplicated handle may move while original is borrowed");
}

#[test]
fn source_shared_handle_can_move_after_final_reference_use() {
    let source = format!(
        "{ITEM}owner = share Item(value = 1)\nr = &owner\nprint r.value\nmoved = owner\nprint moved.value\n"
    );
    lower_source(&source).expect("bounded final-use should release the source shared handle");
}

#[test]
fn owned_reference_and_shared_owner_categories_do_not_convert_implicitly() {
    let source = format!(
        "{ITEM}fn owned(item Item) int\nreturn item.value\nend\nowner = share Item(value = 1)\nprint owned(owner)\n"
    );
    let error = lower_source(&source).expect_err("shared owner must not convert to owned payload");
    assert!(error.message.contains("expects Item"));
    assert!(error.message.contains("shared Item"));
}
