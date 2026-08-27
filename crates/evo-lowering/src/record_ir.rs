use evo_lexer::Span;
use evo_parser::{Program as SyntaxProgram, RecordFieldType as SyntaxFieldType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaType {
    Integer,
    Bool,
    String,
    Record(String),
    Enum(String),
}

pub type RecordType = SchemaType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFieldIr {
    pub name: String,
    pub value_type: RecordType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordIr {
    pub name: String,
    pub fields: Vec<RecordFieldIr>,
    pub span: Span,
}

#[must_use]
pub(crate) fn lower_record_schemas(program: &SyntaxProgram) -> Vec<RecordIr> {
    lower_record_schemas_with_enum_classifier(program, |_| false)
}

#[must_use]
pub(crate) fn lower_record_schemas_with_enum_classifier(
    program: &SyntaxProgram,
    is_enum: impl Fn(&str) -> bool,
) -> Vec<RecordIr> {
    program
        .records
        .iter()
        .map(|record| RecordIr {
            name: record.name.clone(),
            fields: record
                .fields
                .iter()
                .map(|field| RecordFieldIr {
                    name: field.name.clone(),
                    value_type: lower_field_type(&field.type_name, &is_enum),
                    span: field.span,
                })
                .collect(),
            span: record.span,
        })
        .collect()
}

fn lower_field_type(field_type: &SyntaxFieldType, is_enum: &impl Fn(&str) -> bool) -> RecordType {
    match field_type {
        SyntaxFieldType::Int => SchemaType::Integer,
        SyntaxFieldType::Bool => SchemaType::Bool,
        SyntaxFieldType::String => SchemaType::String,
        SyntaxFieldType::Named(name) if is_enum(name) => SchemaType::Enum(name.clone()),
        SyntaxFieldType::Named(name) => SchemaType::Record(name.clone()),
    }
}
