use crate::LowerError;
use evo_lexer::Span;
use evo_parser::{
    BinaryOp, Expr as SyntaxExpr, ExprKind as SyntaxExprKind, MatchArm as SyntaxMatchArm,
    Program as SyntaxProgram, Stmt as SyntaxStmt, StmtKind as SyntaxStmtKind, TypeName,
};
use std::collections::{HashMap, HashSet};

use super::{EnumEnvironment, ResolvedPayloadType};

#[derive(Debug, Clone)]
struct FunctionSignature {
    parameter_types: Vec<ResolvedPayloadType>,
    return_type: ResolvedPayloadType,
}

#[derive(Debug, Clone)]
struct RecordSignature {
    fields: Vec<(String, ResolvedPayloadType)>,
}

struct StaticEnvironment<'a> {
    enums: &'a EnumEnvironment,
    records: HashMap<String, RecordSignature>,
    functions: HashMap<String, FunctionSignature>,
}

impl<'a> StaticEnvironment<'a> {
    fn collect(program: &SyntaxProgram, enums: &'a EnumEnvironment) -> Result<Self, LowerError> {
        let record_names: HashSet<&str> = program
            .records
            .iter()
            .map(|record| record.name.as_str())
            .collect();

        let mut records = HashMap::new();
        for record in &program.records {
            let mut fields = Vec::with_capacity(record.fields.len());
            for field in &record.fields {
                let value_type = match &field.type_name {
                    evo_parser::RecordFieldType::Int => ResolvedPayloadType::Integer,
                    evo_parser::RecordFieldType::Bool => ResolvedPayloadType::Bool,
                    evo_parser::RecordFieldType::String => ResolvedPayloadType::String,
                    evo_parser::RecordFieldType::Named(name)
                        if record_names.contains(name.as_str()) =>
                    {
                        ResolvedPayloadType::Record(name.clone())
                    }
                    evo_parser::RecordFieldType::Named(name) if enums.schema(name).is_some() => {
                        ResolvedPayloadType::Enum(name.clone())
                    }
                    evo_parser::RecordFieldType::Named(name) => {
                        return Err(LowerError {
                            message: format!("unknown nominal type {name:?}"),
                            span: field.span,
                        });
                    }
                };
                fields.push((field.name.clone(), value_type));
            }
            records.insert(record.name.clone(), RecordSignature { fields });
        }

        let mut functions = HashMap::new();
        for function in &program.functions {
            if functions.contains_key(&function.name) {
                return Err(LowerError {
                    message: format!("duplicate function name {:?}", function.name),
                    span: function.span,
                });
            }

            let mut seen_parameters = HashSet::new();
            let mut parameter_types = Vec::with_capacity(function.parameters.len());
            for parameter in &function.parameters {
                if !seen_parameters.insert(parameter.name.as_str()) {
                    return Err(LowerError {
                        message: format!("duplicate parameter name {:?}", parameter.name),
                        span: parameter.span,
                    });
                }
                parameter_types.push(resolve_type_name(
                    &parameter.type_name,
                    &record_names,
                    enums,
                    parameter.span,
                )?);
            }

            let return_type =
                resolve_type_name(&function.return_type, &record_names, enums, function.span)?;
            functions.insert(
                function.name.clone(),
                FunctionSignature {
                    parameter_types,
                    return_type,
                },
            );
        }

        Ok(Self {
            enums,
            records,
            functions,
        })
    }

