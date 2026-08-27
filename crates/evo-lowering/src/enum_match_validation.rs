use crate::LowerError;
use evo_lexer::Span;
use evo_parser::{
    MatchArm as SyntaxMatchArm, Program as SyntaxProgram, Stmt as SyntaxStmt,
    StmtKind as SyntaxStmtKind,
};
use std::collections::{HashMap, HashSet};

use super::{EnumEnvironment, ResolvedPayloadType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedMatchBinding {
    pub(super) name: String,
    pub(super) value_type: ResolvedPayloadType,
    pub(super) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedMatchArm {
    pub(super) enum_name: String,
    pub(super) variant_name: String,
    pub(super) binding: Option<ResolvedMatchBinding>,
    pub(super) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedMatch {
    pub(super) enum_name: String,
    pub(super) arms: Vec<ResolvedMatchArm>,
    pub(super) span: Span,
    pub(super) all_arms_return: bool,
}

#[derive(Debug, Clone, Default)]
pub(super) struct MatchEnvironment {
    matches: Vec<ResolvedMatch>,
    indices: HashMap<usize, usize>,
}

impl MatchEnvironment {
    pub(super) fn match_at(&self, statement_start: usize) -> Option<&ResolvedMatch> {
        self.indices
            .get(&statement_start)
            .map(|index| &self.matches[*index])
    }
}

pub(super) fn collect_match_environment(
    program: &SyntaxProgram,
    enums: &EnumEnvironment,
) -> Result<MatchEnvironment, LowerError> {
    let mut environment = MatchEnvironment::default();
    for function in &program.functions {
        collect_statements(&function.body, enums, &mut environment)?;
    }
    collect_statements(&program.statements, enums, &mut environment)?;
    Ok(environment)
}

fn collect_statements(
    statements: &[SyntaxStmt],
    enums: &EnumEnvironment,
    environment: &mut MatchEnvironment,
) -> Result<(), LowerError> {
    for statement in statements {
        match &statement.kind {
            SyntaxStmtKind::Match { arms, .. } => {
                let (enum_name, resolved_arms) = resolve_match_arms(statement.span, arms, enums)?;
                for arm in arms {
                    collect_statements(&arm.body, enums, environment)?;
                }
                let all_arms_return = arms
                    .iter()
                    .all(|arm| block_always_returns(&arm.body, environment));
                let index = environment.matches.len();
                let previous = environment.indices.insert(statement.span.start, index);
                debug_assert!(previous.is_none());
                environment.matches.push(ResolvedMatch {
                    enum_name,
                    arms: resolved_arms,
                    span: statement.span,
                    all_arms_return,
                });
            }
            SyntaxStmtKind::Repeat { body, .. } => collect_statements(body, enums, environment)?,
            SyntaxStmtKind::If {
                then_body,
                else_body,
                ..
            } => {
                collect_statements(then_body, enums, environment)?;
                collect_statements(else_body, enums, environment)?;
            }
            SyntaxStmtKind::Bind { .. }
            | SyntaxStmtKind::Print(_)
            | SyntaxStmtKind::Return(_) => {}
        }
    }
    Ok(())
}

fn resolve_match_arms(
    match_span: Span,
    arms: &[SyntaxMatchArm],
    enums: &EnumEnvironment,
) -> Result<(String, Vec<ResolvedMatchArm>), LowerError> {
    let first = arms
        .first()
        .expect("parser guarantees at least one arm for every match statement");
    let enum_name = first.pattern.enum_name.as_str();
    let schema = enums.schema(enum_name).ok_or_else(|| LowerError {
        message: format!("unknown enum {:?} in match pattern", first.pattern.enum_name),
        span: first.pattern.span,
    })?;

    let mut seen = HashSet::new();
    let mut resolved_arms = Vec::with_capacity(arms.len());
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

        let binding = match (&variant.payload_type, &arm.pattern.binding) {
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
            (Some(value_type), Some(name)) => Some(ResolvedMatchBinding {
                name: name.clone(),
                value_type: value_type.clone(),
                span: arm.pattern.span,
            }),
            (None, None) => None,
        };

        resolved_arms.push(ResolvedMatchArm {
            enum_name: enum_name.to_owned(),
            variant_name: variant.name.clone(),
            binding,
            span: arm.pattern.span,
        });
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

    Ok((enum_name.to_owned(), resolved_arms))
}

fn block_always_returns(statements: &[SyntaxStmt], environment: &MatchEnvironment) -> bool {
    statements
        .iter()
        .any(|statement| statement_always_returns(statement, environment))
}

fn statement_always_returns(statement: &SyntaxStmt, environment: &MatchEnvironment) -> bool {
    match &statement.kind {
        SyntaxStmtKind::Return(_) => true,
        SyntaxStmtKind::If {
            then_body,
            else_body,
            ..
        } => {
            !else_body.is_empty()
                && block_always_returns(then_body, environment)
                && block_always_returns(else_body, environment)
        }
        SyntaxStmtKind::Match { .. } => environment
            .match_at(statement.span.start)
            .is_some_and(|resolved| resolved.all_arms_return),
        SyntaxStmtKind::Bind { .. }
        | SyntaxStmtKind::Print(_)
        | SyntaxStmtKind::Repeat { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::collect_match_environment;
    use crate::record_environment::enums_impl::{
        ResolvedPayloadType, collect_enum_environment,
    };
    use evo_lexer::lex;
    use evo_parser::parse;

    fn collect(source: &str) -> Result<super::MatchEnvironment, crate::LowerError> {
        let tokens = lex(source).expect("match validation source should lex");
        let program = parse(&tokens).expect("match validation source should parse");
        let enums = collect_enum_environment(&program)?;
        collect_match_environment(&program, &enums)
    }

    fn validate(source: &str) -> Result<(), crate::LowerError> {
        collect(source).map(|_| ())
    }

    #[test]
    fn retains_structured_arm_identity_binding_type_and_spans() {
        let source =
            "enum MaybeInt\nNone\nSome int\nend\nvalue = MaybeInt.None()\nmatch value\ncase MaybeInt.Some(x)\nprint x\ncase MaybeInt.None\nprint 0\nend\n";
        let tokens = lex(source).expect("source should lex");
        let program = parse(&tokens).expect("source should parse");
        let enums = collect_enum_environment(&program).expect("enum environment should resolve");
        let environment =
            collect_match_environment(&program, &enums).expect("match environment should resolve");
        let statement = &program.statements[1];
        let resolved = environment
            .match_at(statement.span.start)
            .expect("resolved match should be indexed by statement span");
        assert_eq!(resolved.enum_name, "MaybeInt");
        assert_eq!(resolved.span, statement.span);
        assert_eq!(resolved.arms[0].enum_name, "MaybeInt");
        assert_eq!(resolved.arms[0].variant_name, "Some");
        let binding = resolved.arms[0]
            .binding
            .as_ref()
            .expect("payload arm should retain typed binding metadata");
        assert_eq!(binding.name, "x");
        assert_eq!(binding.value_type, ResolvedPayloadType::Integer);
        assert_eq!(binding.span.line, 7);
        assert_eq!(resolved.arms[1].variant_name, "None");
        assert!(resolved.arms[1].binding.is_none());
    }

    #[test]
    fn marks_match_returning_only_after_exhaustive_arms_validate() {
        let source = "enum Flag\nOff\nOn\nend\nfn choose(value Flag) int\nmatch value\ncase Flag.Off\nreturn 0\ncase Flag.On\nreturn 1\nend\nend\n";
        let tokens = lex(source).expect("source should lex");
        let program = parse(&tokens).expect("source should parse");
        let enums = collect_enum_environment(&program).expect("enum environment should resolve");
        let environment =
            collect_match_environment(&program, &enums).expect("match environment should resolve");
        let statement = &program.functions[0].body[0];
        assert!(
            environment
                .match_at(statement.span.start)
                .expect("resolved match should exist")
                .all_arms_return
        );
    }

    #[test]
    fn non_returning_arm_keeps_match_non_terminal() {
        let source = "enum Flag\nOff\nOn\nend\nfn choose(value Flag) int\nmatch value\ncase Flag.Off\nreturn 0\ncase Flag.On\nprint 1\nend\nreturn 2\nend\n";
        let tokens = lex(source).expect("source should lex");
        let program = parse(&tokens).expect("source should parse");
        let enums = collect_enum_environment(&program).expect("enum environment should resolve");
        let environment =
            collect_match_environment(&program, &enums).expect("match environment should resolve");
        let statement = &program.functions[0].body[0];
        assert!(
            !environment
                .match_at(statement.span.start)
                .expect("resolved match should exist")
                .all_arms_return
        );
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
