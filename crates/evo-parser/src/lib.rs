use evo_lexer::{Span, Token, TokenKind};
use std::error::Error;
use std::fmt;

const MAX_RECOVERED_ERRORS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub functions: Vec<FunctionDef>,
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDef {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: TypeName,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub type_name: TypeName,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeName {
    Int,
    Bool,
    String,
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
    Return(Expr),
    Repeat { count: Expr, body: Vec<Stmt> },
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
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
    Bool(bool),
    Identifier(String),
    Call { name: String, arguments: Vec<Expr> },
    InputInt,
    LogicalNot(Box<Expr>),
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
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
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

#[derive(Debug, Clone, Copy)]
struct StopSet {
    end: bool,
    else_: bool,
}

impl StopSet {
    const NONE: Self = Self {
        end: false,
        else_: false,
    };
    const END: Self = Self {
        end: true,
        else_: false,
    };
    const END_OR_ELSE: Self = Self {
        end: true,
        else_: true,
    };

    fn contains(self, kind: &TokenKind) -> bool {
        (self.end && matches!(kind, TokenKind::End))
            || (self.else_ && matches!(kind, TokenKind::Else))
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
    function_depth: usize,
}

impl<'a> Parser<'a> {
    const fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            index: 0,
            function_depth: 0,
        }
    }

    fn parse_program(mut self) -> Result<Program, ParseError> {
        if self.tokens.is_empty() {
            return Ok(Program {
                functions: Vec::new(),
                statements: Vec::new(),
            });
        }

        let mut functions = Vec::new();
        let mut statements = Vec::new();
        self.skip_newlines();
        while !self.is_eof() {
            match self.current().kind {
                TokenKind::Fn => {
                    functions.push(self.parse_function()?);
                    self.require_statement_terminator()?;
                }
                TokenKind::End => {
                    return Err(self.error_here("unexpected 'end' without matching block"));
                }
                TokenKind::Else => {
                    return Err(self.error_here("unexpected 'else' without matching 'if'"));
                }
                TokenKind::Return => {
                    return Err(self.error_here("'return' is only valid inside a function"));
                }
                _ => {
                    let statement = self.parse_statement()?;
                    self.require_statement_terminator()?;
                    statements.push(statement);
                }
            }
            self.skip_newlines();
        }
        Ok(Program {
            functions,
            statements,
        })
    }

    fn parse_program_recovering(mut self) -> Result<Program, Vec<ParseError>> {
        if self.tokens.is_empty() {
            return Ok(Program {
                functions: Vec::new(),
                statements: Vec::new(),
            });
        }

        let mut errors = Vec::new();
        let mut functions = Vec::new();
        let mut statements = Vec::new();
        self.skip_newlines();

        while !self.is_eof() && errors.len() < MAX_RECOVERED_ERRORS {
            if matches!(self.current().kind, TokenKind::Fn) {
                let start_index = self.index;
                match self.parse_function() {
                    Ok(function) => {
                        if let Err(error) = self.require_statement_terminator() {
                            self.record_error(&mut errors, error);
                            self.synchronize_statement(StopSet::NONE);
                        } else {
                            functions.push(function);
                        }
                    }
                    Err(error) => {
                        self.record_error(&mut errors, error);
                        self.recover_function_definition();
                    }
                }
                self.skip_newlines();
                if self.index == start_index && !self.is_eof() {
                    self.advance();
                }
                continue;
            }

            if matches!(self.current().kind, TokenKind::Return) {
                self.record_error(
                    &mut errors,
                    self.error_here("'return' is only valid inside a function"),
                );
                self.synchronize_statement(StopSet::NONE);
                self.skip_newlines();
                continue;
            }

            let before = self.index;
            let recovered = self.parse_statements_recovering(StopSet::NONE, &mut errors);
            statements.extend(recovered);
            if self.index == before && !self.is_eof() {
                self.advance();
            }
            self.skip_newlines();
        }

        if errors.is_empty() {
            Ok(Program {
                functions,
                statements,
            })
        } else {
            Err(errors)
        }
    }

    fn parse_function(&mut self) -> Result<FunctionDef, ParseError> {
        let start = self.expect_kind(TokenKind::Fn, "expected 'fn'")?.span;
        let name_token = self.advance();
        let TokenKind::Identifier(name) = name_token.kind else {
            return Err(ParseError {
                message: "expected function name after 'fn'".to_owned(),
                span: name_token.span,
            });
        };
        self.expect_kind(TokenKind::LParen, "expected '(' after function name")?;
        let parameters = self.parse_parameters()?;
        self.expect_kind(TokenKind::RParen, "expected ')' after function parameters")?;
        let return_type = self.parse_type_name()?;
        if !matches!(self.current().kind, TokenKind::Newline) {
            return Err(self.error_here("expected end of line after function signature"));
        }
        self.skip_newlines();

        self.function_depth += 1;
        let body_result = self.parse_function_body();
        self.function_depth -= 1;
        let body = body_result?;
        if self.is_eof() {
            return Err(self.error_here("missing 'end' for function"));
        }
        let close = self.expect_kind(TokenKind::End, "missing 'end' for function")?.span;
        Ok(FunctionDef {
            name,
            parameters,
            return_type,
            body,
            span: start.join(close),
        })
    }

    fn parse_parameters(&mut self) -> Result<Vec<Parameter>, ParseError> {
        let mut parameters = Vec::new();
        if matches!(self.current().kind, TokenKind::RParen) {
            return Ok(parameters);
        }
        loop {
            let name_token = self.advance();
            let TokenKind::Identifier(name) = name_token.kind else {
                return Err(ParseError {
                    message: "expected parameter name".to_owned(),
                    span: name_token.span,
                });
            };
            let type_token = self.current().clone();
            let type_name = self.parse_type_name()?;
            parameters.push(Parameter {
                name,
                type_name,
                span: name_token.span.join(type_token.span),
            });
            if !matches!(self.current().kind, TokenKind::Comma) {
                break;
            }
            self.advance();
            if matches!(self.current().kind, TokenKind::RParen) {
                return Err(self.error_here("expected parameter after ','"));
            }
        }
        Ok(parameters)
    }

    fn parse_type_name(&mut self) -> Result<TypeName, ParseError> {
        let token = self.advance();
        match token.kind {
            TokenKind::TypeInt => Ok(TypeName::Int),
            TokenKind::TypeBool => Ok(TypeName::Bool),
            TokenKind::TypeString => Ok(TypeName::String),
            _ => Err(ParseError {
                message: "expected type name: int, bool, or string".to_owned(),
                span: token.span,
            }),
        }
    }

    fn parse_function_body(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut body = Vec::new();
        while !matches!(self.current().kind, TokenKind::End) {
            if self.is_eof() {
                return Err(self.error_here("missing 'end' for function"));
            }
            if matches!(self.current().kind, TokenKind::Fn) {
                return Err(self.error_here("nested function definitions are not supported in v0"));
            }
            let statement = self.parse_statement()?;
            self.require_statement_terminator()?;
            body.push(statement);
            self.skip_newlines();
        }
        Ok(body)
    }

    fn parse_statements_recovering(
        &mut self,
        stop: StopSet,
        errors: &mut Vec<ParseError>,
    ) -> Vec<Stmt> {
        let mut statements = Vec::new();
        self.skip_newlines();
        while !self.is_eof() && errors.len() < MAX_RECOVERED_ERRORS {
            if stop.contains(&self.current().kind) {
                break;
            }
            if matches!(self.current().kind, TokenKind::Fn) {
                self.record_error(
                    errors,
                    self.error_here("nested function definitions are not supported in v0"),
                );
                self.recover_function_definition();
                self.skip_newlines();
                continue;
            }
            if matches!(self.current().kind, TokenKind::End) {
                self.record_error(
                    errors,
                    self.error_here("unexpected 'end' without matching block"),
                );
                self.advance();
                self.synchronize_statement(StopSet::NONE);
                self.skip_newlines();
                continue;
            }
            if matches!(self.current().kind, TokenKind::Else) {
                self.record_error(
                    errors,
                    self.error_here("unexpected 'else' without matching 'if'"),
                );
                self.advance();
                self.synchronize_statement(StopSet::NONE);
                self.skip_newlines();
                continue;
            }
            let start_index = self.index;
            let statement = match self.current().kind {
                TokenKind::Repeat => self.parse_repeat_recovering(errors),
                TokenKind::If => self.parse_if_recovering(errors),
                _ => match self.parse_statement() {
                    Ok(statement) => Some(statement),
                    Err(error) => {
                        self.record_error(errors, error);
                        self.synchronize_statement(stop);
                        None
                    }
                },
            };
            if let Some(statement) = statement {
                if let Err(error) = self.require_statement_terminator() {
                    self.record_error(errors, error);
                    self.synchronize_statement(stop);
                } else {
                    statements.push(statement);
                }
            }
            self.skip_newlines();
            if self.index == start_index && !self.is_eof() {
                if stop.contains(&self.current().kind) {
                    break;
                }
                self.advance();
            }
            self.skip_newlines();
        }
        statements
    }

    fn parse_repeat_recovering(&mut self, errors: &mut Vec<ParseError>) -> Option<Stmt> {
        let start = self.advance().span;
        let count = match self.parse_expression() {
            Ok(count) => count,
            Err(error) => {
                self.record_error(errors, error);
                let found_end = self.skip_invalid_block();
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
            self.synchronize_statement(StopSet::END);
        }
        self.skip_newlines();
        let body = self.parse_statements_recovering(StopSet::END, errors);
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

    fn parse_if_recovering(&mut self, errors: &mut Vec<ParseError>) -> Option<Stmt> {
        let start = self.advance().span;
        let condition = match self.parse_expression() {
            Ok(condition) => condition,
            Err(error) => {
                self.record_error(errors, error);
                let found_end = self.skip_invalid_block();
                if !found_end && errors.len() < MAX_RECOVERED_ERRORS {
                    self.record_error(errors, self.error_here("missing 'end' for if block"));
                }
                return None;
            }
        };
        if !matches!(self.current().kind, TokenKind::Newline) {
            self.record_error(
                errors,
                self.error_here("expected end of line after if condition"),
            );
            self.synchronize_statement(StopSet::END_OR_ELSE);
        }
        self.skip_newlines();
        let then_body = self.parse_statements_recovering(StopSet::END_OR_ELSE, errors);
        if errors.len() >= MAX_RECOVERED_ERRORS {
            return None;
        }
        if self.is_eof() {
            self.record_error(errors, self.error_here("missing 'end' for if block"));
            return None;
        }
        let else_body = if matches!(self.current().kind, TokenKind::Else) {
            self.advance();
            if !matches!(self.current().kind, TokenKind::Newline) {
                self.record_error(errors, self.error_here("expected end of line after 'else'"));
                self.synchronize_statement(StopSet::END);
            }
            self.skip_newlines();
            let body = self.parse_statements_recovering(StopSet::END, errors);
            if errors.len() >= MAX_RECOVERED_ERRORS {
                return None;
            }
            if self.is_eof() {
                self.record_error(errors, self.error_here("missing 'end' for if block"));
                return None;
            }
            body
        } else {
            Vec::new()
        };
        let close = self.advance().span;
        Some(Stmt {
            kind: StmtKind::If {
                condition,
                then_body,
                else_body,
            },
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
            TokenKind::Return if self.function_depth > 0 => {
                let start = self.advance().span;
                let expr = self.parse_expression()?;
                let span = start.join(expr.span);
                Ok(Stmt {
                    kind: StmtKind::Return(expr),
                    span,
                })
            }
            TokenKind::Return => Err(self.error_here("'return' is only valid inside a function")),
            TokenKind::Repeat => self.parse_repeat(),
            TokenKind::If => self.parse_if(),
            TokenKind::Fn => {
                Err(self.error_here("nested function definitions are not supported in v0"))
            }
            TokenKind::End => Err(self.error_here("unexpected 'end' without matching block")),
            TokenKind::Else => Err(self.error_here("unexpected 'else' without matching 'if'")),
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
            _ => Err(self.error_here(
                "expected binding, 'print', 'return', 'repeat', or 'if' statement",
            )),
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

    fn parse_if(&mut self) -> Result<Stmt, ParseError> {
        let start = self.advance().span;
        let condition = self.parse_expression()?;
        if !matches!(self.current().kind, TokenKind::Newline) {
            return Err(self.error_here("expected end of line after if condition"));
        }
        self.skip_newlines();
        let mut then_body = Vec::new();
        while !matches!(self.current().kind, TokenKind::Else | TokenKind::End) {
            if self.is_eof() {
                return Err(self.error_here("missing 'end' for if block"));
            }
            let statement = self.parse_statement()?;
            self.require_statement_terminator()?;
            then_body.push(statement);
            self.skip_newlines();
        }
        let mut else_body = Vec::new();
        if matches!(self.current().kind, TokenKind::Else) {
            self.advance();
            if !matches!(self.current().kind, TokenKind::Newline) {
                return Err(self.error_here("expected end of line after 'else'"));
            }
            self.skip_newlines();
            while !matches!(self.current().kind, TokenKind::End) {
                if self.is_eof() {
                    return Err(self.error_here("missing 'end' for if block"));
                }
                let statement = self.parse_statement()?;
                self.require_statement_terminator()?;
                else_body.push(statement);
                self.skip_newlines();
            }
        }
        let close = self.advance().span;
        Ok(Stmt {
            kind: StmtKind::If {
                condition,
                then_body,
                else_body,
            },
            span: start.join(close),
        })
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while matches!(self.current().kind, TokenKind::Or) {
            self.advance();
            let right = self.parse_and()?;
            let span = left.span.join(right.span);
            left = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(left),
                    op: BinaryOp::Or,
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_not()?;
        while matches!(self.current().kind, TokenKind::And) {
            self.advance();
            let right = self.parse_not()?;
            let span = left.span.join(right.span);
            left = Expr {
                kind: ExprKind::Binary {
                    left: Box::new(left),
                    op: BinaryOp::And,
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expr, ParseError> {
        if matches!(self.current().kind, TokenKind::Not) {
            let start = self.advance().span;
            let expr = self.parse_not()?;
            let span = start.join(expr.span);
            return Ok(Expr {
                kind: ExprKind::LogicalNot(Box::new(expr)),
                span,
            });
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_add_sub()?;
        let Some(op) = comparison_operator(&self.current().kind) else {
            return Ok(left);
        };
        self.advance();
        let right = self.parse_add_sub()?;
        let span = left.span.join(right.span);
        let expression = Expr {
            kind: ExprKind::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            },
            span,
        };
        if comparison_operator(&self.current().kind).is_some() {
            return Err(self.error_here("chained comparisons are not supported"));
        }
        Ok(expression)
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
            TokenKind::True => Ok(Expr {
                kind: ExprKind::Bool(true),
                span: token.span,
            }),
            TokenKind::False => Ok(Expr {
                kind: ExprKind::Bool(false),
                span: token.span,
            }),
            TokenKind::Identifier(name) => self.parse_identifier_or_call(name, token.span),
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

    fn parse_identifier_or_call(&mut self, name: String, start: Span) -> Result<Expr, ParseError> {
        if !matches!(self.current().kind, TokenKind::LParen) {
            return Ok(Expr {
                kind: ExprKind::Identifier(name),
                span: start,
            });
        }
        self.advance();
        let mut arguments = Vec::new();
        if !matches!(self.current().kind, TokenKind::RParen) {
            loop {
                arguments.push(self.parse_expression()?);
                if !matches!(self.current().kind, TokenKind::Comma) {
                    break;
                }
                self.advance();
                if matches!(self.current().kind, TokenKind::RParen) {
                    return Err(self.error_here("expected argument after ','"));
                }
            }
        }
        if !matches!(self.current().kind, TokenKind::RParen) {
            return Err(self.error_here("expected ')' after function arguments"));
        }
        let close = self.advance().span;
        Ok(Expr {
            kind: ExprKind::Call { name, arguments },
            span: start.join(close),
        })
    }

    fn require_statement_terminator(&self) -> Result<(), ParseError> {
        if matches!(self.current().kind, TokenKind::Newline | TokenKind::Eof) {
            Ok(())
        } else {
            Err(self.error_here("expected end of line after statement"))
        }
    }

    fn synchronize_statement(&mut self, stop: StopSet) {
        if self.index > 0 {
            let previous = &self.tokens[self.index - 1].kind;
            if matches!(previous, TokenKind::Newline) {
                return;
            }
            if stop.contains(previous) {
                self.index -= 1;
                return;
            }
        }
        while !self.is_eof()
            && !matches!(self.current().kind, TokenKind::Newline)
            && !stop.contains(&self.current().kind)
        {
            self.advance();
        }
    }

    fn skip_invalid_block(&mut self) -> bool {
        self.synchronize_statement(StopSet::END);
        self.skip_newlines();
        let mut depth = 1usize;
        while !self.is_eof() {
            match self.current().kind {
                TokenKind::Repeat | TokenKind::If | TokenKind::Fn => {
                    depth += 1;
                    self.advance();
                }
                TokenKind::End => {
                    self.advance();
                    depth -= 1;
                    if depth == 0 {
                        self.synchronize_statement(StopSet::NONE);
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

    fn recover_function_definition(&mut self) {
        if !self.is_eof() {
            let _ = self.skip_invalid_block();
        }
    }

    fn record_error(&self, errors: &mut Vec<ParseError>, error: ParseError) {
        if errors.len() < MAX_RECOVERED_ERRORS {
            errors.push(error);
        }
    }

    fn expect_kind(&mut self, expected: TokenKind, message: &str) -> Result<Token, ParseError> {
        if std::mem::discriminant(&self.current().kind) == std::mem::discriminant(&expected) {
            Ok(self.advance())
        } else {
            Err(self.error_here(message))
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

fn comparison_operator(kind: &TokenKind) -> Option<BinaryOp> {
    match kind {
        TokenKind::EqualEqual => Some(BinaryOp::Equal),
        TokenKind::BangEqual => Some(BinaryOp::NotEqual),
        TokenKind::Less => Some(BinaryOp::Less),
        TokenKind::LessEqual => Some(BinaryOp::LessEqual),
        TokenKind::Greater => Some(BinaryOp::Greater),
        TokenKind::GreaterEqual => Some(BinaryOp::GreaterEqual),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{BinaryOp, ExprKind, StmtKind, TypeName, parse, parse_recovering};
    use evo_lexer::lex;

    fn parse_source(source: &str) -> super::Program {
        let tokens = lex(source).expect("lexing should succeed");
        parse(&tokens).expect("parsing should succeed")
    }

    #[test]
    fn existing_top_level_program_has_no_functions() {
        let program = parse_source("x = 1\nprint x\n");
        assert!(program.functions.is_empty());
        assert_eq!(program.statements.len(), 2);
    }

    #[test]
    fn parses_zero_and_multi_parameter_functions() {
        let program = parse_source(
            "fn answer() int\nreturn 42\nend\nfn add(a int, b int) int\nreturn a + b\nend\n",
        );
        assert_eq!(program.functions.len(), 2);
        assert_eq!(program.functions[0].name, "answer");
        assert!(program.functions[0].parameters.is_empty());
        assert_eq!(program.functions[0].return_type, TypeName::Int);
        assert_eq!(program.functions[1].parameters.len(), 2);
    }

    #[test]
    fn parses_calls_as_expressions_and_nested_arguments() {
        let program = parse_source("print add(1, mul(2, 3))\n");
        let StmtKind::Print(expr) = &program.statements[0].kind else {
            panic!("expected print");
        };
        let ExprKind::Call { name, arguments } = &expr.kind else {
            panic!("expected call");
        };
        assert_eq!(name, "add");
        assert_eq!(arguments.len(), 2);
        assert!(matches!(arguments[1].kind, ExprKind::Call { .. }));
    }

    #[test]
    fn parses_return_only_inside_function() {
        let program = parse_source("fn yes() bool\nreturn true\nend\n");
        assert!(matches!(
            program.functions[0].body[0].kind,
            StmtKind::Return(_)
        ));
        let tokens = lex("return true\n").expect("lexing should succeed");
        let error = parse(&tokens).expect_err("top-level return should fail");
        assert!(error.message.contains("only valid inside"));
    }

    #[test]
    fn rejects_nested_function_definition() {
        let tokens = lex("fn outer() int\nfn inner() int\nreturn 1\nend\nreturn 2\nend\n")
            .expect("lexing should succeed");
        let error = parse(&tokens).expect_err("nested fn should fail");
        assert!(error.message.contains("nested function"));
    }

    #[test]
    fn rejects_trailing_parameter_comma() {
        let tokens = lex("fn add(a int,) int\nreturn a\nend\n").expect("lexing should succeed");
        let error = parse(&tokens).expect_err("trailing comma should fail");
        assert!(error.message.contains("expected parameter"));
    }

    #[test]
    fn logical_and_control_flow_still_parse() {
        let program = parse_source(
            "x = 1\nif x > 0 and not false\nrepeat 1\nprint true\nend\nelse\nprint false\nend\n",
        );
        assert_eq!(program.statements.len(), 2);
        assert!(matches!(program.statements[1].kind, StmtKind::If { .. }));
    }

    #[test]
    fn comparison_precedence_remains_lower_than_arithmetic() {
        let program = parse_source("print 1 + 2 * 3 == 7\n");
        let StmtKind::Print(expr) = &program.statements[0].kind else {
            panic!("expected print");
        };
        let ExprKind::Binary { left, op, .. } = &expr.kind else {
            panic!("expected comparison");
        };
        assert_eq!(*op, BinaryOp::Equal);
        assert!(matches!(left.kind, ExprKind::Binary { op: BinaryOp::Add, .. }));
    }

    #[test]
    fn recovering_parser_matches_fail_fast_on_valid_function_source() {
        let source = "fn add(a int, b int) int\nreturn a + b\nend\nprint add(2, 3)\n";
        let tokens = lex(source).expect("lexing should succeed");
        assert_eq!(
            parse_recovering(&tokens).expect("recovery parse should succeed"),
            parse(&tokens).expect("fail-fast parse should succeed")
        );
    }
}
