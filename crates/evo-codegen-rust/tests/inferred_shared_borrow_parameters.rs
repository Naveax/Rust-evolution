use evo_codegen_rust::generate_lowered_rust;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;

fn compile_source(source: &str) -> String {
    let tokens = lex(source).expect("source should lex");
    let syntax = parse(&tokens).expect("source should parse");
    let program = lower(&syntax).expect("source should lower");
    generate_lowered_rust(&program)
}

#[test]
fn read_only_nominal_parameter_and_repeated_calls_emit_shared_borrows() {
    let generated = compile_source(
        "record Item\nvalue int\nend\nfn read_value(item Item) int\nreturn item.value\nend\nitem = Item(value = 7)\nprint read_value(item)\nprint read_value(item)\n",
    );

    assert!(generated.contains("fn __evo_fn_read_value(__evo_item: &__EvoRecord_Item) -> i64"));
    assert_eq!(
        generated
            .matches("__evo_fn_read_value(&__evo_item)")
            .count(),
        2
    );
    assert!(!generated.contains(".clone()"));
    assert!(!generated.contains("Rc<"));
    assert!(!generated.contains("Arc<"));
    assert!(!generated.contains("Box<"));
}

#[test]
fn borrowed_temporary_is_rendered_only_at_the_call_boundary() {
    let generated = compile_source(
        "record Item\nvalue int\nend\nfn read_value(item Item) int\nreturn item.value\nend\nprint read_value(Item(value = 7))\n",
    );

    assert!(generated.contains("__evo_fn_read_value(&__EvoRecord_Item { __evo_field_value: 7 })"));
}

#[test]
fn owned_parameter_contract_still_emits_by_value() {
    let generated = compile_source(
        "record Item\nvalue int\nend\nfn identity(item Item) Item\nreturn item\nend\nitem = Item(value = 7)\nmoved = identity(item)\n",
    );

    assert!(
        generated
            .contains("fn __evo_fn_identity(__evo_item: __EvoRecord_Item) -> __EvoRecord_Item")
    );
    assert!(generated.contains("__evo_fn_identity(__evo_item)"));
    assert!(!generated.contains("__evo_fn_identity(&__evo_item)"));
}

#[test]
fn forwarding_signature_stays_owned_while_inner_call_borrows() {
    let generated = compile_source(
        "record Item\nvalue int\nend\nfn read_value(item Item) int\nreturn item.value\nend\nfn forward(item Item) int\nreturn read_value(item)\nend\n",
    );

    assert!(generated.contains("fn __evo_fn_forward(__evo_item: __EvoRecord_Item) -> i64"));
    assert!(generated.contains("return __evo_fn_read_value(&__evo_item);"));
}
