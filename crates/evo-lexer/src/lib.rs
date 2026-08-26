use std::error::Error;
use std::fmt;
use std::iter::Peekable;
use std::str::CharIndices;

const MAX_RECOVERED_ERRORS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    #[must_use]
    pub const fn join(self, other: Self) -> Self {
        Self {
            start: self.start,
            end: other.end,
            line: self.line,
            column: self.column,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Identifier(String),
    Integer(i64),
    StringLiteral(String),
    Print,
    Repeat,
    If,
    Else,
    End,
    True,
    False,
    InputInt,
    And,
    Or,
    Not,
    Fn,
    Return,
    TypeInt,
    TypeBool,
    TypeString,
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    LParen,
    RParen,
    Comma,
    Newline,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}:{}",
            self.message, self.span.line, self.span.column
        )
    }
}

impl Error for LexError {}

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).lex_all()
}

pub fn lex_recovering(source: &str) -> Result<Vec<Token>, Vec<LexError>> {
    Lexer::new(source).lex_all_recovering()
}

struct Lexer<'a> {
    source: &'a str,
    chars: Peekable<CharIndices<'a>>,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            line: 1,
            column: 1,
        }
    }

    fn lex_all(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        while let Some((byte, ch)) = self.peek() {
            let line = self.line;
            let column = self.column;
            match ch {
                ' ' | '\t' | '\r' => {
                    let _ = self.bump();
                }
                '\n' => {
                    let _ = self.bump();
                    tokens.push(Token {
                        kind: TokenKind::Newline,
                        span: Span {
                            start: byte,
                            end: byte + 1,
                            line,
                            column,
                        },
                    });
                }
                '#' => self.skip_comment(),
                '+' => tokens.push(self.single(TokenKind::Plus)),
                '-' => tokens.push(self.single(TokenKind::Minus)),
                '*' => tokens.push(self.single(TokenKind::Star)),
                '/' => tokens.push(self.single(TokenKind::Slash)),
                '=' => tokens.push(self.optional_equal(TokenKind::Equal, TokenKind::EqualEqual)),
                '!' => tokens.push(self.bang_equal()?),
                '<' => tokens.push(self.optional_equal(TokenKind::Less, TokenKind::LessEqual)),
                '>' => {
                    tokens.push(self.optional_equal(TokenKind::Greater, TokenKind::GreaterEqual))
                }
                '(' => tokens.push(self.single(TokenKind::LParen)),
                ')' => tokens.push(self.single(TokenKind::RParen)),
                ',' => tokens.push(self.single(TokenKind::Comma)),
                '"' => tokens.push(self.lex_string()?),
                c if c.is_ascii_digit() => tokens.push(self.lex_number()?),
                c if is_ident_start(c) => tokens.push(self.lex_identifier()),
                _ => {
                    let end = byte + ch.len_utf8();
                    return Err(LexError {
                        message: format!("unexpected character {ch:?}"),
                        span: Span {
                            start: byte,
                            end,
                            line,
                            column,
                        },
                    });
                }
            }
        }

        tokens.push(self.eof_token());
        Ok(tokens)
    }

    fn lex_all_recovering(mut self) -> Result<Vec<Token>, Vec<LexError>> {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        while let Some((byte, ch)) = self.peek() {
            let line = self.line;
            let column = self.column;
            match ch {
                ' ' | '\t' | '\r' => {
                    let _ = self.bump();
                }
                '\n' => {
                    let _ = self.bump();
                    tokens.push(Token {
                        kind: TokenKind::Newline,
                        span: Span {
                            start: byte,
                            end: byte + 1,
                            line,
                            column,
                        },
                    });
                }
                '#' => self.skip_comment(),
                '+' => tokens.push(self.single(TokenKind::Plus)),
                '-' => tokens.push(self.single(TokenKind::Minus)),
                '*' => tokens.push(self.single(TokenKind::Star)),
                '/' => tokens.push(self.single(TokenKind::Slash)),
                '=' => tokens.push(self.optional_equal(TokenKind::Equal, TokenKind::EqualEqual)),
                '!' => match self.bang_equal() {
                    Ok(token) => tokens.push(token),
                    Err(error) => errors.push(error),
                },
                '<' => tokens.push(self.optional_equal(TokenKind::Less, TokenKind::LessEqual)),
                '>' => {
                    tokens.push(self.optional_equal(TokenKind::Greater, TokenKind::GreaterEqual))
                }
                '(' => tokens.push(self.single(TokenKind::LParen)),
                ')' => tokens.push(self.single(TokenKind::RParen)),
                ',' => tokens.push(self.single(TokenKind::Comma)),
                '"' => match self.lex_string() {
                    Ok(token) => tokens.push(token),
                    Err(error) => {
                        errors.push(error);
                        self.synchronize_string_recovery();
                    }
                },
                c if c.is_ascii_digit() => match self.lex_number() {
                    Ok(token) => tokens.push(token),
                    Err(error) => errors.push(error),
                },
                c if is_ident_start(c) => tokens.push(self.lex_identifier()),
                _ => {
                    let end = byte + ch.len_utf8();
                    errors.push(LexError {
                        message: format!("unexpected character {ch:?}"),
                        span: Span {
                            start: byte,
                            end,
                            line,
                            column,
                        },
                    });
                    let _ = self.bump();
                }
            }

            if errors.len() >= MAX_RECOVERED_ERRORS {
                return Err(errors);
            }
        }

        if errors.is_empty() {
            tokens.push(self.eof_token());
            Ok(tokens)
        } else {
            Err(errors)
        }
    }

    fn eof_token(&self) -> Token {
        let end = self.source.len();
        Token {
            kind: TokenKind::Eof,
            span: Span {
                start: end,
                end,
                line: self.line,
                column: self.column,
            },
        }
    }

    fn peek(&mut self) -> Option<(usize, char)> {
        self.chars.peek().copied()
    }

    fn bump(&mut self) -> Option<(usize, char, usize, usize)> {
        let (byte, ch) = self.chars.next()?;
        let line = self.line;
        let column = self.column;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some((byte, ch, line, column))
    }

    fn single(&mut self, kind: TokenKind) -> Token {
        let (start, ch, line, column) = self.bump().expect("single token requires input");
        Token {
            kind,
            span: Span {
                start,
                end: start + ch.len_utf8(),
                line,
                column,
            },
        }
    }

    fn optional_equal(&mut self, single: TokenKind, with_equal: TokenKind) -> Token {
        let (start, first, line, column) = self.bump().expect("operator requires input");
        let mut end = start + first.len_utf8();
        let kind = if matches!(self.peek(), Some((_, '='))) {
            let (equal_start, equal, _, _) = self.bump().expect("peeked '='");
            end = equal_start + equal.len_utf8();
            with_equal
        } else {
            single
        };
        Token {
            kind,
            span: Span {
                start,
                end,
                line,
                column,
            },
        }
    }

    fn bang_equal(&mut self) -> Result<Token, LexError> {
        let (start, bang, line, column) = self.bump().expect("'!' requires input");
        let single_end = start + bang.len_utf8();
        if matches!(self.peek(), Some((_, '='))) {
            let (equal_start, equal, _, _) = self.bump().expect("peeked '='");
            return Ok(Token {
                kind: TokenKind::BangEqual,
                span: Span {
                    start,
                    end: equal_start + equal.len_utf8(),
                    line,
                    column,
                },
            });
        }

        Err(LexError {
            message: "expected '=' after '!'".to_owned(),
            span: Span {
                start,
                end: single_end,
                line,
                column,
            },
        })
    }

    fn skip_comment(&mut self) {
        while let Some((_, ch)) = self.peek() {
            if ch == '\n' {
                break;
            }
            let _ = self.bump();
        }
    }

    fn synchronize_string_recovery(&mut self) {
        while let Some((_, ch)) = self.peek() {
            match ch {
                '\n' => break,
                '"' => {
                    let _ = self.bump();
                    break;
                }
                '\\' => {
                    let _ = self.bump();
                    if matches!(self.peek(), Some((_, next)) if next != '\n') {
                        let _ = self.bump();
                    }
                }
                _ => {
                    let _ = self.bump();
                }
            }
        }
    }

    fn lex_number(&mut self) -> Result<Token, LexError> {
        let (start, _, line, column) = self.bump().expect("number requires input");
        while matches!(self.peek(), Some((_, ch)) if ch.is_ascii_digit()) {
            let _ = self.bump();
        }
        let end = self.peek().map_or(self.source.len(), |(byte, _)| byte);
        let text = &self.source[start..end];
        let value = text.parse::<i64>().map_err(|_| LexError {
            message: "integer literal is out of range for i64".to_owned(),
            span: Span {
                start,
                end,
                line,
                column,
            },
        })?;
        Ok(Token {
            kind: TokenKind::Integer(value),
            span: Span {
                start,
                end,
                line,
                column,
            },
        })
    }

    fn lex_identifier(&mut self) -> Token {
        let (start, _, line, column) = self.bump().expect("identifier requires input");
        while matches!(self.peek(), Some((_, ch)) if is_ident_continue(ch)) {
            let _ = self.bump();
        }
        let end = self.peek().map_or(self.source.len(), |(byte, _)| byte);
        let text = &self.source[start..end];
        let kind = match text {
            "print" => TokenKind::Print,
            "repeat" => TokenKind::Repeat,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "end" => TokenKind::End,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "input_int" => TokenKind::InputInt,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "fn" => TokenKind::Fn,
            "return" => TokenKind::Return,
            "int" => TokenKind::TypeInt,
            "bool" => TokenKind::TypeBool,
            "string" => TokenKind::TypeString,
            _ => TokenKind::Identifier(text.to_owned()),
        };
        Token {
            kind,
            span: Span {
                start,
                end,
                line,
                column,
            },
        }
    }

    fn lex_string(&mut self) -> Result<Token, LexError> {
        let (start, _, line, column) = self.bump().expect("string requires opening quote");
        let mut value = String::new();

        while let Some((byte, ch)) = self.peek() {
            if ch == '"' {
                let (_, closing, _, _) = self.bump().expect("peeked closing quote");
                return Ok(Token {
                    kind: TokenKind::StringLiteral(value),
                    span: Span {
                        start,
                        end: byte + closing.len_utf8(),
                        line,
                        column,
                    },
                });
            }
            if ch == '\n' {
                return Err(LexError {
                    message: "unterminated string literal".to_owned(),
                    span: Span {
                        start,
                        end: byte,
                        line,
                        column,
                    },
                });
            }
            if ch == '\\' {
                let _ = self.bump();
                let Some((escape_byte, escape, escape_line, escape_column)) = self.bump() else {
                    return Err(LexError {
                        message: "unterminated escape sequence".to_owned(),
                        span: Span {
                            start,
                            end: self.source.len(),
                            line,
                            column,
                        },
                    });
                };
                let decoded = match escape {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '"' => '"',
                    '\\' => '\\',
                    _ => {
                        return Err(LexError {
                            message: format!("unsupported escape sequence \\{escape}"),
                            span: Span {
                                start: escape_byte,
                                end: escape_byte + escape.len_utf8(),
                                line: escape_line,
                                column: escape_column,
                            },
                        });
                    }
                };
                value.push(decoded);
                continue;
            }
            let _ = self.bump();
            value.push(ch);
        }

        Err(LexError {
            message: "unterminated string literal".to_owned(),
            span: Span {
                start,
                end: self.source.len(),
                line,
                column,
            },
        })
    }
}

const fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

const fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::{MAX_RECOVERED_ERRORS, TokenKind, lex, lex_recovering};

    fn kinds(source: &str) -> Vec<TokenKind> {
        lex(source)
            .expect("source should lex")
            .into_iter()
            .map(|token| token.kind)
            .collect()
    }

    #[test]
    fn tokenizes_basic_script() {
        assert_eq!(
            kinds("x = 1\nprint x + 1\n"),
            vec![
                TokenKind::Identifier("x".to_owned()),
                TokenKind::Equal,
                TokenKind::Integer(1),
                TokenKind::Newline,
                TokenKind::Print,
                TokenKind::Identifier("x".to_owned()),
                TokenKind::Plus,
                TokenKind::Integer(1),
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn tokenizes_function_signature_and_call_punctuation() {
        let tokens = kinds("fn add(a int, b int) int\nreturn add(a, b)\nend\n");
        for expected in [
            TokenKind::Fn,
            TokenKind::Return,
            TokenKind::TypeInt,
            TokenKind::Comma,
            TokenKind::LParen,
            TokenKind::RParen,
        ] {
            assert!(tokens.contains(&expected), "missing {expected:?}");
        }
    }

    #[test]
    fn tokenizes_all_signature_types() {
        let tokens = kinds("fn sample(a int, b bool, c string) string\nreturn c\nend\n");
        assert!(tokens.contains(&TokenKind::TypeInt));
        assert!(tokens.contains(&TokenKind::TypeBool));
        assert!(tokens.contains(&TokenKind::TypeString));
    }

    #[test]
    fn function_keyword_prefixes_remain_identifiers() {
        let tokens = kinds("fnord = 1\nreturning = 2\ninteger = 3\nboolean = 4\nstringify = 5\n");
        for name in ["fnord", "returning", "integer", "boolean", "stringify"] {
            assert!(tokens.contains(&TokenKind::Identifier(name.to_owned())));
        }
    }

    #[test]
    fn recovering_lexer_matches_fail_fast_tokens_for_function_source() {
        let source = "fn add(a int, b int) int\nreturn a + b\nend\nprint add(2, 3)\n";
        assert_eq!(
            lex_recovering(source).expect("recovery lexing should succeed"),
            lex(source).expect("fail-fast lexing should succeed")
        );
    }

    #[test]
    fn tokenizes_runtime_workload_keywords() {
        let tokens = kinds("n = input_int\nrepeat n\nend\n");
        assert!(tokens.contains(&TokenKind::InputInt));
        assert!(tokens.contains(&TokenKind::Repeat));
        assert!(tokens.contains(&TokenKind::End));
    }

    #[test]
    fn tokenizes_control_flow_keywords_and_comparisons() {
        let tokens = kinds(
            "if true\nelse\nif false\nend\nprint 1 == 2\nprint 1 != 2\nprint 1 < 2\nprint 1 <= 2\nprint 2 > 1\nprint 2 >= 1\n",
        );
        for expected in [
            TokenKind::If,
            TokenKind::Else,
            TokenKind::True,
            TokenKind::False,
            TokenKind::EqualEqual,
            TokenKind::BangEqual,
            TokenKind::Less,
            TokenKind::LessEqual,
            TokenKind::Greater,
            TokenKind::GreaterEqual,
        ] {
            assert!(tokens.contains(&expected), "missing {expected:?}");
        }
    }

    #[test]
    fn tokenizes_logical_keywords() {
        let tokens = kinds("if true and not false or true\nend\n");
        assert!(tokens.contains(&TokenKind::And));
        assert!(tokens.contains(&TokenKind::Or));
        assert!(tokens.contains(&TokenKind::Not));
    }

    #[test]
    fn logical_keyword_prefixes_remain_identifiers() {
        let tokens = kinds("android = 1\norigin = 2\nnotice = 3\n");
        for name in ["android", "origin", "notice"] {
            assert!(tokens.contains(&TokenKind::Identifier(name.to_owned())));
        }
    }

    #[test]
    fn lone_bang_is_a_deterministic_error_and_recovery_makes_progress() {
        let error = lex("print !\n").expect_err("lone bang should fail");
        assert_eq!(error.message, "expected '=' after '!'");
        assert_eq!(error.span.column, 7);

        let errors = lex_recovering("!\n!\n").expect_err("lone bangs should recover");
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].span.line, 1);
        assert_eq!(errors[1].span.line, 2);
    }

    #[test]
    fn handles_comments_and_string_escapes() {
        let tokens = kinds("# comment\nprint \"hello\\nworld\"\n");
        assert!(matches!(tokens[0], TokenKind::Newline));
        assert!(matches!(tokens[1], TokenKind::Print));
        assert_eq!(
            tokens[2],
            TokenKind::StringLiteral("hello\nworld".to_owned())
        );
    }

    #[test]
    fn rejects_unknown_character() {
        let error = lex("print @").expect_err("unknown character should fail");
        assert_eq!(error.span.line, 1);
        assert_eq!(error.span.column, 7);
    }

    #[test]
    fn recovering_lexer_reports_multiple_unknown_characters_in_order() {
        let errors = lex_recovering("print @\nprint $\n")
            .expect_err("multiple unknown characters should fail");
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].span.line, 1);
        assert_eq!(errors[1].span.line, 2);
    }

    #[test]
    fn unsupported_escape_synchronizes_to_string_end_and_continues() {
        let errors = lex_recovering("print \"bad\\q rest\"\nprint @\n")
            .expect_err("unsupported escape should recover");
        assert_eq!(errors.len(), 2);
        assert!(errors[0].message.contains("unsupported escape"));
        assert!(errors[1].message.contains("unexpected character"));
    }

    #[test]
    fn unterminated_string_preserves_newline_and_continues() {
        let errors = lex_recovering("print \"bad\nprint @\n")
            .expect_err("unterminated string should recover at newline");
        assert_eq!(errors.len(), 2);
        assert!(errors[0].message.contains("unterminated string"));
        assert_eq!(errors[1].span.line, 2);
    }

    #[test]
    fn integer_overflow_consumes_literal_and_continues() {
        let errors = lex_recovering("999999999999999999999999999999\nprint @\n")
            .expect_err("overflowing integer should recover");
        assert_eq!(errors.len(), 2);
        assert!(errors[0].message.contains("out of range"));
        assert_eq!(errors[1].span.line, 2);
    }

    #[test]
    fn recovering_lexer_caps_diagnostics() {
        let source = "@\n".repeat(MAX_RECOVERED_ERRORS + 4);
        let errors = lex_recovering(&source).expect_err("unknown characters should fail");
        assert_eq!(errors.len(), MAX_RECOVERED_ERRORS);
    }

    #[test]
    fn unknown_unicode_scalar_makes_progress_by_scalar_not_byte() {
        let errors = lex_recovering("@☃\n").expect_err("unknown characters should fail");
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].span.start, 0);
        assert_eq!(errors[0].span.end, 1);
        assert_eq!(errors[1].span.start, 1);
        assert_eq!(errors[1].span.end, 4);
        assert_eq!(errors[1].span.column, 2);
    }
}
