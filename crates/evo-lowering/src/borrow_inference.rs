use crate::ParameterPassingMode;
use evo_parser::{
    Expr as SyntaxExpr, ExprKind as SyntaxExprKind, FunctionDef as SyntaxFunction,
    Program as SyntaxProgram, Stmt as SyntaxStmt, StmtKind as SyntaxStmtKind, TypeName,
};

#[derive(Debug, Default)]
struct ParameterEffects {
    inspect_uses: usize,
    consume_uses: usize,
    reinitialized: bool,
}

pub(crate) fn classify_function_parameters(
    program: &SyntaxProgram,
    function: &SyntaxFunction,
) -> Vec<ParameterPassingMode> {
    function
        .parameters
        .iter()
        .map(|parameter| {
            if !parameter_is_nominal(program, &parameter.type_name) {
                return ParameterPassingMode::Owned;
            }

            let mut effects = ParameterEffects::default();
            collect_statement_effects(&function.body, &parameter.name, &mut effects);
            if !effects.reinitialized && effects.consume_uses == 0 && effects.inspect_uses > 0 {
                ParameterPassingMode::SharedBorrow
            } else {
                ParameterPassingMode::Owned
            }
        })
        .collect()
}

fn parameter_is_nominal(program: &SyntaxProgram, type_name: &TypeName) -> bool {
    let TypeName::Named(type_name) = type_name else {
        return false;
    };
    program
        .records
        .iter()
        .any(|record| record.name == *type_name)
        || program
            .enums
            .iter()
            .any(|enum_def| enum_def.name == *type_name)
}

fn collect_statement_effects(
    statements: &[SyntaxStmt],
    parameter: &str,
    effects: &mut ParameterEffects,
) {
    for statement in statements {
        match &statement.kind {
            SyntaxStmtKind::Bind { name, expr } => {
                if name == parameter {
                    effects.reinitialized = true;
                }
                collect_expr_effects(expr, parameter, UseMode::Consume, effects);
            }
            SyntaxStmtKind::Print(expr) => {
                collect_expr_effects(expr, parameter, UseMode::Inspect, effects);
            }
            SyntaxStmtKind::Return(expr) => {
                collect_expr_effects(expr, parameter, UseMode::Consume, effects);
            }
            SyntaxStmtKind::Repeat { count, body } => {
                collect_expr_effects(count, parameter, UseMode::Inspect, effects);
                collect_statement_effects(body, parameter, effects);
            }
            SyntaxStmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                collect_expr_effects(condition, parameter, UseMode::Inspect, effects);
                collect_statement_effects(then_body, parameter, effects);
                collect_statement_effects(else_body, parameter, effects);
            }
            SyntaxStmtKind::Match { value, arms } => {
                collect_expr_effects(value, parameter, UseMode::Consume, effects);
                for arm in arms {
                    collect_statement_effects(&arm.body, parameter, effects);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum UseMode {
    Inspect,
    Consume,
}

fn collect_expr_effects(
    expr: &SyntaxExpr,
    parameter: &str,
    mode: UseMode,
    effects: &mut ParameterEffects,
) {
    match &expr.kind {
        SyntaxExprKind::Integer(_)
        | SyntaxExprKind::String(_)
        | SyntaxExprKind::Bool(_)
        | SyntaxExprKind::InputInt => {}
        SyntaxExprKind::Identifier(name) => {
            if name == parameter {
                match mode {
                    UseMode::Inspect => effects.inspect_uses += 1,
                    UseMode::Consume => effects.consume_uses += 1,
                }
            }
        }
        SyntaxExprKind::Call { arguments, .. } => {
            // Deliberately non-transitive in v0. A nested call is a consuming boundary while
            // classifying the caller, regardless of the callee's independently inferred mode.
            for argument in arguments {
                collect_expr_effects(argument, parameter, UseMode::Consume, effects);
            }
        }
        SyntaxExprKind::Construct { fields, .. } => {
            for field in fields {
                collect_expr_effects(&field.value, parameter, UseMode::Consume, effects);
            }
        }
        SyntaxExprKind::EnumConstruct { arguments, .. } => {
            for argument in arguments {
                collect_expr_effects(argument, parameter, UseMode::Consume, effects);
            }
        }
        SyntaxExprKind::FieldAccess { base, .. } => {
            collect_expr_effects(base, parameter, UseMode::Inspect, effects);
        }
        SyntaxExprKind::LogicalNot(inner) | SyntaxExprKind::UnaryMinus(inner) => {
            collect_expr_effects(inner, parameter, UseMode::Consume, effects);
        }
        SyntaxExprKind::Binary { left, right, .. } => {
            collect_expr_effects(left, parameter, UseMode::Consume, effects);
            collect_expr_effects(right, parameter, UseMode::Consume, effects);
        }
    }
}
