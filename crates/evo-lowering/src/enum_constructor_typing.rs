use crate::LowerError;
use evo_lexer::Span;
use evo_parser::{
    BinaryOp, Expr as SyntaxExpr, ExprKind as SyntaxExprKind, Program as SyntaxProgram,
    RecordFieldType as SyntaxRecordFieldType, Stmt as SyntaxStmt, StmtKind as SyntaxStmtKind,
    TypeName as SyntaxTypeName,
};
use std::collections::{HashMap, HashSet};

use super::{EnumEnvironment, ResolvedPayloadType};

#[derive(Debug, Clone)]
struct RecordView {
    fields: HashMap<String, ResolvedPayloadType>,
}

#[derive(Debug)]
struct ConstructorTypeEnvironment<'a> {
    enums: &'a EnumEnvironment,
    records: HashMap<String, RecordView>,
    function_returns: HashMap<String, ResolvedPayloadType>,
}

impl<'a> ConstructorTypeEnvironment<'a> {
    fn collect(
        program: &SyntaxProgram,
        enums: &'a EnumEnvironment,
    ) -> Result<Self, LowerError> {
        let record_names: HashSet<&str> = program
            .records
            .iter()
            .map(|record| record.name.as_str())
            .collect();

        let mut records = HashMap::new();
        for record in &program.records {
            let mut fields = HashMap::new();
            for field in &record.fields {
                let value_type = resolve_record_field_type(
                    &field.type_name,
                    &record_names,
                    enums,
                    field.span,
                )?;
                fields.insert(field.name.clone(), value_type);
            }
            records.insert(record.name.clone(), RecordView { fields });
        }

        let mut function_returns = HashMap::new();
        for function in &program.functions {
            let return_type = resolve_signature_type(
                &function.return_type,
                &record_names,
                enums,
                function.span,
            )?;
            function_returns
                .entry(function.name.clone())
                .or_insert(return_type);
        }

        Ok(Self {
            enums,
            records,
            function_returns,
        })
    }

    fn infer_expr(
        &self,
        expr: &SyntaxExpr,
        scopes: &[HashMap<String, ResolvedPayloadType>],
    ) -> Result<Option<ResolvedPayloadType>, LowerError> {
        match &expr.kind {
            SyntaxExprKind::Integer(_) | SyntaxExprKind::InputInt => {
                Ok(Some(ResolvedPayloadType::Integer))
            }
            SyntaxExprKind::String(_) => Ok(Some(ResolvedPayloadType::String)),
            SyntaxExprKind::Bool(_) => Ok(Some(ResolvedPayloadType::Bool)),
            SyntaxExprKind::Identifier(name) => Ok(lookup_local(scopes, name)),
            SyntaxExprKind::Call { name, arguments } => {
                for argument in arguments {
                    let _ = self.infer_expr(argument, scopes)?;
                }
                if self
                    .records
                    .get(name)
                    .is_some_and(|record| record.fields.is_empty())
                {
                    return Ok(Some(ResolvedPayloadType::Record(name.clone())));
                }
                Ok(self.function_returns.get(name).cloned())
            }
            SyntaxExprKind::Construct { name, fields } => {
                for field in fields {
                    let _ = self.infer_expr(&field.value, scopes)?;
                }
                Ok(self
                    .records
                    .contains_key(name)
                    .then(|| ResolvedPayloadType::Record(name.clone())))
            }
            SyntaxExprKind::EnumConstruct {
                enum_name,
                variant_name,
                arguments,
            } => {
                let variant = self.enums.resolve_constructor_variant(
                    enum_name,
                    variant_name,
                    arguments.len(),
                    expr.span,
                )?;
                let mut argument_types = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    argument_types.push(self.infer_expr(argument, scopes)?);
                }
                if let (Some(expected), [Some(actual)]) =
                    (&variant.payload_type, argument_types.as_slice())
                    && actual != expected
                {
                    return Err(payload_type_mismatch(
                        enum_name,
                        variant_name,
                        expected,
                        actual,
                        arguments[0].span,
                    ));
                }
                Ok(Some(ResolvedPayloadType::Enum(enum_name.clone())))
            }
            SyntaxExprKind::FieldAccess { base, field } => {
                let Some(base_type) = self.infer_expr(base, scopes)? else {
                    return Ok(None);
                };
                let ResolvedPayloadType::Record(record_name) = base_type else {
                    return Ok(None);
                };
                Ok(self
                    .records
                    .get(&record_name)
                    .and_then(|record| record.fields.get(field))
                    .cloned())
            }
            SyntaxExprKind::LogicalNot(inner) => {
                let _ = self.infer_expr(inner, scopes)?;
                Ok(Some(ResolvedPayloadType::Bool))
            }
            SyntaxExprKind::UnaryMinus(inner) => {
                let _ = self.infer_expr(inner, scopes)?;
                Ok(Some(ResolvedPayloadType::Integer))
            }
            SyntaxExprKind::Binary { left, op, right } => {
                let _ = self.infer_expr(left, scopes)?;
                let _ = self.infer_expr(right, scopes)?;
                Ok(Some(binary_result_type(*op)))
            }
        }
    }
}

