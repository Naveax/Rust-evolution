use evo_lexer::{Token, TokenKind};

#[derive(Debug, Clone, Copy)]
struct LineSlice {
    number: usize,
    start: usize,
    end: usize,
}

#[must_use]
pub fn format_source(source: &str, tokens: &[Token]) -> String {
    if source.is_empty() {
        return String::new();
    }

    let mut output = String::with_capacity(source.len() + 1);
    let mut depth = 0usize;

    for line in line_slices(source) {
        let line_tokens: Vec<&Token> = tokens
            .iter()
            .filter(|token| {
                token.span.line == line.number
                    && !matches!(token.kind, TokenKind::Newline | TokenKind::Eof)
            })
            .collect();

        let first_kind = line_tokens.first().map(|token| &token.kind);
        if first_kind.is_some_and(|kind| matches!(kind, TokenKind::End | TokenKind::Else)) {
            depth = depth.saturating_sub(1);
        }

        let comment = comment_on_line(source, line, &line_tokens);
        if line_tokens.is_empty() {
            if let Some(comment) = comment {
                output.push_str(&"    ".repeat(depth));
                output.push_str(comment);
            }
        } else {
            output.push_str(&"    ".repeat(depth));
            output.push_str(&render_code(source, &line_tokens));
            if let Some(comment) = comment {
                output.push_str("  ");
                output.push_str(comment);
            }
        }
        output.push('\n');

        if first_kind
            .is_some_and(|kind| matches!(kind, TokenKind::Repeat | TokenKind::If | TokenKind::Else))
        {
            depth += 1;
        }
    }

    output
}

fn line_slices(source: &str) -> Vec<LineSlice> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    let mut number = 1usize;

    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            lines.push(LineSlice {
                number,
                start,
                end: index,
            });
            number += 1;
            start = index + 1;
        }
    }

    if start < source.len() {
        lines.push(LineSlice {
            number,
            start,
            end: source.len(),
        });
    }

    lines
}

fn comment_on_line<'a>(source: &'a str, line: LineSlice, tokens: &[&Token]) -> Option<&'a str> {
    let search_start = tokens
        .last()
        .map_or(line.start, |token| token.span.end.min(line.end));
    let gap = source.get(search_start..line.end)?;
    let comment_offset = gap.find('#')?;
    let comment = &gap[comment_offset..];
    Some(comment.trim_end_matches([' ', '\t', '\r']))
}

fn render_code(source: &str, tokens: &[&Token]) -> String {
    let mut output = String::new();

    for (index, token) in tokens.iter().enumerate() {
        if index > 0 && needs_space(tokens, index) {
            output.push(' ');
        }
        output.push_str(
            source
                .get(token.span.start..token.span.end)
                .unwrap_or_default(),
        );
    }

    output
}

fn needs_space(tokens: &[&Token], index: usize) -> bool {
    let previous = &tokens[index - 1].kind;
    let current = &tokens[index].kind;
    let previous_unary_minus = is_unary_minus(tokens, index - 1);
    let current_unary_minus = is_unary_minus(tokens, index);

    if matches!(current, TokenKind::RParen) || matches!(previous, TokenKind::LParen) {
        return false;
    }
    if previous_unary_minus {
        return false;
    }

    if matches!(current, TokenKind::LParen) {
        return is_expression_prefix(previous)
            || matches!(previous, TokenKind::Equal)
            || is_binary_operator(previous, previous_unary_minus);
    }

    if current_unary_minus {
        return is_expression_prefix(previous)
            || matches!(previous, TokenKind::Equal)
            || is_binary_operator(previous, previous_unary_minus);
    }

    if matches!(current, TokenKind::Equal) || is_binary_operator(current, current_unary_minus) {
        return true;
    }

    is_expression_prefix(previous)
        || matches!(previous, TokenKind::Equal)
        || is_binary_operator(previous, previous_unary_minus)
}

fn is_expression_prefix(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Print | TokenKind::Repeat | TokenKind::If | TokenKind::Not
    )
}

