from pathlib import Path


def replace_once(path, old, new):
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one anchor in {path}, found {count}: {old[:100]!r}")
    p.write_text(text.replace(old, new, 1))


parser = "crates/evo-parser/src/lib.rs"
replace_once(parser,
'''    SharedOwner(String),
    SharedRef(Box<TypeName>),
}''',
'''    SharedOwner(String),
    SharedRef(Box<TypeName>),
    Sequence(Box<TypeName>),
}''')
replace_once(parser,
'''    Match {
        value: Expr,
        arms: Vec<MatchArm>,
    },
}''',
'''    Match {
        value: Expr,
        arms: Vec<MatchArm>,
    },
    SequenceAppend {
        owner: String,
        value: Expr,
    },
    SequenceLookup {
        owner: String,
        index: Expr,
        binding: String,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
}''')
replace_once(parser,
'''    SharedDuplicate(Box<Expr>),
    Binary {''',
'''    SharedDuplicate(Box<Expr>),
    SequenceNew {
        element_type: TypeName,
    },
    Binary {''')
replace_once(parser,
'''    fn parse_type_name(&mut self) -> Result<TypeName, ParseError> {
        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "shared") {''',
'''    fn parse_type_name(&mut self) -> Result<TypeName, ParseError> {
        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "seq")
            && self.sequence_element_type_starts_at(self.index + 1)
        {
            self.advance();
            return Ok(TypeName::Sequence(Box::new(self.parse_sequence_element_type()?)));
        }

        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "shared") {''')
replace_once(parser,
'''    fn parse_owned_type_name(&mut self) -> Result<TypeName, ParseError> {''',
'''    fn parse_sequence_element_type(&mut self) -> Result<TypeName, ParseError> {
        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "shared") {
            let marker = self.advance().span;
            let token = self.advance();
            return match token.kind {
                TokenKind::Identifier(name) => Ok(TypeName::SharedOwner(name)),
                _ => Err(ParseError {
                    message: "sequence shared-owner elements require a nominal record type".to_owned(),
                    span: marker.join(token.span),
                }),
            };
        }
        let token = self.advance();
        match token.kind {
            TokenKind::TypeInt => Ok(TypeName::Int),
            TokenKind::TypeBool => Ok(TypeName::Bool),
            TokenKind::TypeString => Ok(TypeName::String),
            TokenKind::Identifier(name) if name == "seq" => Err(ParseError {
                message: "nested sequence element types are not supported in v0".to_owned(),
                span: token.span,
            }),
            TokenKind::Identifier(name) => Ok(TypeName::Named(name)),
            TokenKind::Ampersand => Err(ParseError {
                message: "reference sequence element types are not supported in v0".to_owned(),
                span: token.span,
            }),
            _ => Err(ParseError {
                message: "expected sequence element type".to_owned(),
                span: token.span,
            }),
        }
    }

    fn sequence_element_type_starts_at(&self, index: usize) -> bool {
        match self.tokens.get(index).map(|token| &token.kind) {
            Some(TokenKind::TypeInt | TokenKind::TypeBool | TokenKind::TypeString) => true,
            Some(TokenKind::Identifier(name)) if name == "shared" => self
                .tokens
                .get(index + 1)
                .is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_))),
            Some(TokenKind::Identifier(_)) => true,
            _ => false,
        }
    }

    fn parse_owned_type_name(&mut self) -> Result<TypeName, ParseError> {''')
replace_once(parser,
'''            TokenKind::Identifier(name) => {
                let start = self.advance().span;
                if !matches!(self.current().kind, TokenKind::Equal) {''',
'''            TokenKind::Identifier(name) => {
                let start = self.advance().span;
                if name == "append" && !matches!(self.current().kind, TokenKind::Equal) {
                    return self.parse_sequence_append(start);
                }
                if name == "lookup" && !matches!(self.current().kind, TokenKind::Equal) {
                    return self.parse_sequence_lookup(start);
                }
                if !matches!(self.current().kind, TokenKind::Equal) {''')
