use crate::LowerError;
use evo_lexer::Span;
use evo_parser::{Program as SyntaxProgram, RecordFieldType as SyntaxFieldType};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResolvedType {
    Integer,
    Bool,
    String,
    Record(String),
}

#[derive(Debug, Clone)]
struct ResolvedField {
    name: String,
    value_type: ResolvedType,
    span: Span,
}

#[derive(Debug, Clone)]
struct RecordSchema {
    name: String,
    fields: Vec<ResolvedField>,
}

pub(crate) fn validate_record_declarations(program: &SyntaxProgram) -> Result<(), LowerError> {
    if program.records.is_empty() {
        return Ok(());
    }

    let record_names = collect_record_names(program)?;
    reject_function_collisions(program, &record_names)?;
    let schemas = resolve_record_schemas(program, &record_names)?;
    reject_recursive_by_value_layouts(&schemas)
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
                SyntaxFieldType::Int => ResolvedType::Integer,
                SyntaxFieldType::Bool => ResolvedType::Bool,
                SyntaxFieldType::String => ResolvedType::String,
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
                    ResolvedType::Record(name.clone())
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
        let ResolvedType::Record(target_name) = &field.value_type else {
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

#[cfg(test)]
mod tests {
    use super::validate_record_declarations;
    use evo_lexer::lex;
    use evo_parser::parse;

    fn validate(source: &str) -> Result<(), crate::LowerError> {
        let tokens = lex(source).expect("record validation source should lex");
        let program = parse(&tokens).expect("record validation source should parse");
        validate_record_declarations(&program)
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
}