    fn infer_expr(
        &self,
        expr: &SyntaxExpr,
        scopes: &[HashMap<String, ResolvedPayloadType>],
    ) -> Result<ResolvedPayloadType, LowerError> {
        match &expr.kind {
            SyntaxExprKind::Integer(_) | SyntaxExprKind::InputInt => {
                Ok(ResolvedPayloadType::Integer)
            }
            SyntaxExprKind::String(_) => Ok(ResolvedPayloadType::String),
            SyntaxExprKind::Bool(_) => Ok(ResolvedPayloadType::Bool),
            SyntaxExprKind::Identifier(name) => lookup_local(scopes, name).ok_or_else(|| LowerError {
                message: format!("use of local {name:?} before definition or outside its scope"),
                span: expr.span,
            }),
            SyntaxExprKind::Call { name, arguments } => {
                let mut argument_types = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    argument_types.push(self.infer_expr(argument, scopes)?);
                }

                if let Some(record) = self.records.get(name) {
                    if !arguments.is_empty() {
                        return Err(LowerError {
                            message: format!(
                                "record constructor {name:?} requires named fields; positional arguments are not supported"
                            ),
                            span: expr.span,
                        });
                    }
                    if !record.fields.is_empty() {
                        return Err(LowerError {
                            message: format!(
                                "record constructor {name:?} is missing field(s): {}",
                                record
                                    .fields
                                    .iter()
                                    .map(|(field, _)| field.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            span: expr.span,
                        });
                    }
                    return Ok(ResolvedPayloadType::Record(name.clone()));
                }

                let signature = self.functions.get(name).ok_or_else(|| LowerError {
                    message: format!("unknown function {name:?}"),
                    span: expr.span,
                })?;
                if argument_types.len() != signature.parameter_types.len() {
                    return Err(LowerError {
                        message: format!(
                            "function {name:?} expects {} arguments, found {}",
                            signature.parameter_types.len(),
                            argument_types.len()
                        ),
                        span: expr.span,
                    });
                }
                for (index, ((argument, actual), expected)) in arguments
                    .iter()
                    .zip(argument_types.iter())
                    .zip(signature.parameter_types.iter())
                    .enumerate()
                {
                    if actual != expected {
                        return Err(LowerError {
                            message: format!(
                                "argument {} for function {name:?} expects {}, found {}",
                                index + 1,
                                type_label(expected),
                                type_label(actual)
                            ),
                            span: argument.span,
                        });
                    }
                }
                Ok(signature.return_type.clone())
            }
            SyntaxExprKind::Construct { name, fields } => {
                let record = self.records.get(name).ok_or_else(|| LowerError {
                    message: format!("unknown record constructor {name:?}"),
                    span: expr.span,
                })?;
                let mut seen = HashSet::new();
                for field in fields {
                    let actual = self.infer_expr(&field.value, scopes)?;
                    if !seen.insert(field.name.as_str()) {
                        return Err(LowerError {
                            message: format!(
                                "duplicate constructor field {:?} for record {:?}",
                                field.name, name
                            ),
                            span: field.span,
                        });
                    }
                    let (_, expected) = record
                        .fields
                        .iter()
                        .find(|(candidate, _)| candidate == &field.name)
                        .ok_or_else(|| LowerError {
                            message: format!(
                                "unknown constructor field {:?} for record {:?}",
                                field.name, name
                            ),
                            span: field.span,
                        })?;
                    if &actual != expected {
                        return Err(LowerError {
                            message: format!(
                                "constructor field {:?} for record {:?} expects {}, found {}",
                                field.name,
                                name,
                                type_label(expected),
                                type_label(&actual)
                            ),
                            span: field.span,
                        });
                    }
                }
                let missing: Vec<&str> = record
                    .fields
                    .iter()
                    .filter(|(field, _)| !seen.contains(field.as_str()))
                    .map(|(field, _)| field.as_str())
                    .collect();
                if !missing.is_empty() {
                    return Err(LowerError {
                        message: format!(
                            "record constructor {name:?} is missing field(s): {}",
                            missing.join(", ")
                        ),
                        span: expr.span,
                    });
                }
                Ok(ResolvedPayloadType::Record(name.clone()))
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
                let actual = match arguments.as_slice() {
                    [] => None,
                    [argument] => Some((argument, self.infer_expr(argument, scopes)?)),
                    _ => unreachable!("enum constructor arity is validated before strict typing"),
                };
                if let (Some(expected), Some((argument, actual))) = (&variant.payload_type, actual)
                    && expected != &actual
                {
                    return Err(LowerError {
                        message: format!(
                            "payload for enum variant {enum_name:?}.{variant_name:?} expects {}, found {}",
                            type_label(expected),
                            type_label(&actual)
                        ),
                        span: argument.span,
                    });
                }
                Ok(ResolvedPayloadType::Enum(enum_name.clone()))
            }
            SyntaxExprKind::FieldAccess { base, field } => {
                let base_type = self.infer_expr(base, scopes)?;
                let ResolvedPayloadType::Record(record_name) = base_type else {
                    return Err(LowerError {
                        message: "field access requires a record value".to_owned(),
                        span: expr.span,
                    });
                };
                let record = self
                    .records
                    .get(&record_name)
                    .expect("record expression types originate from validated record declarations");
                record
                    .fields
                    .iter()
                    .find(|(candidate, _)| candidate == field)
                    .map(|(_, value_type)| value_type.clone())
                    .ok_or_else(|| LowerError {
                        message: format!("unknown field {field:?} on record {record_name:?}"),
                        span: expr.span,
                    })
            }
            SyntaxExprKind::LogicalNot(inner) => {
                let inner_type = self.infer_expr(inner, scopes)?;
                if inner_type != ResolvedPayloadType::Bool {
                    return Err(LowerError {
                        message: "logical 'not' requires a boolean operand".to_owned(),
                        span: expr.span,
                    });
                }
                Ok(ResolvedPayloadType::Bool)
            }
            SyntaxExprKind::UnaryMinus(inner) => {
                let inner_type = self.infer_expr(inner, scopes)?;
                if inner_type != ResolvedPayloadType::Integer {
                    return Err(LowerError {
                        message: "unary '-' requires an integer operand".to_owned(),
                        span: expr.span,
                    });
                }
                Ok(ResolvedPayloadType::Integer)
            }
            SyntaxExprKind::Binary { left, op, right } => {
                let left_type = self.infer_expr(left, scopes)?;
                let right_type = self.infer_expr(right, scopes)?;
                match op {
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                        if left_type != ResolvedPayloadType::Integer
                            || right_type != ResolvedPayloadType::Integer
                        {
                            return Err(LowerError {
                                message: "arithmetic operators require integer operands".to_owned(),
                                span: expr.span,
                            });
                        }
                        Ok(ResolvedPayloadType::Integer)
                    }
                    BinaryOp::Equal | BinaryOp::NotEqual => {
                        if matches!(left_type, ResolvedPayloadType::Record(_))
                            || matches!(right_type, ResolvedPayloadType::Record(_))
                        {
                            return Err(LowerError {
                                message: "record equality is not supported in Records v0".to_owned(),
                                span: expr.span,
                            });
                        }
                        if matches!(left_type, ResolvedPayloadType::Enum(_))
                            || matches!(right_type, ResolvedPayloadType::Enum(_))
                        {
                            return Err(LowerError {
                                message: "enum equality is not supported in Enums v0".to_owned(),
                                span: expr.span,
                            });
                        }
                        if left_type != right_type {
                            return Err(LowerError {
                                message: "equality operands must have the same value type".to_owned(),
                                span: expr.span,
                            });
                        }
                        Ok(ResolvedPayloadType::Bool)
                    }
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        if left_type != ResolvedPayloadType::Integer
                            || right_type != ResolvedPayloadType::Integer
                        {
                            return Err(LowerError {
                                message: "ordering operators require integer operands".to_owned(),
                                span: expr.span,
                            });
                        }
                        Ok(ResolvedPayloadType::Bool)
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        if left_type != ResolvedPayloadType::Bool
                            || right_type != ResolvedPayloadType::Bool
                        {
                            return Err(LowerError {
                                message: "logical 'and'/'or' operators require boolean operands"
                                    .to_owned(),
                                span: expr.span,
                            });
                        }
                        Ok(ResolvedPayloadType::Bool)
                    }
                }
            }
        }
    }
}

