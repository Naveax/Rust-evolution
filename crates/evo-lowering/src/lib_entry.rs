mod diagnostic_context;

include!("lib.rs");

pub use diagnostic_context::{LowerFailure, LowerRelated};

pub fn lower_with_diagnostics(program: &evo_parser::Program) -> Result<Program, LowerFailure> {
    diagnostic_context::clear();
    match lower(program) {
        Ok(program) => {
            diagnostic_context::clear();
            Ok(program)
        }
        Err(error) => {
            let related = diagnostic_context::take(&error);
            Err(LowerFailure { error, related })
        }
    }
}

pub mod enum_codegen_view;
