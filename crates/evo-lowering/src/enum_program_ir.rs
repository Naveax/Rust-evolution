use super::{
    ir::{EnumConstructorIr, EnumIr, MatchIr, RecordSchemaIr},
    ownership_ir::OwnershipUseIr,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnumProgramIr {
    pub(crate) enums: Vec<EnumIr>,
    pub(crate) records: Vec<RecordSchemaIr>,
    pub(crate) constructors: Vec<EnumConstructorIr>,
    pub(crate) matches: Vec<MatchIr>,
    pub(crate) ownership_uses: Vec<OwnershipUseIr>,
}

#[cfg(test)]
mod tests {
    use super::super::{
        collect_validated_enum_program_ir, ir::SchemaType,
        ownership_ir::OwnershipUseModeIr,
    };
    use evo_lexer::lex;
    use evo_parser::parse;

    #[test]
    fn validated_enum_program_ir_collects_all_pre_codegen_metadata() {
        let source = "enum Flag\nOff\nOn\nend\nrecord Holder\nflag Flag\nend\nvalue = Flag.On()\nmatch value\ncase Flag.Off\nprint 0\ncase Flag.On\nprint 1\nend\n";
        let tokens = lex(source).expect("enum program IR source should lex");
        let program = parse(&tokens).expect("enum program IR source should parse");
        let lowered = collect_validated_enum_program_ir(&program)
            .expect("enum program IR source should pass validated pre-codegen lowering");

        assert_eq!(lowered.enums.len(), 1);
        assert_eq!(lowered.enums[0].name, "Flag");
        assert_eq!(lowered.records.len(), 1);
        assert_eq!(lowered.records[0].name, "Holder");
        assert_eq!(
            lowered.records[0].fields[0].value_type,
            SchemaType::Enum("Flag".to_owned())
        );

        assert_eq!(lowered.constructors.len(), 1);
        assert_eq!(lowered.constructors[0].enum_name, "Flag");
        assert_eq!(lowered.constructors[0].variant_name, "On");
        assert_eq!(lowered.constructors[0].span.line, 8);

        assert_eq!(lowered.matches.len(), 1);
        assert_eq!(lowered.matches[0].enum_name, "Flag");
        assert_eq!(lowered.matches[0].arms.len(), 2);
        assert_eq!(lowered.matches[0].span.line, 9);

        assert_eq!(lowered.ownership_uses.len(), 1);
        assert_eq!(lowered.ownership_uses[0].name, "value");
        assert_eq!(
            lowered.ownership_uses[0].value_type,
            SchemaType::Enum("Flag".to_owned())
        );
        assert_eq!(
            lowered.ownership_uses[0].mode,
            OwnershipUseModeIr::Consume
        );
        assert_eq!(lowered.ownership_uses[0].span.line, 9);
    }
}
