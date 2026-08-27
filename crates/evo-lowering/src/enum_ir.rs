use crate::record_ir::SchemaType;
use evo_lexer::Span;

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

fn lower_payload_type(value_type: &super::ResolvedPayloadType) -> SchemaType {
    match value_type {
        super::ResolvedPayloadType::Integer => SchemaType::Integer,
        super::ResolvedPayloadType::Bool => SchemaType::Bool,
        super::ResolvedPayloadType::String => SchemaType::String,
        super::ResolvedPayloadType::Record(name) => SchemaType::Record(name.clone()),
        super::ResolvedPayloadType::Enum(name) => SchemaType::Enum(name.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::lower_enum_schemas;
    use crate::record_ir::{SchemaType, lower_record_schemas_with_enum_classifier};
    use evo_lexer::lex;
    use evo_parser::parse;

    #[test]
    fn validated_schema_ir_preserves_nominal_kind_and_spans() {
        let source = "record Item\nvalue int\nend\nrecord Holder\nitem Item\ninner Inner\nend\nenum Inner\nUnit\nend\nenum Wrapped\nNone\nCount int\nRecord Item\nNested Inner\nend\n";
        let tokens = lex(source).expect("enum IR source should lex");
        let program = parse(&tokens).expect("enum IR source should parse");
        let environment = super::super::collect_validated_enum_environment(&program)
            .expect("enum IR source should pass semantic and ownership validation");
        let schemas = lower_enum_schemas(&environment);

        assert_eq!(schemas.len(), 2);
        let wrapped = schemas
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
        assert_eq!(wrapped.variants[1].span.line, 13);
        assert_eq!(
            wrapped.variants[2].payload_type,
            Some(SchemaType::Record("Item".to_owned()))
        );
        assert_eq!(wrapped.variants[2].span.line, 14);
        assert_eq!(
            wrapped.variants[3].payload_type,
            Some(SchemaType::Enum("Inner".to_owned()))
        );
        assert_eq!(wrapped.variants[3].span.line, 15);

        let records = lower_record_schemas_with_enum_classifier(&program, |name| {
            environment.schema(name).is_some()
        });
        let holder = records
            .iter()
            .find(|record| record.name == "Holder")
            .expect("Holder record should be retained in IR");
        assert_eq!(holder.fields.len(), 2);
        assert_eq!(
            holder.fields[0].value_type,
            SchemaType::Record("Item".to_owned())
        );
        assert_eq!(
            holder.fields[1].value_type,
            SchemaType::Enum("Inner".to_owned())
        );
    }
}
