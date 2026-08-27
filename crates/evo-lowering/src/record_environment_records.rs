use crate::LowerError;
use evo_lexer::Span;
use evo_parser::{
    Program as SyntaxProgram, RecordFieldType as SyntaxFieldType, TypeName as SyntaxTypeName,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SemanticType {
    Integer,
    Bool,
    String,
    Record(String),
}

impl SemanticType {
    #[must_use]
    pub(crate) fn is_trivially_reusable_v0(&self) -> bool {
        !matches!(self, Self::Record(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConstructorFieldInput {
    pub(crate) name: String,
    pub(crate) value_type: SemanticType,
    pub(crate) span: Span,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedField {
    pub(crate) name: String,
    pub(crate) value_type: SemanticType,
    pub(crate) span: Span,
}

#[derive(Debug, Clone)]
pub(crate) struct RecordSchema {
    pub(crate) name: String,
    pub(crate) fields: Vec<ResolvedField>,
    pub(crate) span: Span,
}

#[derive(Debug, Clone)]
pub(crate) struct RecordEnvironment {
    schemas: Vec<RecordSchema>,
    indices: HashMap<String, usize>,
}

impl RecordEnvironment {
    pub(crate) fn resolve_type_name(
        &self,
        type_name: &SyntaxTypeName,
        span: Span,
    ) -> Result<SemanticType, LowerError> {
        match type_name {
            SyntaxTypeName::Int => Ok(SemanticType::Integer),
            SyntaxTypeName::Bool => Ok(SemanticType::Bool),
            SyntaxTypeName::String => Ok(SemanticType::String),
            SyntaxTypeName::Named(name) => {
                if self.indices.contains_key(name) {
                    Ok(SemanticType::Record(name.clone()))
                } else {
                    Err(LowerError {
                        message: format!("unknown record type {name:?}"),
                        span,
                    })
                }
            }
        }
    }

    pub(crate) fn validate_constructor(
        &self,
        name: &str,
        fields: &[ConstructorFieldInput],
        constructor_span: Span,
    ) -> Result<SemanticType, LowerError> {
        let schema = self.schema(name).ok_or_else(|| LowerError {
            message: format!("unknown record constructor {name:?}"),
            span: constructor_span,
        })?;

        let mut seen = HashSet::new();
        for field in fields {
            if !seen.insert(field.name.as_str()) {
                return Err(LowerError {
                    message: format!(
                        "duplicate constructor field {:?} for record {:?}",
                        field.name, name
                    ),
                    span: field.span,
                });
            }

            let declared = schema
                .fields
                .iter()
                .find(|candidate| candidate.name == field.name)
                .ok_or_else(|| LowerError {
                    message: format!(
                        "unknown constructor field {:?} for record {:?}",
                        field.name, name
                    ),
                    span: field.span,
                })?;

            if declared.value_type != field.value_type {
                return Err(LowerError {
                    message: format!(
                        "constructor field {:?} for record {:?} expects {}, found {}",
                        field.name,
                        name,
                        semantic_type_label(&declared.value_type),
                        semantic_type_label(&field.value_type)
                    ),
                    span: field.span,
                });
            }
        }

        let missing: Vec<&str> = schema
            .fields
            .iter()
            .filter(|field| !seen.contains(field.name.as_str()))
            .map(|field| field.name.as_str())
            .collect();
        if !missing.is_empty() {
            return Err(LowerError {
                message: format!(
                    "record constructor {name:?} is missing field(s): {}",
                    missing.join(", ")
                ),
                span: constructor_span,
            });
        }

        Ok(SemanticType::Record(name.to_owned()))
    }

    pub(crate) fn field_type(
        &self,
        base_type: &SemanticType,
        field_name: &str,
        access_span: Span,
    ) -> Result<SemanticType, LowerError> {
        let SemanticType::Record(record_name) = base_type else {
            return Err(LowerError {
                message: "field access requires a record value".to_owned(),
                span: access_span,
            });
        };

        let schema = self
            .schema(record_name)
            .expect("record semantic types originate from a validated environment");
        schema
            .fields
            .iter()
            .find(|field| field.name == field_name)
            .map(|field| field.value_type.clone())
            .ok_or_else(|| LowerError {
                message: format!("unknown field {field_name:?} on record {:?}", record_name),
                span: access_span,
            })
    }

    #[must_use]
    pub(crate) fn schema(&self, name: &str) -> Option<&RecordSchema> {
        self.indices.get(name).map(|index| &self.schemas[*index])
    }
}

pub(crate) fn collect_record_environment(
    program: &SyntaxProgram,
) -> Result<RecordEnvironment, LowerError> {
    let record_names = collect_record_names(program)?;
    reject_function_collisions(program, &record_names)?;
    let schemas = resolve_record_schemas(program, &record_names)?;
    reject_recursive_by_value_layouts(&schemas)?;
    let indices = schemas
        .iter()
        .enumerate()
        .map(|(index, schema)| (schema.name.clone(), index))
        .collect();
    Ok(RecordEnvironment { schemas, indices })
}

pub(crate) fn validate_record_declarations(program: &SyntaxProgram) -> Result<(), LowerError> {
    collect_record_environment(program).map(|_| ())
}

fn collect_record_names(program: &SyntaxProgram) -> Result<HashMap<String, Span>, LowerError> {
    let mut names = HashMap::new();
    for record in &program.records {
        if names.insert(record.name.clone(), record.span).is_some() {
            return Err(LowerError {
                message: format!("duplicate record name {:?}", record.name),
                span: record.span,
            });
        }
    }
    Ok(names)
}

fn reject_function_collisions(
    program: &SyntaxProgram,
    record_names: &HashMap<String, Span>,
) -> Result<(), LowerError> {
    for function in &program.functions {
        if record_names.contains_key(&function.name) {
            return Err(LowerError {
                message: format!(
                    "record and function names share a namespace in Records v0; duplicate name {:?}",
                    function.name
                ),
                span: function.span,
            });
        }
    }
    Ok(())
}

fn resolve_record_schemas(
    program: &SyntaxProgram,
    record_names: &HashMap<String, Span>,
) -> Result<Vec<RecordSchema>, LowerError> {
    let mut schemas = Vec::with_capacity(program.records.len());

    for record in &program.records {
        let mut seen_fields = HashSet::new();
        let mut fields = Vec::with_capacity(record.fields.len());

        for field in &record.fields {
            if !seen_fields.insert(field.name.clone()) {
                return Err(LowerError {
                    message: format!(
                        "duplicate field {:?} in record {:?}",
                        field.name, record.name
                    ),
                    span: field.span,
                });
            }

            let value_type = match &field.type_name {
                SyntaxFieldType::Int => SemanticType::Integer,
                SyntaxFieldType::Bool => SemanticType::Bool,
                SyntaxFieldType::String => SemanticType::String,
                SyntaxFieldType::Named(name) => {
                    if !record_names.contains_key(name) {
                        return Err(LowerError {
                            message: format!(
                                "unknown record type {name:?} for field {:?} in record {:?}",
                                field.name, record.name
                            ),
                            span: field.span,
                        });
                    }
                    SemanticType::Record(name.clone())
                }
            };

            fields.push(ResolvedField {
                name: field.name.clone(),
                value_type,
                span: field.span,
            });
        }

        schemas.push(RecordSchema {
            name: record.name.clone(),
            fields,
            span: record.span,
        });
    }

    Ok(schemas)
}

fn reject_recursive_by_value_layouts(schemas: &[RecordSchema]) -> Result<(), LowerError> {
    let indices: HashMap<&str, usize> = schemas
        .iter()
        .enumerate()
        .map(|(index, schema)| (schema.name.as_str(), index))
        .collect();
    let mut states = vec![0u8; schemas.len()];
    let mut stack = Vec::new();

    for index in 0..schemas.len() {
        if states[index] == 0 {
            visit_record(index, schemas, &indices, &mut states, &mut stack)?;
        }
    }
    Ok(())
}

fn visit_record(
    index: usize,
    schemas: &[RecordSchema],
    indices: &HashMap<&str, usize>,
    states: &mut [u8],
    stack: &mut Vec<usize>,
) -> Result<(), LowerError> {
    states[index] = 1;
    stack.push(index);

    for field in &schemas[index].fields {
        let SemanticType::Record(target_name) = &field.value_type else {
            continue;
        };
        let target = *indices
            .get(target_name.as_str())
            .expect("named field types are resolved before cycle checking");

        if states[target] == 1 {
            let cycle_start = stack
                .iter()
                .position(|candidate| *candidate == target)
                .expect("visiting target must be present in DFS stack");
            let mut cycle_names: Vec<&str> = stack[cycle_start..]
                .iter()
                .map(|record_index| schemas[*record_index].name.as_str())
                .collect();
            cycle_names.push(schemas[target].name.as_str());
            return Err(LowerError {
                message: format!(
                    "field {:?} creates illegal recursive by-value record layout: {}",
                    field.name,
                    cycle_names.join(" -> ")
                ),
                span: field.span,
            });
        }

        if states[target] == 0 {
            visit_record(target, schemas, indices, states, stack)?;
        }
    }

    let popped = stack.pop();
    debug_assert_eq!(popped, Some(index));
    states[index] = 2;
    Ok(())
}

fn semantic_type_label(value_type: &SemanticType) -> String {
    match value_type {
        SemanticType::Integer => "int".to_owned(),
        SemanticType::Bool => "bool".to_owned(),
        SemanticType::String => "string".to_owned(),
        SemanticType::Record(name) => name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ConstructorFieldInput, SemanticType, collect_record_environment,
        validate_record_declarations,
    };
    use evo_lexer::{Span, lex};
    use evo_parser::{TypeName, parse};

    fn parse_source(source: &str) -> evo_parser::Program {
        let tokens = lex(source).expect("record validation source should lex");
        parse(&tokens).expect("record validation source should parse")
    }

    fn validate(source: &str) -> Result<(), crate::LowerError> {
        validate_record_declarations(&parse_source(source))
    }

    fn test_span(line: usize) -> Span {
        Span {
            start: 0,
            end: 1,
            line,
            column: 1,
        }
    }

    #[test]
    fn accepts_forward_acyclic_record_references() {
        validate("record Wrapper\npoint Point\nend\nrecord Point\nx int\ny int\nend\n")
            .expect("forward acyclic record reference should validate");
    }

    #[test]
    fn rejects_duplicate_record_names() {
        let error = validate("record Point\nx int\nend\nrecord Point\ny int\nend\n")
            .expect_err("duplicate record should fail");
        assert!(error.message.contains("duplicate record"));
        assert_eq!(error.span.line, 4);
    }

    #[test]
    fn rejects_duplicate_field_names() {
        let error = validate("record Point\nx int\nx bool\nend\n")
            .expect_err("duplicate field should fail");
        assert!(error.message.contains("duplicate field"));
        assert_eq!(error.span.line, 3);
    }

    #[test]
    fn rejects_unknown_named_field_type() {
        let error = validate("record Wrapper\nvalue Missing\nend\n")
            .expect_err("unknown record field type should fail");
        assert!(error.message.contains("unknown record type"));
        assert_eq!(error.span.line, 2);
    }

    #[test]
    fn rejects_record_function_namespace_collision() {
        let error = validate("record Point\nx int\nend\nfn Point() int\nreturn 1\nend\n")
            .expect_err("record/function name collision should fail");
        assert!(error.message.contains("share a namespace"));
        assert_eq!(error.span.line, 4);
    }

    #[test]
    fn rejects_direct_recursive_by_value_layout() {
        let error = validate("record Node\nnext Node\nend\n")
            .expect_err("direct by-value recursion should fail");
        assert!(error.message.contains("Node -> Node"));
        assert_eq!(error.span.line, 2);
    }

    #[test]
    fn rejects_indirect_recursive_by_value_layout() {
        let error = validate("record A\nb B\nend\nrecord B\nc C\nend\nrecord C\na A\nend\n")
            .expect_err("indirect by-value recursion should fail");
        assert!(error.message.contains("A -> B -> C -> A"));
        assert_eq!(error.span.line, 8);
    }

    #[test]
    fn resolves_record_types_in_function_signatures() {
        let program = parse_source("record Point\nx int\nend\n");
        let environment = collect_record_environment(&program).expect("environment should build");
        assert_eq!(
            environment
                .resolve_type_name(&TypeName::Named("Point".to_owned()), test_span(1))
                .expect("known record type should resolve"),
            SemanticType::Record("Point".to_owned())
        );
        let error = environment
            .resolve_type_name(&TypeName::Named("Missing".to_owned()), test_span(9))
            .expect_err("unknown record type should fail");
        assert_eq!(error.span.line, 9);
    }

    #[test]
    fn constructor_requires_exact_named_fields_and_types() {
        let program = parse_source("record Point\nx int\ny bool\nend\n");
        let environment = collect_record_environment(&program).expect("environment should build");
        let fields = vec![
            ConstructorFieldInput {
                name: "y".to_owned(),
                value_type: SemanticType::Bool,
                span: test_span(2),
            },
            ConstructorFieldInput {
                name: "x".to_owned(),
                value_type: SemanticType::Integer,
                span: test_span(3),
            },
        ];
        assert_eq!(
            environment
                .validate_constructor("Point", &fields, test_span(1))
                .expect("named field order should not matter"),
            SemanticType::Record("Point".to_owned())
        );

        let missing = environment
            .validate_constructor("Point", &fields[..1], test_span(4))
            .expect_err("missing field should fail");
        assert!(missing.message.contains("missing field"));

        let duplicate = vec![fields[0].clone(), fields[0].clone()];
        let error = environment
            .validate_constructor("Point", &duplicate, test_span(5))
            .expect_err("duplicate constructor field should fail");
        assert!(error.message.contains("duplicate constructor field"));

        let unknown = vec![ConstructorFieldInput {
            name: "z".to_owned(),
            value_type: SemanticType::Integer,
            span: test_span(6),
        }];
        let error = environment
            .validate_constructor("Point", &unknown, test_span(6))
            .expect_err("unknown constructor field should fail");
        assert!(error.message.contains("unknown constructor field"));

        let wrong_type = vec![
            ConstructorFieldInput {
                name: "x".to_owned(),
                value_type: SemanticType::Bool,
                span: test_span(7),
            },
            fields[0].clone(),
        ];
        let error = environment
            .validate_constructor("Point", &wrong_type, test_span(7))
            .expect_err("field type mismatch should fail");
        assert!(error.message.contains("expects int, found bool"));
    }

    #[test]
    fn field_access_is_static_and_supports_chaining() {
        let program =
            parse_source("record Inner\nvalue int\nend\nrecord Outer\ninner Inner\nend\n");
        let environment = collect_record_environment(&program).expect("environment should build");
        let inner = environment
            .field_type(
                &SemanticType::Record("Outer".to_owned()),
                "inner",
                test_span(1),
            )
            .expect("outer.inner should type");
        assert_eq!(inner, SemanticType::Record("Inner".to_owned()));
        let value = environment
            .field_type(&inner, "value", test_span(2))
            .expect("outer.inner.value should type");
        assert_eq!(value, SemanticType::Integer);

        let scalar_error = environment
            .field_type(&SemanticType::Integer, "x", test_span(3))
            .expect_err("field access on scalar should fail");
        assert!(scalar_error.message.contains("requires a record"));

        let unknown = environment
            .field_type(
                &SemanticType::Record("Outer".to_owned()),
                "missing",
                test_span(4),
            )
            .expect_err("unknown record field should fail");
        assert!(unknown.message.contains("unknown field"));
    }

    #[test]
    fn records_are_move_only_in_v0_semantic_classification() {
        assert!(SemanticType::Integer.is_trivially_reusable_v0());
        assert!(SemanticType::String.is_trivially_reusable_v0());
        assert!(!SemanticType::Record("Point".to_owned()).is_trivially_reusable_v0());
    }
}
