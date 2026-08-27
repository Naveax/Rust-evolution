use crate::{
    LowerError,
    record_environment::{ConstructorFieldInput, RecordEnvironment, SemanticType},
};
use evo_lexer::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoweredConstructorFields {
    pub(crate) value_type: SemanticType,
    pub(crate) fields: Vec<ConstructorFieldInput>,
}

pub(crate) fn lower_constructor_fields(
    records: &RecordEnvironment,
    name: &str,
    fields: Vec<ConstructorFieldInput>,
    constructor_span: Span,
) -> Result<LoweredConstructorFields, LowerError> {
    let value_type = records.validate_constructor(name, &fields, constructor_span)?;
    let schema = records
        .schema(name)
        .expect("validated constructor names resolve to a record schema");

    let mut ordered = Vec::with_capacity(fields.len());
    for declared in &schema.fields {
        let supplied = fields
            .iter()
            .find(|field| field.name == declared.name)
            .expect("constructor validation guarantees every declared field is supplied");
        ordered.push(supplied.clone());
    }

    Ok(LoweredConstructorFields {
        value_type,
        fields: ordered,
    })
}
