use std::error::Error;
use std::fmt;
use std::iter::Peekable;
use std::str::CharIndices;

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
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    LParen,
    RParen,
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
        write!(f, "{} at {}:{}", self.message, self.span.line, self.span.column)
    }
}

impl Error for LexError {}

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).lex_all()
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
                    self.bump();
                }
                '\n' => {
                    self.bump();
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
                '=' => tokens.push(self.single(TokenKind::Equal)),
                '(' => tokens.push(self.single(TokenKind::LParen)),
                ')' => tokens.push(self.single(TokenKind::RParen)),
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

        let end = self.source.len();
        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span {
                start: end,
                end,
                line: self.line,
                column: self.column,
            },
        });
        Ok(tokens)
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

    fn skip_comment(&mut self) {
        while let Some((_, ch)) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.bump();
        }
    }

    fn lex_number(&mut self) -> Result<Token, LexError> {
        let (start, _, line, column) = self.bump().expect("number requires input");
        while matches!(self.peek(), Some((_, ch)) if ch.is_ascii_digit()) {
            self.bump();
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
            self.bump();
        }
        let end = self.peek().map_or(self.source.len(), |(byte, _)| byte);
        let text = &self.source[start..end];
        let kind = if text == "print" {
            TokenKind::Print
        } else {
            TokenKind::Identifier(text.to_owned())
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
                self.bump();
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
            self.bump();
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
    use super::{TokenKind, lex};

    #[test]
    fn tokenizes_basic_script() {
        let tokens = lex("x = 1\nprint x + 1\n").expect("lexing should succeed");
        let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
        assert_eq!(
            kinds,
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
    fn handles_comments_and_string_escapes() {
        let tokens = lex("# comment\nprint \"hello\\nworld\"\n").expect("lexing should succeed");
        assert!(matches!(tokens[0].kind, TokenKind::Newline));
        assert!(matches!(tokens[1].kind, TokenKind::Print));
        assert_eq!(
            tokens[2].kind,
            TokenKind::StringLiteral("hello\nworld".to_owned())
        );
    }

    #[test]
    fn rejects_unknown_character() {
        let error = lex("print @").expect_err("unknown character should fail");
        assert_eq!(error.span.line, 1);
        assert_eq!(error.span.column, 7);
    }
}
