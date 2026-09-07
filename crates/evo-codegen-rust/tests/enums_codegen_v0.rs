use evo_codegen_rust::{GeneratedRust, try_generate_lowered_rust_with_map};
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;

fn generate(source: &str) -> GeneratedRust {
    let tokens = lex(source).expect("enum codegen source should lex");
    let syntax = parse(&tokens).expect("enum codegen source should parse");
    let program = lower(&syntax).expect("enum codegen source should lower");
    try_generate_lowered_rust_with_map(&program).expect("enum codegen should succeed")
}

fn generated_line(source: &str, needle: &str) -> usize {
    source
        .lines()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("generated Rust should contain {needle:?}\n{source}"))
        + 1
}

fn normalize_checkout_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}

#[test]
fn enum_codegen_orders_static_definitions_and_preserves_line_mappings() {
    let generated = generate(
        "enum Flag\nOff\nOn\nend\nfn read(value Flag) int\nmatch value\ncase Flag.Off\nreturn 0\ncase Flag.On\nreturn 1\nend\nend\nvalue = Flag.On()\nprint read(value)\n",
    );

    let enum_line = generated_line(&generated.source, "enum __EvoEnum_Flag {");
    let on_variant_line = generated_line(&generated.source, "__EvoVariant_On,");
    let function_line = generated_line(&generated.source, "fn __evo_fn_read(");
    let constructor_line = generated_line(
        &generated.source,
        "let __evo_value = __EvoEnum_Flag::__EvoVariant_On;",
    );
    let match_line = generated_line(&generated.source, "match __evo_value");
    let on_arm_line = generated_line(&generated.source, "__EvoEnum_Flag::__EvoVariant_On => {");
    let main_line = generated_line(&generated.source, "fn main() {");

    assert!(enum_line < function_line);
    assert!(function_line < main_line);
    assert_eq!(
        generated
            .source_span_for_line(enum_line)
            .map(|span| span.line),
        Some(1)
    );
    assert_eq!(
        generated
            .source_span_for_line(on_variant_line)
            .map(|span| span.line),
        Some(3)
    );
    assert_eq!(
        generated
            .source_span_for_line(constructor_line)
            .map(|span| span.line),
        Some(13)
    );
    assert_eq!(
        generated
            .source_span_for_line(match_line)
            .map(|span| span.line),
        Some(6)
    );
    assert_eq!(
        generated
            .source_span_for_line(on_arm_line)
            .map(|span| span.line),
        Some(9)
    );
}

#[test]
fn enum_payload_types_lower_to_direct_builtin_record_and_enum_rust_types() {
    let generated = generate(
        "record Item\nvalue int\nend\nenum Inner\nA int\nend\nenum Wrapped\nNone\nWithInner Inner\nWithItem Item\nend\nfirst = Wrapped.WithInner(Inner.A(1))\nsecond = Wrapped.WithItem(Item(value = 2))\n",
    );

    assert!(generated.source.contains("__EvoVariant_A(i64),"));
    assert!(
        generated
            .source
            .contains("__EvoVariant_WithInner(__EvoEnum_Inner),")
    );
    assert!(
        generated
            .source
            .contains("__EvoVariant_WithItem(__EvoRecord_Item),")
    );
    assert!(generated.source.contains(
        "let __evo_first = __EvoEnum_Wrapped::__EvoVariant_WithInner(__EvoEnum_Inner::__EvoVariant_A(1));"
    ));
    assert!(generated.source.contains(
        "let __evo_second = __EvoEnum_Wrapped::__EvoVariant_WithItem(__EvoRecord_Item { __evo_field_value: 2 });"
    ));
    assert!(!generated.source.contains(".clone("));
    assert!(!generated.source.contains("Box<"));
    assert!(!generated.source.contains("Rc<"));
    assert!(!generated.source.contains("Arc<"));
    assert!(!generated.source.contains("HashMap"));
    assert!(!generated.source.contains("dyn "));
    assert!(!generated.source.contains("TypeId"));
}

#[test]
fn enum_benchmark_codegen_remains_byte_identical_to_accepted_reference() {
    let source = include_str!("../../../benchmarks/cases/enums-v0/evolution.evo");
    let reference = normalize_checkout_newlines(include_str!(
        "../../../benchmarks/cases/enums-v0/reference.rs"
    ));
    let generated = generate(source);

    assert_eq!(generated.source.as_bytes(), reference.as_bytes());
}