pub(super) fn validate_enum_static_semantics(
    program: &SyntaxProgram,
    enums: &EnumEnvironment,
) -> Result<(), LowerError> {
    let environment = StaticEnvironment::collect(program, enums)?;

    for function in &program.functions {
        let signature = environment
            .functions
            .get(&function.name)
            .expect("function signatures are collected before validating bodies");
        let mut root = HashMap::new();
        for (parameter, value_type) in function
            .parameters
            .iter()
            .zip(signature.parameter_types.iter())
        {
            root.insert(parameter.name.clone(), value_type.clone());
        }
        let mut scopes = vec![root];
        validate_statements(
            &function.body,
            &environment,
            &mut scopes,
            Some(&signature.return_type),
        )?;
        if !block_always_returns(&function.body) {
            return Err(LowerError {
                message: format!(
                    "function {:?} must return {} on every terminal path",
                    function.name,
                    type_label(&signature.return_type)
                ),
                span: function.span,
            });
        }
    }

    let mut top_level_scopes = vec![HashMap::new()];
    validate_statements(
        &program.statements,
        &environment,
        &mut top_level_scopes,
        None,
    )?;

    Ok(())
}

fn validate_statements(
    statements: &[SyntaxStmt],
    environment: &StaticEnvironment<'_>,
    scopes: &mut [HashMap<String, ResolvedPayloadType>],
    expected_return: Option<&ResolvedPayloadType>,
) -> Result<(), LowerError> {
    for statement in statements {
        match &statement.kind {
            SyntaxStmtKind::Bind { name, expr } => {
                let actual = environment.infer_expr(expr, scopes)?;
                if let Some(existing) = lookup_local(scopes, name) {
                    if existing != actual {
                        return Err(LowerError {
                            message: format!(
                                "cannot assign a different value type to existing local {name:?}"
                            ),
                            span: statement.span,
                        });
                    }
                } else {
                    scopes
                        .last_mut()
                        .expect("static semantics always has a lexical scope")
                        .insert(name.clone(), actual);
                }
            }
            SyntaxStmtKind::Print(expr) => {
                let actual = environment.infer_expr(expr, scopes)?;
                match actual {
                    ResolvedPayloadType::Record(_) => {
                        return Err(LowerError {
                            message: "printing whole record values is not supported in Records v0"
                                .to_owned(),
                            span: expr.span,
                        });
                    }
                    ResolvedPayloadType::Enum(_) => {
                        return Err(LowerError {
                            message: "printing whole enum values is not supported in Enums v0"
                                .to_owned(),
                            span: expr.span,
                        });
                    }
                    ResolvedPayloadType::Integer
                    | ResolvedPayloadType::Bool
                    | ResolvedPayloadType::String => {}
                }
            }
            SyntaxStmtKind::Return(expr) => {
                let expected = expected_return.ok_or_else(|| LowerError {
                    message: "return is only valid inside a function".to_owned(),
                    span: statement.span,
                })?;
                let actual = environment.infer_expr(expr, scopes)?;
                if &actual != expected {
                    return Err(LowerError {
                        message: format!(
                            "return type mismatch: expected {}, found {}",
                            type_label(expected),
                            type_label(&actual)
                        ),
                        span: statement.span,
                    });
                }
            }
            SyntaxStmtKind::Repeat { count, body } => {
                let count_type = environment.infer_expr(count, scopes)?;
                if count_type != ResolvedPayloadType::Integer {
                    return Err(LowerError {
                        message: "repeat count must be an integer".to_owned(),
                        span: count.span,
                    });
                }
                validate_child_scope(body, environment, scopes, expected_return)?;
            }
            SyntaxStmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition_type = environment.infer_expr(condition, scopes)?;
                if condition_type != ResolvedPayloadType::Bool {
                    return Err(LowerError {
                        message: "if condition must be a boolean".to_owned(),
                        span: condition.span,
                    });
                }
                validate_child_scope(then_body, environment, scopes, expected_return)?;
                validate_child_scope(else_body, environment, scopes, expected_return)?;
            }
            SyntaxStmtKind::Match { value, arms } => {
                validate_match(value, arms, environment, scopes, expected_return)?;
            }
        }
    }
    Ok(())
}

