use evo_lexer::Span;
use evo_lowering::{
    BinaryOp, Expr, ExprKind, Function, Program, RecordIr, RecordType, Stmt, StmtKind, ValueType,
};

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
        for record in &program.records {
            self.write_record(record);
            self.push_unmapped("\n");
        }

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

        for function in &program.functions {
            self.write_function(function);
            self.push_unmapped("\n");
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

    fn write_record(&mut self, record: &RecordIr) {
        self.push_mapped_line(
            format!("struct {} {{\n", generated_record_name(&record.name)),
            record.span,
        );
        for field in &record.fields {
            self.push_mapped_line(
                format!(
                    "    {}: {},\n",
                    generated_record_field_name(&field.name),
                    rust_record_type(&field.value_type)
                ),
                field.span,
            );
        }
        self.push_mapped_line("}\n".to_owned(), record.span);
    }

    fn write_function(&mut self, function: &Function) {
        let mut signature = format!("fn {}(", generated_function_name(&function.name));
        for (index, parameter) in function.parameters.iter().enumerate() {
            if index > 0 {
                signature.push_str(", ");
            }
            if parameter.mutable {
                signature.push_str("mut ");
            }
            signature.push_str(&generated_identifier(&parameter.name));
            signature.push_str(": ");
            signature.push_str(&rust_type(&parameter.value_type));
        }
        signature.push_str(") -> ");
        signature.push_str(&rust_type(&function.return_type));
        signature.push_str(" {\n");
        self.push_mapped_line(signature, function.span);
        for statement in &function.body {
            self.write_statement(statement, 1);
        }
        self.push_mapped_line("}\n".to_owned(), function.span);
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
            StmtKind::Return(expr) => {
                self.push_mapped_line(
                    format!("{padding}return {};\n", render_expr(expr)),
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

fn rust_type(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Integer => "i64".to_owned(),
        ValueType::Bool => "bool".to_owned(),
        ValueType::String => "&'static str".to_owned(),
        ValueType::Record(name) => generated_record_name(name),
    }
}

fn rust_record_type(value_type: &RecordType) -> String {
    match value_type {
        RecordType::Integer => "i64".to_owned(),
        RecordType::Bool => "bool".to_owned(),
        RecordType::String => "&'static str".to_owned(),
        RecordType::Record(name) => generated_record_name(name),
        RecordType::Enum(name) => generated_enum_name(name),
    }
}

fn render_expr(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Integer(value) => value.to_string(),
        ExprKind::String(value) => format!("{value:?}"),
        ExprKind::Bool(value) => value.to_string(),
        ExprKind::Local(name) => generated_identifier(name),
        ExprKind::Call { name, arguments } => {
            let arguments = arguments
                .iter()
                .map(render_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({arguments})", generated_function_name(name))
        }
        ExprKind::Construct { name, fields } => {
            if fields.is_empty() {
                format!("{} {{}}", generated_record_name(name))
            } else {
                let fields = fields
                    .iter()
                    .map(|field| {
                        format!(
                            "{}: {}",
                            generated_record_field_name(&field.name),
                            render_expr(&field.value)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} {{ {fields} }}", generated_record_name(name))
            }
        }
        ExprKind::FieldAccess { base, field } => format!(
            "({}).{}",
            render_expr(base),
            generated_record_field_name(field)
        ),
        ExprKind::InputInt => "__evo_input_int()".to_owned(),
        ExprKind::LogicalNot(inner) => format!("(!{})", render_expr(inner)),
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
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

fn generated_identifier(source_name: &str) -> String {
    format!("__evo_{source_name}")
}

fn generated_function_name(source_name: &str) -> String {
    format!("__evo_fn_{source_name}")
}

fn generated_record_name(source_name: &str) -> String {
    format!("__EvoRecord_{source_name}")
}

fn generated_enum_name(source_name: &str) -> String {
    format!("__EvoEnum_{source_name}")
}

fn generated_record_field_name(source_name: &str) -> String {
    format!("__evo_field_{source_name}")
}

fn program_uses_input_int(program: &Program) -> bool {
    program.statements.iter().any(statement_uses_input_int)
        || program
            .functions
            .iter()
            .flat_map(|function| &function.body)
            .any(statement_uses_input_int)
}

fn statement_uses_input_int(statement: &Stmt) -> bool {
    match &statement.kind {
        StmtKind::Let { expr, .. }
        | StmtKind::Assign { expr, .. }
        | StmtKind::Print(expr)
        | StmtKind::Return(expr) => expr_uses_input_int(expr),
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
        ExprKind::Call { arguments, .. } => arguments.iter().any(expr_uses_input_int),
        ExprKind::Construct { fields, .. } => {
            fields.iter().any(|field| expr_uses_input_int(&field.value))
        }
        ExprKind::FieldAccess { base, .. } => expr_uses_input_int(base),
        ExprKind::LogicalNot(inner) | ExprKind::UnaryMinus(inner) => expr_uses_input_int(inner),
        ExprKind::Binary { left, right, .. } => {
            expr_uses_input_int(left) || expr_uses_input_int(right)
        }
        ExprKind::Integer(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::Local(_) => {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GeneratedRust, generate_lowered_rust, generate_lowered_rust_with_map};
    use evo_lexer::lex;
    use evo_lowering::lower;
    use evo_parser::parse;

    fn lower_source(source: &str) -> evo_lowering::Program {
        let tokens = lex(source).expect("lexing should succeed");
        let syntax = parse(&tokens).expect("parsing should succeed");
        lower(&syntax).expect("lowering should succeed")
    }

    fn compile_source(source: &str) -> String {
        generate_lowered_rust(&lower_source(source))
    }

    fn mapped_source_line(generated: &GeneratedRust, line: usize) -> Option<usize> {
        generated.source_span_for_line(line).map(|span| span.line)
    }

    #[test]
    fn existing_top_level_generation_is_unchanged() {
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
    fn generates_direct_static_function_and_call() {
        assert_eq!(
            compile_source("fn add(a int, b int) int\nreturn a + b\nend\nprint add(2, 3)\n",),
            concat!(
                "fn __evo_fn_add(__evo_a: i64, __evo_b: i64) -> i64 {\n",
                "    return (__evo_a + __evo_b);\n",
                "}\n",
                "\n",
                "fn main() {\n",
                "    println!(\"{}\", __evo_fn_add(2, 3));\n",
                "}\n"
            )
        );
    }

    #[test]
    fn mutable_parameter_is_explicit_only_when_reassigned() {
        let generated = compile_source("fn bump(x int) int\nx = x + 1\nreturn x\nend\n");
        assert!(generated.contains("fn __evo_fn_bump(mut __evo_x: i64) -> i64"));
    }

    #[test]
    fn bool_and_string_signatures_use_static_rust_types() {
        let generated = compile_source(
            "fn yes(flag bool) bool\nreturn flag and true\nend\nfn echo(s string) string\nreturn s\nend\n",
        );
        assert!(generated.contains("__evo_flag: bool) -> bool"));
        assert!(generated.contains("__evo_s: &'static str) -> &'static str"));
    }

    #[test]
    fn record_codegen_uses_static_structs_and_nominal_types() {
        let generated = compile_source(
            "record Point\nx int\nend\nrecord Wrapper\npoint Point\nend\nfn wrap(point Point) Wrapper\nreturn Wrapper(point = point)\nend\nfn get_x(wrapper Wrapper) int\nreturn wrapper.point.x\nend\n",
        );
        assert!(generated.contains("struct __EvoRecord_Point {"));
        assert!(generated.contains("__evo_field_x: i64,"));
        assert!(generated.contains("struct __EvoRecord_Wrapper {"));
        assert!(generated.contains("__evo_field_point: __EvoRecord_Point,"));
        assert!(generated.contains("__evo_point: __EvoRecord_Point) -> __EvoRecord_Wrapper"));
        assert!(
            generated.contains("return __EvoRecord_Wrapper { __evo_field_point: __evo_point };")
        );
        assert!(generated.contains("return ((__evo_wrapper).__evo_field_point).__evo_field_x;"));
    }

    #[test]
    fn zero_field_record_codegen_uses_empty_struct_literal() {
        let generated =
            compile_source("record Marker\nend\nfn make() Marker\nreturn Marker()\nend\n");
        assert!(generated.contains("struct __EvoRecord_Marker {\n}\n"));
        assert!(generated.contains("return __EvoRecord_Marker {};"));
    }

    #[test]
    fn record_codegen_adds_no_hidden_runtime_or_clone_scaffolding() {
        let generated =
            compile_source("record Point\nx int\nend\nfn make() Point\nreturn Point(x = 1)\nend\n");
        assert!(!generated.contains(".clone()"));
        assert!(!generated.contains("Box<"));
        assert!(!generated.contains("dyn "));
        assert!(!generated.contains("HashMap"));
        assert!(!generated.contains("derive(Clone"));
        assert!(!generated.contains("derive(Copy"));
    }

    #[test]
    fn record_source_map_covers_struct_and_fields() {
        let program = lower_source("record Point\nx int\nend\n");
        let generated = generate_lowered_rust_with_map(&program);
        assert_eq!(mapped_source_line(&generated, 1), Some(1));
        assert_eq!(mapped_source_line(&generated, 2), Some(2));
        assert_eq!(mapped_source_line(&generated, 3), Some(1));
        assert_eq!(generated.source_span_for_line(4), None);
    }

    #[test]
    fn function_source_map_covers_signature_body_and_close() {
        let program = lower_source("fn id(x int) int\nreturn x\nend\nprint id(1)\n");
        let generated = generate_lowered_rust_with_map(&program);
        assert_eq!(mapped_source_line(&generated, 1), Some(1));
        assert_eq!(mapped_source_line(&generated, 2), Some(2));
        assert_eq!(mapped_source_line(&generated, 3), Some(1));
        assert_eq!(generated.source_span_for_line(4), None);
        assert_eq!(generated.source_span_for_line(5), None);
        assert_eq!(mapped_source_line(&generated, 6), Some(4));
    }

    #[test]
    fn input_helper_is_emitted_when_only_function_body_uses_it() {
        let generated = compile_source("fn read() int\nreturn input_int\nend\nprint read()\n");
        assert_eq!(generated.matches("fn __evo_input_int()").count(), 1);
        assert!(generated.contains("return __evo_input_int();"));
    }

    #[test]
    fn functions_use_no_runtime_dispatch_scaffolding() {
        let generated =
            compile_source("fn add(a int, b int) int\nreturn a + b\nend\nprint add(1, 2)\n");
        assert!(!generated.contains("Box<"));
        assert!(!generated.contains("dyn "));
        assert!(!generated.contains("HashMap"));
        assert!(!generated.contains("function_registry"));
    }
}
