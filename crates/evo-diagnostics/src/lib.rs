mod diagnostic_help;
mod suggestion;

pub use diagnostic_help::{clear_help, set_help};
pub use suggestion::best_suggestion;

use diagnostic_help::take_help;
use evo_lexer::Span;
use std::cell::RefCell;
use std::path::Path;

const TAB_WIDTH: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedLocation {
    pub message: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingRelatedLocation {
    primary_message: String,
    primary_span: Span,
    related: RelatedLocation,
}

thread_local! {
    static PENDING_RELATED_LOCATION: RefCell<Option<PendingRelatedLocation>> = const {
        RefCell::new(None)
    };
}

/// Registers one bounded related source location for the next matching primary diagnostic.
///
/// Lowering uses this as a compile-time diagnostic sidecar so the public lowering error shape and
/// accepted generated-program representation remain unchanged. A render with a non-matching
/// primary consumes and discards the pending location, preventing unrelated diagnostics from
/// inheriting stale move provenance.
pub fn set_related_location(
    primary_message: &str,
    primary_span: Span,
    related_message: &str,
    related_span: Span,
) {
    PENDING_RELATED_LOCATION.with(|pending| {
        *pending.borrow_mut() = Some(PendingRelatedLocation {
            primary_message: primary_message.to_owned(),
            primary_span,
            related: RelatedLocation {
                message: related_message.to_owned(),
                span: related_span,
            },
        });
    });
}

/// Clears pending compile-time related-location metadata.
pub fn clear_related_location() {
    PENDING_RELATED_LOCATION.with(|pending| {
        let _ = pending.borrow_mut().take();
    });
}

#[must_use]
pub fn render_error(path: &Path, source: &str, message: &str, span: Span) -> String {
    let related = take_related_location(message, span);
    let help = take_help(message, span);
    render_error_with_context(
        path,
        source,
        message,
        span,
        related.as_ref(),
        help.as_deref(),
    )
}

#[must_use]
pub fn render_error_with_related(
    path: &Path,
    source: &str,
    message: &str,
    span: Span,
    related: Option<&RelatedLocation>,
) -> String {
    render_error_with_context(path, source, message, span, related, None)
}

fn render_error_with_context(
    path: &Path,
    source: &str,
    message: &str,
    span: Span,
    related: Option<&RelatedLocation>,
    help: Option<&str>,
) -> String {
    let mut rendered = format!("error: {message}\n{}", render_location(path, source, span));
    if let Some(related) = related {
        rendered.push('\n');
        rendered.push_str(&format!(
            "note: {}\n{}",
            related.message,
            render_location(path, source, related.span)
        ));
    }
    if let Some(help) = help {
        rendered.push('\n');
        rendered.push_str(&format!("help: {help}"));
    }
    rendered
}

fn take_related_location(message: &str, span: Span) -> Option<RelatedLocation> {
    PENDING_RELATED_LOCATION.with(|pending| {
        pending
            .borrow_mut()
            .take()
            .filter(|pending| pending.primary_message == message && pending.primary_span == span)
            .map(|pending| pending.related)
    })
}

fn render_location(path: &Path, source: &str, span: Span) -> String {
    let start = clamp_to_char_boundary(source, span.start);
    let end = clamp_to_char_boundary(source, span.end.max(start));
    let line_start = source[..start].rfind('\n').map_or(0, |index| index + 1);
    let line_end = source[start..]
        .find('\n')
        .map_or(source.len(), |offset| start + offset);
    let underline_end = end.min(line_end);

    let line_text = source.get(line_start..line_end).unwrap_or_default();
    let before = source.get(line_start..start).unwrap_or_default();
    let visual_start = visual_width(before, 0);
    let visual_end = if underline_end > start {
        visual_width(source.get(line_start..underline_end).unwrap_or_default(), 0)
    } else {
        visual_start + 1
    };
    let underline_width = visual_end.saturating_sub(visual_start).max(1);

    let line_number = span.line.max(1);
    let column = span.column.max(1);
    let gutter_width = line_number.to_string().len();
    let gutter_padding = " ".repeat(gutter_width);
    let rendered_line = expand_tabs(line_text);

    format!(
        concat!(
            " --> {path}:{line}:{column}\n",
            "{gutter} |\n",
            "{line_label} | {source_line}\n",
            "{gutter} | {padding}{underline}\n"
        ),
        path = path.display(),
        line = line_number,
        column = column,
        gutter = gutter_padding,
        line_label = line_number,
        source_line = rendered_line,
        padding = " ".repeat(visual_start),
        underline = "^".repeat(underline_width),
    )
}

fn clamp_to_char_boundary(source: &str, index: usize) -> usize {
    let mut index = index.min(source.len());
    while !source.is_char_boundary(index) {
        index = index.saturating_sub(1);
    }
    index
}

fn visual_width(text: &str, initial_column: usize) -> usize {
    text.chars().fold(initial_column, |column, ch| {
        if ch == '\t' {
            column + (TAB_WIDTH - (column % TAB_WIDTH))
        } else {
            column + 1
        }
    })
}

fn expand_tabs(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut column = 0;
    for ch in text.chars() {
        if ch == '\t' {
            let spaces = TAB_WIDTH - (column % TAB_WIDTH);
            output.push_str(&" ".repeat(spaces));
            column += spaces;
        } else {
            output.push(ch);
            column += 1;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{
        RelatedLocation, render_error, render_error_with_related, set_help, set_related_location,
    };
    use evo_lexer::{Span, lex};
    use std::path::Path;

    #[test]
    fn renders_lexical_error_with_source_context() {
        let source = "print @\n";
        let error = lex(source).expect_err("invalid character should fail lexing");
        let rendered = render_error(Path::new("sample.evo"), source, &error.message, error.span);

        assert!(rendered.starts_with("error: "));
        assert!(rendered.contains(" --> sample.evo:1:7"));
        assert!(rendered.contains("1 | print @"));
        assert!(rendered.contains("  |       ^"));
    }

    #[test]
    fn zero_width_eof_span_still_gets_a_caret() {
        let source = "repeat 1\n";
        let rendered = render_error(
            Path::new("eof.evo"),
            source,
            "missing 'end'",
            Span {
                start: source.len(),
                end: source.len(),
                line: 2,
                column: 1,
            },
        );

        assert!(rendered.contains(" --> eof.evo:2:1"));
        assert!(rendered.contains("2 | \n"));
        assert!(rendered.ends_with("  | ^\n"));
    }

    #[test]
    fn utf8_byte_offsets_do_not_shift_visual_caret() {
        let source = "é@\n";
        let rendered = render_error(
            Path::new("utf8.evo"),
            source,
            "unexpected character",
            Span {
                start: "é".len(),
                end: "é@".len(),
                line: 1,
                column: 2,
            },
        );

        assert!(rendered.contains("1 | é@"));
        assert!(rendered.contains("  |  ^"));
    }

    #[test]
    fn range_span_underlines_the_whole_range() {
        let source = "abcdef\n";
        let rendered = render_error(
            Path::new("range.evo"),
            source,
            "bad range",
            Span {
                start: 2,
                end: 5,
                line: 1,
                column: 3,
            },
        );

        assert!(rendered.contains("  |   ^^^"));
    }

    #[test]
    fn related_location_reuses_utf8_and_tab_alignment() {
        let source = "é\tmove\nuse\n";
        let primary_start = source.find("use").expect("use exists");
        let related_start = source.find("move").expect("move exists");
        let related = RelatedLocation {
            message: "value was moved here".to_owned(),
            span: Span {
                start: related_start,
                end: related_start + "move".len(),
                line: 1,
                column: 3,
            },
        };
        let rendered = render_error_with_related(
            Path::new("related.evo"),
            source,
            "use of moved local",
            Span {
                start: primary_start,
                end: primary_start + "use".len(),
                line: 2,
                column: 1,
            },
            Some(&related),
        );

        assert!(rendered.contains("note: value was moved here"));
        assert!(rendered.contains(" --> related.evo:1:3"));
        assert!(rendered.contains("1 | é   move"));
        assert!(rendered.contains("  |     ^^^^"));
    }

    #[test]
    fn zero_width_related_eof_span_is_safe() {
        let source = "use\n";
        let related = RelatedLocation {
            message: "move ended here".to_owned(),
            span: Span {
                start: source.len(),
                end: source.len(),
                line: 2,
                column: 1,
            },
        };
        let rendered = render_error_with_related(
            Path::new("related-eof.evo"),
            source,
            "primary",
            Span {
                start: 0,
                end: 3,
                line: 1,
                column: 1,
            },
            Some(&related),
        );

        assert!(rendered.contains("note: move ended here"));
        assert!(rendered.contains(" --> related-eof.evo:2:1"));
        assert!(rendered.ends_with("  | ^\n"));
    }

    #[test]
    fn absent_related_location_preserves_single_span_bytes() {
        let source = "print @\n";
        let span = Span {
            start: 6,
            end: 7,
            line: 1,
            column: 7,
        };
        let direct = render_error_with_related(
            Path::new("same.evo"),
            source,
            "unexpected character",
            span,
            None,
        );
        let ordinary = render_error(Path::new("same.evo"), source, "unexpected character", span);
        assert_eq!(ordinary, direct);
    }

    #[test]
    fn mismatched_pending_related_location_is_dropped_instead_of_leaking() {
        let source = "x\n";
        let span = Span {
            start: 0,
            end: 1,
            line: 1,
            column: 1,
        };
        set_related_location("move error", span, "value was moved here", span);

        let unrelated = render_error(Path::new("stale.evo"), source, "different error", span);
        assert!(!unrelated.contains("note:"));

        let later = render_error(Path::new("stale.evo"), source, "move error", span);
        assert!(!later.contains("note:"));
    }

    #[test]
    fn matching_help_renders_without_changing_the_primary_location() {
        let source = "print coutn\n";
        let start = source.find("coutn").expect("identifier exists");
        let span = Span {
            start,
            end: start + 5,
            line: 1,
            column: 7,
        };
        set_help("unknown local \"coutn\"", span, "did you mean \"count\"?");

        let rendered = render_error(
            Path::new("help.evo"),
            source,
            "unknown local \"coutn\"",
            span,
        );
        assert!(rendered.contains("1 | print coutn"));
        assert!(rendered.contains("help: did you mean \"count\"?"));
    }

    #[test]
    fn help_and_related_location_can_coexist_and_stale_help_is_dropped() {
        let source = "move\nuse\n";
        let primary = Span {
            start: 5,
            end: 8,
            line: 2,
            column: 1,
        };
        let related = Span {
            start: 0,
            end: 4,
            line: 1,
            column: 1,
        };
        set_related_location("primary", primary, "value was moved here", related);
        set_help("primary", primary, "try rebuilding the value first");

        let rendered = render_error(Path::new("both.evo"), source, "primary", primary);
        assert!(rendered.contains("note: value was moved here"));
        assert!(rendered.contains("help: try rebuilding the value first"));

        set_help("old", primary, "stale");
        let unrelated = render_error(Path::new("both.evo"), source, "new", primary);
        assert!(!unrelated.contains("help:"));
        let later = render_error(Path::new("both.evo"), source, "old", primary);
        assert!(!later.contains("help:"));
    }
}
