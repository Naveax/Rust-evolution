use crate::LowerError;
use evo_lexer::Span;
use evo_parser::{
    Program as SyntaxProgram, RecordFieldType as SyntaxRecordFieldType,
    TypeName as SyntaxTypeName,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolvedPayloadType {
    Integer,
    Bool,
    String,
    Record(String),
    Enum(String),
}

impl ResolvedPayloadType {
    fn nominal_name(&self) -> Option<&str> {
        match self {
            Self::Record(name) | Self::Enum(name) => Some(name),
            Self::Integer | Self::Bool | Self::String => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedVariant {
    pub(crate) name: String,
    pub(crate) payload_type: Option<ResolvedPayloadType>,
    pub(crate) span: Span,
}

#[derive(Debug, Clone)]
pub(crate) struct EnumSchema {
    pub(crate) name: String,
    pub(crate) variants: Vec<ResolvedVariant>,
    pub(crate) span: Span,
}

#[derive(Debug, Clone)]
pub(crate) struct EnumEnvironment {
    schemas: Vec<EnumSchema>,
    indices: HashMap<String, usize>,
}

impl EnumEnvironment {
    pub(crate) fn schema(&self, name: &str) -> Option<&EnumSchema> {
        self.indices.get(name).map(|index| &self.schemas[*index])
    }
}

#[derive(Debug, Clone)]
struct NominalEdge {
    target: String,
    member: String,
    span: Span,
}

#[derive(Debug, Clone)]
struct NominalNode {
    name: String,
    edges: Vec<NominalEdge>,
}

pub(crate) fn validate_enum_declarations(program: &SyntaxProgram) -> Result<(), LowerError> {
    collect_enum_environment(program).map(|_| ())
}

pub(crate) fn collect_enum_environment(
    program: &SyntaxProgram,
) -> Result<EnumEnvironment, LowerError> {
    let record_names = collect_record_names(program)?;
    let enum_names = collect_enum_names(program)?;
    reject_nominal_collisions(program, &record_names)?;
    reject_function_collisions(program, &record_names, &enum_names)?;
    validate_record_named_types(program, &record_names, &enum_names)?;
    let schemas = collect_enum_schemas(program, &record_names, &enum_names)?;
    let indices = schemas
        .iter()
        .enumerate()
        .map(|(index, schema)| (schema.name.clone(), index))
        .collect();
    let environment = EnumEnvironment { schemas, indices };
    reject_recursive_by_value_layouts(program, &record_names, &enum_names, &environment)?;
    Ok(environment)
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

fn validate_record_named_types(
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

    Ok(())
}

fn collect_enum_schemas(
    program: &SyntaxProgram,
    record_names: &HashMap<String, Span>,
    enum_names: &HashMap<String, Span>,
) -> Result<Vec<EnumSchema>, LowerError> {
    let mut schemas = Vec::with_capacity(program.enums.len());

    for enum_def in &program.enums {
        let mut seen = HashSet::new();
        let mut variants = Vec::with_capacity(enum_def.variants.len());
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

            let payload_type = variant
                .payload_type
                .as_ref()
                .map(|type_name| {
                    resolve_payload_type(
                        type_name,
                        record_names,
                        enum_names,
                        &enum_def.name,
                        &variant.name,
                        variant.span,
                    )
                })
                .transpose()?;
            variants.push(ResolvedVariant {
                name: variant.name.clone(),
                payload_type,
                span: variant.span,
            });
        }

        schemas.push(EnumSchema {
            name: enum_def.name.clone(),
            variants,
            span: enum_def.span,
        });
    }

    Ok(schemas)
}

fn resolve_payload_type(
    type_name: &SyntaxTypeName,
    record_names: &HashMap<String, Span>,
    enum_names: &HashMap<String, Span>,
    enum_name: &str,
    variant_name: &str,
    span: Span,
) -> Result<ResolvedPayloadType, LowerError> {
    match type_name {
        SyntaxTypeName::Int => Ok(ResolvedPayloadType::Integer),
        SyntaxTypeName::Bool => Ok(ResolvedPayloadType::Bool),
        SyntaxTypeName::String => Ok(ResolvedPayloadType::String),
        SyntaxTypeName::Named(name) if record_names.contains_key(name) => {
            Ok(ResolvedPayloadType::Record(name.clone()))
        }
        SyntaxTypeName::Named(name) if enum_names.contains_key(name) => {
            Ok(ResolvedPayloadType::Enum(name.clone()))
        }
        SyntaxTypeName::Named(name) => Err(LowerError {
            message: format!(
                "unknown payload type {name:?} for variant {variant_name:?} in enum {enum_name:?}"
            ),
            span,
        }),
    }
}

fn reject_recursive_by_value_layouts(
    program: &SyntaxProgram,
    record_names: &HashMap<String, Span>,
    enum_names: &HashMap<String, Span>,
    environment: &EnumEnvironment,
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
                    member: field.name.clone(),
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
        let schema = environment
            .schema(&enum_def.name)
            .expect("validated enum names resolve to a schema");
        debug_assert_eq!(schema.span, enum_def.span);
        let edges = schema
            .variants
            .iter()
            .filter_map(|variant| {
                variant.payload_type.as_ref().and_then(|payload_type| {
                    payload_type.nominal_name().map(|name| NominalEdge {
                        target: name.to_owned(),
                        member: variant.name.clone(),
                        span: variant.span,
                    })
                })
            })
            .collect();
        nodes.push(NominalNode {
            name: schema.name.clone(),
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
                    "member {:?} creates illegal recursive by-value nominal layout: {}",
                    edge.member,
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
    use super::{ResolvedPayloadType, collect_enum_environment, validate_enum_declarations};
    use evo_lexer::lex;
    use evo_parser::parse;

    fn parse_source(source: &str) -> evo_parser::Program {
        let tokens = lex(source).expect("enum semantic validation source should lex");
        parse(&tokens).expect("enum semantic validation source should parse")
    }

    fn validate(source: &str) -> Result<(), crate::LowerError> {
        validate_enum_declarations(&parse_source(source))
    }

    #[test]
    fn accepts_distinct_enum_and_variant_names() {
        validate("enum MaybeInt\nNone\nSome int\nend\nenum Flag\nOff\nOn\nend\n")
            .expect("distinct enum declarations should pass local semantic validation");
    }

    #[test]
    fn resolves_builtin_record_and_enum_variant_payloads() {
        let program = parse_source(
            "record Point\nx int\nend\nenum MaybePoint\nNone\nSome Point\nend\nenum Wrapped\nEmpty\nValue MaybePoint\nend\n",
        );
        let environment =
            collect_enum_environment(&program).expect("resolved enum environment should build");
        let maybe_point = environment
            .schema("MaybePoint")
            .expect("MaybePoint schema should resolve");
        assert_eq!(maybe_point.variants[0].payload_type, None);
        assert_eq!(
            maybe_point.variants[1].payload_type,
            Some(ResolvedPayloadType::Record("Point".to_owned()))
        );
        let wrapped = environment
            .schema("Wrapped")
            .expect("Wrapped schema should resolve");
        assert_eq!(
            wrapped.variants[1].payload_type,
            Some(ResolvedPayloadType::Enum("MaybePoint".to_owned()))
        );
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
