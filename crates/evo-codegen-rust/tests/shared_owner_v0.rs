use evo_codegen_rust::generate_lowered_rust;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;

fn compile_source(source: &str) -> String {
    let tokens = lex(source).expect("shared-owner codegen source should lex");
    let syntax = parse(&tokens).expect("shared-owner codegen source should parse");
    let lowered = lower(&syntax).expect("shared-owner codegen source should lower");
    generate_lowered_rust(&lowered)
}

const ITEM: &str = "record Item\nvalue int\nend\n";

#[test]
fn shared_owner_signatures_lower_directly_to_std_rc_by_value() {
    let source = format!("{ITEM}fn forward(item shared Item) shared Item\nreturn item\nend\n");
    let generated = compile_source(&source);
    assert!(generated.contains(
        "fn __evo_fn_forward(__evo_item: std::rc::Rc<__EvoRecord_Item>) -> std::rc::Rc<__EvoRecord_Item> {"
    ));
    assert!(!generated.contains("&std::rc::Rc<__EvoRecord_Item>"));
}

#[test]
fn share_maps_to_one_rc_new_and_dup_maps_to_one_rc_clone() {
    let source = format!(
        "{ITEM}owner = share Item(value = 1)\nalias = dup owner\nprint owner.value\nprint alias.value\n"
    );
    let generated = compile_source(&source);
    assert_eq!(generated.matches("std::rc::Rc::new(").count(), 1);
    assert_eq!(generated.matches("std::rc::Rc::clone(").count(), 1);
    assert!(generated.contains(
        "let __evo_owner = std::rc::Rc::new(__EvoRecord_Item { __evo_field_value: 1 });"
    ));
    assert!(generated.contains("let __evo_alias = std::rc::Rc::clone(&(__evo_owner));"));
}

#[test]
fn ordinary_handle_move_does_not_insert_hidden_rc_clone() {
    let source = format!("{ITEM}owner = share Item(value = 1)\nmoved = owner\nprint moved.value\n");
    let generated = compile_source(&source);
    assert!(generated.contains("let __evo_moved = __evo_owner;"));
    assert_eq!(generated.matches("std::rc::Rc::clone(").count(), 0);
    assert!(!generated.contains(".clone()"));
}

#[test]
fn duplicated_owner_forwarding_has_only_the_explicit_clone() {
    let source = format!(
        "{ITEM}fn forward(item shared Item) shared Item\nreturn item\nend\nowner = share Item(value = 1)\nkept = forward(dup owner)\nprint owner.value\nprint kept.value\n"
    );
    let generated = compile_source(&source);
    assert_eq!(generated.matches("std::rc::Rc::clone(").count(), 1);
    assert!(
        generated
            .contains("let __evo_kept = __evo_fn_forward(std::rc::Rc::clone(&(__evo_owner)));")
    );
}

#[test]
fn payload_field_read_relies_on_rc_deref_without_runtime_scaffolding() {
    let source = format!("{ITEM}owner = share Item(value = 1)\nprint owner.value\n");
    let generated = compile_source(&source);
    assert!(generated.contains("(__evo_owner).__evo_field_value"));
    assert!(!generated.contains("RefCell"));
    assert!(!generated.contains("Arc<"));
    assert!(!generated.contains("Mutex"));
    assert!(!generated.contains("RwLock"));
    assert!(!generated.contains("HashMap"));
    assert!(!generated.contains("unsafe"));
}

#[test]
fn payload_reference_from_shared_owner_is_a_reference_to_payload_not_rc_wrapper() {
    let source = format!(
        "{ITEM}fn read(item &Item) int\nreturn item.value\nend\nowner = share Item(value = 1)\nr = &owner\nprint read(r)\n"
    );
    let generated = compile_source(&source);
    assert!(
        generated.contains("let __evo_r = std::rc::Rc::as_ref(&__evo_owner);")
            || generated.contains("let __evo_r = &*(__evo_owner);")
            || generated.contains("let __evo_r = &*__evo_owner;")
    );
    assert!(!generated.contains("let __evo_r = &(__evo_owner);"));
}

#[test]
fn shared_owner_codegen_adds_no_hidden_deep_clone_or_custom_runtime() {
    let source =
        format!("{ITEM}owner = share Item(value = 1)\nalias = dup owner\nprint alias.value\n");
    let generated = compile_source(&source);
    assert!(!generated.contains("derive(Clone"));
    assert!(!generated.contains("Box<"));
    assert!(!generated.contains("dyn "));
    assert!(!generated.contains("ownership_registry"));
    assert!(!generated.contains("reference_count"));
    assert!(!generated.contains("unsafe"));
}
