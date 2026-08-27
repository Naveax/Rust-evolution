use crate::LowerError;
use evo_lexer::Span;
use evo_parser::{Program as SyntaxProgram, TypeName as SyntaxTypeName};

mod enums_impl {
    include!("enum_environment.rs");
}

mod records_impl {
    include!("record_environment_records.rs");
}

pub(crate) use records_impl::{ConstructorFieldInput, RecordSchema, SemanticType};

#[derive(Debug, Clone)]
pub(crate) struct TypeEnvironment {
    records: records_impl::RecordEnvironment,
}

// Transitional compatibility name for Records v0 callers. New nominal-type work
// should use TypeEnvironment so enum support can share the same semantic boundary.
pub(crate) type RecordEnvironment = TypeEnvironment;

impl TypeEnvironment {
    pub(crate) fn resolve_type_name(
        &self,
        type_name: &SyntaxTypeName,
        span: Span,
    ) -> Result<SemanticType, LowerError> {
        self.records.resolve_type_name(type_name, span)
    }

    pub(crate) fn validate_constructor(
        &self,
        name: &str,
        fields: &[ConstructorFieldInput],
        constructor_span: Span,
    ) -> Result<SemanticType, LowerError> {
        self.records
            .validate_constructor(name, fields, constructor_span)
    }

    pub(crate) fn field_type(
        &self,
        base_type: &SemanticType,
        field_name: &str,
        access_span: Span,
    ) -> Result<SemanticType, LowerError> {
        self.records.field_type(base_type, field_name, access_span)
    }

    #[must_use]
    pub(crate) fn schema(&self, name: &str) -> Option<&RecordSchema> {
        self.records.schema(name)
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

    enums_impl::validate_enum_declarations(program)?;
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
}
