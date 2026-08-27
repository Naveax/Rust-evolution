use evo_codegen_rust::generate_lowered_rust_with_map;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;

fn generate(source: &str) -> evo_codegen_rust::GeneratedRust {
    let tokens = lex(source).expect("record source should lex");
    let syntax = parse(&tokens).expect("record source should parse");
    let program = lower(&syntax).expect("record source should lower");
    generate_lowered_rust_with_map(&program)
}

fn generated_line_containing(generated: &evo_codegen_rust::GeneratedRust, needle: &str) -> usize {
    generated
        .source
        .lines()
        .position(|line| line.contains(needle))
        .map(|index| index + 1)
        .expect("generated source should contain requested line")
}

#[test]
fn record_declaration_and_field_lines_map_to_record_source_spans() {
    let generated = generate("record Point\nx int\ny bool\nend\n");

    let struct_line = generated_line_containing(&generated, "struct __EvoRecord_Point");
    let x_line = generated_line_containing(&generated, "__evo_field_x: i64");
    let y_line = generated_line_containing(&generated, "__evo_field_y: bool");

    assert_eq!(
        generated
            .source_span_for_line(struct_line)
            .map(|span| span.line),
        Some(1)
    );
    assert_eq!(
        generated.source_span_for_line(x_line).map(|span| span.line),
        Some(2)
    );
    assert_eq!(
        generated.source_span_for_line(y_line).map(|span| span.line),
        Some(3)
    );
}

#[test]
fn record_constructor_and_access_keep_existing_statement_mapping_policy() {
    let generated =
        generate("record Point\nx int\nend\npoint = Point(x = 41)\nprint point.x + 1\n");

    let constructor_line = generated_line_containing(&generated, "let __evo_point");
    let access_line = generated_line_containing(&generated, "println!");

    assert_eq!(
        generated
            .source_span_for_line(constructor_line)
            .map(|span| span.line),
        Some(4)
    );
    assert_eq!(
        generated
            .source_span_for_line(access_line)
            .map(|span| span.line),
        Some(5)
    );
}
