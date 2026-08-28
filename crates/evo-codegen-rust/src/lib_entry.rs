mod legacy {
    include!("lib.rs");
}

use evo_lowering::Program;
pub use legacy::{GeneratedRust, SourceMapping};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenError {
    message: String,
    span: Option<evo_lexer::Span>,
}

impl CodegenError {
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub const fn span(&self) -> Option<evo_lexer::Span> {
        self.span
    }
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CodegenError {}

#[must_use]
pub fn generate_lowered_rust(program: &Program) -> String {
    assert_legacy_program(program);
    legacy::generate_lowered_rust(program)
}

#[must_use]
pub fn generate_lowered_rust_with_map(program: &Program) -> GeneratedRust {
    assert_legacy_program(program);
    legacy::generate_lowered_rust_with_map(program)
}

pub fn try_generate_lowered_rust(program: &Program) -> Result<String, CodegenError> {
    reject_unimplemented_enum_codegen(program)?;
    Ok(legacy::generate_lowered_rust(program))
}

pub fn try_generate_lowered_rust_with_map(
    program: &Program,
) -> Result<GeneratedRust, CodegenError> {
    reject_unimplemented_enum_codegen(program)?;
    Ok(legacy::generate_lowered_rust_with_map(program))
}

fn reject_unimplemented_enum_codegen(program: &Program) -> Result<(), CodegenError> {
    if !program.has_enum_program() {
        return Ok(());
    }

    Err(CodegenError {
        message: "Enums v0 executable lowering is complete, but Rust enum/match codegen is not implemented yet"
            .to_owned(),
        span: program.enum_source_span(),
    })
}

fn assert_legacy_program(program: &Program) {
    assert!(
        !program.has_enum_program(),
        "enum-enabled Program reached legacy infallible Rust codegen; use try_generate_lowered_rust or try_generate_lowered_rust_with_map"
    );
}

#[cfg(test)]
mod fallible_api_tests {
    use super::{
        generate_lowered_rust, generate_lowered_rust_with_map, try_generate_lowered_rust,
        try_generate_lowered_rust_with_map,
    };
    use evo_lexer::lex;
    use evo_lowering::lower;
    use evo_parser::parse;

    fn lower_source(source: &str) -> evo_lowering::Program {
        let tokens = lex(source).expect("fallible codegen source should lex");
        let syntax = parse(&tokens).expect("fallible codegen source should parse");
        lower(&syntax).expect("fallible codegen source should lower")
    }

    #[test]
    fn fallible_codegen_matches_legacy_output_for_existing_programs() {
        let program = lower_source("x = 1\nprint x\n");
        assert_eq!(
            try_generate_lowered_rust(&program).expect("legacy program should generate"),
            generate_lowered_rust(&program)
        );

        let generated = try_generate_lowered_rust_with_map(&program)
            .expect("legacy program should generate with source mappings");
        assert_eq!(generated, generate_lowered_rust_with_map(&program));
    }

    #[test]
    fn enum_program_fails_closed_before_legacy_rust_emission() {
        let program = lower_source(
            "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nmatch value\ncase Flag.Off\nprint 0\ncase Flag.On\nprint 1\nend\n",
        );
        assert!(program.has_enum_program());

        let error = try_generate_lowered_rust_with_map(&program)
            .expect_err("enum Rust emission should remain deliberately closed");
        assert!(error.message().contains("Rust enum/match codegen"));
        assert_eq!(error.span().map(|span| span.line), Some(1));
    }

    #[test]
    #[should_panic(expected = "enum-enabled Program reached legacy infallible Rust codegen")]
    fn legacy_infallible_codegen_never_silently_drops_enum_ir() {
        let program = lower_source("enum Flag\nOff\nOn\nend\nprint 1\n");
        let _ = generate_lowered_rust(&program);
    }
}