fn is_unary_minus(tokens: &[&Token], index: usize) -> bool {
    if !matches!(tokens[index].kind, TokenKind::Minus) {
        return false;
    }
    if index == 0 {
        return true;
    }

    let previous = &tokens[index - 1].kind;
    matches!(previous, TokenKind::Equal | TokenKind::LParen)
        || is_expression_prefix(previous)
        || is_binary_operator(previous, false)
}

fn is_binary_operator(kind: &TokenKind, unary_minus: bool) -> bool {
    matches!(
        kind,
        TokenKind::Plus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::EqualEqual
            | TokenKind::BangEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::And
            | TokenKind::Or
    ) || (matches!(kind, TokenKind::Minus) && !unary_minus)
}

#[cfg(test)]
mod tests {
    use super::format_source;
    use evo_lexer::lex;

    fn format(source: &str) -> String {
        let tokens = lex(source).expect("format test source should lex");
        format_source(source, &tokens)
    }

    #[test]
    fn normalizes_binding_print_and_expression_spacing() {
        assert_eq!(format("x=1\nprint(1+2*3)\n"), "x = 1\nprint (1 + 2 * 3)\n");
    }

    #[test]
    fn formats_comparisons_and_if_else_indentation() {
        let source = concat!(
            "x=1\n",
            "if(x+2>=3)# yes\n",
            "print true\n",
            "else# no\n",
            "print false\n",
            "end\n"
        );
        let expected = concat!(
            "x = 1\n",
            "if (x + 2 >= 3)  # yes\n",
            "    print true\n",
            "else  # no\n",
            "    print false\n",
            "end\n"
        );
        assert_eq!(format(source), expected);
    }

    #[test]
    fn formats_logical_keywords_with_canonical_spacing() {
        let source = "if(true and not(false)or true)\nprint true\nend\n";
        let expected = "if (true and not (false) or true)\n    print true\nend\n";
        assert_eq!(format(source), expected);
    }

    #[test]
    fn logical_formatter_is_idempotent() {
        let source = "if true and not (false or true)\nprint false\nend\n";
        let once = format(source);
        assert_eq!(format(&once), once);
    }

    #[test]
    fn keeps_unary_minus_attached_after_comparison_operator() {
        assert_eq!(
            format("if -1<-2\nprint 1\nend\n"),
            "if -1 < -2\n    print 1\nend\n"
        );
    }

    #[test]
    fn keeps_unary_minus_attached_and_binary_minus_spaced() {
        assert_eq!(
            format("x=-1\nprint x--2\nrepeat -1\nend\n"),
            "x = -1\nprint x - -2\nrepeat -1\nend\n"
        );
    }

    #[test]
    fn indents_nested_repeat_blocks_and_comments() {
        let source = concat!(
            "repeat 2# outer\n",
            "# body\n",
            "repeat 1 # inner\n",
            "print \"#not-comment\"# tail\n",
            "end# inner end\n",
            "end\n"
        );
        let expected = concat!(
            "repeat 2  # outer\n",
            "    # body\n",
            "    repeat 1  # inner\n",
            "        print \"#not-comment\"  # tail\n",
            "    end  # inner end\n",
            "end\n"
        );
        assert_eq!(format(source), expected);
    }

    #[test]
    fn preserves_raw_string_spelling_and_escaped_hashes() {
        let source = "print \"a\\n#b\\t\\\"c\"# keep\n";
        assert_eq!(format(source), "print \"a\\n#b\\t\\\"c\"  # keep\n");
    }

    #[test]
    fn preserves_blank_lines_and_adds_final_newline() {
        assert_eq!(format("x=1\n\n\nprint x"), "x = 1\n\n\nprint x\n");
    }

    #[test]
    fn formatter_is_idempotent() {
        let source = concat!(
            "if true # branch\n",
            "repeat 2 # outer\n",
            "x= -1\n",
            "print(x+2)# value\n",
            "end\n",
            "else\n",
            "print 0\n",
            "end\n"
        );
        let once = format(source);
        let twice = format(&once);
        assert_eq!(twice, once);
    }
}
