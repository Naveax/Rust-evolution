use evo_lexer::{Span, Token, TokenKind};
use std::error::Error;
use std::fmt;

const MAX_RECOVERED_ERRORS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StmtKind {
    Bind { name: String, expr: Expr },
    Print(Expr),
    Repeat { count: Expr, body: Vec<Stmt> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Integer(i64),
    String(String),
    Identifier(String),
    InputInt,
    UnaryMinus(Box<Expr>),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}:{}",
            self.message, self.span.line, self.span.column
        )
    }
}

impl Error for ParseError {}

pub fn parse(tokens: &[Token]) -> Result<Program, ParseError> {
    Parser::new(tokens).parse_program()
}

pub fn parse_recovering(tokens: &[Token]) -> Result<Program, Vec<ParseError>> {
    Parser::new(tokens).parse_program_recovering()
}

struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl<'a> Parser<'a> {
    const fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, index: 0 }
    }

    fn parse_program(mut self) -> Result<Program, ParseError> {
        if self.tokens.is_empty() {
            return Ok(Program {
                statements: Vec::new(),
            });
        }

        let mut statements = Vec::new();
        self.skip_newlines();
        while !self.is_eof() {
            if matches!(self.current().kind, TokenKind::End) {
                return Err(self.error_here("unexpected 'end' without matching 'repeat'"));
            }
            let statement = self.parse_statement()?;
            self.require_statement_terminator()?;
            statements.push(statement);
            self.skip_newlines();
        }
        Ok(Program { statements })
    }

    fn parse_program_recovering(mut self) -> Result<Program, Vec<ParseError>> {
        if self.tokens.is_empty() {
            return Ok(Program {
                statements: Vec::new(),
            });
        }

        let mut errors = Vec::new();
        let statements = self.parse_statements_recovering(false, &mut errors);
        if errors.is_empty() {
            Ok(Program { statements })
        } else {
            Err(errors)
        }
    }

    fn parse_statements_recovering(
        &mut self,
        stop_at_end: bool,
        errors: &mut Vec<ParseError>,
    ) -> Vec<Stmt> {
        let mut statements = Vec::new();
        self.skip_newlines();

        while !self.is_eof() && errors.len() < MAX_RECOVERED_ERRORS {
            if matches!(self.current().kind, TokenKind::End) {
                if stop_at_end {
                    break;
                }
                self.record_error(
                    errors,
                    self.error_here("unexpected 'end' without matching 'repeat'"),
                );
                self.advance();
                self.synchronize_statement(false);
                self.skip_newlines();
                continue;
            }

            let start_index = self.index;
            let statement = if matches!(self.current().kind, TokenKind::Repeat) {
                self.parse_repeat_recovering(errors)
            } else {
                match self.parse_statement() {
                    Ok(statement) => Some(statement),
                    Err(error) => {
                        self.record_error(errors, error);
                        self.synchronize_statement(stop_at_end);
                        None
                    }
                }
            };

            if let Some(statement) = statement {
                if let Err(error) = self.require_statement_terminator() {
                    self.record_error(errors, error);
                    self.synchronize_statement(stop_at_end);
                } else {
                    statements.push(statement);
                }
            }

            self.skip_newlines();
            if self.index == start_index && !self.is_eof() {
                if stop_at_end && matches!(self.current().kind, TokenKind::End) {
                    break;
                }
                self.advance();
            }
        }

        statements
    }

    fn parse_repeat_recovering(&mut self, errors: &mut Vec<ParseError>) -> Option<Stmt> {
        let start = self.advance().span;
        let count = match self.parse_expression() {
            Ok(count) => count,
            Err(error) => {
                self.record_error(errors, error);
                let found_end = self.skip_invalid_repeat();
                if !found_end && errors.len() < MAX_RECOVERED_ERRORS {
                    self.record_error(errors, self.error_here("missing 'end' for repeat block"));
                }
                return None;
            }
        };

        if !matches!(self.current().kind, TokenKind::Newline) {
            self.record_error(
                errors,
                self.error_here("expected end of line after repeat count"),
            );
            self.synchronize_statement(true);
        }
        self.skip_newlines();

        let body = self.parse_statements_recovering(true, errors);
        if errors.len() >= MAX_RECOVERED_ERRORS {
            return None;
        }
        if self.is_eof() {
            self.record_error(errors, self.error_here("missing 'end' for repeat block"));
            return None;
        }

        let close = self.advance().span;
        Some(Stmt {
            kind: StmtKind::Repeat { count, body },
            span: start.join(close),
        })
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        match self.current().kind.clone() {
            TokenKind::Print => {
                let start = self.advance().span;
                let expr = self.parse_expression()?;
                let span = start.join(expr.span);
                Ok(Stmt {
                    kind: StmtKind::Print(expr),
                    span,
                })
            }
            TokenKind::Repeat => self.parse_repeat(),
            TokenKind::End => Err(self.error_here("unexpected 'end' without matching 'repeat'")),
            TokenKind::Identifier(name) => {
                let start = self.advance().span;
                if !matches!(self.current().kind, TokenKind::Equal) {
                    return Err(self.error_here("expected '=' after binding name"));
                }
                self.advance();
                let expr = self.parse_expression()?;
                let span = start.join(expr.span);
                Ok(Stmt {
                    kind: StmtKind::Bind { name, expr },
                    span,
                })
            }
            _ => Err(self.error_here("expected binding, 'print', or 'repeat' statement")),
        }
    }

    fn parse_repeat(&mut self) -> Result<Stmt, ParseError> {
        let start = self.advance().span;
        let count = self.parse_expression()?;
        if !matches!(self.current().kind, TokenKind::Newline) {
            return Err(self.error_here("expected end of line after repeat count"));
        }
        self.skip_newlines();

        let mut body = Vec::new();
        while !matches!(self.current().kind, TokenKind::End) {
            if self.is_eof() {
                return Err(self.error_here("missing 'end' for repeat block"));
            }
            let statement = self.parse_statement()?;
            self.require_statement_terminator()?;
            body.push(statement);
            self.skip_newlines();
        }

        let close = self.advance().span;
        Ok(Stmt {
            kind: StmtKind::Repeat { count, body },
            span: start.join(close),
        })
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_add_sub()
    }

    fn parse_add_sub(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_mul_div()?;
        loop {
            let op = match self.current().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Subtract,
                _ => break,
            };
            self.advance();
            let right = self.parse_mul_div()?;
            let span = left.span.join(right.span);
            left = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_mul_div(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.current().kind {
                TokenKind::Star => BinaryOp::Multiply,
                TokenKind::Slash => BinaryOp::Divide,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            let span = left.span.join(right.span);
            left = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if matches!(self.current().kind, TokenKind::Minus) {
            let start = self.advance().span;
            let expr = self.parse_unary()?;
            let span = start.join(expr.span);
            return Ok(Expr {
                kind: ExprKind::UnaryMinus(Box::new(expr)),
                span,
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.advance();
        match token.kind {
            TokenKind::Integer(value) => Ok(Expr {
                kind: ExprKind::Integer(value),
                span: token.span,
            }),
            TokenKind::StringLiteral(value) => Ok(Expr {
                kind: ExprKind::String(value),
                span: token.span,
            }),
            TokenKind::Identifier(name) => Ok(Expr {
                kind: ExprKind::Identifier(name),
                span: token.span,
            }),
            TokenKind::InputInt => Ok(Expr {
                kind: ExprKind::InputInt,
                span: token.span,
            }),
            TokenKind::LParen => {
                let mut expr = self.parse_expression()?;
                if !matches!(self.current().kind, TokenKind::RParen) {
                    return Err(self.error_here("expected ')'"));
                }
                let close = self.advance().span;
                expr.span = token.span.join(close);
                Ok(expr)
            }
            _ => Err(ParseError {
                message: "expected expression".to_owned(),
                span: token.span,
            }),
        }
    }

    fn require_statement_terminator(&self) -> Result<(), ParseError> {
        if matches!(self.current().kind, TokenKind::Newline | TokenKind::Eof) {
            Ok(())
        } else {
            Err(self.error_here("expected end of line after statement"))
        }
    }

    fn synchronize_statement(&mut self, preserve_end: bool) {
        if self.index > 0 {
            let previous = &self.tokens[self.index - 1].kind;
            if matches!(previous, TokenKind::Newline) {
                return;
            }
            if preserve_end && matches!(previous, TokenKind::End) {
                self.index -= 1;
                return;
            }
        }

        while !self.is_eof()
            && !matches!(self.current().kind, TokenKind::Newline)
            && !(preserve_end && matches!(self.current().kind, TokenKind::End))
        {
            self.advance();
        }
    }

    fn skip_invalid_repeat(&mut self) -> bool {
        self.synchronize_statement(false);
        self.skip_newlines();
        let mut depth = 1usize;

        while !self.is_eof() {
            match self.current().kind {
                TokenKind::Repeat => {
                    depth += 1;
                    self.advance();
                }
                TokenKind::End => {
                    self.advance();
                    depth -= 1;
                    if depth == 0 {
                        self.synchronize_statement(false);
                        return true;
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }

        false
    }

    fn record_error(&self, errors: &mut Vec<ParseError>, error: ParseError) {
        if errors.len() < MAX_RECOVERED_ERRORS {
            errors.push(error);
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current().kind, TokenKind::Newline) {
            self.advance();
        }
    }

    fn is_eof(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.index).unwrap_or_else(|| {
            self.tokens
                .last()
                .expect("parser requires at least one token")
        })
    }

    fn advance(&mut self) -> Token {
        let token = self.current().clone();
        if !matches!(token.kind, TokenKind::Eof) {
            self.index += 1;
        }
        token
    }

    fn error_here(&self, message: &str) -> ParseError {
        ParseError {
            message: message.to_owned(),
            span: self.current().span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BinaryOp, ExprKind, MAX_RECOVERED_ERRORS, StmtKind, parse, parse_recovering};
    use evo_lexer::lex;

    fn parse_source(source: &str) -> super::Program {
        let tokens = lex(source).expect("lexing should succeed");
        parse(&tokens).expect("parsing should succeed")
    }

    fn recover_source(source: &str) -> Result<super::Program, Vec<super::ParseError>> {
        let tokens = lex(source).expect("lexing should succeed");
        parse_recovering(&tokens)
    }

    #[test]
    fn parses_binding_and_print() {
        let program = parse_source("x = 1\nprint x + 2\n");
        assert_eq!(program.statements.len(), 2);
        assert!(matches!(
            &program.statements[0].kind,
            StmtKind::Bind { name, .. } if name == "x"
        ));
        match &program.statements[1].kind {
            StmtKind::Print(expr) => assert!(matches!(
                &expr.kind,
                ExprKind::Binary {
                    op: BinaryOp::Add,
                    ..
                }
            )),
            _ => panic!("expected print statement"),
        }
    }

    #[test]
    fn parses_runtime_input_and_nested_repeat_blocks() {
        let program = parse_source(
            "n = input_int\nsum = 0\nrepeat n\nrepeat 2\nsum = sum + 1\nend\nend\nprint sum\n",
        );
        assert!(matches!(
            &program.statements[0].kind,
            StmtKind::Bind {
                expr: super::Expr {
                    kind: ExprKind::InputInt,
                    ..
                },
                ..
            }
        ));
        let StmtKind::Repeat { body, .. } = &program.statements[2].kind else {
            panic!("expected repeat statement");
        };
        assert!(matches!(&body[0].kind, StmtKind::Repeat { .. }));
    }

    #[test]
    fn respects_operator_precedence() {
        let program = parse_source("print 1 + 2 * 3\n");
        let StmtKind::Print(expr) = &program.statements[0].kind else {
            panic!("expected print statement");
        };
        let ExprKind::Binary { left: _, op, right } = &expr.kind else {
            panic!("expected outer binary expression");
        };
        assert_eq!(*op, BinaryOp::Add);
        assert!(matches!(
            &right.kind,
            ExprKind::Binary {
                op: BinaryOp::Multiply,
                ..
            }
        ));
    }

    #[test]
    fn rejects_missing_assignment_operator() {
        let tokens = lex("x 1\n").expect("lexing should succeed");
        let error = parse(&tokens).expect_err("invalid binding should fail");
        assert!(error.message.contains("expected '='"));
    }

    #[test]
    fn rejects_missing_repeat_end() {
        let tokens = lex("repeat 2\nprint 1\n").expect("lexing should succeed");
        let error = parse(&tokens).expect_err("unterminated repeat should fail");
        assert!(error.message.contains("missing 'end'"));
    }

    #[test]
    fn rejects_unmatched_end() {
        let tokens = lex("end\n").expect("lexing should succeed");
        let error = parse(&tokens).expect_err("unmatched end should fail");
        assert!(error.message.contains("unexpected 'end'"));
    }

    #[test]
    fn recovering_parser_matches_fail_fast_parser_on_valid_input() {
        let source = "n = input_int\nsum = 0\nrepeat n\nsum = sum + 1\nend\nprint sum\n";
        let tokens = lex(source).expect("lexing should succeed");
        assert_eq!(
            parse_recovering(&tokens).expect("recovery parse should succeed"),
            parse(&tokens).expect("fail-fast parse should succeed")
        );
    }

    #[test]
    fn recovers_two_independent_top_level_errors_in_order() {
        let errors = recover_source("x 1\ny 2\n").expect_err("source should be invalid");
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].span.line, 1);
        assert_eq!(errors[1].span.line, 2);
        assert!(
            errors
                .iter()
                .all(|error| error.message.contains("expected '='"))
        );
    }

    #[test]
    fn missing_expression_does_not_consume_next_statement() {
        let errors = recover_source("print\ny 2\n").expect_err("source should be invalid");
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].span.line, 1);
        assert_eq!(errors[1].span.line, 2);
        assert!(errors[0].message.contains("expected expression"));
        assert!(errors[1].message.contains("expected '='"));
    }

    #[test]
    fn repeat_body_recovery_preserves_closing_end_and_continues_after_block() {
        let source = "repeat 2\nx 1\nprint 1\nend\ny 2\n";
        let errors = recover_source(source).expect_err("source should be invalid");
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert_eq!(errors[0].span.line, 2);
        assert_eq!(errors[1].span.line, 5);
        assert!(
            errors
                .iter()
                .all(|error| !error.message.contains("unexpected 'end'"))
        );
    }

    #[test]
    fn nested_repeat_recovery_keeps_each_block_boundary() {
        let source = concat!(
            "repeat 2\n",
            "repeat 1\n",
            "x 1\n",
            "end\n",
            "print 1\n",
            "end\n",
            "y 2\n"
        );
        let errors = recover_source(source).expect_err("source should be invalid");
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert_eq!(errors[0].span.line, 3);
        assert_eq!(errors[1].span.line, 7);
        assert!(
            errors
                .iter()
                .all(|error| !error.message.contains("unexpected 'end'"))
        );
    }

    #[test]
    fn recovering_parser_reports_unmatched_end_and_keeps_going() {
        let errors = recover_source("end\nx 1\n").expect_err("source should be invalid");
        assert_eq!(errors.len(), 2);
        assert!(errors[0].message.contains("unexpected 'end'"));
        assert_eq!(errors[1].span.line, 2);
    }

    #[test]
    fn recovering_parser_reports_missing_end() {
        let errors = recover_source("repeat 1\nprint 1\n").expect_err("source should be invalid");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].message.contains("missing 'end'"));
    }

    #[test]
    fn trailing_garbage_recovers_at_newline() {
        let errors = recover_source("x = 1 2\ny 2\n").expect_err("source should be invalid");
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].message.contains("expected end of line"));
        assert_eq!(errors[1].span.line, 2);
    }

    #[test]
    fn recovery_error_count_is_bounded() {
        let source = "x 1\n".repeat(MAX_RECOVERED_ERRORS + 4);
        let errors = recover_source(&source).expect_err("source should be invalid");
        assert_eq!(errors.len(), MAX_RECOVERED_ERRORS);
    }

    #[test]
    fn no_progress_input_terminates_and_reports_each_line() {
        let errors = recover_source(")\n)\n").expect_err("source should be invalid");
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].span.line, 1);
        assert_eq!(errors[1].span.line, 2);
    }
}