replace_once(parser,
'''    fn parse_repeat(&mut self) -> Result<Stmt, ParseError> {''',
'''    fn parse_sequence_append(&mut self, start: Span) -> Result<Stmt, ParseError> {
        let owner_token = self.advance();
        let TokenKind::Identifier(owner) = owner_token.kind else {
            return Err(ParseError {
                message: "expected sequence owner after 'append'".to_owned(),
                span: owner_token.span,
            });
        };
        self.expect_kind(TokenKind::Comma, "expected ',' after sequence owner")?;
        let value = self.parse_expression()?;
        Ok(Stmt {
            span: start.join(value.span),
            kind: StmtKind::SequenceAppend { owner, value },
        })
    }

    fn parse_sequence_lookup(&mut self, start: Span) -> Result<Stmt, ParseError> {
        let owner_token = self.advance();
        let TokenKind::Identifier(owner) = owner_token.kind else {
            return Err(ParseError {
                message: "expected sequence owner after 'lookup'".to_owned(),
                span: owner_token.span,
            });
        };
        self.expect_kind(TokenKind::Comma, "expected ',' after sequence owner")?;
        let index = self.parse_expression()?;
        let as_token = self.advance();
        if !matches!(&as_token.kind, TokenKind::Identifier(name) if name == "as") {
            return Err(ParseError {
                message: "expected contextual 'as' after lookup index".to_owned(),
                span: as_token.span,
            });
        }
        let binding_token = self.advance();
        let TokenKind::Identifier(binding) = binding_token.kind else {
            return Err(ParseError {
                message: "expected success binding after lookup 'as'".to_owned(),
                span: binding_token.span,
            });
        };
        if !matches!(self.current().kind, TokenKind::Newline) {
            return Err(self.error_here("expected end of line after lookup binding"));
        }
        self.skip_newlines();
        let mut then_body = Vec::new();
        while !matches!(self.current().kind, TokenKind::Else | TokenKind::End) {
            if self.is_eof() {
                return Err(self.error_here("missing 'else' and 'end' for lookup block"));
            }
            let statement = self.parse_statement()?;
            self.require_statement_terminator()?;
            then_body.push(statement);
            self.skip_newlines();
        }
        if !matches!(self.current().kind, TokenKind::Else) {
            return Err(self.error_here("checked lookup requires an explicit 'else' branch"));
        }
        self.advance();
        if !matches!(self.current().kind, TokenKind::Newline) {
            return Err(self.error_here("expected end of line after lookup 'else'"));
        }
        self.skip_newlines();
        let mut else_body = Vec::new();
        while !matches!(self.current().kind, TokenKind::End) {
            if self.is_eof() {
                return Err(self.error_here("missing 'end' for lookup block"));
            }
            let statement = self.parse_statement()?;
            self.require_statement_terminator()?;
            else_body.push(statement);
            self.skip_newlines();
        }
        let close = self.advance().span;
        Ok(Stmt {
            kind: StmtKind::SequenceLookup {
                owner,
                index,
                binding,
                then_body,
                else_body,
            },
            span: start.join(close),
        })
    }

    fn parse_repeat(&mut self) -> Result<Stmt, ParseError> {''')
replace_once(parser,
'''            TokenKind::Identifier(name) => self.parse_identifier_or_call(name, token.span),''',
'''            TokenKind::Identifier(name)
                if name == "seq" && self.sequence_constructor_starts_here() =>
            {
                let element_type = self.parse_sequence_element_type()?;
                self.expect_kind(TokenKind::LParen, "expected '(' after sequence element type")?;
                let close = self
                    .expect_kind(TokenKind::RParen, "expected ')' for empty sequence constructor")?
                    .span;
                Ok(Expr {
                    kind: ExprKind::SequenceNew { element_type },
                    span: token.span.join(close),
                })
            }
            TokenKind::Identifier(name) => self.parse_identifier_or_call(name, token.span),''')
