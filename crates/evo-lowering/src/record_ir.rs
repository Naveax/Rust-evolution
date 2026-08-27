use evo_lexer::Span;
use evo_parser::{Program as SyntaxProgram, RecordFieldType as SyntaxFieldType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RecordType {
    Integer,
    Bool,
    String,
    Named(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordFieldIr {
    pub(crate) name: String,
    pub(crate) value_type: RecordType,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordIr {
    pub(crate) name: String,
    pub(crate) fields: Vec<RecordFieldIr>,
    pub(crate) span: Span,
}

#[must_use]
pub(crate) fn lower_record_schemas(program: &SyntaxProgram) -> Vec<RecordIr> {
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
                    value_type: lower_field_type(&field.type_name),
                    span: field.span,
                })
                .collect(),
            span: record.span,
        })
        .collect()
}

fn lower_field_type(field_type: &SyntaxFieldType) -> RecordType {
    match field_type {
        SyntaxFieldType::Int => RecordType::Integer,
        SyntaxFieldType::Bool => RecordType::Bool,
        SyntaxFieldType::String => RecordType::String,
        SyntaxFieldType::Named(name) => RecordType::Named(name.clone()),
    }
}
