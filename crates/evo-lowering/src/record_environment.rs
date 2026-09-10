use crate::{
    LowerError, diagnostic_suggestions::register_name_suggestion,
    source_suggestions::register_program_suggestions,
};
use evo_lexer::Span;
use evo_parser::{
    Program as SyntaxProgram, RecordFieldType as SyntaxRecordFieldType, TypeName as SyntaxTypeName,
};
use std::ops::Deref;

mod enums_impl {
    include!("enum_environment.rs");

    type ParameterModes =
        std::collections::HashMap<String, Vec<crate::ParameterPassingMode>>;

    mod ownership_state {
        include!("move_state.rs");
    }

    mod constructor_typing {
        include!("enum_constructor_typing.rs");

        mod ownership {
            include!("enum_ownership.rs");
        }

        mod borrow_ownership {
            include!("enum_borrow_ownership.rs");
        }

        pub(super) use ownership::{OwnershipUseMode, ResolvedOwnershipUse};

        pub(super) fn collect_enum_ownership(
            program: &SyntaxProgram,
            enums: &EnumEnvironment,
            matches: &super::match_validation::MatchEnvironment,
            parameter_modes: &super::ParameterModes,
        ) -> Result<Vec<ResolvedOwnershipUse>, LowerError> {
            borrow_ownership::collect_enum_ownership(program, enums, matches, parameter_modes)
        }

        pub(super) fn validate_enum_ownership(
            program: &SyntaxProgram,
            enums: &EnumEnvironment,
            matches: &super::match_validation::MatchEnvironment,
            parameter_modes: &super::ParameterModes,
        ) -> Result<(), LowerError> {
            collect_enum_ownership(program, enums, matches, parameter_modes).map(|_| ())
        }
    }

    mod static_semantics {
        include!("enum_static_semantics.rs");
    }

    mod match_validation {
        include!("enum_match_validation.rs");
    }

    mod match_sidecar {
        include!("enum_match_sidecar.rs");
    }

    mod ir {
        include!("enum_ir.rs");
    }

    mod ownership_ir {
        include!("enum_ownership_ir.rs");
    }

    mod program_ir {
        include!("enum_program_ir.rs");
    }

    mod executable_ir {
        include!("enum_executable_ir.rs");
    }

    pub(crate) use executable_ir::{
        ExecutableEnumIr, ExecutableEnumProgramIr, ExecutableEnumVariantIr, ExecutableExprIr,
        ExecutableExprKind, ExecutableFunctionIr, ExecutableMatchArmIr, ExecutableMatchBindingIr,
        ExecutableOwnershipMode, ExecutableParameterIr, ExecutableRecordFieldIr,
        ExecutableRecordFieldValueIr, ExecutableRecordIr, ExecutableStmtIr, ExecutableStmtKind,
        ExecutableValueType,
    };

    fn collect_parameter_modes(program: &SyntaxProgram) -> ParameterModes {
        program
            .functions
            .iter()
            .map(|function| {
                (
                    function.name.clone(),
                    crate::borrow_inference::classify_function_parameters(program, function),
                )
            })
            .collect()
    }