replace_once(parser,
'''    fn parse_identifier_or_call(&mut self, name: String, start: Span) -> Result<Expr, ParseError> {''',
'''    fn sequence_constructor_starts_here(&self) -> bool {
        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "seq")
            && self.sequence_element_type_starts_at(self.index + 1)
        {
            return true;
        }
        if !self.sequence_element_type_starts_at(self.index) {
            return false;
        }
        let offset = if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "shared") {
            2
        } else {
            1
        };
        self.tokens
            .get(self.index + offset)
            .is_some_and(|token| matches!(token.kind, TokenKind::LParen))
    }

    fn parse_identifier_or_call(&mut self, name: String, start: Span) -> Result<Expr, ParseError> {''')

# Keep the new syntax fail-closed downstream until the semantic commit.
lowering = "crates/evo-lowering/src/lib.rs"
replace_once(lowering,
'''            SyntaxStmtKind::Match { .. } => {
                return Err(LowerError {''',
'''            SyntaxStmtKind::SequenceAppend { .. } | SyntaxStmtKind::SequenceLookup { .. } => {
                return Err(LowerError {
                    message: "append-only sequence syntax is parsed, but sequence semantic lowering is not implemented yet".to_owned(),
                    span: statement.span,
                });
            }
            SyntaxStmtKind::Match { .. } => {
                return Err(LowerError {''')
replace_once(lowering,
'''            SyntaxExprKind::SharedDuplicate(inner) => {''',
'''            SyntaxExprKind::SequenceNew { .. } => {
                return Err(LowerError {
                    message: "sequence construction syntax is parsed, but sequence semantic lowering is not implemented yet".to_owned(),
                    span: expr.span,
                });
            }
            SyntaxExprKind::SharedDuplicate(inner) => {''')
replace_once(lowering,
'''            | SyntaxExprKind::SharedDuplicate(_)
            | SyntaxExprKind::Binary { .. } => Ok(None),''',
'''            | SyntaxExprKind::SharedDuplicate(_)
            | SyntaxExprKind::SequenceNew { .. }
            | SyntaxExprKind::Binary { .. } => Ok(None),''')
replace_once(lowering,
'''        SyntaxStmtKind::Match { value, arms } => {''',
'''        SyntaxStmtKind::SequenceAppend { owner, value } => {
            *uses.entry(owner.clone()).or_insert(0) += 1;
            Self::collect_expr_identifier_uses(value, uses);
        }
        SyntaxStmtKind::SequenceLookup {
            owner,
            index,
            then_body,
            else_body,
            ..
        } => {
            *uses.entry(owner.clone()).or_insert(0) += 1;
            Self::collect_expr_identifier_uses(index, uses);
            for statement in then_body {
                Self::collect_statement_identifier_uses(statement, uses);
            }
            for statement in else_body {
                Self::collect_statement_identifier_uses(statement, uses);
            }
        }
        SyntaxStmtKind::Match { value, arms } => {''')
replace_once(lowering,
'''        | SyntaxExprKind::InputInt => {}''',
'''        | SyntaxExprKind::InputInt
        | SyntaxExprKind::SequenceNew { .. } => {}''')

borrow = "crates/evo-lowering/src/borrow_inference.rs"
replace_once(borrow,
'''            SyntaxStmtKind::Match { value, arms } => {''',
'''            SyntaxStmtKind::SequenceAppend { owner, value } => {
                if owner == parameter {
                    effects.inspect_uses += 1;
                }
                collect_expr_effects(value, parameter, UseMode::Consume, effects);
            }
            SyntaxStmtKind::SequenceLookup {
                owner,
                index,
                then_body,
                else_body,
                ..
            } => {
                if owner == parameter {
                    effects.inspect_uses += 1;
                }
                collect_expr_effects(index, parameter, UseMode::Consume, effects);
                collect_statement_effects(then_body, parameter, effects);
                collect_statement_effects(else_body, parameter, effects);
            }
            SyntaxStmtKind::Match { value, arms } => {''')
