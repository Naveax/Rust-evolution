use evo_lexer::Span;
use std::cell::RefCell;

const MAX_PENDING_HELP: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingHelp {
    primary_message: String,
    primary_span: Span,
    help: String,
}

thread_local! {
    static PENDING_HELP: RefCell<Vec<PendingHelp>> = const {
        RefCell::new(Vec::new())
    };
}

/// Registers one bounded text-help line for a matching primary diagnostic.
///
/// Multiple exact `(message, span)` entries may be staged during semantic preparation. The
/// bounded list preserves earlier source-order entries when full rather than evicting them, since
/// lowering reports the earliest reachable error first. Registering the same key replaces its help
/// text deterministically.
pub fn set_help(primary_message: &str, primary_span: Span, help: &str) {
    PENDING_HELP.with(|pending| {
        let mut pending = pending.borrow_mut();
        if let Some(existing) = pending.iter_mut().find(|candidate| {
            candidate.primary_message == primary_message && candidate.primary_span == primary_span
        }) {
            existing.help = help.to_owned();
            return;
        }
        if pending.len() < MAX_PENDING_HELP {
            pending.push(PendingHelp {
                primary_message: primary_message.to_owned(),
                primary_span,
                help: help.to_owned(),
            });
        }
    });
}

/// Clears pending compile-time text-help metadata.
pub fn clear_help() {
    PENDING_HELP.with(|pending| pending.borrow_mut().clear());
}

pub(crate) fn take_help(message: &str, span: Span) -> Option<String> {
    PENDING_HELP.with(|pending| {
        let mut pending = pending.borrow_mut();
        let selected = pending
            .iter()
            .find(|candidate| {
                candidate.primary_message == message && candidate.primary_span == span
            })
            .map(|candidate| candidate.help.clone());
        pending.clear();
        selected
    })
}

#[cfg(test)]
mod tests {
    use super::{clear_help, set_help, take_help};
    use evo_lexer::Span;

    const fn span(line: usize) -> Span {
        Span {
            start: line * 10,
            end: line * 10 + 1,
            line,
            column: 1,
        }
    }

    #[test]
    fn multiple_exact_help_entries_select_only_the_matching_primary() {
        clear_help();
        set_help("first", span(1), "first help");
        set_help("second", span(2), "second help");
        assert_eq!(take_help("second", span(2)).as_deref(), Some("second help"));
        assert_eq!(take_help("first", span(1)), None);
    }

    #[test]
    fn duplicate_exact_key_replaces_help_without_growing_stale_state() {
        clear_help();
        set_help("error", span(1), "old");
        set_help("error", span(1), "new");
        assert_eq!(take_help("error", span(1)).as_deref(), Some("new"));
    }
}
