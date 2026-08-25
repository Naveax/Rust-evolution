use evo_lowering::{BinaryOp, Expr, ExprKind, Program, Stmt, StmtKind, lower};
use evo_parser::Program as SyntaxProgram;
use std::fmt::Write as _;

#[must_use]
pub fn generate_rust(program: &SyntaxProgram) -> String {
    let lowered = lower(program).expect("program must pass semantic lowering before code generation");
    generate_lowered_rust(&lowered)
}

#[must_use]
pub fn generate_lowered_rust(program: &Program) -> String {
    let mut output = String::new();
    if program_uses_input_int(program) {
        output.push_str(
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
            ),
        );
    }

    output.push_str("fn main() {\n");
    for statement in &program.statements {
        write_statement(&mut output, statement, 1);
    }
    output.push_str("}\n");
    output
}

fn write_statement(output: &mut String, statement: &Stmt, indent: usize) {
    let padding = "    ".repeat(indent);
    match &statement.kind {
        StmtKind::Let {
            name,
            mutable,
            expr,
        } => {
            let mutable = if *mutable { "mut " } else { "" };
            let _ = writeln!(
                output,
                "{padding}let {mutable}{} = {};",
                generated_identifier(name),
                render_expr(expr)
            );
        }
        StmtKind::Assign { name, expr } => {
            let _ = writeln!(
                output,
                "{padding}{} = {};",
                generated_identifier(name),
                render_expr(expr)
            );
        }
        StmtKind::Print(expr) => {
            let _ = writeln!(output, "{padding}println!(\"{{}}\", {});", render_expr(expr));
        }
        StmtKind::Repeat { count, body } => {
            let _ = writeln!(output, "{padding}for _ in 0..{} {{", render_expr(count));
            for statement in body {
                write_statement(output, statement, indent + 1);
            }
            let _ = writeln!(output, "{padding}}}");
        }
    }
}

fn render_expr(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Integer(value) => value.to_string(),
        ExprKind::String(value) => format!("{value:?}"),
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
    }
}

fn expr_uses_input_int(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::InputInt => true,
        ExprKind::UnaryMinus(inner) => expr_uses_input_int(inner),
        ExprKind::Binary { left, right, .. } => {
            expr_uses_input_int(left) || expr_uses_input_int(right)
        }
        ExprKind::Integer(_) | ExprKind::String(_) | ExprKind::Local(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{generate_lowered_rust, generate_rust};
    use evo_lexer::lex;
    use evo_lowering::lower;
    use evo_parser::parse;

    fn parse_source(source: &str) -> evo_parser::Program {
        let tokens = lex(source).expect("lexing should succeed");
        parse(&tokens).expect("parsing should succeed")
    }

    fn compile_source(source: &str) -> String {
        let syntax = parse_source(source);
        let lowered = lower(&syntax).expect("lowering should succeed");
        generate_lowered_rust(&lowered)
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
    fn lowers_runtime_input_repeat_and_inferred_mutability() {
        assert_eq!(
            compile_source(
                "n = input_int\nsum = 0\nrepeat n\nsum = sum + 1\nend\nprint sum\n",
            ),
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

    #[test]
    fn compatibility_generator_uses_semantic_lowering() {
        let syntax = parse_source("x = 1\nx = x + 1\n");
        let generated = generate_rust(&syntax);
        assert!(generated.contains("let mut __evo_x = 1;"));
        assert!(generated.contains("__evo_x = (__evo_x + 1);"));
    }
}
