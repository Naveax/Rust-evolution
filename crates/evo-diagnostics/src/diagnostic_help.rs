use evo_lexer::Span;
use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingHelp {
    primary_message: String,
    primary_span: Span,
    help: String,
}

thread_local! {
    static PENDING_HELP: RefCell<Option<PendingHelp>> = const {
        RefCell::new(None)
    };
}

/// Registers one bounded text-help line for the next matching primary diagnostic.
pub fn set_help(primary_message: &str, primary_span: Span, help: &str) {
    PENDING_HELP.with(|pending| {
        *pending.borrow_mut() = Some(PendingHelp {
            primary_message: primary_message.to_owned(),
            primary_span,
            help: help.to_owned(),
        });
    });
}

/// Clears pending compile-time text-help metadata.
pub fn clear_help() {
    PENDING_HELP.with(|pending| {
        let _ = pending.borrow_mut().take();
    });
}

pub(crate) fn take_help(message: &str, span: Span) -> Option<String> {
    PENDING_HELP.with(|pending| {
        pending
            .borrow_mut()
            .take()
            .filter(|pending| pending.primary_message == message && pending.primary_span == span)
            .map(|pending| pending.help)
    })
}
