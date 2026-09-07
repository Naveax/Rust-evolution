use crate::LowerError;
use evo_lexer::Span;
use std::cell::RefCell;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerRelated {
    pub message: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerFailure {
    pub error: LowerError,
    pub related: Option<LowerRelated>,
}

impl fmt::Display for LowerFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}

impl Error for LowerFailure {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingLowerRelated {
    primary_message: String,
    primary_span: Span,
    related: LowerRelated,
}

thread_local! {
    static PENDING_LOWER_RELATED: RefCell<Option<PendingLowerRelated>> = const {
        RefCell::new(None)
    };
}

pub(crate) fn set(
    primary_message: &str,
    primary_span: Span,
    related_message: &str,
    related_span: Span,
) {
    PENDING_LOWER_RELATED.with(|pending| {
        *pending.borrow_mut() = Some(PendingLowerRelated {
            primary_message: primary_message.to_owned(),
            primary_span,
            related: LowerRelated {
                message: related_message.to_owned(),
                span: related_span,
            },
        });
    });
}

pub(crate) fn clear() {
    PENDING_LOWER_RELATED.with(|pending| {
        let _ = pending.borrow_mut().take();
    });
}

pub(crate) fn take(error: &LowerError) -> Option<LowerRelated> {
    PENDING_LOWER_RELATED.with(|pending| {
        pending
            .borrow_mut()
            .take()
            .filter(|pending| {
                pending.primary_message == error.message && pending.primary_span == error.span
            })
            .map(|pending| pending.related)
    })
}

#[cfg(test)]
mod tests {
    use super::{clear, set, take};
    use crate::LowerError;
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
    fn matching_primary_drains_structured_related_location() {
        clear();
        set("move error", span(4), "value was moved here", span(2));
        let error = LowerError {
            message: "move error".to_owned(),
            span: span(4),
        };

        let related = take(&error).expect("matching related location should be retained");
        assert_eq!(related.message, "value was moved here");
        assert_eq!(related.span, span(2));
        assert!(take(&error).is_none());
    }

    #[test]
    fn mismatched_primary_drops_stale_context() {
        clear();
        set("move error", span(4), "value was moved here", span(2));
        let other = LowerError {
            message: "different error".to_owned(),
            span: span(4),
        };
        assert!(take(&other).is_none());

        let original = LowerError {
            message: "move error".to_owned(),
            span: span(4),
        };
        assert!(take(&original).is_none());
    }
}