pub(super) fn validate_constructor_payload_types(
    program: &SyntaxProgram,
    enums: &EnumEnvironment,
) -> Result<(), LowerError> {
    let environment = ConstructorTypeEnvironment::collect(program, enums)?;

    for function in &program.functions {
        let record_names: HashSet<&str> = program
            .records
            .iter()
            .map(|record| record.name.as_str())
            .collect();
        let mut root = HashMap::new();
        for parameter in &function.parameters {
            let value_type = resolve_signature_type(
                &parameter.type_name,
                &record_names,
                enums,
                parameter.span,
            )?;
            root.insert(parameter.name.clone(), value_type);
        }
        let mut scopes = vec![root];
        validate_statements(&function.body, &environment, &mut scopes)?;
    }

    let mut top_level_scopes = vec![HashMap::new()];
    validate_statements(&program.statements, &environment, &mut top_level_scopes)
}

fn validate_statements(
    statements: &[SyntaxStmt],
    environment: &ConstructorTypeEnvironment<'_>,
    scopes: &mut [HashMap<String, ResolvedPayloadType>],
) -> Result<(), LowerError> {
    for statement in statements {
        match &statement.kind {
            SyntaxStmtKind::Bind { name, expr } => {
                let inferred = environment.infer_expr(expr, scopes)?;
                if let Some(value_type) = inferred {
                    remember_binding_type(scopes, name, value_type);
                }
            }
            SyntaxStmtKind::Print(expr) | SyntaxStmtKind::Return(expr) => {
                let _ = environment.infer_expr(expr, scopes)?;
            }
            SyntaxStmtKind::Repeat { count, body } => {
                let _ = environment.infer_expr(count, scopes)?;
                validate_child_scope(body, environment, scopes)?;
            }
            SyntaxStmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                let _ = environment.infer_expr(condition, scopes)?;
                validate_child_scope(then_body, environment, scopes)?;
                validate_child_scope(else_body, environment, scopes)?;
            }
            SyntaxStmtKind::Match { value, arms } => {
                let _ = environment.infer_expr(value, scopes)?;
                for arm in arms {
                    // The pattern payload binding is deliberately not introduced here.
                    // Its type and lexical scope belong to exhaustive match semantics (#57).
                    validate_child_scope(&arm.body, environment, scopes)?;
                }
            }
        }
    }
    Ok(())
}

fn validate_child_scope(
    statements: &[SyntaxStmt],
    environment: &ConstructorTypeEnvironment<'_>,
    scopes: &[HashMap<String, ResolvedPayloadType>],
) -> Result<(), LowerError> {
    let mut child_scopes = scopes.to_vec();
    child_scopes.push(HashMap::new());
    validate_statements(statements, environment, &mut child_scopes)
}

fn remember_binding_type(
    scopes: &mut [HashMap<String, ResolvedPayloadType>],
    name: &str,
    value_type: ResolvedPayloadType,
) {
    for scope in scopes.iter_mut().rev() {
        if scope.contains_key(name) {
            // Existing bindings cannot change type in the production lowerer. Keep the
            // established type here rather than creating a second assignment checker.
            return;
        }
    }
    scopes
        .last_mut()
        .expect("constructor typing always has a lexical scope")
        .insert(name.to_owned(), value_type);
}

fn lookup_local(
    scopes: &[HashMap<String, ResolvedPayloadType>],
    name: &str,
) -> Option<ResolvedPayloadType> {
    scopes
        .iter()
        .rev()
        .find_map(|scope| scope.get(name).cloned())
}

fn resolve_record_field_type(
    field_type: &SyntaxRecordFieldType,
    record_names: &HashSet<&str>,
    enums: &EnumEnvironment,
    span: Span,
) -> Result<ResolvedPayloadType, LowerError> {
    match field_type {
        SyntaxRecordFieldType::Int => Ok(ResolvedPayloadType::Integer),
        SyntaxRecordFieldType::Bool => Ok(ResolvedPayloadType::Bool),
        SyntaxRecordFieldType::String => Ok(ResolvedPayloadType::String),
        SyntaxRecordFieldType::Named(name) if record_names.contains(name.as_str()) => {
            Ok(ResolvedPayloadType::Record(name.clone()))
        }
        SyntaxRecordFieldType::Named(name) if enums.schema(name).is_some() => {
            Ok(ResolvedPayloadType::Enum(name.clone()))
        }
        SyntaxRecordFieldType::Named(name) => Err(LowerError {
            message: format!("unknown nominal type {name:?}"),
            span,
        }),
    }
}

