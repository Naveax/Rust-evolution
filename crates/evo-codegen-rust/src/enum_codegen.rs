use crate::{GeneratedRust, SourceMapping};
use evo_lexer::Span;
use evo_lowering::enum_codegen_view::{
    EnumCodegenExprKindView, EnumCodegenExprView, EnumCodegenFunctionView, EnumCodegenMatchArmView,
    EnumCodegenProgramView, EnumCodegenRecordView, EnumCodegenStmtKindView, EnumCodegenStmtView,
    EnumCodegenValueType,
};
use evo_lowering::{BinaryOp, ParameterPassingMode};

pub(super) fn generate_enum_rust(program: EnumCodegenProgramView<'_>) -> GeneratedRust {
    EnumGenerator::new().generate(program)
}

struct EnumGenerator {
    source: String,
    mappings: Vec<SourceMapping>,
    next_line: usize,
}

impl EnumGenerator {
    fn new() -> Self {
        Self {
            source: String::new(),
            mappings: Vec::new(),
            next_line: 1,
        }
    }

    fn generate(mut self, program: EnumCodegenProgramView<'_>) -> GeneratedRust {
        for enum_ir in program.enums().iter() {
            self.write_enum(enum_ir);
            self.push_unmapped("\n");
        }

        for record in program.records().iter() {
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

        for function in program.functions().iter() {
            self.write_function(function);
            self.push_unmapped("\n");
        }

        self.push_unmapped("fn main() {\n");
        for statement in program.statements().iter() {
            self.write_statement(statement, 1);
        }
        self.push_unmapped("}\n");

        GeneratedRust {
            source: self.source,
            mappings: self.mappings,
        }
    }

    fn write_enum(&mut self, enum_ir: evo_lowering::enum_codegen_view::EnumCodegenEnumView<'_>) {
        self.push_mapped_line(
            format!("enum {} {{\n", generated_enum_name(enum_ir.name())),
            enum_ir.span(),
        );
        for variant in enum_ir.variants().iter() {
            let declaration = if let Some(payload_type) = variant.payload_type() {
                format!(
                    "    {}({}),\n",
                    generated_variant_name(variant.name()),
                    rust_type(payload_type)
                )
            } else {
                format!("    {},\n", generated_variant_name(variant.name()))
            };
            self.push_mapped_line(declaration, variant.span());
        }
        self.push_mapped_line("}\n".to_owned(), enum_ir.span());
    }

    fn write_record(&mut self, record: EnumCodegenRecordView<'_>) {
        self.push_mapped_line(
            format!("struct {} {{\n", generated_record_name(record.name())),
            record.span(),
        );
        for field in record.fields().iter() {
            self.push_mapped_line(
                format!(
                    "    {}: {},\n",
                    generated_record_field_name(field.name()),
                    rust_type(field.value_type())
                ),
                field.span(),
            );
        }
        self.push_mapped_line("}\n".to_owned(), record.span());
    }

    fn write_function(&mut self, function: EnumCodegenFunctionView<'_>) {
        let mut signature = format!("fn {}(", generated_function_name(function.name()));
        for (index, parameter) in function.parameters().iter().enumerate() {
            if index > 0 {
                signature.push_str(", ");
            }
            if parameter.mutable() {
                signature.push_str("mut ");
            }
            signature.push_str(&generated_identifier(parameter.name()));
            signature.push_str(": ");
            if parameter.passing_mode() == ParameterPassingMode::SharedBorrow {
                debug_assert!(!parameter.mutable());
                signature.push('&');
            }
            signature.push_str(&rust_type(parameter.value_type()));
        }
        signature.push_str(") -> ");
        signature.push_str(&rust_type(function.return_type()));
        signature.push_str(" {\n");
        self.push_mapped_line(signature, function.span());
        for statement in function.body().iter() {
            self.write_statement(statement, 1);
        }
        self.push_mapped_line("}\n".to_owned(), function.span());
    }

    fn write_statement(&mut self, statement: EnumCodegenStmtView<'_>, indent: usize) {
        let padding = "    ".repeat(indent);
        match statement.kind() {
            EnumCodegenStmtKindView::Let {
                name,
                mutable,
                expr,
            } => {
                let mutable = if mutable { "mut " } else { "" };
                self.push_mapped_line(
                    format!(
                        "{padding}let {mutable}{} = {};\n",
                        generated_identifier(name),
                        render_expr(expr)
                    ),
                    statement.span(),
                );
            }
            EnumCodegenStmtKindView::Assign { name, expr } => {
                self.push_mapped_line(
                    format!(
                        "{padding}{} = {};\n",
                        generated_identifier(name),
                        render_expr(expr)
                    ),
                    statement.span(),
                );
            }
            EnumCodegenStmtKindView::Print(expr) => {
                self.push_mapped_line(
                    format!("{padding}println!(\"{{}}\", {});\n", render_expr(expr)),
                    statement.span(),
                );
            }
            EnumCodegenStmtKindView::Return(expr) => {
                self.push_mapped_line(
                    format!("{padding}return {};\n", render_expr(expr)),
                    statement.span(),
                );
            }
            EnumCodegenStmtKindView::Repeat { count, body } => {
                self.push_mapped_line(
                    format!("{padding}for _ in 0..{} {{\n", render_expr(count)),
                    statement.span(),
                );
                for child in body.iter() {
                    self.write_statement(child, indent + 1);
                }
                self.push_mapped_line(format!("{padding}}}\n"), statement.span());
            }
            EnumCodegenStmtKindView::If {
                condition,
                then_body,
                else_body,
            } => {
                self.push_mapped_line(
                    format!("{padding}if {} {{\n", render_expr(condition)),
                    statement.span(),
                );
                for child in then_body.iter() {
                    self.write_statement(child, indent + 1);
                }
                if else_body.is_empty() {
                    self.push_mapped_line(format!("{padding}}}\n"), statement.span());
                } else {
                    self.push_mapped_line(format!("{padding}}} else {{\n"), statement.span());
                    for child in else_body.iter() {
                        self.write_statement(child, indent + 1);
                    }
                    self.push_mapped_line(format!("{padding}}}\n"), statement.span());
                }
            }
            EnumCodegenStmtKindView::Match { value, arms, .. } => {
                self.push_mapped_line(
                    format!("{padding}match {} {{\n", render_expr(value)),
                    statement.span(),
                );
                for arm in arms.iter() {
                    self.write_match_arm(arm, indent + 1);
                }
                self.push_mapped_line(format!("{padding}}}\n"), statement.span());
            }
        }
    }

    fn write_match_arm(&mut self, arm: EnumCodegenMatchArmView<'_>, indent: usize) {
        let padding = "    ".repeat(indent);
        let mut pattern = format!(
            "{}::{}",
            generated_enum_name(arm.enum_name()),
            generated_variant_name(arm.variant_name())
        );
        if let Some(binding) = arm.binding() {
            pattern.push('(');
            if binding.mutable() {
                pattern.push_str("mut ");
            }
            pattern.push_str(&generated_identifier(binding.name()));
            pattern.push(')');
        }
        self.push_mapped_line(format!("{padding}{pattern} => {{\n"), arm.span());
        for statement in arm.body().iter() {
            self.write_statement(statement, indent + 1);
        }
        self.push_mapped_line(format!("{padding}}},\n"), arm.span());
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

fn rust_type(value_type: EnumCodegenValueType<'_>) -> String {
    match value_type {
        EnumCodegenValueType::Integer => "i64".to_owned(),
        EnumCodegenValueType::Bool => "bool".to_owned(),
        EnumCodegenValueType::String => "&'static str".to_owned(),
        EnumCodegenValueType::Record(name) => generated_record_name(name),
        EnumCodegenValueType::Enum(name) => generated_enum_name(name),
    }
}

fn render_expr(expr: EnumCodegenExprView<'_>) -> String {
    match expr.kind() {
        EnumCodegenExprKindView::Integer(value) => value.to_string(),
        EnumCodegenExprKindView::String(value) => format!("{value:?}"),
        EnumCodegenExprKindView::Bool(value) => value.to_string(),
        EnumCodegenExprKindView::Local { name, .. } => generated_identifier(name),
        EnumCodegenExprKindView::Call {
            name,
            arguments,
            argument_modes,
        } => {
            debug_assert_eq!(arguments.len(), argument_modes.len());
            let arguments = arguments
                .iter()
                .zip(argument_modes)
                .map(|(argument, passing_mode)| {
                    let rendered = render_expr(argument);
                    if *passing_mode == ParameterPassingMode::SharedBorrow {
                        format!("&{rendered}")
                    } else {
                        rendered
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({arguments})", generated_function_name(name))
        }
        EnumCodegenExprKindView::RecordConstruct { name, fields } => {
            if fields.is_empty() {
                format!("{} {{}}", generated_record_name(name))
            } else {
                let fields = fields
                    .iter()
                    .map(|field| {
                        format!(
                            "{}: {}",
                            generated_record_field_name(field.name()),
                            render_expr(field.value())
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} {{ {fields} }}", generated_record_name(name))
            }
        }
        EnumCodegenExprKindView::EnumConstruct {
            enum_name,
            variant_name,
            payload,
            ..
        } => {
            let target = format!(
                "{}::{}",
                generated_enum_name(enum_name),
                generated_variant_name(variant_name)
            );
            if let Some(payload) = payload {
                format!("{target}({})", render_expr(payload))
            } else {
                target
            }
        }
        EnumCodegenExprKindView::FieldAccess { base, field } => format!(
            "({}).{}",
            render_expr(base),
            generated_record_field_name(field)
        ),
        EnumCodegenExprKindView::InputInt => "__evo_input_int()".to_owned(),
        EnumCodegenExprKindView::LogicalNot(inner) => format!("(!{})", render_expr(inner)),
        EnumCodegenExprKindView::UnaryMinus(inner) => format!("(-{})", render_expr(inner)),
        EnumCodegenExprKindView::Binary { left, op, right } => format!(
            "({} {} {})",
            render_expr(left),
            render_binary_op(op),
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

fn generated_record_field_name(source_name: &str) -> String {
    format!("__evo_field_{source_name}")
}

fn generated_enum_name(source_name: &str) -> String {
    format!("__EvoEnum_{source_name}")
}

fn generated_variant_name(source_name: &str) -> String {
    format!("__EvoVariant_{source_name}")
}

fn program_uses_input_int(program: EnumCodegenProgramView<'_>) -> bool {
    program.statements().iter().any(statement_uses_input_int)
        || program
            .functions()
            .iter()
            .any(|function| function.body().iter().any(statement_uses_input_int))
}

fn statement_uses_input_int(statement: EnumCodegenStmtView<'_>) -> bool {
    match statement.kind() {
        EnumCodegenStmtKindView::Let { expr, .. }
        | EnumCodegenStmtKindView::Assign { expr, .. }
        | EnumCodegenStmtKindView::Print(expr)
        | EnumCodegenStmtKindView::Return(expr) => expr_uses_input_int(expr),
        EnumCodegenStmtKindView::Repeat { count, body } => {
            expr_uses_input_int(count) || body.iter().any(statement_uses_input_int)
        }
        EnumCodegenStmtKindView::If {
            condition,
            then_body,
            else_body,
        } => {
            expr_uses_input_int(condition)
                || then_body.iter().any(statement_uses_input_int)
                || else_body.iter().any(statement_uses_input_int)
        }
        EnumCodegenStmtKindView::Match { value, arms, .. } => {
            expr_uses_input_int(value)
                || arms
                    .iter()
                    .any(|arm| arm.body().iter().any(statement_uses_input_int))
        }
    }
}

fn expr_uses_input_int(expr: EnumCodegenExprView<'_>) -> bool {
    match expr.kind() {
        EnumCodegenExprKindView::InputInt => true,
        EnumCodegenExprKindView::Call { arguments, .. } => {
            arguments.iter().any(expr_uses_input_int)
        }
        EnumCodegenExprKindView::RecordConstruct { fields, .. } => fields
            .iter()
            .any(|field| expr_uses_input_int(field.value())),
        EnumCodegenExprKindView::EnumConstruct { payload, .. } => {
            payload.is_some_and(expr_uses_input_int)
        }
        EnumCodegenExprKindView::FieldAccess { base, .. }
        | EnumCodegenExprKindView::LogicalNot(base)
        | EnumCodegenExprKindView::UnaryMinus(base) => expr_uses_input_int(base),
        EnumCodegenExprKindView::Binary { left, right, .. } => {
            expr_uses_input_int(left) || expr_uses_input_int(right)
        }
        EnumCodegenExprKindView::Integer(_)
        | EnumCodegenExprKindView::String(_)
        | EnumCodegenExprKindView::Bool(_)
        | EnumCodegenExprKindView::Local { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::generate_enum_rust;
    use evo_lexer::lex;
    use evo_lowering::lower;
    use evo_parser::parse;

    fn generate(source: &str) -> crate::GeneratedRust {
        let tokens = lex(source).expect("enum codegen source should lex");
        let syntax = parse(&tokens).expect("enum codegen source should parse");
        let program = lower(&syntax).expect("enum codegen source should lower");
        let view = program
            .enum_codegen_view()
            .expect("enum codegen source should expose executable view");
        generate_enum_rust(view)
    }

    #[test]
    fn emits_direct_static_enum_record_constructor_and_match_rust() {
        let generated = generate(
            "enum Inner\nA int\nend\nenum Wrapped\nNone\nSome Inner\nend\nrecord Holder\nvalue Wrapped\nend\nfn unwrap(value Wrapped) int\nmatch value\ncase Wrapped.None\nreturn 0\ncase Wrapped.Some(x)\nmatch x\ncase Inner.A(y)\nreturn y\nend\nend\nend\nvalue = Wrapped.Some(Inner.A(7))\nprint unwrap(value)\n",
        );

        assert!(generated.source.contains("enum __EvoEnum_Inner {"));
        assert!(generated.source.contains("__EvoVariant_A(i64),"));
        assert!(generated.source.contains("enum __EvoEnum_Wrapped {"));
        assert!(
            generated
                .source
                .contains("__EvoVariant_Some(__EvoEnum_Inner),")
        );
        assert!(
            generated
                .source
                .contains("__evo_field_value: __EvoEnum_Wrapped,")
        );
        assert!(generated.source.contains(
            "let __evo_value = __EvoEnum_Wrapped::__EvoVariant_Some(__EvoEnum_Inner::__EvoVariant_A(7));"
        ));
        assert!(generated.source.contains("match __evo_value {"));
        assert!(
            generated
                .source
                .contains("__EvoEnum_Wrapped::__EvoVariant_Some(__evo_x) => {")
        );
        assert!(
            generated
                .source
                .contains("__EvoEnum_Inner::__EvoVariant_A(__evo_y) => {")
        );
        assert!(!generated.source.contains(".clone("));
        assert!(!generated.source.contains("Box<"));
        assert!(!generated.source.contains("Rc<"));
        assert!(!generated.source.contains("Arc<"));
        assert!(!generated.source.contains("HashMap"));
    }

    #[test]
    fn maps_enum_variant_and_match_structure_back_to_source() {
        let generated = generate(
            "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nmatch value\ncase Flag.Off\nprint 0\ncase Flag.On\nprint 1\nend\n",
        );

        let enum_line = generated
            .source
            .lines()
            .position(|line| line == "enum __EvoEnum_Flag {")
            .expect("generated enum declaration should exist")
            + 1;
        let on_variant_line = generated
            .source
            .lines()
            .position(|line| line.trim() == "__EvoVariant_On,")
            .expect("generated On variant should exist")
            + 1;
        let match_line = generated
            .source
            .lines()
            .position(|line| line.trim_start().starts_with("match __evo_value"))
            .expect("generated match should exist")
            + 1;
        let on_arm_line = generated
            .source
            .lines()
            .position(|line| {
                line.trim_start()
                    .starts_with("__EvoEnum_Flag::__EvoVariant_On =>")
            })
            .expect("generated On arm should exist")
            + 1;

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
}
