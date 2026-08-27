use crate::{
    LowerError,
    record_environment::{ConstructorFieldInput, RecordEnvironment, SemanticType},
};
use evo_lexer::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CallNameResolution {
    Function,
    ZeroFieldRecordConstructor,
}

pub(crate) fn resolve_call_name(
    records: &RecordEnvironment,
    name: &str,
    positional_argument_count: usize,
    span: Span,
) -> Result<CallNameResolution, LowerError> {
    let Some(schema) = records.schema(name) else {
        return Ok(CallNameResolution::Function);
    };

    if positional_argument_count != 0 {
        return Err(LowerError {
            message: format!(
                "record constructor {name:?} requires named fields; positional arguments are not supported"
            ),
            span,
        });
    }

    if schema.fields.is_empty() {
        return Ok(CallNameResolution::ZeroFieldRecordConstructor);
    }

    let no_fields: Vec<ConstructorFieldInput> = Vec::new();
    let _: SemanticType = records.validate_constructor(name, &no_fields, span)?;
    unreachable!("non-empty record constructor with no fields must report missing fields")
}

#[cfg(test)]
mod tests {
    use super::{CallNameResolution, resolve_call_name};
    use crate::record_environment::collect_record_environment;
    use evo_lexer::{Span, lex};
    use evo_parser::parse;

    fn span() -> Span {
        Span {
            start: 0,
            end: 1,
            line: 1,
            column: 1,
        }
    }

    fn environment(source: &str) -> crate::record_environment::RecordEnvironment {
        let tokens = lex(source).expect("resolution source should lex");
        let program = parse(&tokens).expect("resolution source should parse");
        collect_record_environment(&program).expect("record environment should build")
    }

    #[test]
    fn zero_field_record_call_resolves_to_constructor() {
        let records = environment("record Marker\nend\n");
        assert_eq!(
            resolve_call_name(&records, "Marker", 0, span()).expect("Marker() should resolve"),
            CallNameResolution::ZeroFieldRecordConstructor
        );
    }

    #[test]
    fn nonempty_record_call_without_named_fields_reports_missing_fields() {
        let records = environment("record Point\nx int\nend\n");
        let error = resolve_call_name(&records, "Point", 0, span())
            .expect_err("Point() must not silently become a function call");
        assert!(error.message.contains("missing field"));
    }

    #[test]
    fn positional_record_construction_is_rejected() {
        let records = environment("record Point\nx int\nend\n");
        let error = resolve_call_name(&records, "Point", 1, span())
            .expect_err("record positional construction is not v0 syntax");
        assert!(error.message.contains("requires named fields"));
    }

    #[test]
    fn nonrecord_name_continues_to_function_resolution() {
        let records = environment("");
        assert_eq!(
            resolve_call_name(&records, "compute", 0, span()).expect("function name should pass"),
            CallNameResolution::Function
        );
    }
}
