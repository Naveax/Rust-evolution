use evo_lexer::Span;
use evo_parser::{Program as SyntaxProgram, RecordFieldType as SyntaxRecordFieldType};

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
    use super::{SchemaType, lower_enum_schemas, lower_record_schemas};
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
}
