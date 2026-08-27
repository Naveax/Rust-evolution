use crate::LowerError;
use evo_lexer::Span;
use evo_parser::Program as SyntaxProgram;
use std::collections::{HashMap, HashSet};

pub(crate) fn validate_enum_declarations(program: &SyntaxProgram) -> Result<(), LowerError> {
    let enum_names = collect_enum_names(program)?;
    reject_record_collisions(program, &enum_names)?;
    reject_function_collisions(program, &enum_names)?;
    reject_duplicate_variants(program)?;
    Ok(())
}

fn collect_enum_names(program: &SyntaxProgram) -> Result<HashMap<String, Span>, LowerError> {
    let mut names = HashMap::new();
    for enum_def in &program.enums {
        if names.insert(enum_def.name.clone(), enum_def.span).is_some() {
            return Err(LowerError {
                message: format!("duplicate enum name {:?}", enum_def.name),
                span: enum_def.span,
            });
        }
    }
    Ok(names)
}

fn reject_record_collisions(
    program: &SyntaxProgram,
    enum_names: &HashMap<String, Span>,
) -> Result<(), LowerError> {
    for record in &program.records {
        if enum_names.contains_key(&record.name) {
            return Err(LowerError {
                message: format!(
                    "record and enum names share a nominal namespace in Enums v0; duplicate name {:?}",
                    record.name
                ),
                span: record.span,
            });
        }
    }
    Ok(())
}

fn reject_function_collisions(
    program: &SyntaxProgram,
    enum_names: &HashMap<String, Span>,
) -> Result<(), LowerError> {
    for function in &program.functions {
        if enum_names.contains_key(&function.name) {
            return Err(LowerError {
                message: format!(
                    "enum and function names share a namespace in Enums v0; duplicate name {:?}",
                    function.name
                ),
                span: function.span,
            });
        }
    }
    Ok(())
}

fn reject_duplicate_variants(program: &SyntaxProgram) -> Result<(), LowerError> {
    for enum_def in &program.enums {
        let mut seen = HashSet::new();
        for variant in &enum_def.variants {
            if !seen.insert(variant.name.as_str()) {
                return Err(LowerError {
                    message: format!(
                        "duplicate variant name {:?} in enum {:?}",
                        variant.name, enum_def.name
                    ),
                    span: variant.span,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_enum_declarations;
    use evo_lexer::lex;
    use evo_parser::parse;

    fn validate(source: &str) -> Result<(), crate::LowerError> {
        let tokens = lex(source).expect("enum semantic validation source should lex");
        let program = parse(&tokens).expect("enum semantic validation source should parse");
        validate_enum_declarations(&program)
    }

    #[test]
    fn accepts_distinct_enum_and_variant_names() {
        validate("enum MaybeInt\nNone\nSome int\nend\nenum Flag\nOff\nOn\nend\n")
            .expect("distinct enum declarations should pass local semantic validation");
    }

    #[test]
    fn rejects_duplicate_enum_names() {
        let error = validate("enum Flag\nOff\nend\nenum Flag\nOn\nend\n")
            .expect_err("duplicate enum names must fail");
        assert!(error.message.contains("duplicate enum name"));
        assert_eq!(error.span.line, 4);
    }

    #[test]
    fn rejects_duplicate_variant_names_within_one_enum() {
        let error = validate("enum Flag\nOn\nOn\nend\n")
            .expect_err("duplicate variants in one enum must fail");
        assert!(error.message.contains("duplicate variant name"));
        assert_eq!(error.span.line, 3);
    }

    #[test]
    fn permits_same_variant_name_in_different_enums() {
        validate("enum Left\nNone\nend\nenum Right\nNone\nend\n")
            .expect("variant identity is scoped by enum name");
    }

    #[test]
    fn rejects_record_enum_nominal_namespace_collision() {
        let error = validate("record Value\nx int\nend\nenum Value\nNone\nend\n")
            .expect_err("record and enum type names must not collide");
        assert!(error.message.contains("nominal namespace"));
        assert_eq!(error.span.line, 1);
    }

    #[test]
    fn rejects_enum_function_namespace_collision() {
        let error = validate("enum Value\nNone\nend\nfn Value() int\nreturn 1\nend\n")
            .expect_err("enum and function names must not collide");
        assert!(error.message.contains("share a namespace"));
        assert_eq!(error.span.line, 4);
    }
}