    fn collect_validated_enum_state(
        program: &SyntaxProgram,
    ) -> Result<(EnumEnvironment, program_ir::EnumProgramIr, ParameterModes), LowerError> {
        super::register_program_suggestions(program);
        validate_enum_declarations(program)?;
        let environment = collect_enum_environment(program)?;
        let matches = match_validation::collect_match_environment(program, &environment)?;
        constructor_typing::validate_enum_type_semantics(program, &environment)?;
        static_semantics::validate_enum_static_semantics(program, &environment)?;
        match_sidecar::validate_match_sidecar(program, &matches)?;
        let parameter_modes = collect_parameter_modes(program);
        let ownership = constructor_typing::collect_enum_ownership(
            program,
            &environment,
            &matches,
            &parameter_modes,
        )?;
        debug_assert!(
            constructor_typing::validate_enum_ownership(
                program,
                &environment,
                &matches,
                &parameter_modes,
            )
            .is_ok()
        );
        debug_assert!(ownership.iter().all(|usage| {
            let _ = (&usage.value_type, usage.mode);
            !usage.name.is_empty() && usage.span.start < usage.span.end
        }));

        let ownership_uses = ownership_ir::lower_ownership_uses(&ownership);
        debug_assert_eq!(ownership_uses.len(), ownership.len());
        debug_assert!(ownership_uses.iter().all(|usage| {
            let _ = (&usage.value_type, usage.mode);
            !usage.name.is_empty() && usage.span.start < usage.span.end
        }));

        let enums = ir::lower_enum_schemas(&environment);
        debug_assert_eq!(enums.len(), program.enums.len());
        debug_assert!(enums.iter().all(|schema| {
            environment.schema(&schema.name).is_some_and(|resolved| {
                resolved.span == schema.span && resolved.variants.len() == schema.variants.len()
            })
        }));

        let records = ir::lower_record_schemas(program, &environment);
        debug_assert_eq!(records.len(), program.records.len());

        let lowered_matches = ir::lower_matches(program, &matches);
        debug_assert!(
            lowered_matches
                .iter()
                .all(|resolved| matches.match_at(resolved.span.start).is_some())
        );

        let constructors = ir::lower_constructors(program, &environment);
        debug_assert!(
            constructors
                .iter()
                .all(|constructor| environment.schema(&constructor.enum_name).is_some())
        );

        Ok((
            environment,
            program_ir::EnumProgramIr {
                enums,
                records,
                constructors,
                matches: lowered_matches,
                ownership_uses,
            },
            parameter_modes,
        ))
    }

    #[cfg(test)]
    pub(crate) fn collect_validated_enum_environment(
        program: &SyntaxProgram,
    ) -> Result<EnumEnvironment, LowerError> {
        collect_validated_enum_state(program).map(|(environment, _, _)| environment)
    }

    pub(crate) fn collect_validated_enum_program_ir(
        program: &SyntaxProgram,
    ) -> Result<program_ir::EnumProgramIr, LowerError> {
        collect_validated_enum_state(program).map(|(_, lowered, _)| lowered)
    }

    pub(crate) fn collect_executable_enum_program_ir(
        program: &SyntaxProgram,
    ) -> Result<ExecutableEnumProgramIr, LowerError> {
        let (_, validated, parameter_modes) = collect_validated_enum_state(program)?;
        Ok(executable_ir::lower_executable_enum_program_with_parameter_modes(
            program,
            &validated,
            &parameter_modes,
        ))
    }
}

// Records integration harnesses include this file directly without the crate-root
// enum codegen view. In the normal crate these bridge imports are all consumed.
#[allow(unused_imports)]
pub(crate) use enums_impl::{
    ExecutableEnumIr, ExecutableEnumProgramIr, ExecutableEnumVariantIr, ExecutableExprIr,
    ExecutableExprKind, ExecutableFunctionIr, ExecutableMatchArmIr, ExecutableMatchBindingIr,
    ExecutableOwnershipMode, ExecutableParameterIr, ExecutableRecordFieldIr,
    ExecutableRecordFieldValueIr, ExecutableRecordIr, ExecutableStmtIr, ExecutableStmtKind,
    ExecutableValueType,
};

mod records_impl {
    include!("record_environment_records.rs");
}

pub(crate) use records_impl::{
    ConstructorFieldInput, RecordEnvironment as RecordStorage, SemanticType,
};

#[derive(Debug, Clone)]
pub(crate) struct TypeEnvironment {
    records: RecordStorage,
    record_names: Vec<String>,
    function_names: Vec<String>,
}

// Transitional compatibility name for Records v0 callers. New nominal-type work
// should use TypeEnvironment so enum support can share the same semantic boundary.
pub(crate) type RecordEnvironment = TypeEnvironment;

impl Deref for TypeEnvironment {
    type Target = RecordStorage;

    fn deref(&self) -> &Self::Target {
        &self.records
    }
}

