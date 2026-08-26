use evo_lexer::Span;
use evo_lowering::{BinaryOp, Expr, ExprKind, Program, Stmt, StmtKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceMapping {
    pub generated_start_line: usize,
    pub generated_end_line: usize,
    pub source_span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedRust {
    pub source: String,
    pub mappings: Vec<SourceMapping>,
}

impl GeneratedRust {
    #[must_use]
    pub fn source_span_for_line(&self, generated_line: usize) -> Option<Span> {
        self.mappings
            .iter()
            .find(|mapping| {
                generated_line >= mapping.generated_start_line
                    && generated_line <= mapping.generated_end_line
            })
            .map(|mapping| mapping.source_span)
    }
}

#[must_use]
pub fn generate_lowered_rust(program: &Program) -> String {
    generate_lowered_rust_with_map(program).source
}

#[must_use]
pub fn generate_lowered_rust_with_map(program: &Program) -> GeneratedRust {
    Generator::new().generate(program)
}

struct Generator {
    source: String,
    mappings: Vec<SourceMapping>,
    next_line: usize,
}

impl Generator {
    fn new() -> Self {
        Self {
            source: String::new(),
            mappings: Vec::new(),
            next_line: 1,
        }
    }

    fn generate(mut self, program: &Program) -> GeneratedRust {
        if program_uses_input_int(program) {
            self.push_unmapped(concat!(
                "fn __evo_input_int() -> i64 {\n",
                "    let mut __evo_input = String::new();\n",
                "    std::io::stdin()\n",
                "        .read_line(&mut __evo_input)\n",
                "        .expect(\"failed to read integer input\");\n",
                "    __evo_input\n",
                "        .trim()\n",
                "        .parse::<i64>()\n",
                "        .expect(\"expected signed integer input\")\n",
                "}\n\n",
            ));
        }

        self.push_unmapped("fn main() {\n");
        for statement in &program.statements {
            self.write_statement(statement, 1);
        }
        self.push_unmapped("}\n");

        GeneratedRust {
            source: self.source,
            mappings: self.mappings,
        }
    }

    fn write_statement(&mut self, statement: &Stmt, indent: usize) {
        let padding = "    ".repeat(indent);
        match &statement.kind {
            StmtKind::Let {
                name,
                mutable,
                expr,
            } => {
                let mutable = if *mutable { "mut " } else { "" };
                self.push_mapped_line(
                    format!(
                        "{padding}let {mutable}{} = {};\n",
                        generated_identifier(name),
                        render_expr(expr)
                    ),
                    statement.span,
                );
            }
            StmtKind::Assign { name, expr } => {
                self.push_mapped_line(
                    format!(
                        "{padding}{} = {};\n",
                        generated_identifier(name),
                        render_expr(expr)
                    ),
                    statement.span,
                );
            }
            StmtKind::Print(expr) => {
                self.push_mapped_line(
                    format!("{padding}println!(\"{{}}\", {});\n", render_expr(expr)),
                    statement.span,
                );
            }
            StmtKind::Repeat { count, body } => {
                self.push_mapped_line(
                    format!("{padding}for _ in 0..{} {{\n", render_expr(count)),
                    statement.span,
                );
                for statement in body {
                    self.write_statement(statement, indent + 1);
                }
                self.push_mapped_line(format!("{padding}}}\n"), statement.span);
            }
            StmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                self.push_mapped_line(
                    format!("{padding}if {} {{\n", render_expr(condition)),
                    statement.span,
                );
                for statement in then_body {
                    self.write_statement(statement, indent + 1);
                }
                if else_body.is_empty() {
                    self.push_mapped_line(format!("{padding}}}\n"), statement.span);
                } else {
                    self.push_mapped_line(format!("{padding}}} else {{\n"), statement.span);
                    for statement in else_body {
                        self.write_statement(statement, indent + 1);
                    }
                    self.push_mapped_line(format!("{padding}}}\n"), statement.span);
                }
            }
        }
    }

    fn push_unmapped(&mut self, text: &str) {
        self.source.push_str(text);
        self.next_line += text.bytes().filter(|byte| *byte == b'\n').count();
    }

    fn push_mapped_line(&mut self, line: String, source_span: Span) {
        debug_assert!(line.ends_with('\n'));
        debug_assert_eq!(line.bytes().filter(|byte| *byte == b'\n').count(), 1);
        let generated_line = self.next_line;
        self.source.push_str(&line);
        self.mappings.push(SourceMapping {
            generated_start_line: generated_line,
            generated_end_line: generated_line,
            source_span,
        });
        self.next_line += 1;
    }
}

fn render_expr(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Integer(value) => value.to_string(),
        ExprKind::String(value) => format!("{value:?}"),
        ExprKind::Bool(value) => value.to_string(),
        ExprKind::Local(name) => generated_identifier(name),
        ExprKind::InputInt => "__evo_input_int()".to_owned(),
        ExprKind::UnaryMinus(inner) => format!("(-{})", render_expr(inner)),
        ExprKind::Binary { left, op, right } => format!(
            "({} {} {})",
            render_expr(left),
            render_binary_op(*op),
            render_expr(right)
        ),
    }
}

