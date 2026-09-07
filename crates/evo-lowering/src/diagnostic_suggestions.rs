use evo_diagnostics::{best_suggestion, set_help};
use evo_lexer::Span;

pub(crate) fn register_name_suggestion<'a>(
    message: &str,
    span: Span,
    input: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) {
    if let Some(candidate) = best_suggestion(input, candidates) {
        set_help(message, span, &format!("did you mean {candidate:?}?"));
    }
}
