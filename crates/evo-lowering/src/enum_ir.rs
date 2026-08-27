use evo_lexer::Span;
use evo_parser::{
    Program as SyntaxProgram, RecordFieldType as SyntaxRecordFieldType, Stmt as SyntaxStmt,
    StmtKind as SyntaxStmtKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SchemaType {
    Integer,
    Bool,
    String,
    Record(String),
    Enum(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnumVariantIr {
    pub(crate) name: String,
    pub(crate) payload_type: Option<SchemaType>,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnumIr {
    pub(crate) name: String,
    pub(crate) variants: Vec<EnumVariantIr>,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordSchemaFieldIr {
    pub(crate) name: String,
    pub(crate) value_type: SchemaType,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordSchemaIr {
    pub(crate) name: String,
    pub(crate) fields: Vec<RecordSchemaFieldIr>,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MatchBindingIr {
    pub(crate) name: String,
    pub(crate) value_type: SchemaType,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MatchArmIr {
    pub(crate) enum_name: String,
    pub(crate) variant_name: String,
    pub(crate) binding: Option<MatchBindingIr>,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MatchIr {
    pub(crate) enum_name: String,
    pub(crate) arms: Vec<MatchArmIr>,
    pub(crate) span: Span,
    pub(crate) all_arms_return: bool,
}

pub(super) fn lower_enum_schemas(environment: &super::EnumEnvironment) -> Vec<EnumIr> {
    environment
        .schemas
        .iter()
        .map(|schema| EnumIr {
            name: schema.name.clone(),
            variants: schema
                .variants
                .iter()
                .map(|variant| EnumVariantIr {
                    name: variant.name.clone(),
                    payload_type: variant.payload_type.as_ref().map(lower_payload_type),
                    span: variant.span,
                })
                .collect(),
            span: schema.span,
        })
        .collect()
}

pub(super) fn lower_record_schemas(
    program: &SyntaxProgram,
    environment: &super::EnumEnvironment,
) -> Vec<RecordSchemaIr> {
    program
        .records
        .iter()
        .map(|record| RecordSchemaIr {
            name: record.name.clone(),
            fields: record
                .fields
                .iter()
                .map(|field| RecordSchemaFieldIr {
                    name: field.name.clone(),
                    value_type: lower_record_field_type(&field.type_name, environment),
                    span: field.span,
                })
                .collect(),
            span: record.span,
        })
        .collect()
}

pub(super) fn lower_matches(
    program: &SyntaxProgram,
    matches: &super::match_validation::MatchEnvironment,
) -> Vec<MatchIr> {
    let mut lowered = Vec::new();
    for function in &program.functions {
        collect_matches(&function.body, matches, &mut lowered);
    }
    collect_matches(&program.statements, matches, &mut lowered);
    lowered
}

fn collect_matches(
    statements: &[SyntaxStmt],
    matches: &super::match_validation::MatchEnvironment,
    lowered: &mut Vec<MatchIr>,
) {
    for statement in statements {
        match &statement.kind {
            SyntaxStmtKind::Match { arms, .. } => {
                let resolved = matches
                    .match_at(statement.span.start)
                    .expect("match IR promotion runs after resolved exhaustive match validation");
                lowered.push(MatchIr {
                    enum_name: resolved.enum_name.clone(),
                    arms: resolved
                        .arms
                        .iter()
                        .map(|arm| MatchArmIr {
                            enum_name: arm.enum_name.clone(),
                            variant_name: arm.variant_name.clone(),
                            binding: arm.binding.as_ref().map(|binding| MatchBindingIr {
                                name: binding.name.clone(),
                                value_type: lower_payload_type(&binding.value_type),
                                span: binding.span,
                            }),
                            span: arm.span,
                        })
                        .collect(),
                    span: resolved.span,
                    all_arms_return: resolved.all_arms_return,
                });
                for arm in arms {
                    collect_matches(&arm.body, matches, lowered);
                }
            }
            SyntaxStmtKind::Repeat { body, .. } => collect_matches(body, matches, lowered),
            SyntaxStmtKind::If {
                then_body,
                else_body,
                ..
            } => {
                collect_matches(then_body, matches, lowered);
                collect_matches(else_body, matches, lowered);
            }
            SyntaxStmtKind::Bind { .. }
            | SyntaxStmtKind::Print(_)
            | SyntaxStmtKind::Return(_) => {}
        }
    }
}

fn lower_payload_type(value_type: &super::ResolvedPayloadType) -> SchemaType {
    match value_type {
        super::ResolvedPayloadType::Integer => SchemaType::Integer,
        super::ResolvedPayloadType::Bool => SchemaType::Bool,
        super::ResolvedPayloadType::String => SchemaType::String,
        super::ResolvedPayloadType::Record(name) => SchemaType::Record(name.clone()),
        super::ResolvedPayloadType::Enum(name) => SchemaType::Enum(name.clone()),
    }
}

fn lower_record_field_type(
    value_type: &SyntaxRecordFieldType,
    environment: &super::EnumEnvironment,
) -> SchemaType {
    match value_type {
        SyntaxRecordFieldType::Int => SchemaType::Integer,
        SyntaxRecordFieldType::Bool => SchemaType::Bool,
        SyntaxRecordFieldType::String => SchemaType::String,
        SyntaxRecordFieldType::Named(name) if environment.schema(name).is_some() => {
            SchemaType::Enum(name.clone())
        }
        SyntaxRecordFieldType::Named(name) => SchemaType::Record(name.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::{SchemaType, lower_enum_schemas, lower_matches, lower_record_schemas};
    use evo_lexer::lex;
    use evo_parser::parse;

    #[test]
    fn validated_schema_ir_preserves_nominal_kind_and_spans() {
        let source = "record Item\nvalue int\nend\nrecord Holder\nitem Item\ninner Inner\nend\nenum Inner\nUnit\nend\nenum Wrapped\nNone\nCount int\nRecord Item\nNested Inner\nend\n";
        let tokens = lex(source).expect("enum IR source should lex");
        let program = parse(&tokens).expect("enum IR source should parse");
        let environment = super::super::collect_validated_enum_environment(&program)
            .expect("schema IR source should pass semantic and ownership validation");

        let enums = lower_enum_schemas(&environment);
        assert_eq!(enums.len(), 2);
        let wrapped = enums
            .iter()
            .find(|schema| schema.name == "Wrapped")
            .expect("Wrapped enum should be retained in IR");
        assert_eq!(wrapped.span.line, 11);
        assert_eq!(wrapped.variants.len(), 4);
        assert_eq!(wrapped.variants[0].name, "None");
        assert_eq!(wrapped.variants[0].payload_type, None);
        assert_eq!(wrapped.variants[0].span.line, 12);
        assert_eq!(
            wrapped.variants[1].payload_type,
            Some(SchemaType::Integer)
        );
        assert_eq!(
            wrapped.variants[2].payload_type,
            Some(SchemaType::Record("Item".to_owned()))
        );
        assert_eq!(
            wrapped.variants[3].payload_type,
            Some(SchemaType::Enum("Inner".to_owned()))
        );

        let records = lower_record_schemas(&program, &environment);
        let holder = records
            .iter()
            .find(|record| record.name == "Holder")
            .expect("Holder record should be retained in schema IR");
        assert_eq!(holder.span.line, 4);
        assert_eq!(holder.fields.len(), 2);
        assert_eq!(holder.fields[0].name, "item");
        assert_eq!(holder.fields[0].span.line, 5);
        assert_eq!(
            holder.fields[0].value_type,
            SchemaType::Record("Item".to_owned())
        );
        assert_eq!(holder.fields[1].name, "inner");
        assert_eq!(holder.fields[1].span.line, 6);
        assert_eq!(
            holder.fields[1].value_type,
            SchemaType::Enum("Inner".to_owned())
        );
    }

    #[test]
    fn resolved_match_ir_preserves_identity_binding_types_and_return_summary() {
        let source = "enum Inner\nA\nend\nenum Wrapped\nNone\nSome Inner\nend\nfn use(value Wrapped) int\nmatch value\ncase Wrapped.None\nreturn 0\ncase Wrapped.Some(x)\nmatch x\ncase Inner.A\nreturn 1\nend\nend\nend\n";
        let tokens = lex(source).expect("match IR source should lex");
        let program = parse(&tokens).expect("match IR source should parse");
        let environment = super::super::collect_validated_enum_environment(&program)
            .expect("match IR source should pass semantic and ownership validation");
        let matches = super::super::match_validation::collect_match_environment(
            &program,
            &environment,
        )
        .expect("match IR source should have resolved exhaustive matches");

        let lowered = lower_matches(&program, &matches);
        assert_eq!(lowered.len(), 2);

        let outer = &lowered[0];
        assert_eq!(outer.enum_name, "Wrapped");
        assert_eq!(outer.span.line, 9);
        assert!(outer.all_arms_return);
        assert_eq!(outer.arms.len(), 2);
        assert_eq!(outer.arms[0].enum_name, "Wrapped");
        assert_eq!(outer.arms[0].variant_name, "None");
        assert_eq!(outer.arms[0].binding, None);
        assert_eq!(outer.arms[0].span.line, 10);
        assert_eq!(outer.arms[1].variant_name, "Some");
        let binding = outer.arms[1]
            .binding
            .as_ref()
            .expect("payload arm should retain its typed binding");
        assert_eq!(binding.name, "x");
        assert_eq!(binding.value_type, SchemaType::Enum("Inner".to_owned()));
        assert_eq!(binding.span.line, 12);

        let inner = &lowered[1];
        assert_eq!(inner.enum_name, "Inner");
        assert_eq!(inner.span.line, 13);
        assert!(inner.all_arms_return);
        assert_eq!(inner.arms[0].variant_name, "A");
        assert_eq!(inner.arms[0].span.line, 14);
    }
}
