use super::{
    MAX_RECOVERED_ERRORS, MatchArm, MatchPattern, ParseError, Parser, Stmt, StmtKind, StopSet,
};
use evo_lexer::TokenKind;

impl<'a> Parser<'a> {
    pub(super) fn parse_match(&mut self) -> Result<Stmt, ParseError> {
        let start = self.advance().span;
        if matches!(self.current().kind, TokenKind::Newline | TokenKind::Eof) {
            return Err(self.error_here("expected expression after 'match'"));
        }
        let value = self.parse_expression()?;
        if !matches!(self.current().kind, TokenKind::Newline) {
            return Err(self.error_here("expected end of line after match expression"));
        }
        self.skip_newlines();

        if !matches!(self.current().kind, TokenKind::Case) {
            return Err(self.error_here("expected at least one 'case' arm in match"));
        }

        let mut arms = Vec::new();
        while matches!(self.current().kind, TokenKind::Case) {
            arms.push(self.parse_match_arm()?);
        }

        if self.is_eof() {
            return Err(self.error_here("missing 'end' for match block"));
        }
        let close = self
            .expect_kind(TokenKind::End, "missing 'end' for match block")?
            .span;
        Ok(Stmt {
            kind: StmtKind::Match { value, arms },
            span: start.join(close),
        })
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, ParseError> {
        let start = self.expect_kind(TokenKind::Case, "expected 'case'")?.span;
        let pattern = self.parse_match_pattern()?;
        if !matches!(self.current().kind, TokenKind::Newline) {
            return Err(self.error_here("expected end of line after case pattern"));
        }
        self.skip_newlines();

        let mut body = Vec::new();
        while !matches!(self.current().kind, TokenKind::Case | TokenKind::End) {
            if self.is_eof() {
                break;
            }
            let statement = self.parse_statement()?;
            self.require_statement_terminator()?;
            body.push(statement);
            self.skip_newlines();
        }

        let end = body.last().map_or(pattern.span, |statement| statement.span);
        Ok(MatchArm {
            pattern,
            body,
            span: start.join(end),
        })
    }

    fn parse_match_pattern(&mut self) -> Result<MatchPattern, ParseError> {
        let enum_token = self.advance();
        let TokenKind::Identifier(enum_name) = enum_token.kind else {
            return Err(ParseError {
                message: "expected enum name after 'case'".to_owned(),
                span: enum_token.span,
            });
        };
        self.expect_kind(TokenKind::Dot, "expected '.' after enum name in case pattern")?;
        let variant_token = self.advance();
        let TokenKind::Identifier(variant_name) = variant_token.kind else {
            return Err(ParseError {
                message: "expected variant name after '.' in case pattern".to_owned(),
                span: variant_token.span,
            });
        };

        let mut span = enum_token.span.join(variant_token.span);
        let binding = if matches!(self.current().kind, TokenKind::LParen) {
            self.advance();
            let binding_token = self.advance();
            let TokenKind::Identifier(binding) = binding_token.kind else {
                return Err(ParseError {
                    message: "expected payload binding inside case pattern".to_owned(),
                    span: binding_token.span,
                });
            };
            let close = self.expect_kind(
                TokenKind::RParen,
                "expected ')' after payload binding in case pattern",
            )?;
            span = enum_token.span.join(close.span);
            Some(binding)
        } else {
            None
        };

        Ok(MatchPattern {
            enum_name,
            variant_name,
            binding,
            span,
        })
    }

    pub(super) fn parse_match_recovering(&mut self, errors: &mut Vec<ParseError>) -> Option<Stmt> {
        let start = self.advance().span;
        if matches!(self.current().kind, TokenKind::Newline | TokenKind::Eof) {
            self.record_error(errors, self.error_here("expected expression after 'match'"));
            let found_end = self.skip_invalid_block();
            if !found_end && errors.len() < MAX_RECOVERED_ERRORS {
                self.record_error(errors, self.error_here("missing 'end' for match block"));
            }
            return None;
        }

        let value = match self.parse_expression() {
            Ok(value) => value,
            Err(error) => {
                self.record_error(errors, error);
                let found_end = self.skip_invalid_block();
                if !found_end && errors.len() < MAX_RECOVERED_ERRORS {
                    self.record_error(errors, self.error_here("missing 'end' for match block"));
                }
                return None;
            }
        };
        if !matches!(self.current().kind, TokenKind::Newline) {
            self.record_error(
                errors,
                self.error_here("expected end of line after match expression"),
            );
            self.synchronize_statement(StopSet::END_OR_CASE);
        }
        self.skip_newlines();

        if !matches!(self.current().kind, TokenKind::Case) {
            self.record_error(
                errors,
                self.error_here("expected at least one 'case' arm in match"),
            );
            self.skip_until_case_or_end();
        }

        let mut arms = Vec::new();
        while matches!(self.current().kind, TokenKind::Case)
            && errors.len() < MAX_RECOVERED_ERRORS
        {
            let arm_start = self.advance().span;
            let pattern = match self.parse_match_pattern() {
                Ok(pattern) => Some(pattern),
                Err(error) => {
                    self.record_error(errors, error);
                    self.synchronize_statement(StopSet::END_OR_CASE);
                    None
                }
            };

            if !matches!(self.current().kind, TokenKind::Newline)
                && !matches!(self.current().kind, TokenKind::Case | TokenKind::End)
            {
                self.record_error(
                    errors,
                    self.error_here("expected end of line after case pattern"),
                );
                self.synchronize_statement(StopSet::END_OR_CASE);
            }
            self.skip_newlines();

            let body = self.parse_statements_recovering(StopSet::END_OR_CASE, errors);
            if let Some(pattern) = pattern {
                let end = body.last().map_or(pattern.span, |statement| statement.span);
                arms.push(MatchArm {
                    pattern,
                    body,
                    span: arm_start.join(end),
                });
            }
        }

        if errors.len() >= MAX_RECOVERED_ERRORS {
            return None;
        }
        if self.is_eof() {
            self.record_error(errors, self.error_here("missing 'end' for match block"));
            return None;
        }
        if !matches!(self.current().kind, TokenKind::End) {
            self.record_error(errors, self.error_here("missing 'end' for match block"));
            return None;
        }
        let close = self.advance().span;
        Some(Stmt {
            kind: StmtKind::Match { value, arms },
            span: start.join(close),
        })
    }

    fn skip_until_case_or_end(&mut self) {
        while !self.is_eof()
            && !matches!(self.current().kind, TokenKind::Case | TokenKind::End)
        {
            self.advance();
        }
    }
}
