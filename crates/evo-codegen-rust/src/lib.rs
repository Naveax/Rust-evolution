use evo_parser::{BinaryOp, Expr, ExprKind, Program, StmtKind};
use std::fmt::Write as _;

#[must_use]
pub fn generate_rust(program: &Program) -> String {
    let mut output = String::from("fn main() {\n");
    for statement in &program.statements {
        match &statement.kind {
            StmtKind::Bind { name, expr } => {
                let _ = writeln!(
                    output,
                    "    let {} = {};",
                    generated_identifier(name),
                    render_expr(expr)
                );
            }
            StmtKind::Print(expr) => {
                let _ = writeln!(output, "    println!(\"{{}}\", {});", render_expr(expr));
            }
        }
    }
    output.push_str("}\n");
    output
}

fn render_expr(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Integer(value) => value.to_string(),
        ExprKind::String(value) => format!("{value:?}"),
        ExprKind::Identifier(name) => generated_identifier(name),
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

#[cfg(test)]
mod tests {
    use super::generate_rust;
    use evo_lexer::lex;
    use evo_parser::parse;

    fn compile_source(source: &str) -> String {
        let tokens = lex(source).expect("lexing should succeed");
        let program = parse(&tokens).expect("parsing should succeed");
        generate_rust(&program)
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
        assert!(generated.contains("\"hello\\\\nworld\""));
    }
}