impl TypeEnvironment {
    pub(crate) fn record_names(&self) -> impl Iterator<Item = &str> {
        self.record_names.iter().map(String::as_str)
    }

    pub(crate) fn function_names(&self) -> impl Iterator<Item = &str> {
        self.function_names.iter().map(String::as_str)
    }

    pub(crate) fn has_function(&self, name: &str) -> bool {
        self.function_names
            .iter()
            .any(|candidate| candidate == name)
    }

    pub(crate) fn resolve_type_name(
        &self,
        type_name: &SyntaxTypeName,
        span: Span,
    ) -> Result<SemanticType, LowerError> {
        let result = self.records.resolve_type_name(type_name, span);
        if let (Err(error), SyntaxTypeName::Named(name)) = (&result, type_name) {
            register_name_suggestion(&error.message, error.span, name, self.record_names());
        }
        result
    }

    pub(crate) fn validate_constructor(
        &self,
        name: &str,
        fields: &[ConstructorFieldInput],
        constructor_span: Span,
    ) -> Result<SemanticType, LowerError> {
        let result = self
            .records
            .validate_constructor(name, fields, constructor_span);
        if let Err(error) = &result {
            if let Some(schema) = self.records.schema(name) {
                for field in fields {
                    let expected_message = format!(
                        "unknown constructor field {:?} for record {:?}",
                        field.name, name
                    );
                    if error.message == expected_message {
                        register_name_suggestion(
                            &error.message,
                            error.span,
                            &field.name,
                            schema
                                .fields
                                .iter()
                                .map(|candidate| candidate.name.as_str()),
                        );
                        break;
                    }
                }
            } else {
                register_name_suggestion(&error.message, error.span, name, self.record_names());
            }
        }
        result
    }

    pub(crate) fn field_type(
        &self,
        base_type: &SemanticType,
        field_name: &str,
        access_span: Span,
    ) -> Result<SemanticType, LowerError> {
        let result = self.records.field_type(base_type, field_name, access_span);
        if let Err(error) = &result
            && let SemanticType::Record(record_name) = base_type
            && let Some(schema) = self.records.schema(record_name)
        {
            let expected_message =
                format!("unknown field {field_name:?} on record {record_name:?}");
            if error.message == expected_message {
                register_name_suggestion(
                    &error.message,
                    error.span,
                    field_name,
                    schema.fields.iter().map(|field| field.name.as_str()),
                );
            }
        }
        result
    }
}

pub(crate) fn collect_executable_enum_program_ir(
    program: &SyntaxProgram,
) -> Result<ExecutableEnumProgramIr, LowerError> {
    enums_impl::collect_executable_enum_program_ir(program)
}

pub(crate) fn collect_record_environment(
    program: &SyntaxProgram,
) -> Result<RecordEnvironment, LowerError> {
    reject_enum_declarations(program)?;
    register_record_declaration_suggestions(program);

    let records = records_impl::collect_record_environment(program)?;
    let record_names = program
        .records
        .iter()
        .map(|record| record.name.clone())
        .collect();
    let function_names = program
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect();
    Ok(TypeEnvironment {
        records,
        record_names,
        function_names,
    })
}

pub(crate) fn validate_record_declarations(program: &SyntaxProgram) -> Result<(), LowerError> {
    register_program_suggestions(program);
    reject_enum_declarations(program)?;
    register_record_declaration_suggestions(program);
    records_impl::validate_record_declarations(program)
}

fn register_record_declaration_suggestions(program: &SyntaxProgram) {
    let record_names: Vec<&str> = program
        .records
        .iter()
        .map(|record| record.name.as_str())
        .collect();
    for record in &program.records {
        for field in &record.fields {
            if let SyntaxRecordFieldType::Named(name) = &field.type_name
                && !record_names.contains(&name.as_str())
            {
                let message = format!(
                    "unknown record type {name:?} for field {:?} in record {:?}",
                    field.name, record.name
                );
                register_name_suggestion(&message, field.span, name, record_names.iter().copied());
            }
        }
    }
}

