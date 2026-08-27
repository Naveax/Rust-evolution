use crate::LowerError;
use evo_lexer::Span;
use evo_parser::{MatchArm as SyntaxMatchArm, Program as SyntaxProgram, Stmt as SyntaxStmt, StmtKind as SyntaxStmtKind};
use std::collections::HashSet;

use super::EnumEnvironment;

pub(super) fn validate_match_patterns(
    program: &SyntaxProgram,
    enums: &EnumEnvironment,
) -> Result<(), LowerError> {
    for function in &program.functions {
        validate_statements(&function.body, enums)?;
    }
    validate_statements(&program.statements, enums)
}

fn validate_statements(
    statements: &[SyntaxStmt],
    enums: &EnumEnvironment,
) -> Result<(), LowerError> {
    for statement in statements {
        match &statement.kind {
            SyntaxStmtKind::Match { arms, .. } => {
                validate_match(statement.span, arms, enums)?;
                for arm in arms {
                    validate_statements(&arm.body, enums)?;
                }
            }
            SyntaxStmtKind::Repeat { body, .. } => validate_statements(body, enums)?,
            SyntaxStmtKind::If {
                then_body,
                else_body,
                ..
            } => {
                validate_statements(then_body, enums)?;
                validate_statements(else_body, enums)?;
            }
            SyntaxStmtKind::Bind { .. }
            | SyntaxStmtKind::Print(_)
            | SyntaxStmtKind::Return(_) => {}
        }
    }
    Ok(())
}

