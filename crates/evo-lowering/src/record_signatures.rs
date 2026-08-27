use crate::{
    LowerError,
    record_environment::{RecordEnvironment, SemanticType},
};
use evo_parser::Program as SyntaxProgram;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SemanticFunctionSignature {
    pub(crate) parameter_types: Vec<SemanticType>,
    pub(crate) return_type: SemanticType,
}

pub(crate) fn collect_semantic_function_signatures(
    program: &SyntaxProgram,
    records: &RecordEnvironment,
) -> Result<HashMap<String, SemanticFunctionSignature>, LowerError> {
    let mut signatures = HashMap::new();

    for function in &program.functions {
        if signatures.contains_key(&function.name) {
            return Err(LowerError {
                message: format!("duplicate function name {:?}", function.name),
                span: function.span,
            });
        }

        let mut seen_parameters = HashSet::new();
        let mut parameter_types = Vec::with_capacity(function.parameters.len());
        for parameter in &function.parameters {
            if !seen_parameters.insert(parameter.name.clone()) {
                return Err(LowerError {
                    message: format!("duplicate parameter name {:?}", parameter.name),
                    span: parameter.span,
                });
            }
            parameter_types.push(records.resolve_type_name(&parameter.type_name, parameter.span)?);
        }

        let return_type = records.resolve_type_name(&function.return_type, function.span)?;
        signatures.insert(
            function.name.clone(),
            SemanticFunctionSignature {
                parameter_types,
                return_type,
            },
        );
    }

    Ok(signatures)
}

#[cfg(test)]
mod tests {
    use super::collect_semantic_function_signatures;
    use crate::record_environment::{SemanticType, collect_record_environment};
    use evo_lexer::lex;
    use evo_parser::parse;

    fn parse_source(source: &str) -> evo_parser::Program {
        let tokens = lex(source).expect("signature source should lex");
        parse(&tokens).expect("signature source should parse")
    }

    #[test]
    fn accepts_record_parameter_and_return_types() {
        let program = parse_source(
            "record Point\nx int\nend\nfn identity(point Point) Point\nreturn point\nend\n",
        );
        let records =
            collect_record_environment(&program).expect("record environment should build");
        let signatures = collect_semantic_function_signatures(&program, &records)
            .expect("record function signature should resolve");
        let identity = signatures.get("identity").expect("identity signature");
        assert_eq!(
            identity.parameter_types,
            vec![SemanticType::Record("Point".to_owned())]
        );
        assert_eq!(
            identity.return_type,
            SemanticType::Record("Point".to_owned())
        );
    }

    #[test]
    fn collection_is_order_independent_for_forward_calls() {
        let program = parse_source(
            "print second(1)\nfn second(value int) int\nreturn first(value)\nend\nfn first(value int) int\nreturn value\nend\n",
        );
        let records =
            collect_record_environment(&program).expect("empty record environment should build");
        let signatures = collect_semantic_function_signatures(&program, &records)
            .expect("forward function signatures should collect");
        assert!(signatures.contains_key("first"));
        assert!(signatures.contains_key("second"));
    }

    #[test]
    fn rejects_unknown_record_type_in_signature() {
        let program = parse_source("fn bad(value Missing) int\nreturn 1\nend\n");
        let records =
            collect_record_environment(&program).expect("empty record environment should build");
        let error = collect_semantic_function_signatures(&program, &records)
            .expect_err("unknown record signature type should fail");
        assert!(error.message.contains("unknown record type"));
        assert_eq!(error.span.line, 1);
    }
}
