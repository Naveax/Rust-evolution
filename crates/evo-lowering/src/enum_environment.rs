use crate::LowerError;
use evo_lexer::Span;
use evo_parser::{
    Program as SyntaxProgram, RecordFieldType as SyntaxRecordFieldType,
    TypeName as SyntaxTypeName,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
struct NominalEdge {
    target: String,
    span: Span,
}

#[derive(Debug, Clone)]
struct NominalNode {
    name: String,
    edges: Vec<NominalEdge>,
}

pub(crate) fn validate_enum_declarations(program: &SyntaxProgram) -> Result<(), LowerError> {
    let record_names = collect_record_names(program)?;
    let enum_names = collect_enum_names(program)?;
    reject_nominal_collisions(program, &record_names)?;
    reject_function_collisions(program, &record_names, &enum_names)?;
    reject_duplicate_variants(program)?;
    validate_named_types(program, &record_names, &enum_names)?;
    reject_recursive_by_value_layouts(program, &record_names, &enum_names)?;
    Ok(())
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

fn reject_nominal_collisions(
    program: &SyntaxProgram,
    record_names: &HashMap<String, Span>,
) -> Result<(), LowerError> {
    for enum_def in &program.enums {
        if record_names.contains_key(&enum_def.name) {
            return Err(LowerError {
                message: format!(
                    "record and enum names share a nominal namespace in Enums v0; duplicate name {:?}",
                    enum_def.name
                ),
                span: enum_def.span,
            });
        }
    }
    Ok(())
}

fn reject_function_collisions(
    program: &SyntaxProgram,
    record_names: &HashMap<String, Span>,
    enum_names: &HashMap<String, Span>,
) -> Result<(), LowerError> {
    for function in &program.functions {
        if record_names.contains_key(&function.name) {
            return Err(LowerError {
                message: format!(
                    "record and function names share a namespace in Enums v0; duplicate name {:?}",
                    function.name
                ),
                span: function.span,
            });
        }
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

fn validate_named_types(
    program: &SyntaxProgram,
    record_names: &HashMap<String, Span>,
    enum_names: &HashMap<String, Span>,
) -> Result<(), LowerError> {
    let known = |name: &str| record_names.contains_key(name) || enum_names.contains_key(name);

    for record in &program.records {
        for field in &record.fields {
            if let SyntaxRecordFieldType::Named(name) = &field.type_name
                && !known(name)
            {
                return Err(LowerError {
                    message: format!(
                        "unknown nominal type {name:?} for field {:?} in record {:?}",
                        field.name, record.name
                    ),
                    span: field.span,
                });
            }
        }
    }

    for enum_def in &program.enums {
        for variant in &enum_def.variants {
            if let Some(SyntaxTypeName::Named(name)) = &variant.payload_type
                && !known(name)
            {
                return Err(LowerError {
                    message: format!(
                        "unknown payload type {name:?} for variant {:?} in enum {:?}",
                        variant.name, enum_def.name
                    ),
                    span: variant.span,
                });
            }
        }
    }

    Ok(())
}

fn reject_recursive_by_value_layouts(
    program: &SyntaxProgram,
    record_names: &HashMap<String, Span>,
    enum_names: &HashMap<String, Span>,
) -> Result<(), LowerError> {
    let known = |name: &str| record_names.contains_key(name) || enum_names.contains_key(name);
    let mut nodes = Vec::with_capacity(program.records.len() + program.enums.len());

    for record in &program.records {
        let edges = record
            .fields
            .iter()
            .filter_map(|field| match &field.type_name {
                SyntaxRecordFieldType::Named(name) if known(name) => Some(NominalEdge {
                    target: name.clone(),
                    span: field.span,
                }),
                _ => None,
            })
            .collect();
        nodes.push(NominalNode {
            name: record.name.clone(),
            edges,
        });
    }

    for enum_def in &program.enums {
        let edges = enum_def
            .variants
            .iter()
            .filter_map(|variant| match &variant.payload_type {
                Some(SyntaxTypeName::Named(name)) if known(name) => Some(NominalEdge {
                    target: name.clone(),
                    span: variant.span,
                }),
                _ => None,
            })
            .collect();
        nodes.push(NominalNode {
            name: enum_def.name.clone(),
            edges,
        });
    }

    let indices: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.name.as_str(), index))
        .collect();
    let mut states = vec![0u8; nodes.len()];
    let mut stack = Vec::new();

    for index in 0..nodes.len() {
        if states[index] == 0 {
            visit_nominal(index, &nodes, &indices, &mut states, &mut stack)?;
        }
    }
    Ok(())
}

fn visit_nominal(
    index: usize,
    nodes: &[NominalNode],
    indices: &HashMap<&str, usize>,
    states: &mut [u8],
    stack: &mut Vec<usize>,
) -> Result<(), LowerError> {
    states[index] = 1;
    stack.push(index);

    for edge in &nodes[index].edges {
        let target = *indices
            .get(edge.target.as_str())
            .expect("nominal references are validated before cycle checking");

        if states[target] == 1 {
            let cycle_start = stack
                .iter()
                .position(|candidate| *candidate == target)
                .expect("visiting target must be present in DFS stack");
            let mut cycle_names: Vec<&str> = stack[cycle_start..]
                .iter()
                .map(|node_index| nodes[*node_index].name.as_str())
                .collect();
            cycle_names.push(nodes[target].name.as_str());
            return Err(LowerError {
                message: format!(
                    "illegal recursive by-value nominal layout: {}",
                    cycle_names.join(" -> ")
                ),
                span: edge.span,
            });
        }

        if states[target] == 0 {
            visit_nominal(target, nodes, indices, states, stack)?;
        }
    }

    let popped = stack.pop();
    debug_assert_eq!(popped, Some(index));
    states[index] = 2;
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
        assert_eq!(error.span.line, 4);
    }

    #[test]
    fn rejects_enum_function_namespace_collision() {
        let error = validate("enum Value\nNone\nend\nfn Value() int\nreturn 1\nend\n")
            .expect_err("enum and function names must not collide");
        assert!(error.message.contains("share a namespace"));
        assert_eq!(error.span.line, 4);
    }

    #[test]
    fn accepts_builtin_record_and_enum_payload_references() {
        validate(
            "record Point\nx int\nend\nenum MaybePoint\nNone\nSome Point\nend\nenum Wrapped\nEmpty\nValue MaybePoint\nend\n",
        )
        .expect("acyclic builtin and nominal payload references should validate");
    }

    #[test]
    fn accepts_record_fields_that_reference_enums_acyclically() {
        validate("record Holder\nvalue MaybeInt\nend\nenum MaybeInt\nNone\nSome int\nend\n")
            .expect("record-to-enum references should participate in the nominal graph");
    }

    #[test]
    fn rejects_unknown_enum_payload_type() {
        let error = validate("enum MaybeValue\nNone\nSome Missing\nend\n")
            .expect_err("unknown enum payload type must fail");
        assert!(error.message.contains("unknown payload type"));
        assert_eq!(error.span.line, 3);
    }

    #[test]
    fn rejects_unknown_record_field_type_in_mixed_nominal_program() {
        let error = validate("record Holder\nvalue Missing\nend\nenum Flag\nOff\nOn\nend\n")
            .expect_err("unknown record field type must fail in the shared nominal graph");
        assert!(error.message.contains("unknown nominal type"));
        assert_eq!(error.span.line, 2);
    }

    #[test]
    fn rejects_direct_recursive_enum_payload_layout() {
        let error = validate("enum Loop\nAgain Loop\nend\n")
            .expect_err("direct enum recursion must not imply hidden boxing");
        assert!(error.message.contains("Loop -> Loop"));
        assert_eq!(error.span.line, 2);
    }

    #[test]
    fn rejects_record_enum_recursive_by_value_layout() {
        let error = validate(
            "record Wrapper\nvalue MaybeWrapper\nend\nenum MaybeWrapper\nNone\nSome Wrapper\nend\n",
        )
        .expect_err("record/enum cycles must not imply hidden boxing");
        assert!(error.message.contains("Wrapper -> MaybeWrapper -> Wrapper"));
        assert_eq!(error.span.line, 6);
    }
}
