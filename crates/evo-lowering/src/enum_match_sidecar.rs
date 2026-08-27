use crate::LowerError;
use evo_lexer::Span;
use evo_parser::{Program as SyntaxProgram, Stmt as SyntaxStmt, StmtKind as SyntaxStmtKind};

use super::{ResolvedPayloadType, match_validation::MatchEnvironment};

pub(super) fn validate_match_sidecar(
    program: &SyntaxProgram,
    matches: &MatchEnvironment,
) -> Result<(), LowerError> {
    for function in &program.functions {
        validate_statements(&function.body, matches)?;
    }
    validate_statements(&program.statements, matches)
}

fn validate_statements(
    statements: &[SyntaxStmt],
    matches: &MatchEnvironment,
) -> Result<(), LowerError> {
    for statement in statements {
        match &statement.kind {
            SyntaxStmtKind::Match { arms, .. } => {
                let resolved = matches.match_at(statement.span.start).ok_or_else(|| {
                    sidecar_error("missing resolved match entry", statement.span)
                })?;
                if resolved.span != statement.span || resolved.arms.len() != arms.len() {
                    return Err(sidecar_error(
                        "statement span or arm count differs from parser structure",
                        statement.span,
                    ));
                }

                for (arm, resolved_arm) in arms.iter().zip(&resolved.arms) {
                    if resolved_arm.enum_name != resolved.enum_name
                        || resolved_arm.enum_name != arm.pattern.enum_name
                        || resolved_arm.variant_name != arm.pattern.variant_name
                        || resolved_arm.span != arm.pattern.span
                    {
                        return Err(sidecar_error(
                            "resolved arm identity differs from parser pattern",
                            arm.pattern.span,
                        ));
                    }

                    match (&arm.pattern.binding, &resolved_arm.binding) {
                        (None, None) => {}
                        (Some(name), Some(binding))
                            if binding.name == *name
                                && binding.span == arm.pattern.span
                                && valid_payload_type(&binding.value_type) => {}
                        _ => {
                            return Err(sidecar_error(
                                "resolved payload binding differs from parser pattern",
                                arm.pattern.span,
                            ));
                        }
                    }
                    validate_statements(&arm.body, matches)?;
                }

                // This bit is deliberately retained only after structural exhaustiveness
                // succeeded in collect_match_environment. Later return-path lowering can
                // consume it without inferring terminality from the mere presence of match.
                let _all_arms_return = resolved.all_arms_return;
            }
            SyntaxStmtKind::Repeat { body, .. } => validate_statements(body, matches)?,
            SyntaxStmtKind::If {
                then_body,
                else_body,
                ..
            } => {
                validate_statements(then_body, matches)?;
                validate_statements(else_body, matches)?;
            }
            SyntaxStmtKind::Bind { .. }
            | SyntaxStmtKind::Print(_)
            | SyntaxStmtKind::Return(_) => {}
        }
    }
    Ok(())
}

fn valid_payload_type(value_type: &ResolvedPayloadType) -> bool {
    match value_type {
        ResolvedPayloadType::Integer
        | ResolvedPayloadType::Bool
        | ResolvedPayloadType::String => true,
        ResolvedPayloadType::Record(name) | ResolvedPayloadType::Enum(name) => !name.is_empty(),
    }
}

fn sidecar_error(message: &str, span: Span) -> LowerError {
    LowerError {
        message: format!("internal Enums v0 match semantic sidecar mismatch: {message}"),
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::validate_match_sidecar;
    use crate::record_environment::enums_impl::{
        collect_enum_environment, match_validation::collect_match_environment,
    };
    use evo_lexer::lex;
    use evo_parser::parse;

    #[test]
    fn resolved_match_sidecar_stays_aligned_with_parser_structure() {
        let source = "enum MaybeInt\nNone\nSome int\nend\nvalue = MaybeInt.Some(1)\nmatch value\ncase MaybeInt.None\nprint 0\ncase MaybeInt.Some(x)\nprint x\nend\n";
        let tokens = lex(source).expect("source should lex");
        let program = parse(&tokens).expect("source should parse");
        let enums = collect_enum_environment(&program).expect("enum environment should resolve");
        let matches = collect_match_environment(&program, &enums)
            .expect("match environment should resolve");
        validate_match_sidecar(&program, &matches)
            .expect("resolved match metadata should mirror parser structure");
    }
}
