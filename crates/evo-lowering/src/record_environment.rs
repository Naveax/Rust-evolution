use crate::LowerError;
use evo_parser::Program as SyntaxProgram;

mod records_impl {
    include!("record_environment_records.rs");
}

pub(crate) use records_impl::{ConstructorFieldInput, RecordEnvironment, SemanticType};

pub(crate) fn collect_record_environment(
    program: &SyntaxProgram,
) -> Result<RecordEnvironment, LowerError> {
    reject_enum_declarations(program)?;
    records_impl::collect_record_environment(program)
}

pub(crate) fn validate_record_declarations(program: &SyntaxProgram) -> Result<(), LowerError> {
    reject_enum_declarations(program)?;
    records_impl::validate_record_declarations(program)
}

fn reject_enum_declarations(program: &SyntaxProgram) -> Result<(), LowerError> {
    let Some(enum_def) = program.enums.first() else {
        return Ok(());
    };

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
}
