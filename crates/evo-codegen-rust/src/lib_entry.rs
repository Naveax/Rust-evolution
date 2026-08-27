include!("lib.rs");

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

pub fn try_generate_lowered_rust(program: &Program) -> Result<String, CodegenError> {
    Ok(generate_lowered_rust(program))
}

pub fn try_generate_lowered_rust_with_map(
    program: &Program,
) -> Result<GeneratedRust, CodegenError> {
    Ok(generate_lowered_rust_with_map(program))
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
}