fn reject_enum_declarations(program: &SyntaxProgram) -> Result<(), LowerError> {
    if program.enums.is_empty() {
        return Ok(());
    }

    let executable = collect_executable_enum_program_ir(program)?;
    debug_assert_eq!(executable.enums.len(), program.enums.len());
    debug_assert_eq!(executable.records.len(), program.records.len());
    debug_assert_eq!(executable.functions.len(), program.functions.len());
    let _ = executable.statements.len();

    let enum_def = &program.enums[0];
    Err(LowerError {
        message: "enum declarations are parsed, but Enums v0 semantic lowering/codegen is not implemented yet"
            .to_owned(),
        span: enum_def.span,
    })
}

#[cfg(test)]
mod tests {
    use super::{collect_record_environment, validate_record_declarations};
    use evo_lexer::lex;
    use evo_parser::parse;

    fn parse_source(source: &str) -> evo_parser::Program {
        let tokens = lex(source).expect("enum gate source should lex");
        parse(&tokens).expect("enum gate source should parse")
    }

    #[test]
    fn enum_declarations_fail_closed_before_semantic_lowering() {
        let program = parse_source("enum MaybeInt\nNone\nSome int\nend\nprint 1\n");

        let validation_error = validate_record_declarations(&program)
            .expect_err("enum declaration should remain fail-closed");
        assert!(
            validation_error
                .message
                .contains("Enums v0 semantic lowering")
        );
        assert_eq!(validation_error.span.line, 1);

        let collection_error = collect_record_environment(&program)
            .expect_err("enum declaration should not disappear during environment collection");
        assert!(
            collection_error
                .message
                .contains("Enums v0 semantic lowering")
        );
        assert_eq!(collection_error.span.line, 1);
    }

    #[test]
    fn invalid_enum_declarations_are_diagnosed_before_the_fail_closed_gate() {
        let program = parse_source("enum Flag\nOn\nOn\nend\n");
        let error = validate_record_declarations(&program)
            .expect_err("duplicate variants should fail before unsupported enum execution");
        assert!(error.message.contains("duplicate variant name"));
        assert_eq!(error.span.line, 3);
    }

    #[test]
    fn constructor_payload_type_errors_are_diagnosed_before_fail_closed_gate() {
        let program = parse_source(
            "enum MaybeInt\nNone\nSome int\nend\nvalue = true\nwrapped = MaybeInt.Some(value)\n",
        );
        let error = validate_record_declarations(&program)
            .expect_err("constructor payload mismatch should precede unsupported codegen gate");
        assert!(error.message.contains("expects int, found bool"));
        assert_eq!(error.span.line, 6);
    }

    #[test]
    fn match_pattern_errors_are_diagnosed_before_fail_closed_gate() {
        let program = parse_source(
            "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nmatch value\ncase Flag.On\nprint 1\nend\n",
        );
        let error = validate_record_declarations(&program)
            .expect_err("non-exhaustive match should precede unsupported codegen gate");
        assert!(error.message.contains("missing variant(s): Off"));
        assert_eq!(error.span.line, 6);
    }

    #[test]
    fn match_scrutinee_type_errors_are_diagnosed_before_fail_closed_gate() {
        let program = parse_source(
            "enum Flag\nOff\nOn\nend\nmatch true\ncase Flag.Off\nprint 0\ncase Flag.On\nprint 1\nend\n",
        );
        let error = validate_record_declarations(&program)
            .expect_err("non-enum scrutinee should precede unsupported codegen gate");
        assert!(error.message.contains("scrutinee must have an enum type"));
        assert_eq!(error.span.line, 5);
    }

    #[test]
    fn enum_ownership_errors_are_diagnosed_before_fail_closed_gate() {
        let program = parse_source(
            "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nfirst = value\nsecond = value\n",
        );
        let error = validate_record_declarations(&program)
            .expect_err("enum reuse-after-move should precede unsupported codegen gate");
        assert!(error.message.contains("moved enum local \"value\""));
        assert_eq!(error.span.line, 7);
    }
}