replace_once(borrow,
'''        | SyntaxExprKind::InputInt => {}''',
'''        | SyntaxExprKind::InputInt
        | SyntaxExprKind::SequenceNew { .. } => {}''')

formatter = "crates/evo-formatter/src/lib.rs"
replace_once(formatter,
'''                    | TokenKind::Case
            )
        }) {''',
'''                    | TokenKind::Case
            ) || matches!(kind, TokenKind::Identifier(name) if name == "lookup")
        }) {''')

Path("crates/evo-parser/tests/sequence_syntax_v0.rs").write_text(r'''use evo_lexer::lex;
use evo_parser::{ExprKind, StmtKind, TypeName, parse};

fn parse_source(source: &str) -> evo_parser::Program {
    parse(&lex(source).expect("lexing should succeed")).expect("parsing should succeed")
}

#[test]
fn parses_contextual_sequence_surface() {
    let program = parse_source(
        "record Item\nvalue int\nend\nfn keep(items seq Item) seq Item\nreturn items\nend\nitems = seq Item()\nappend items, Item(value = 7)\nlookup items, 0 as item\nprint item.value\nelse\nprint 0\nend\n",
    );
    assert_eq!(
        program.functions[0].parameters[0].type_name,
        TypeName::Sequence(Box::new(TypeName::Named("Item".to_owned())))
    );
    assert_eq!(
        program.functions[0].return_type,
        TypeName::Sequence(Box::new(TypeName::Named("Item".to_owned())))
    );
    let StmtKind::Bind { expr, .. } = &program.statements[0].kind else {
        panic!("expected sequence binding");
    };
    assert!(matches!(expr.kind, ExprKind::SequenceNew { .. }));
    assert!(matches!(program.statements[1].kind, StmtKind::SequenceAppend { .. }));
    let StmtKind::SequenceLookup { binding, then_body, else_body, .. } = &program.statements[2].kind else {
        panic!("expected checked lookup");
    };
    assert_eq!(binding, "item");
    assert_eq!(then_body.len(), 1);
    assert_eq!(else_body.len(), 1);
}

#[test]
fn contextual_words_remain_identifiers_outside_exact_positions() {
    let program = parse_source(
        "fn seq(value int) int\nreturn value\nend\nseq = 1\nappend = seq(seq)\nlookup = append\nas = lookup\nprint as\n",
    );
    assert_eq!(program.statements.len(), 5);
}

#[test]
fn checked_lookup_requires_explicit_failure_branch() {
    let error = parse(&lex("items = seq int()\nlookup items, 0 as value\nprint value\nend\n").unwrap())
        .expect_err("lookup must expose failure");
    assert!(error.message.contains("explicit 'else'"));
}

#[test]
fn rejects_nested_sequence_constructor_surface() {
    let error = parse(&lex("items = seq seq int()\n").unwrap())
        .expect_err("nested sequence is outside v0");
    assert!(error.message.contains("nested sequence"));
}
''')

Path("crates/evo-formatter/tests/sequence_syntax_v0.rs").write_text(r'''use evo_formatter::format_source;
use evo_lexer::lex;

#[test]
fn formats_contextual_lookup_block() {
    let source = "items=seq Item()\nappend items,Item(value=1)\nlookup items,0 as item\nprint item.value\nelse\nprint 0\nend\n";
    let tokens = lex(source).expect("sequence source should lex");
    assert_eq!(
        format_source(source, &tokens),
        "items = seq Item()\nappend items, Item(value = 1)\nlookup items, 0 as item\n    print item.value\nelse\n    print 0\nend\n"
    );
}
''')