fn validate_match(
    match_span: Span,
    arms: &[SyntaxMatchArm],
    enums: &EnumEnvironment,
) -> Result<(), LowerError> {
    let first = arms
        .first()
        .expect("parser guarantees at least one arm for every match statement");
    let enum_name = first.pattern.enum_name.as_str();
    let schema = enums.schema(enum_name).ok_or_else(|| LowerError {
        message: format!("unknown enum {:?} in match pattern", first.pattern.enum_name),
        span: first.pattern.span,
    })?;

    let mut seen = HashSet::new();
    for arm in arms {
        if arm.pattern.enum_name != enum_name {
            return Err(LowerError {
                message: format!(
                    "match arms must use enum {enum_name:?}; found qualifier {:?}",
                    arm.pattern.enum_name
                ),
                span: arm.pattern.span,
            });
        }

        let variant = schema
            .variants
            .iter()
            .find(|candidate| candidate.name == arm.pattern.variant_name)
            .ok_or_else(|| LowerError {
                message: format!(
                    "unknown variant {:?} for enum {enum_name:?} in match pattern",
                    arm.pattern.variant_name
                ),
                span: arm.pattern.span,
            })?;

        if !seen.insert(variant.name.as_str()) {
            return Err(LowerError {
                message: format!(
                    "duplicate match arm for variant {:?} of enum {enum_name:?}",
                    variant.name
                ),
                span: arm.pattern.span,
            });
        }

        match (&variant.payload_type, &arm.pattern.binding) {
            (None, Some(_)) => {
                return Err(LowerError {
                    message: format!(
                        "unit variant {enum_name:?}.{:?} cannot bind a payload",
                        variant.name
                    ),
                    span: arm.pattern.span,
                });
            }
            (Some(_), None) => {
                return Err(LowerError {
                    message: format!(
                        "payload variant {enum_name:?}.{:?} requires one payload binding",
                        variant.name
                    ),
                    span: arm.pattern.span,
                });
            }
            (None, None) | (Some(_), Some(_)) => {}
        }
    }

    let missing: Vec<&str> = schema
        .variants
        .iter()
        .filter(|variant| !seen.contains(variant.name.as_str()))
        .map(|variant| variant.name.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(LowerError {
            message: format!(
                "non-exhaustive match for enum {enum_name:?}; missing variant(s): {}",
                missing.join(", ")
            ),
            span: match_span,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_match_patterns;
    use crate::record_environment::enums_impl::collect_enum_environment;
    use evo_lexer::lex;
    use evo_parser::parse;

    fn validate(source: &str) -> Result<(), crate::LowerError> {
        let tokens = lex(source).expect("match validation source should lex");
        let program = parse(&tokens).expect("match validation source should parse");
        let enums = collect_enum_environment(&program)?;
        validate_match_patterns(&program, &enums)
    }

    #[test]
    fn accepts_exhaustive_unit_and_payload_arms() {
        validate(
            "enum MaybeInt\nNone\nSome int\nend\nvalue = MaybeInt.None()\nmatch value\ncase MaybeInt.Some(x)\nprint x\ncase MaybeInt.None\nprint 0\nend\n",
        )
        .expect("all declared variants should validate exactly once");
    }

    #[test]
    fn rejects_unknown_match_enum() {
        let error = validate(
            "enum Flag\nOff\nOn\nend\nmatch Flag.On()\ncase Missing.On\nprint 1\nend\n",
        )
        .expect_err("unknown enum qualifier should fail");
        assert!(error.message.contains("unknown enum"));
        assert_eq!(error.span.line, 6);
    }

    #[test]
    fn rejects_wrong_enum_qualifier_across_arms() {
        let error = validate(
            "enum Left\nOne\nTwo\nend\nenum Right\nOther\nend\nmatch Left.One()\ncase Left.One\nprint 1\ncase Right.Other\nprint 2\nend\n",
        )
        .expect_err("one match must not mix enum qualifiers");
        assert!(error.message.contains("must use enum \"Left\""));
        assert_eq!(error.span.line, 11);
    }

    #[test]
    fn rejects_unknown_variant_in_match_arm() {
        let error = validate(
            "enum Flag\nOff\nOn\nend\nmatch Flag.On()\ncase Flag.Missing\nprint 1\nend\n",
        )
        .expect_err("unknown variant should fail at its pattern");
        assert!(error.message.contains("unknown variant"));
        assert_eq!(error.span.line, 6);
    }

    #[test]
    fn rejects_duplicate_variant_arms() {
        let error = validate(
            "enum Flag\nOff\nOn\nend\nmatch Flag.On()\ncase Flag.On\nprint 1\ncase Flag.On\nprint 2\ncase Flag.Off\nprint 0\nend\n",
        )
        .expect_err("duplicate variant arms should fail");
        assert!(error.message.contains("duplicate match arm"));
        assert_eq!(error.span.line, 8);
    }

    #[test]
    fn reports_missing_variants_in_declaration_order() {
        let error = validate(
            "enum State\nFirst\nSecond\nThird\nend\nmatch State.First()\ncase State.First\nprint 1\nend\n",
        )
        .expect_err("non-exhaustive match should fail");
        assert!(error.message.contains("Second, Third"));
        assert_eq!(error.span.line, 6);
    }

    #[test]
    fn unit_variant_rejects_payload_binding() {
        let error = validate(
            "enum Flag\nOff\nOn\nend\nmatch Flag.On()\ncase Flag.On(value)\nprint value\ncase Flag.Off\nprint 0\nend\n",
        )
        .expect_err("unit variants must not bind payloads");
        assert!(error.message.contains("cannot bind a payload"));
        assert_eq!(error.span.line, 6);
    }

    #[test]
    fn payload_variant_requires_binding() {
        let error = validate(
            "enum MaybeInt\nNone\nSome int\nend\nmatch MaybeInt.None()\ncase MaybeInt.Some\nprint 1\ncase MaybeInt.None\nprint 0\nend\n",
        )
        .expect_err("payload variants must bind their payload");
        assert!(error.message.contains("requires one payload binding"));
        assert_eq!(error.span.line, 6);
    }

    #[test]
    fn validates_nested_match_patterns_recursively() {
        let error = validate(
            "enum Flag\nOff\nOn\nend\nmatch Flag.On()\ncase Flag.On\nmatch Flag.Off()\ncase Flag.Off\nprint 0\nend\ncase Flag.Off\nprint 1\nend\n",
        )
        .expect_err("nested non-exhaustive match should be validated");
        assert!(error.message.contains("missing variant(s): On"));
        assert_eq!(error.span.line, 7);
    }
}
