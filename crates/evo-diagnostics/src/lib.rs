use evo_lexer::Span;
use std::path::Path;

const TAB_WIDTH: usize = 4;

#[must_use]
pub fn render_error(path: &Path, source: &str, message: &str, span: Span) -> String {
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
            "error: {message}\n",
            " --> {path}:{line}:{column}\n",
            "{gutter} |\n",
            "{line_label} | {source_line}\n",
            "{gutter} | {padding}{underline}\n"
        ),
        message = message,
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
    use super::render_error;
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
}