fn validate_match(
    value: &SyntaxExpr,
    arms: &[SyntaxMatchArm],
    environment: &StaticEnvironment<'_>,
    scopes: &[HashMap<String, ResolvedPayloadType>],
    expected_return: Option<&ResolvedPayloadType>,
) -> Result<(), LowerError> {
    let scrutinee_type = environment.infer_expr(value, scopes)?;
    let enum_name = match scrutinee_type {
        ResolvedPayloadType::Enum(name) => name,
        actual => {
            return Err(LowerError {
                message: format!(
                    "match scrutinee must have an enum type; found {}",
                    type_label(&actual)
                ),
                span: value.span,
            });
        }
    };
    let schema = environment
        .enums
        .schema(&enum_name)
        .expect("enum expression types originate from resolved declarations");

    for arm in arms {
        let variant = schema
            .variants
            .iter()
            .find(|candidate| candidate.name == arm.pattern.variant_name)
            .expect("structural match validation resolves every arm before static semantics");
        let mut arm_scopes = scopes.to_vec();
        arm_scopes.push(HashMap::new());
        if let (Some(payload_type), Some(binding)) = (&variant.payload_type, &arm.pattern.binding) {
            arm_scopes
                .last_mut()
                .expect("match arm always has a lexical child scope")
                .insert(binding.clone(), payload_type.clone());
        }
        validate_statements(&arm.body, environment, &mut arm_scopes, expected_return)?;
    }
    Ok(())
}

