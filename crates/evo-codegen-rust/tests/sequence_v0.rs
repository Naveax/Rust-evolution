use evo_codegen_rust::{generate_lowered_rust, generate_lowered_rust_with_map};
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;

fn generate(source: &str) -> String {
    let tokens = lex(source).expect("sequence codegen source should lex");
    let syntax = parse(&tokens).expect("sequence codegen source should parse");
    let lowered = lower(&syntax).expect("sequence codegen source should lower");
    generate_lowered_rust(&lowered)
}

#[test]
fn scalar_sequence_codegen_is_direct_vec_push_and_checked_get() {
    let generated = generate(
        "items = seq int()\nappend items, 7\nlookup items, -1 as value\nprint value\nelse\nprint 0\nend\n",
    );
    assert!(generated.contains("let mut __evo_items = Vec::<i64>::new();"));
    assert!(generated.contains("__evo_items.push(7);"));
    assert!(generated.contains("usize::try_from((-1)).ok().and_then"));
    assert!(generated.contains("if let Some(&__evo_value)"));
    assert!(generated.contains("__evo_items.get(__evo_lookup_index)"));
    assert!(!generated.contains("unsafe"));
    assert!(!generated.contains("RefCell"));
    assert!(!generated.contains("Mutex"));
}

#[test]
fn record_lookup_keeps_plain_reference_and_allows_nll_growth_after_last_use() {
    let generated = generate(
        "record Item\nvalue int\nend\nitems = seq Item()\nappend items, Item(value = 1)\nlookup items, 0 as item\nprint item.value\nappend items, Item(value = 2)\nelse\nprint 0\nend\n",
    );
    assert!(generated.contains("Vec::<__EvoRecord_Item>::new()"));
    assert!(generated.contains("if let Some(__evo_item)"));
    assert!(generated.contains("(__evo_item).__evo_field_value"));
    assert!(!generated.contains("clone()"));
    assert!(!generated.contains("Rc::clone"));
}

#[test]
fn shared_owner_sequence_uses_vec_of_rc_without_hidden_clone() {
    let generated = generate(
        "record Item\nvalue int\nend\nowner = share Item(value = 9)\nitems = seq shared Item()\nappend items, owner\nlookup items, 0 as item\nprint item.value\nelse\nprint 0\nend\n",
    );
    assert!(generated.contains("Vec::<std::rc::Rc<__EvoRecord_Item>>::new()"));
    assert!(generated.contains("__evo_items.push(__evo_owner);"));
    assert!(generated.contains("if let Some(__evo_item)"));
    assert!(!generated.contains("Rc::clone"));
}

#[test]
fn sequence_parameter_codegen_marks_only_grown_parameter_mutable() {
    let generated = generate(
        "record Item\nvalue int\nend\nfn add(items seq Item, item Item) seq Item\nappend items, item\nreturn items\nend\n",
    );
    assert!(generated.contains(
        "fn __evo_fn_add(mut __evo_items: Vec<__EvoRecord_Item>, __evo_item: __EvoRecord_Item) -> Vec<__EvoRecord_Item>"
    ));
}

#[test]
fn unused_lookup_binding_codegen_avoids_unused_variable_and_retained_borrow() {
    let generated = generate(
        "record Item\nvalue int\nend\nitems = seq Item()\nappend items, Item(value = 1)\nlookup items, 0 as item\nmoved = items\nprint 1\nelse\nprint 0\nend\n",
    );
    assert!(generated.contains("if let Some(_)"));
    assert!(generated.contains("let __evo_moved = __evo_items;"));
    assert!(!generated.contains("Some(__evo_item)"));
}

#[test]
fn sequence_payloads_use_plain_vec_scope_drop_without_leak_scaffolding() {
    let generated = generate(
        "record Item\nvalue int\nend\nitems = seq Item()\nappend items, Item(value = 1)\nprint 1\n",
    );
    assert!(generated.contains("let mut __evo_items = Vec::<__EvoRecord_Item>::new();"));
    assert!(generated.contains("__evo_items.push("));
    assert!(!generated.contains("ManuallyDrop"));
    assert!(!generated.contains("mem::forget"));
    assert!(!generated.contains("Box::leak"));
    assert!(!generated.contains("unsafe"));
}

#[test]
fn lookup_generated_lines_map_back_to_lookup_source_span() {
    let source = "items = seq int()\nappend items, 7\nlookup items, 0 as value\nprint value\nelse\nprint 0\nend\n";
    let tokens = lex(source).unwrap();
    let syntax = parse(&tokens).unwrap();
    let lowered = lower(&syntax).unwrap();
    let generated = generate_lowered_rust_with_map(&lowered);
    let line = generated
        .source
        .lines()
        .position(|line| line.contains("if let Some"))
        .unwrap()
        + 1;
    assert_eq!(
        generated.source_span_for_line(line).map(|span| span.line),
        Some(3)
    );
}
