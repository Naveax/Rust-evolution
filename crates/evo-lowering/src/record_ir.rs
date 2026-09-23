use evo_lexer::Span;
use evo_parser::{
    Program as SyntaxProgram, RecordFieldType as SyntaxFieldType, TypeName as SyntaxTypeName,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordHandleType {
    Integer,
    Bool,
    String,
    Record(String),
    SharedOwner(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordType {
    Integer,
    Bool,
    String,
    Named(String),
    Handle(RecordHandleType),
}

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
        SyntaxFieldType::Handle(payload) => RecordType::Handle(lower_handle_type(payload)),
    }
}

fn lower_handle_type(type_name: &SyntaxTypeName) -> RecordHandleType {
    match type_name {
        SyntaxTypeName::Int => RecordHandleType::Integer,
        SyntaxTypeName::Bool => RecordHandleType::Bool,
        SyntaxTypeName::String => RecordHandleType::String,
        SyntaxTypeName::Named(name) => RecordHandleType::Record(name.clone()),
        SyntaxTypeName::SharedOwner(name) => RecordHandleType::SharedOwner(name.clone()),
        SyntaxTypeName::WeakOwner(_)
        | SyntaxTypeName::SharedRef(_)
        | SyntaxTypeName::Sequence(_)
        | SyntaxTypeName::Arena(_)
        | SyntaxTypeName::Handle(_) => {
            unreachable!("parser restricts record handle fields to arena payload types")
        }
    }
}
