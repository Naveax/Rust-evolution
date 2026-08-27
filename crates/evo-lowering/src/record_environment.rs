use crate::LowerError;
use evo_parser::Program as SyntaxProgram;
use std::ops::Deref;

mod enums_impl {
    include!("enum_environment.rs");

    mod ownership_state {
        include!("move_state.rs");
    }

    mod constructor_typing {
        include!("enum_constructor_typing.rs");

        mod ownership {
            include!("enum_ownership.rs");
        }

        pub(super) fn validate_enum_ownership(
            program: &SyntaxProgram,
            enums: &EnumEnvironment,
            matches: &super::match_validation::MatchEnvironment,
        ) -> Result<(), LowerError> {
            ownership::validate_enum_ownership(program, enums, matches)
        }
    }

    mod match_validation {
        include!("enum_match_validation.rs");
    }

    mod match_sidecar {
        include!("enum_match_sidecar.rs");
    }

    mod ir {
        include!("enum_ir.rs");
    }

    pub(crate) fn collect_validated_enum_environment(
        program: &SyntaxProgram,
    ) -> Result<EnumEnvironment, LowerError> {
        validate_enum_declarations(program)?;
        let environment = collect_enum_environment(program)?;
        let matches = match_validation::collect_match_environment(program, &environment)?;
        constructor_typing::validate_enum_type_semantics(program, &environment)?;
        match_sidecar::validate_match_sidecar(program, &matches)?;
        constructor_typing::validate_enum_ownership(program, &environment, &matches)?;

        let schemas = ir::lower_enum_schemas(&environment);
        debug_assert_eq!(schemas.len(), program.enums.len());
        debug_assert!(schemas.iter().all(|schema| {
            environment.schema(&schema.name).is_some_and(|resolved| {
                resolved.span == schema.span && resolved.variants.len() == schema.variants.len()
            })
        }));

        let records = ir::lower_record_schemas(program, &environment);
        debug_assert_eq!(records.len(), program.records.len());

        let lowered_matches = ir::lower_matches(program, &matches);
        debug_assert!(lowered_matches
            .iter()
            .all(|resolved| matches.match_at(resolved.span.start).is_some()));

        Ok(environment)
    }

    pub(crate) fn validate_enum_pre_codegen_semantics(
        program: &SyntaxProgram,
    ) -> Result<(), LowerError> {
        collect_validated_enum_environment(program).map(|_| ())
    }
}

mod records_impl {
    include!("record_environment_records.rs");
}

pub(crate) use records_impl::{
    ConstructorFieldInput, RecordEnvironment as RecordStorage, SemanticType,
};

#[derive(Debug, Clone)]
pub(crate) struct TypeEnvironment {
    records: RecordStorage,
}

// Transitional compatibility name for Records v0 callers. New nominal-type work
// should use TypeEnvironment so enum support can share the same semantic boundary.
pub(crate) type RecordEnvironment = TypeEnvironment;

impl Deref for TypeEnvironment {
    type Target = RecordStorage;

    fn deref(&self) -> &Self::Target {
        &self.records
    }
}

pub(crate) fn collect_record_environment(
    program: &SyntaxProgram,
) -> Result<RecordEnvironment, LowerError> {
    reject_enum_declarations(program)?;
    let records = records_impl::collect_record_environment(program)?;
    Ok(TypeEnvironment { records })
}

pub(crate) fn validate_record_declarations(program: &SyntaxProgram) -> Result<(), LowerError> {
    reject_enum_declarations(program)?;
    records_impl::validate_record_declarations(program)
}

fn reject_enum_declarations(program: &SyntaxProgram) -> Result<(), LowerError> {
    if program.enums.is_empty() {
        return Ok(());
    }

    enums_impl::validate_enum_pre_codegen_semantics(program)?;
    let enum_def = &program.enums[0];
    Err(LowerError {
        message: "enum declarations are parsed, but Enums v0 semantic lowering/codegen is not implemented yet"
            .to_owned(),
        span: enum_def.span,
    })
}

#[cfg(test)]
mod tests {
    use super::{collect_record_environment, validate_record_declarations};
    use evo_lexer::lex;
    use evo_parser::parse;

    fn parse_source(source: &str) -> evo_parser::Program {
        let tokens = lex(source).expect("enum gate source should lex");
        parse(&tokens).expect("enum gate source should parse")
    }

    #[test]
    fn enum_declarations_fail_closed_before_semantic_lowering() {
        let program = parse_source("enum MaybeInt\nNone\nSome int\nend\nprint 1\n");

        let validation_error = validate_record_declarations(&program)
            .expect_err("enum declaration should remain fail-closed");
        assert!(
            validation_error
                .message
                .contains("Enums v0 semantic lowering")
        );
        assert_eq!(validation_error.span.line, 1);

        let collection_error = collect_record_environment(&program)
            .expect_err("enum declaration should not disappear during environment collection");
        assert!(
            collection_error
                .message
                .contains("Enums v0 semantic lowering")
        );
        assert_eq!(collection_error.span.line, 1);
    }

    #[test]
    fn invalid_enum_declarations_are_diagnosed_before_the_fail_closed_gate() {
        let program = parse_source("enum Flag\nOn\nOn\nend\n");
        let error = validate_record_declarations(&program)
            .expect_err("duplicate variants should fail before unsupported enum execution");
        assert!(error.message.contains("duplicate variant name"));
        assert_eq!(error.span.line, 3);
    }

    #[test]
    fn constructor_payload_type_errors_are_diagnosed_before_fail_closed_gate() {
        let program = parse_source(
            "enum MaybeInt\nNone\nSome int\nend\nvalue = true\nwrapped = MaybeInt.Some(value)\n",
        );
        let error = validate_record_declarations(&program)
            .expect_err("constructor payload mismatch should precede unsupported codegen gate");
        assert!(error.message.contains("expects int, found bool"));
        assert_eq!(error.span.line, 6);
    }

    #[test]
    fn match_pattern_errors_are_diagnosed_before_fail_closed_gate() {
        let program = parse_source(
            "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nmatch value\ncase Flag.On\nprint 1\nend\n",
        );
        let error = validate_record_declarations(&program)
            .expect_err("non-exhaustive match should precede unsupported codegen gate");
        assert!(error.message.contains("missing variant(s): Off"));
        assert_eq!(error.span.line, 6);
    }

    #[test]
    fn match_scrutinee_type_errors_are_diagnosed_before_fail_closed_gate() {
        let program = parse_source(
            "enum Flag\nOff\nOn\nend\nmatch true\ncase Flag.Off\nprint 0\ncase Flag.On\nprint 1\nend\n",
        );
        let error = validate_record_declarations(&program)
            .expect_err("non-enum scrutinee should precede unsupported codegen gate");
        assert!(error.message.contains("scrutinee must have an enum type"));
        assert_eq!(error.span.line, 5);
    }

    #[test]
    fn enum_ownership_errors_are_diagnosed_before_fail_closed_gate() {
        let program = parse_source(
            "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nfirst = value\nsecond = value\n",
        );
        let error = validate_record_declarations(&program)
            .expect_err("enum reuse-after-move should precede unsupported codegen gate");
        assert!(error.message.contains("moved enum local \"value\""));
        assert_eq!(error.span.line, 7);
    }
}
