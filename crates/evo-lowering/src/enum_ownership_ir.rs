use evo_lexer::Span;

use super::{
    ResolvedPayloadType,
    constructor_typing::{OwnershipUseMode, ResolvedOwnershipUse},
    ir::SchemaType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnershipUseModeIr {
    Inspect,
    Consume,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnershipUseIr {
    pub(crate) name: String,
    pub(crate) value_type: SchemaType,
    pub(crate) mode: OwnershipUseModeIr,
    pub(crate) span: Span,
}

pub(super) fn lower_ownership_uses(uses: &[ResolvedOwnershipUse]) -> Vec<OwnershipUseIr> {
    uses.iter()
        .map(|usage| OwnershipUseIr {
            name: usage.name.clone(),
            value_type: lower_value_type(&usage.value_type),
            mode: match usage.mode {
                OwnershipUseMode::Inspect => OwnershipUseModeIr::Inspect,
                OwnershipUseMode::Consume => OwnershipUseModeIr::Consume,
            },
            span: usage.span,
        })
        .collect()
}

fn lower_value_type(value_type: &ResolvedPayloadType) -> SchemaType {
    match value_type {
        ResolvedPayloadType::Integer => SchemaType::Integer,
        ResolvedPayloadType::Bool => SchemaType::Bool,
        ResolvedPayloadType::String => SchemaType::String,
        ResolvedPayloadType::Record(name) => SchemaType::Record(name.clone()),
        ResolvedPayloadType::Enum(name) => SchemaType::Enum(name.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::{OwnershipUseModeIr, SchemaType, lower_ownership_uses};
    use evo_lexer::lex;
    use evo_parser::parse;

    #[test]
    fn ownership_ir_preserves_modes_nominal_types_and_spans() {
        let source = "enum Flag\nOff\nOn\nend\nn = 1\nvalue = Flag.On()\nprint n\nmoved = value\n";
        let tokens = lex(source).expect("ownership IR source should lex");
        let program = parse(&tokens).expect("ownership IR source should parse");
        let enums = super::super::collect_enum_environment(&program)
            .expect("ownership IR enum environment should resolve");
        let matches = super::super::match_validation::collect_match_environment(&program, &enums)
            .expect("ownership IR match environment should resolve");
        super::super::constructor_typing::validate_enum_type_semantics(&program, &enums)
            .expect("ownership IR source should type-check");
        let uses = super::super::constructor_typing::collect_enum_ownership(
            &program,
            &enums,
            &matches,
        )
        .expect("ownership IR source should validate ownership");

        let lowered = lower_ownership_uses(&uses);
        assert_eq!(lowered.len(), 2);
        assert_eq!(lowered[0].name, "n");
        assert_eq!(lowered[0].value_type, SchemaType::Integer);
        assert_eq!(lowered[0].mode, OwnershipUseModeIr::Inspect);
        assert_eq!(lowered[0].span.line, 7);

        assert_eq!(lowered[1].name, "value");
        assert_eq!(
            lowered[1].value_type,
            SchemaType::Enum("Flag".to_owned())
        );
        assert_eq!(lowered[1].mode, OwnershipUseModeIr::Consume);
        assert_eq!(lowered[1].span.line, 8);
    }
}
