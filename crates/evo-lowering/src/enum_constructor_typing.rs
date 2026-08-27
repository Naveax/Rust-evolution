use crate::LowerError;
use evo_lexer::Span;
use evo_parser::{
    BinaryOp, Expr as SyntaxExpr, ExprKind as SyntaxExprKind, MatchArm as SyntaxMatchArm,
    Program as SyntaxProgram, RecordFieldType as SyntaxRecordFieldType, Stmt as SyntaxStmt,
    StmtKind as SyntaxStmtKind, TypeName as SyntaxTypeName,
};
use std::collections::{HashMap, HashSet};

use super::{EnumEnvironment, ResolvedPayloadType};

#[derive(Debug, Clone)]
struct RecordView {
    fields: HashMap<String, ResolvedPayloadType>,
}

#[derive(Debug)]
struct EnumTypeEnvironment<'a> {
    enums: &'a EnumEnvironment,
    records: HashMap<String, RecordView>,
    function_returns: HashMap<String, ResolvedPayloadType>,
}

impl<'a> EnumTypeEnvironment<'a> {
    fn collect(program: &SyntaxProgram, enums: &'a EnumEnvironment) -> Result<Self, LowerError> {
        let record_names: HashSet<&str> = program
            .records
            .iter()
            .map(|record| record.name.as_str())
            .collect();

        let mut records = HashMap::new();
        for record in &program.records {
            let mut fields = HashMap::new();
            for field in &record.fields {
                let value_type =
                    resolve_record_field_type(&field.type_name, &record_names, enums, field.span)?;
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
            SyntaxExprKind::Identifier(name) => lookup_local(scopes, name)
                .map(Some)
                .ok_or_else(|| LowerError {
                    message: format!(
                        "use of local {name:?} before definition or outside its scope"
                    ),
                    span: expr.span,
                }),
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

pub(super) fn validate_enum_type_semantics(
    program: &SyntaxProgram,
    enums: &EnumEnvironment,
) -> Result<(), LowerError> {
    let environment = EnumTypeEnvironment::collect(program, enums)?;

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
    environment: &EnumTypeEnvironment<'_>,
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
                validate_match(value, arms, environment, scopes)?;
            }
        }
    }
    Ok(())
}

fn validate_match(
    value: &SyntaxExpr,
    arms: &[SyntaxMatchArm],
    environment: &EnumTypeEnvironment<'_>,
    scopes: &[HashMap<String, ResolvedPayloadType>],
) -> Result<(), LowerError> {
    let scrutinee_type = environment.infer_expr(value, scopes)?;
    let scrutinee_enum = match scrutinee_type {
        Some(ResolvedPayloadType::Enum(name)) => name,
        Some(actual) => {
            return Err(LowerError {
                message: format!(
                    "match scrutinee must have an enum type; found {}",
                    type_label(&actual)
                ),
                span: value.span,
            });
        }
        None => {
            return Err(LowerError {
                message: "match scrutinee must have a statically known enum type".to_owned(),
                span: value.span,
            });
        }
    };

    let schema = environment
        .enums
        .schema(&scrutinee_enum)
        .expect("enum expression types originate from the resolved enum environment");

    for arm in arms {
        if arm.pattern.enum_name != scrutinee_enum {
            return Err(LowerError {
                message: format!(
                    "match arm uses enum {:?}, but scrutinee has enum type {:?}",
                    arm.pattern.enum_name, scrutinee_enum
                ),
                span: arm.pattern.span,
            });
        }

        let variant = schema
            .variants
            .iter()
            .find(|candidate| candidate.name == arm.pattern.variant_name)
            .expect("structural match validation resolves every arm variant first");

        let mut arm_scopes = scopes.to_vec();
        arm_scopes.push(HashMap::new());
        match (&variant.payload_type, &arm.pattern.binding) {
            (Some(payload_type), Some(binding)) => {
                if lookup_local(scopes, binding).is_some() {
                    return Err(LowerError {
                        message: format!(
                            "match payload binding {binding:?} conflicts with an already-visible local"
                        ),
                        span: arm.pattern.span,
                    });
                }
                arm_scopes
                    .last_mut()
                    .expect("match arm always has a lexical child scope")
                    .insert(binding.clone(), payload_type.clone());
            }
            (None, None) => {}
            (None, Some(_)) | (Some(_), None) => {
                unreachable!("structural match validation checks payload binding shape first")
            }
        }
        validate_statements(&arm.body, environment, &mut arm_scopes)?;
    }

    Ok(())
}

fn validate_child_scope(
    statements: &[SyntaxStmt],
    environment: &EnumTypeEnvironment<'_>,
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
        .expect("enum typing always has a lexical scope")
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
    use super::validate_enum_type_semantics;
    use crate::record_environment::enums_impl::collect_enum_environment;
    use evo_lexer::lex;
    use evo_parser::parse;

    fn validate(source: &str) -> Result<(), crate::LowerError> {
        let tokens = lex(source).expect("enum typing source should lex");
        let program = parse(&tokens).expect("enum typing source should parse");
        let enums = collect_enum_environment(&program)?;
        validate_enum_type_semantics(&program, &enums)
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
        assert_eq!(error.span.line, 9);
    }

    #[test]
    fn match_scrutinee_must_have_enum_type() {
        let error = validate(
            "enum Flag\nOff\nOn\nend\nmatch true\ncase Flag.Off\nprint 0\ncase Flag.On\nprint 1\nend\n",
        )
        .expect_err("boolean scrutinee must not type-check as an enum");
        assert!(error.message.contains("scrutinee must have an enum type"));
        assert_eq!(error.span.line, 5);
    }

    #[test]
    fn arm_enum_must_match_scrutinee_nominal_type() {
        let error = validate(
            "enum Left\nOne\nend\nenum Right\nOther\nend\nvalue = Left.One()\nmatch value\ncase Right.Other\nprint 0\nend\n",
        )
        .expect_err("arm qualifier must match the scrutinee enum type");
        assert!(error.message.contains("scrutinee has enum type \"Left\""));
        assert_eq!(error.span.line, 9);
    }

    #[test]
    fn payload_binding_type_flows_inside_its_arm() {
        let error = validate(
            "enum MaybeInt\nNone\nSome int\nend\nenum MaybeBool\nNone\nSome bool\nend\nvalue = MaybeInt.Some(1)\nmatch value\ncase MaybeInt.None\nprint 0\ncase MaybeInt.Some(x)\nwrapped = MaybeBool.Some(x)\nend\n",
        )
        .expect_err("int payload binding must retain its declared type inside the arm");
        assert!(error.message.contains("expects bool, found int"));
        assert_eq!(error.span.line, 14);
    }

    #[test]
    fn sibling_arm_payload_bindings_are_independent() {
        validate(
            "enum Choice\nLeft int\nRight bool\nend\nenum MaybeInt\nNone\nSome int\nend\nenum MaybeBool\nNone\nSome bool\nend\nvalue = Choice.Left(1)\nmatch value\ncase Choice.Left(x)\na = MaybeInt.Some(x)\ncase Choice.Right(x)\nb = MaybeBool.Some(x)\nend\n",
        )
        .expect("same binding spelling should be independent across sibling arms");
    }

    #[test]
    fn payload_binding_does_not_leak_after_match() {
        let error = validate(
            "enum MaybeInt\nNone\nSome int\nend\nvalue = MaybeInt.Some(1)\nmatch value\ncase MaybeInt.None\nprint 0\ncase MaybeInt.Some(x)\nprint x\nend\nwrapped = MaybeInt.Some(x)\n",
        )
        .expect_err("payload binding must not escape its match arm");
        assert!(error.message.contains("outside its scope"));
        assert_eq!(error.span.line, 12);
    }

    #[test]
    fn payload_binding_conflicts_with_visible_outer_local() {
        let error = validate(
            "enum MaybeInt\nNone\nSome int\nend\nx = 1\nvalue = MaybeInt.Some(2)\nmatch value\ncase MaybeInt.None\nprint 0\ncase MaybeInt.Some(x)\nprint x\nend\n",
        )
        .expect_err("payload binding must not shadow an already-visible local");
        assert!(error.message.contains("conflicts with an already-visible local"));
        assert_eq!(error.span.line, 10);
    }

    #[test]
    fn enum_typed_function_parameter_can_be_matched() {
        validate(
            "enum MaybeInt\nNone\nSome int\nend\nfn read(value MaybeInt) int\nmatch value\ncase MaybeInt.None\nreturn 0\ncase MaybeInt.Some(x)\nreturn x\nend\nend\n",
        )
        .expect("enum-typed parameters should be valid match scrutinees");
    }
}