const fn render_binary_op(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
    }
}

fn generated_identifier(source_name: &str) -> String {
    format!("__evo_{source_name}")
}

fn program_uses_input_int(program: &Program) -> bool {
    program.statements.iter().any(statement_uses_input_int)
}

fn statement_uses_input_int(statement: &Stmt) -> bool {
    match &statement.kind {
        StmtKind::Let { expr, .. } | StmtKind::Assign { expr, .. } | StmtKind::Print(expr) => {
            expr_uses_input_int(expr)
        }
        StmtKind::Repeat { count, body } => {
            expr_uses_input_int(count) || body.iter().any(statement_uses_input_int)
        }
        StmtKind::If {
            condition,
            then_body,
            else_body,
        } => {
            expr_uses_input_int(condition)
                || then_body.iter().any(statement_uses_input_int)
                || else_body.iter().any(statement_uses_input_int)
        }
    }
}

fn expr_uses_input_int(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::InputInt => true,
        ExprKind::UnaryMinus(inner) => expr_uses_input_int(inner),
        ExprKind::Binary { left, right, .. } => {
            expr_uses_input_int(left) || expr_uses_input_int(right)
        }
        ExprKind::Integer(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::Local(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{GeneratedRust, generate_lowered_rust, generate_lowered_rust_with_map};
    use evo_lexer::lex;
    use evo_lowering::lower;
    use evo_parser::parse;

    fn parse_source(source: &str) -> evo_parser::Program {
        let tokens = lex(source).expect("lexing should succeed");
        parse(&tokens).expect("parsing should succeed")
    }

    fn lower_source(source: &str) -> evo_lowering::Program {
        let syntax = parse_source(source);
        lower(&syntax).expect("lowering should succeed")
    }

    fn compile_source(source: &str) -> String {
        generate_lowered_rust(&lower_source(source))
    }

    fn mapped_source_line(generated: &GeneratedRust, line: usize) -> Option<usize> {
        generated.source_span_for_line(line).map(|span| span.line)
    }

    #[test]
    fn generates_deterministic_rust() {
        assert_eq!(
            compile_source("x = 1\ny = 1\nprint x + y\n"),
            concat!(
                "fn main() {\n",
                "    let __evo_x = 1;\n",
                "    let __evo_y = 1;\n",
                "    println!(\"{}\", (__evo_x + __evo_y));\n",
                "}\n"
            )
        );
    }

    #[test]
    fn generates_plain_rust_if_else_and_comparisons() {
        assert_eq!(
            compile_source("x = 1\nif x >= 1\nprint true\nelse\nprint false\nend\n"),
            concat!(
                "fn main() {\n",
                "    let __evo_x = 1;\n",
                "    if (__evo_x >= 1) {\n",
                "        println!(\"{}\", true);\n",
                "    } else {\n",
                "        println!(\"{}\", false);\n",
                "    }\n",
                "}\n"
            )
        );
    }

    #[test]
    fn if_without_else_does_not_emit_synthetic_else_block() {
        let generated = compile_source("flag = true\nif flag\nprint 1\nend\n");
        assert!(generated.contains("if __evo_flag {"));
        assert!(!generated.contains("else"));
    }

    #[test]
    fn conditional_source_map_preserves_nested_statement_lines() {
        let program = lower_source("x = 1\nif x > 0\nprint true\nelse\nprint false\nend\n");
        let generated = generate_lowered_rust_with_map(&program);
        assert_eq!(mapped_source_line(&generated, 2), Some(1));
        assert_eq!(mapped_source_line(&generated, 3), Some(2));
        assert_eq!(mapped_source_line(&generated, 4), Some(3));
        assert_eq!(mapped_source_line(&generated, 5), Some(2));
        assert_eq!(mapped_source_line(&generated, 6), Some(5));
        assert_eq!(mapped_source_line(&generated, 7), Some(2));
        assert_eq!(generated.source_span_for_line(8), None);
    }

    #[test]
    fn mapped_api_preserves_exact_generated_source() {
        let program = lower_source("x = 1\ny = 1\nprint x + y\n");
        let plain = generate_lowered_rust(&program);
        let mapped = generate_lowered_rust_with_map(&program);
        assert_eq!(mapped.source, plain);
    }

    #[test]
    fn simple_statement_lines_map_back_to_source_spans() {
        let program = lower_source("x = 1\nprint x\n");
        let generated = generate_lowered_rust_with_map(&program);

        assert_eq!(generated.source_span_for_line(1), None);
        assert_eq!(mapped_source_line(&generated, 2), Some(1));
        assert_eq!(mapped_source_line(&generated, 3), Some(2));
        assert_eq!(generated.source_span_for_line(4), None);
        assert_eq!(generated.source_span_for_line(999), None);
    }

    #[test]
    fn reassignment_line_maps_to_reassignment_span() {
        let program = lower_source("x = 1\nx = x + 1\n");
        let generated = generate_lowered_rust_with_map(&program);

        assert_eq!(mapped_source_line(&generated, 2), Some(1));
        assert_eq!(mapped_source_line(&generated, 3), Some(2));
    }

    #[test]
    fn nested_repeat_keeps_inner_statement_mappings() {
        let program = lower_source("x = 0\nrepeat 2\nrepeat 3\nx = x + 1\nend\nend\n");
        let generated = generate_lowered_rust_with_map(&program);

        assert_eq!(mapped_source_line(&generated, 2), Some(1));
        assert_eq!(mapped_source_line(&generated, 3), Some(2));
        assert_eq!(mapped_source_line(&generated, 4), Some(3));
        assert_eq!(mapped_source_line(&generated, 5), Some(4));
        assert_eq!(mapped_source_line(&generated, 6), Some(3));
        assert_eq!(mapped_source_line(&generated, 7), Some(2));
        assert_eq!(generated.source_span_for_line(8), None);
    }

    #[test]
    fn runtime_input_helper_and_main_wrapper_are_unmapped() {
        let program = lower_source("value = input_int\nprint value\n");
        let generated = generate_lowered_rust_with_map(&program);
        let first_mapped_line = generated
            .mappings
            .first()
            .expect("user statements should be mapped")
            .generated_start_line;

        for line in 1..first_mapped_line {
            assert_eq!(generated.source_span_for_line(line), None, "line {line}");
        }
        assert_eq!(mapped_source_line(&generated, first_mapped_line), Some(1));
        let final_line = generated.source.lines().count();
        assert_eq!(generated.source_span_for_line(final_line), None);
    }

    #[test]
    fn lowers_runtime_input_repeat_and_inferred_mutability() {
        let source = concat!(
            "n = input_int\n",
            "sum = 0\n",
            "repeat n\n",
            "sum = sum + 1\n",
            "end\n",
            "print sum\n"
        );
        assert_eq!(
            compile_source(source),
            concat!(
                "fn __evo_input_int() -> i64 {\n",
                "    let mut __evo_input = String::new();\n",
                "    std::io::stdin()\n",
                "        .read_line(&mut __evo_input)\n",
                "        .expect(\"failed to read integer input\");\n",
                "    __evo_input\n",
                "        .trim()\n",
                "        .parse::<i64>()\n",
                "        .expect(\"expected signed integer input\")\n",
                "}\n\n",
                "fn main() {\n",
                "    let __evo_n = __evo_input_int();\n",
                "    let mut __evo_sum = 0;\n",
                "    for _ in 0..__evo_n {\n",
                "        __evo_sum = (__evo_sum + 1);\n",
                "    }\n",
                "    println!(\"{}\", __evo_sum);\n",
                "}\n"
            )
        );
    }

    #[test]
    fn zero_and_one_repeat_counts_lower_directly_to_ranges() {
        let generated = compile_source("repeat 0\nend\nrepeat 1\nend\n");
        assert!(generated.contains("for _ in 0..0 {"));
        assert!(generated.contains("for _ in 0..1 {"));
    }

    #[test]
    fn input_int_has_explicit_parse_failure_contract() {
        let generated = compile_source("value = input_int\nprint value\n");
        assert!(generated.contains(".parse::<i64>()"));
        assert!(generated.contains(".expect(\"expected signed integer input\")"));
    }

    #[test]
    fn nested_repeat_uses_plain_rust_ranges_without_allocation() {
        let generated = compile_source("x = 0\nrepeat 2\nrepeat 3\nx = x + 1\nend\nend\n");
        assert_eq!(generated.matches("for _ in 0..").count(), 2);
        assert!(!generated.contains("Box"));
        assert!(!generated.contains("Vec"));
    }

    #[test]
    fn generated_names_do_not_collide_with_rust_keywords() {
        assert_eq!(
            compile_source("type = 7\nprint type\n"),
            concat!(
                "fn main() {\n",
                "    let __evo_type = 7;\n",
                "    println!(\"{}\", __evo_type);\n",
                "}\n"
            )
        );
    }

    #[test]
    fn escapes_strings_using_rust_debug_literal_rules() {
        let generated = compile_source("print \"hello\\nworld\"\n");
        assert!(generated.contains("\"hello\\nworld\""));
    }
}