fn validate_child_scope(
    statements: &[SyntaxStmt],
    environment: &StaticEnvironment<'_>,
    scopes: &[HashMap<String, ResolvedPayloadType>],
    expected_return: Option<&ResolvedPayloadType>,
) -> Result<(), LowerError> {
    let mut child_scopes = scopes.to_vec();
    child_scopes.push(HashMap::new());
    validate_statements(
        statements,
        environment,
        &mut child_scopes,
        expected_return,
    )
}

fn block_always_returns(statements: &[SyntaxStmt]) -> bool {
    statements.iter().any(statement_always_returns)
}

fn statement_always_returns(statement: &SyntaxStmt) -> bool {
    match &statement.kind {
        SyntaxStmtKind::Return(_) => true,
        SyntaxStmtKind::If {
            then_body,
            else_body,
            ..
        } => {
            !else_body.is_empty()
                && block_always_returns(then_body)
                && block_always_returns(else_body)
        }
        SyntaxStmtKind::Match { arms, .. } => {
            !arms.is_empty() && arms.iter().all(|arm| block_always_returns(&arm.body))
        }
        SyntaxStmtKind::Bind { .. }
        | SyntaxStmtKind::Print(_)
        | SyntaxStmtKind::Repeat { .. } => false,
    }
}

fn resolve_type_name(
    type_name: &TypeName,
    record_names: &HashSet<&str>,
    enums: &EnumEnvironment,
    span: Span,
) -> Result<ResolvedPayloadType, LowerError> {
    match type_name {
        TypeName::Int => Ok(ResolvedPayloadType::Integer),
        TypeName::Bool => Ok(ResolvedPayloadType::Bool),
        TypeName::String => Ok(ResolvedPayloadType::String),
        TypeName::Named(name) if record_names.contains(name.as_str()) => {
            Ok(ResolvedPayloadType::Record(name.clone()))
        }
        TypeName::Named(name) if enums.schema(name).is_some() => {
            Ok(ResolvedPayloadType::Enum(name.clone()))
        }
        TypeName::Named(name) => Err(LowerError {
            message: format!("unknown nominal type {name:?} in function signature"),
            span,
        }),
    }
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
    use super::validate_enum_static_semantics;
    use crate::record_environment::enums_impl::collect_enum_environment;
    use evo_lexer::lex;
    use evo_parser::parse;

    fn validate(source: &str) -> Result<(), crate::LowerError> {
        let tokens = lex(source).expect("strict enum semantics source should lex");
        let program = parse(&tokens).expect("strict enum semantics source should parse");
        let enums = collect_enum_environment(&program)?;
        validate_enum_static_semantics(&program, &enums)
    }

    #[test]
    fn validates_function_call_arity_and_argument_types() {
        let arity = validate(
            "enum Flag\nOff\nOn\nend\nfn choose(flag bool) int\nreturn 1\nend\nprint choose(true, false)\n",
        )
        .expect_err("wrong function arity must fail before codegen");
        assert!(arity.message.contains("expects 1 arguments, found 2"));

        let argument = validate(
            "enum Flag\nOff\nOn\nend\nfn choose(flag bool) int\nreturn 1\nend\nprint choose(1)\n",
        )
        .expect_err("wrong argument type must fail before codegen");
        assert!(argument.message.contains("argument 1"));
    }

    #[test]
    fn validates_return_and_terminal_paths() {
        let mismatch = validate("enum Flag\nOff\nOn\nend\nfn bad() int\nreturn true\nend\n")
            .expect_err("wrong return type must fail before codegen");
        assert!(mismatch.message.contains("return type mismatch"));

        let missing = validate(
            "enum Flag\nOff\nOn\nend\nfn bad(flag bool) int\nif flag\nreturn 1\nend\nend\n",
        )
        .expect_err("missing terminal return path must fail before codegen");
        assert!(missing.message.contains("every terminal path"));
    }

    #[test]
    fn validates_control_flow_operand_types() {
        let repeat = validate("enum Flag\nOff\nOn\nend\nrepeat true\nprint 1\nend\n")
            .expect_err("repeat bool must fail before codegen");
        assert!(repeat.message.contains("repeat count"));

        let condition = validate("enum Flag\nOff\nOn\nend\nif 1\nprint 1\nend\n")
            .expect_err("integer if condition must fail before codegen");
        assert!(condition.message.contains("if condition"));
    }

    #[test]
    fn validates_record_constructor_and_field_access() {
        let missing = validate(
            "record Point\nx int\nend\nenum Flag\nOff\nOn\nend\nvalue = Point()\n",
        )
        .expect_err("missing record fields must fail before codegen");
        assert!(missing.message.contains("missing field"));

        let field = validate(
            "record Point\nx int\nend\nenum Flag\nOff\nOn\nend\nvalue = Point(x = 1)\nprint value.y\n",
        )
        .expect_err("unknown record field must fail before codegen");
        assert!(field.message.contains("unknown field"));
    }

    #[test]
    fn validates_operators_and_nominal_prints_before_codegen() {
        let arithmetic = validate("enum Flag\nOff\nOn\nend\nprint true + 1\n")
            .expect_err("bad arithmetic must fail before codegen");
        assert!(arithmetic.message.contains("arithmetic operators"));

        let enum_print = validate(
            "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nprint value\n",
        )
        .expect_err("whole enum print must fail before codegen");
        assert!(enum_print.message.contains("printing whole enum"));
    }

    #[test]
    fn rejects_unknown_functions() {
        let unknown = validate("enum Flag\nOff\nOn\nend\nprint missing()\n")
            .expect_err("unknown function must fail before codegen");
        assert!(unknown.message.contains("unknown function"));
    }

    #[test]
    fn exhaustive_match_can_satisfy_function_return_path() {
        validate(
            "enum Flag\nOff\nOn\nend\nfn choose(value Flag) int\nmatch value\ncase Flag.Off\nreturn 0\ncase Flag.On\nreturn 1\nend\nend\n",
        )
        .expect("exhaustive terminal match should satisfy function return analysis");
    }
}