fn resolve_signature_type(
    type_name: &SyntaxTypeName,
    record_names: &HashSet<&str>,
    enums: &EnumEnvironment,
    span: Span,
) -> Result<ResolvedPayloadType, LowerError> {
    match type_name {
        SyntaxTypeName::Int => Ok(ResolvedPayloadType::Integer),
        SyntaxTypeName::Bool => Ok(ResolvedPayloadType::Bool),
        SyntaxTypeName::String => Ok(ResolvedPayloadType::String),
        SyntaxTypeName::Named(name) if record_names.contains(name.as_str()) => {
            Ok(ResolvedPayloadType::Record(name.clone()))
        }
        SyntaxTypeName::Named(name) if enums.schema(name).is_some() => {
            Ok(ResolvedPayloadType::Enum(name.clone()))
        }
        SyntaxTypeName::Named(name) => Err(LowerError {
            message: format!("unknown nominal type {name:?} in function signature"),
            span,
        }),
    }
}

fn binary_result_type(op: BinaryOp) -> ResolvedPayloadType {
    match op {
        BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
            ResolvedPayloadType::Integer
        }
        BinaryOp::Equal
        | BinaryOp::NotEqual
        | BinaryOp::Less
        | BinaryOp::LessEqual
        | BinaryOp::Greater
        | BinaryOp::GreaterEqual
        | BinaryOp::And
        | BinaryOp::Or => ResolvedPayloadType::Bool,
    }
}

fn payload_type_mismatch(
    enum_name: &str,
    variant_name: &str,
    expected: &ResolvedPayloadType,
    actual: &ResolvedPayloadType,
    span: Span,
) -> LowerError {
    LowerError {
        message: format!(
            "payload for enum variant {enum_name:?}.{variant_name:?} expects {}, found {}",
            type_label(expected),
            type_label(actual)
        ),
        span,
    }
}

fn type_label(value_type: &ResolvedPayloadType) -> &str {
    match value_type {
        ResolvedPayloadType::Integer => "int",
        ResolvedPayloadType::Bool => "bool",
        ResolvedPayloadType::String => "string",
        ResolvedPayloadType::Record(name) | ResolvedPayloadType::Enum(name) => name,
    }
}

#[cfg(test)]
mod tests {
    use super::validate_constructor_payload_types;
    use crate::record_environment::enums_impl::collect_enum_environment;
    use evo_lexer::lex;
    use evo_parser::parse;

    fn validate(source: &str) -> Result<(), crate::LowerError> {
        let tokens = lex(source).expect("constructor typing source should lex");
        let program = parse(&tokens).expect("constructor typing source should parse");
        let enums = collect_enum_environment(&program)?;
        validate_constructor_payload_types(&program, &enums)
    }

    #[test]
    fn local_binding_type_flows_into_enum_payload_check() {
        let error = validate(
            "enum MaybeInt\nNone\nSome int\nend\nvalue = true\nwrapped = MaybeInt.Some(value)\n",
        )
        .expect_err("local bool should not satisfy int payload");
        assert!(error.message.contains("expects int, found bool"));
        assert_eq!(error.span.line, 6);
    }

    #[test]
    fn function_return_type_flows_into_enum_payload_check() {
        let error = validate(
            "enum MaybeInt\nNone\nSome int\nend\nfn flag() bool\nreturn true\nend\nvalue = MaybeInt.Some(flag())\n",
        )
        .expect_err("bool-returning call should not satisfy int payload");
        assert!(error.message.contains("expects int, found bool"));
        assert_eq!(error.span.line, 8);
    }

    #[test]
    fn function_parameter_enum_type_is_available_inside_body() {
        validate(
            "enum MaybeInt\nNone\nSome int\nend\nenum Wrapped\nEmpty\nValue MaybeInt\nend\nfn wrap(value MaybeInt) Wrapped\nreturn Wrapped.Value(value)\nend\n",
        )
        .expect("enum-typed parameter should satisfy matching enum payload");
    }

    #[test]
    fn named_record_constructor_type_flows_into_enum_payload_check() {
        validate(
            "record Point\nx int\nend\nenum MaybePoint\nNone\nSome Point\nend\nvalue = MaybePoint.Some(Point(x = 1))\n",
        )
        .expect("record constructor nominal type should satisfy record payload");
    }

    #[test]
    fn record_field_type_flows_into_enum_payload_check() {
        let error = validate(
            "record FlagBox\nvalue bool\nend\nenum MaybeInt\nNone\nSome int\nend\nbox = FlagBox(value = true)\nwrapped = MaybeInt.Some(box.value)\n",
        )
        .expect_err("bool record field should not satisfy int payload");
        assert!(error.message.contains("expects int, found bool"));
        assert_eq!(error.span.line, 8);
    }
}
