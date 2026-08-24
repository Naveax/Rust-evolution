use evo_lexer::{Span, Token, TokenKind};
use std::error::Error;
use std::fmt;

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
            let statement = self.parse_statement()?;
            if !matches!(self.current().kind, TokenKind::Newline | TokenKind::Eof) {
                return Err(self.error_here("expected end of line after statement"));
            }
            statements.push(statement);
            self.skip_newlines();
        }
        Ok(Program { statements })
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
            _ => Err(self.error_here("expected binding or 'print' statement")),
        }
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
    use super::{BinaryOp, ExprKind, StmtKind, parse};
    use evo_lexer::lex;

    fn parse_source(source: &str) -> super::Program {
        let tokens = lex(source).expect("lexing should succeed");
        parse(&tokens).expect("parsing should succeed")
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
            StmtKind::Bind { .. } => panic!("expected print statement"),
        }
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
}
