from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one anchor, found {count}: {old[:120]!r}")
    p.write_text(text.replace(old, new, 1))


PARSER = "crates/evo-parser/src/lib.rs"
LOWERING = "crates/evo-lowering/src/lib.rs"
RECORD_ENV = "crates/evo-lowering/src/record_environment_records.rs"
RECORD_IR = "crates/evo-lowering/src/record_ir.rs"
OWNERSHIP = "crates/evo-lowering/src/record_ownership.rs"
BORROW_INFERENCE = "crates/evo-lowering/src/borrow_inference.rs"
FORMATTER = "crates/evo-formatter/src/lib.rs"
CODEGEN = "crates/evo-codegen-rust/src/lib.rs"

# Parser data model.
replace_once(
    PARSER,
    "pub enum RecordFieldType {\n    Int,\n    Bool,\n    String,\n    Named(String),\n}\n",
    "pub enum RecordFieldType {\n    Int,\n    Bool,\n    String,\n    Named(String),\n    Handle(String),\n}\n",
)
replace_once(
    PARSER,
    "    SharedRef(Box<TypeName>),\n    Sequence(Box<TypeName>),\n}\n",
    "    SharedRef(Box<TypeName>),\n    Sequence(Box<TypeName>),\n    Arena(Box<TypeName>),\n    Handle(Box<TypeName>),\n}\n",
)
replace_once(
    PARSER,
    "    SequenceLookup {\n        owner: String,\n        index: Expr,\n        binding: String,\n        then_body: Vec<Stmt>,\n        else_body: Vec<Stmt>,\n    },\n}\n",
    "    SequenceLookup {\n        owner: String,\n        index: Expr,\n        binding: String,\n        then_body: Vec<Stmt>,\n        else_body: Vec<Stmt>,\n    },\n    ArenaInsert {\n        owner: String,\n        value: Expr,\n        binding: String,\n    },\n    ArenaRemove {\n        owner: String,\n        handle: Expr,\n        binding: String,\n        then_body: Vec<Stmt>,\n        else_body: Vec<Stmt>,\n    },\n}\n",
)
replace_once(
    PARSER,
    "    SequenceNew {\n        element_type: TypeName,\n    },\n    Binary {\n",
    "    SequenceNew {\n        element_type: TypeName,\n    },\n    ArenaNew {\n        element_type: TypeName,\n    },\n    Binary {\n",
)

# Record handle fields are contextual and nominal in v0.
replace_once(
    PARSER,
    "            let type_token = self.current().clone();\n            let type_name = self.parse_record_field_type()?;\n            fields.push(RecordField {\n                name: field_name,\n                type_name,\n                span: field_name_token.span.join(type_token.span),\n            });\n",
    "            let type_name = self.parse_record_field_type()?;\n            let type_end = self.tokens[self.index.saturating_sub(1)].span;\n            fields.push(RecordField {\n                name: field_name,\n                type_name,\n                span: field_name_token.span.join(type_end),\n            });\n",
)
replace_once(
    PARSER,
    "    fn parse_record_field_type(&mut self) -> Result<RecordFieldType, ParseError> {\n        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"shared\")\n",
    "    fn parse_record_field_type(&mut self) -> Result<RecordFieldType, ParseError> {\n        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"handle\") {\n            let marker = self.advance().span;\n            let token = self.advance();\n            return match token.kind {\n                TokenKind::Identifier(name) => Ok(RecordFieldType::Handle(name)),\n                _ => Err(ParseError {\n                    message: \"record handle fields require a nominal record type\".to_owned(),\n                    span: marker.join(token.span),\n                }),\n            };\n        }\n        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"shared\")\n",
)

# Contextual arena/handle function types.
replace_once(
    PARSER,
    "    fn parse_type_name(&mut self) -> Result<TypeName, ParseError> {\n        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"seq\")\n",
    "    fn parse_type_name(&mut self) -> Result<TypeName, ParseError> {\n        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"arena\")\n            && self.arena_element_type_starts_at(self.index + 1)\n        {\n            self.advance();\n            return Ok(TypeName::Arena(Box::new(self.parse_arena_element_type()?)));\n        }\n        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"handle\")\n            && self.arena_element_type_starts_at(self.index + 1)\n        {\n            self.advance();\n            return Ok(TypeName::Handle(Box::new(self.parse_arena_element_type()?)));\n        }\n        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"seq\")\n",
)
replace_once(
    PARSER,
    "    fn parse_sequence_element_type(&mut self) -> Result<TypeName, ParseError> {\n        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"shared\") {\n",
    "    fn parse_sequence_element_type(&mut self) -> Result<TypeName, ParseError> {\n        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"handle\") {\n            let marker = self.advance().span;\n            if !self.arena_element_type_starts_at(self.index) {\n                return Err(ParseError {\n                    message: \"sequence handle elements require an arena payload type\".to_owned(),\n                    span: marker,\n                });\n            }\n            return Ok(TypeName::Handle(Box::new(self.parse_arena_element_type()?)));\n        }\n        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"shared\") {\n",
)
replace_once(
    PARSER,
    "    fn sequence_element_type_starts_at(&self, index: usize) -> bool {\n        match self.tokens.get(index).map(|token| &token.kind) {\n            Some(TokenKind::TypeInt | TokenKind::TypeBool | TokenKind::TypeString) => true,\n            Some(TokenKind::Identifier(name)) if name == \"shared\" => self\n                .tokens\n                .get(index + 1)\n                .is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_))),\n            Some(TokenKind::Identifier(_)) => true,\n            _ => false,\n        }\n    }\n",
    "    fn sequence_element_type_starts_at(&self, index: usize) -> bool {\n        match self.tokens.get(index).map(|token| &token.kind) {\n            Some(TokenKind::TypeInt | TokenKind::TypeBool | TokenKind::TypeString) => true,\n            Some(TokenKind::Identifier(name)) if name == \"shared\" => self\n                .tokens\n                .get(index + 1)\n                .is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_))),\n            Some(TokenKind::Identifier(name)) if name == \"handle\" => {\n                self.arena_element_type_starts_at(index + 1)\n            }\n            Some(TokenKind::Identifier(_)) => true,\n            _ => false,\n        }\n    }\n\n    fn arena_element_type_starts_at(&self, index: usize) -> bool {\n        match self.tokens.get(index).map(|token| &token.kind) {\n            Some(TokenKind::TypeInt | TokenKind::TypeBool | TokenKind::TypeString) => true,\n            Some(TokenKind::Identifier(name)) if name == \"shared\" => self\n                .tokens\n                .get(index + 1)\n                .is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_))),\n            Some(TokenKind::Identifier(_)) => true,\n            _ => false,\n        }\n    }\n\n    fn parse_arena_element_type(&mut self) -> Result<TypeName, ParseError> {\n        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"shared\") {\n            let marker = self.advance().span;\n            let token = self.advance();\n            return match token.kind {\n                TokenKind::Identifier(name) => Ok(TypeName::SharedOwner(name)),\n                _ => Err(ParseError {\n                    message: \"arena shared-owner payloads require a nominal record type\".to_owned(),\n                    span: marker.join(token.span),\n                }),\n            };\n        }\n        let token = self.advance();\n        match token.kind {\n            TokenKind::TypeInt => Ok(TypeName::Int),\n            TokenKind::TypeBool => Ok(TypeName::Bool),\n            TokenKind::TypeString => Ok(TypeName::String),\n            TokenKind::Identifier(name) if matches!(name.as_str(), \"arena\" | \"handle\" | \"seq\") => Err(ParseError {\n                message: \"nested arena/handle/sequence payload types are not supported in arena v0\".to_owned(),\n                span: token.span,\n            }),\n            TokenKind::Identifier(name) => Ok(TypeName::Named(name)),\n            TokenKind::Ampersand => Err(ParseError {\n                message: \"reference arena payload types are not supported in v0\".to_owned(),\n                span: token.span,\n            }),\n            _ => Err(ParseError {\n                message: \"expected arena payload type\".to_owned(),\n                span: token.span,\n            }),\n        }\n    }\n",
)

# Contextual insert/remove statements.
replace_once(
    PARSER,
    "                if name == \"append\" && !matches!(self.current().kind, TokenKind::Equal) {\n                    return self.parse_sequence_append(start);\n                }\n                if name == \"lookup\" && !matches!(self.current().kind, TokenKind::Equal) {\n                    return self.parse_sequence_lookup(start);\n                }\n",
    "                if name == \"insert\" && !matches!(self.current().kind, TokenKind::Equal) {\n                    return self.parse_arena_insert(start);\n                }\n                if name == \"remove\" && !matches!(self.current().kind, TokenKind::Equal) {\n                    return self.parse_arena_remove(start);\n                }\n                if name == \"append\" && !matches!(self.current().kind, TokenKind::Equal) {\n                    return self.parse_sequence_append(start);\n                }\n                if name == \"lookup\" && !matches!(self.current().kind, TokenKind::Equal) {\n                    return self.parse_sequence_lookup(start);\n                }\n",
)
insert_methods = r'''    fn parse_arena_insert(&mut self, start: Span) -> Result<Stmt, ParseError> {
        let owner_token = self.advance();
        let TokenKind::Identifier(owner) = owner_token.kind else {
            return Err(ParseError {
                message: "expected arena owner after 'insert'".to_owned(),
                span: owner_token.span,
            });
        };
        self.expect_kind(TokenKind::Comma, "expected ',' after arena owner")?;
        let value = self.parse_expression()?;
        let as_token = self.advance();
        if !matches!(&as_token.kind, TokenKind::Identifier(name) if name == "as") {
            return Err(ParseError {
                message: "expected contextual 'as' after inserted value".to_owned(),
                span: as_token.span,
            });
        }
        let binding_token = self.advance();
        let TokenKind::Identifier(binding) = binding_token.kind else {
            return Err(ParseError {
                message: "expected handle binding after insert 'as'".to_owned(),
                span: binding_token.span,
            });
        };
        Ok(Stmt {
            kind: StmtKind::ArenaInsert {
                owner,
                value,
                binding,
            },
            span: start.join(binding_token.span),
        })
    }

    fn parse_arena_remove(&mut self, start: Span) -> Result<Stmt, ParseError> {
        let owner_token = self.advance();
        let TokenKind::Identifier(owner) = owner_token.kind else {
            return Err(ParseError {
                message: "expected arena owner after 'remove'".to_owned(),
                span: owner_token.span,
            });
        };
        self.expect_kind(TokenKind::Comma, "expected ',' after arena owner")?;
        let handle = self.parse_expression()?;
        let as_token = self.advance();
        if !matches!(&as_token.kind, TokenKind::Identifier(name) if name == "as") {
            return Err(ParseError {
                message: "expected contextual 'as' after removal handle".to_owned(),
                span: as_token.span,
            });
        }
        let binding_token = self.advance();
        let TokenKind::Identifier(binding) = binding_token.kind else {
            return Err(ParseError {
                message: "expected success binding after remove 'as'".to_owned(),
                span: binding_token.span,
            });
        };
        if !matches!(self.current().kind, TokenKind::Newline) {
            return Err(self.error_here("expected end of line after remove binding"));
        }
        self.skip_newlines();
        let mut then_body = Vec::new();
        while !matches!(self.current().kind, TokenKind::Else | TokenKind::End) {
            if self.is_eof() {
                return Err(self.error_here("missing 'else' and 'end' for remove block"));
            }
            let statement = self.parse_statement()?;
            self.require_statement_terminator()?;
            then_body.push(statement);
            self.skip_newlines();
        }
        if !matches!(self.current().kind, TokenKind::Else) {
            return Err(self.error_here("checked remove requires an explicit 'else' branch"));
        }
        self.advance();
        if !matches!(self.current().kind, TokenKind::Newline) {
            return Err(self.error_here("expected end of line after remove 'else'"));
        }
        self.skip_newlines();
        let mut else_body = Vec::new();
        while !matches!(self.current().kind, TokenKind::End) {
            if self.is_eof() {
                return Err(self.error_here("missing 'end' for remove block"));
            }
            let statement = self.parse_statement()?;
            self.require_statement_terminator()?;
            else_body.push(statement);
            self.skip_newlines();
        }
        let close = self.advance().span;
        Ok(Stmt {
            kind: StmtKind::ArenaRemove {
                owner,
                handle,
                binding,
                then_body,
                else_body,
            },
            span: start.join(close),
        })
    }

'''
replace_once(PARSER, "    fn parse_sequence_append(&mut self, start: Span) -> Result<Stmt, ParseError> {\n", insert_methods + "    fn parse_sequence_append(&mut self, start: Span) -> Result<Stmt, ParseError> {\n")

# Contextual arena constructor and constructor lookahead.
replace_once(
    PARSER,
    "            TokenKind::Identifier(name)\n                if name == \"seq\" && self.sequence_constructor_starts_here() =>\n            {\n",
    "            TokenKind::Identifier(name)\n                if name == \"arena\" && self.arena_constructor_starts_here() =>\n            {\n                let element_type = self.parse_arena_element_type()?;\n                self.expect_kind(TokenKind::LParen, \"expected '(' after arena payload type\")?;\n                let close = self\n                    .expect_kind(TokenKind::RParen, \"expected ')' for empty arena constructor\")?\n                    .span;\n                Ok(Expr {\n                    kind: ExprKind::ArenaNew { element_type },\n                    span: token.span.join(close),\n                })\n            }\n            TokenKind::Identifier(name)\n                if name == \"seq\" && self.sequence_constructor_starts_here() =>\n            {\n",
)
replace_once(
    PARSER,
    "    fn sequence_constructor_starts_here(&self) -> bool {\n        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"seq\")\n            && self.sequence_element_type_starts_at(self.index + 1)\n        {\n            return true;\n        }\n        if !self.sequence_element_type_starts_at(self.index) {\n            return false;\n        }\n        let offset = if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"shared\")\n        {\n            2\n        } else {\n            1\n        };\n        self.tokens\n            .get(self.index + offset)\n            .is_some_and(|token| matches!(token.kind, TokenKind::LParen))\n    }\n",
    "    fn arena_element_type_width_at(&self, index: usize) -> Option<usize> {\n        match self.tokens.get(index).map(|token| &token.kind) {\n            Some(TokenKind::TypeInt | TokenKind::TypeBool | TokenKind::TypeString) => Some(1),\n            Some(TokenKind::Identifier(name)) if name == \"shared\" => self\n                .tokens\n                .get(index + 1)\n                .is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_)))\n                .then_some(2),\n            Some(TokenKind::Identifier(_)) | Some(TokenKind::Ampersand) => Some(1),\n            _ => None,\n        }\n    }\n\n    fn sequence_element_type_width_at(&self, index: usize) -> Option<usize> {\n        match self.tokens.get(index).map(|token| &token.kind) {\n            Some(TokenKind::Identifier(name)) if name == \"handle\" => self\n                .arena_element_type_width_at(index + 1)\n                .map(|width| width + 1),\n            _ => self.arena_element_type_width_at(index),\n        }\n    }\n\n    fn sequence_constructor_starts_here(&self) -> bool {\n        self.sequence_element_type_width_at(self.index)\n            .and_then(|width| self.tokens.get(self.index + width))\n            .is_some_and(|token| matches!(token.kind, TokenKind::LParen))\n    }\n\n    fn arena_constructor_starts_here(&self) -> bool {\n        self.arena_element_type_width_at(self.index)\n            .and_then(|width| self.tokens.get(self.index + width))\n            .is_some_and(|token| matches!(token.kind, TokenKind::LParen))\n    }\n",
)

# Formatter: remove is a checked block exactly like lookup.
replace_once(
    FORMATTER,
    "            ) || matches!(kind, TokenKind::Identifier(name) if name == \"lookup\")\n",
    "            ) || matches!(kind, TokenKind::Identifier(name) if matches!(name.as_str(), \"lookup\" | \"remove\"))\n",
)

# Semantic type plumbing. Arena is move-only; handles are copy-like.
replace_once(
    RECORD_ENV,
    "    SharedRef(Box<SemanticType>),\n    Sequence(Box<SemanticType>),\n}\n",
    "    SharedRef(Box<SemanticType>),\n    Sequence(Box<SemanticType>),\n    Arena(Box<SemanticType>),\n    Handle(Box<SemanticType>),\n}\n",
)
replace_once(
    RECORD_ENV,
    "        !matches!(self, Self::Record(_) | Self::SharedOwner(_) | Self::Sequence(_))\n",
    "        !matches!(\n            self,\n            Self::Record(_) | Self::SharedOwner(_) | Self::Sequence(_) | Self::Arena(_)\n        )\n",
)
replace_once(
    RECORD_ENV,
    "            SyntaxTypeName::Sequence(inner) => self\n                .resolve_type_name(inner, span)\n                .map(|inner| SemanticType::Sequence(Box::new(inner))),\n",
    "            SyntaxTypeName::Sequence(inner) => self\n                .resolve_type_name(inner, span)\n                .map(|inner| SemanticType::Sequence(Box::new(inner))),\n            SyntaxTypeName::Arena(inner) => self\n                .resolve_type_name(inner, span)\n                .map(|inner| SemanticType::Arena(Box::new(inner))),\n            SyntaxTypeName::Handle(inner) => self\n                .resolve_type_name(inner, span)\n                .map(|inner| SemanticType::Handle(Box::new(inner))),\n",
)
replace_once(
    RECORD_ENV,
    "                SyntaxFieldType::Named(name) => {\n                    if !record_names.contains_key(name) {\n                        return Err(LowerError {\n                            message: format!(\n                                \"unknown record type {name:?} for field {:?} in record {:?}\",\n                                field.name, record.name\n                            ),\n                            span: field.span,\n                        });\n                    }\n                    SemanticType::Record(name.clone())\n                }\n",
    "                SyntaxFieldType::Named(name) => {\n                    if !record_names.contains_key(name) {\n                        return Err(LowerError {\n                            message: format!(\n                                \"unknown record type {name:?} for field {:?} in record {:?}\",\n                                field.name, record.name\n                            ),\n                            span: field.span,\n                        });\n                    }\n                    SemanticType::Record(name.clone())\n                }\n                SyntaxFieldType::Handle(name) => {\n                    if !record_names.contains_key(name) {\n                        return Err(LowerError {\n                            message: format!(\n                                \"unknown record type {name:?} for handle field {:?} in record {:?}\",\n                                field.name, record.name\n                            ),\n                            span: field.span,\n                        });\n                    }\n                    SemanticType::Handle(Box::new(SemanticType::Record(name.clone())))\n                }\n",
)
replace_once(
    RECORD_ENV,
    "        SemanticType::Sequence(inner) => format!(\"seq {}\", semantic_type_label(inner)),\n",
    "        SemanticType::Sequence(inner) => format!(\"seq {}\", semantic_type_label(inner)),\n        SemanticType::Arena(inner) => format!(\"arena {}\", semantic_type_label(inner)),\n        SemanticType::Handle(inner) => format!(\"handle {}\", semantic_type_label(inner)),\n",
)

# Record IR preserves fixed-size handle fields without treating them as by-value record recursion.
replace_once(RECORD_IR, "    Named(String),\n}\n", "    Named(String),\n    Handle(String),\n}\n")
replace_once(
    RECORD_IR,
    "        SyntaxFieldType::Named(name) => RecordType::Named(name.clone()),\n",
    "        SyntaxFieldType::Named(name) => RecordType::Named(name.clone()),\n        SyntaxFieldType::Handle(name) => RecordType::Handle(name.clone()),\n",
)

# Lowering value types and exhaustive syntax traversal. Actual arena operations remain fail-closed in this syntax commit.
replace_once(
    LOWERING,
    "    SharedRef(Box<ValueType>),\n    Sequence(Box<ValueType>),\n}\n",
    "    SharedRef(Box<ValueType>),\n    Sequence(Box<ValueType>),\n    Arena(Box<ValueType>),\n    Handle(Box<ValueType>),\n}\n",
)
replace_once(
    LOWERING,
    "        StmtKind::SequenceLookup {\n            then_body,\n            else_body,\n            ..\n        } => block_always_returns(then_body) && block_always_returns(else_body),\n",
    "        StmtKind::SequenceLookup {\n            then_body,\n            else_body,\n            ..\n        } => block_always_returns(then_body) && block_always_returns(else_body),\n",
)
replace_once(
    LOWERING,
    "        ValueType::Sequence(inner) => SemanticType::Sequence(Box::new(semantic_type(inner))),\n",
    "        ValueType::Sequence(inner) => SemanticType::Sequence(Box::new(semantic_type(inner))),\n        ValueType::Arena(inner) => SemanticType::Arena(Box::new(semantic_type(inner))),\n        ValueType::Handle(inner) => SemanticType::Handle(Box::new(semantic_type(inner))),\n",
)
replace_once(
    LOWERING,
    "        SemanticType::Sequence(inner) => ValueType::Sequence(Box::new(lowered_value_type(inner))),\n",
    "        SemanticType::Sequence(inner) => ValueType::Sequence(Box::new(lowered_value_type(inner))),\n        SemanticType::Arena(inner) => ValueType::Arena(Box::new(lowered_value_type(inner))),\n        SemanticType::Handle(inner) => ValueType::Handle(Box::new(lowered_value_type(inner))),\n",
)
replace_once(
    LOWERING,
    "        ValueType::Sequence(inner) => format!(\"seq {}\", type_label(inner)),\n",
    "        ValueType::Sequence(inner) => format!(\"seq {}\", type_label(inner)),\n        ValueType::Arena(inner) => format!(\"arena {}\", type_label(inner)),\n        ValueType::Handle(inner) => format!(\"handle {}\", type_label(inner)),\n",
)
replace_once(
    LOWERING,
    "                        ValueType::Sequence(_) => self.reference_tracker.ensure_owner_operation_allowed(\n                            name,\n                            \"sequence\",\n                            OwnerOperation::Reinitialize,\n                            statement.span,\n                        )?,\n",
    "                        ValueType::Sequence(_) => self.reference_tracker.ensure_owner_operation_allowed(\n                            name,\n                            \"sequence\",\n                            OwnerOperation::Reinitialize,\n                            statement.span,\n                        )?,\n                        ValueType::Arena(_) => self.reference_tracker.ensure_owner_operation_allowed(\n                            name,\n                            \"arena\",\n                            OwnerOperation::Reinitialize,\n                            statement.span,\n                        )?,\n",
)
replace_once(
    LOWERING,
    "                | ValueType::SharedRef(_)\n                | ValueType::Sequence(_)\n",
    "                | ValueType::SharedRef(_)\n                | ValueType::Sequence(_)\n                | ValueType::Arena(_)\n                | ValueType::Handle(_)\n",
)
replace_once(
    LOWERING,
    "            SyntaxStmtKind::Match { .. } => {\n",
    "            SyntaxStmtKind::ArenaInsert { .. } | SyntaxStmtKind::ArenaRemove { .. } => {\n                return Err(LowerError {\n                    message: \"generational arena syntax is parsed and typed, but arena semantic lowering is not enabled in this commit\".to_owned(),\n                    span: statement.span,\n                });\n            }\n            SyntaxStmtKind::Match { .. } => {\n",
)
replace_once(
    LOWERING,
    "            SyntaxStmtKind::Print(_)\n            | SyntaxStmtKind::Return(_)\n            | SyntaxStmtKind::SequenceAppend { .. } => false,\n",
    "            SyntaxStmtKind::Print(_)\n            | SyntaxStmtKind::Return(_)\n            | SyntaxStmtKind::SequenceAppend { .. }\n            | SyntaxStmtKind::ArenaInsert { .. } => false,\n            SyntaxStmtKind::ArenaRemove { then_body, else_body, .. } => {\n                Self::block_reassigns_name(then_body, target)\n                    || Self::block_reassigns_name(else_body, target)\n            }\n",
)
replace_once(
    LOWERING,
    "                        ValueType::Sequence(_) => self.reference_tracker.ensure_owner_operation_allowed(\n                            name,\n                            \"sequence\",\n                            OwnerOperation::Move,\n                            expr.span,\n                        )?,\n",
    "                        ValueType::Sequence(_) => self.reference_tracker.ensure_owner_operation_allowed(\n                            name,\n                            \"sequence\",\n                            OwnerOperation::Move,\n                            expr.span,\n                        )?,\n                        ValueType::Arena(_) => self.reference_tracker.ensure_owner_operation_allowed(\n                            name,\n                            \"arena\",\n                            OwnerOperation::Move,\n                            expr.span,\n                        )?,\n",
)
replace_once(
    LOWERING,
    "            SyntaxExprKind::SequenceNew { element_type } => {\n",
    "            SyntaxExprKind::ArenaNew { .. } => {\n                return Err(LowerError {\n                    message: \"generational arena construction is parsed, but arena semantic lowering is not enabled in this commit\".to_owned(),\n                    span: expr.span,\n                });\n            }\n            SyntaxExprKind::SequenceNew { element_type } => {\n",
)
replace_once(
    LOWERING,
    "                        ValueType::Record(_) | ValueType::SharedOwner(_) | ValueType::Sequence(_)\n",
    "                        ValueType::Record(_)\n                            | ValueType::SharedOwner(_)\n                            | ValueType::Sequence(_)\n                            | ValueType::Arena(_)\n                            | ValueType::Handle(_)\n",
)
replace_once(
    LOWERING,
    "                            ValueType::Record(_) | ValueType::SharedOwner(_) | ValueType::Sequence(_)\n",
    "                            ValueType::Record(_)\n                                | ValueType::SharedOwner(_)\n                                | ValueType::Sequence(_)\n                                | ValueType::Arena(_)\n                                | ValueType::Handle(_)\n",
)
replace_once(
    LOWERING,
    "            ValueType::Integer | ValueType::Bool | ValueType::String | ValueType::Sequence(_) => Err(LowerError {\n",
    "            ValueType::Integer\n            | ValueType::Bool\n            | ValueType::String\n            | ValueType::Sequence(_)\n            | ValueType::Arena(_)\n            | ValueType::Handle(_) => Err(LowerError {\n",
)
replace_once(
    LOWERING,
    "            | SyntaxExprKind::SequenceNew { .. }\n            | SyntaxExprKind::Binary { .. } => Ok(None),\n",
    "            | SyntaxExprKind::SequenceNew { .. }\n            | SyntaxExprKind::ArenaNew { .. }\n            | SyntaxExprKind::Binary { .. } => Ok(None),\n",
)
replace_once(
    LOWERING,
    "                    | ValueType::String\n                    | ValueType::Sequence(_) => {\n",
    "                    | ValueType::String\n                    | ValueType::Sequence(_)\n                    | ValueType::Arena(_)\n                    | ValueType::Handle(_) => {\n",
)
replace_once(
    LOWERING,
    "        SyntaxStmtKind::SequenceLookup {\n            owner,\n            index,\n            then_body,\n            else_body,\n            ..\n        } => {\n",
    "        SyntaxStmtKind::SequenceLookup {\n            owner,\n            index,\n            then_body,\n            else_body,\n            ..\n        } => {\n",
)
# Insert identifier-use handling for arena statements immediately before Match.
replace_once(
    LOWERING,
    "        SyntaxStmtKind::Match { value, arms } => {\n",
    "        SyntaxStmtKind::ArenaInsert { owner, value, .. } => {\n            *uses.entry(owner.clone()).or_insert(0) += 1;\n            Self::collect_expr_identifier_uses(value, uses);\n        }\n        SyntaxStmtKind::ArenaRemove {\n            owner,\n            handle,\n            then_body,\n            else_body,\n            ..\n        } => {\n            *uses.entry(owner.clone()).or_insert(0) += 1;\n            Self::collect_expr_identifier_uses(handle, uses);\n            for statement in then_body {\n                Self::collect_statement_identifier_uses(statement, uses);\n            }\n            for statement in else_body {\n                Self::collect_statement_identifier_uses(statement, uses);\n            }\n        }\n        SyntaxStmtKind::Match { value, arms } => {\n",
)
replace_once(
    LOWERING,
    "        | SyntaxExprKind::SequenceNew { .. } => {}\n",
    "        | SyntaxExprKind::SequenceNew { .. }\n        | SyntaxExprKind::ArenaNew { .. } => {}\n",
)

# Borrow inference understands syntax ownership boundaries without enabling arena semantics yet.
replace_once(
    BORROW_INFERENCE,
    "            SyntaxStmtKind::Match { value, arms } => {\n",
    "            SyntaxStmtKind::ArenaInsert { owner, value, .. } => {\n                if owner == parameter {\n                    effects.inspect_uses += 1;\n                }\n                collect_expr_effects(value, parameter, UseMode::Consume, effects);\n            }\n            SyntaxStmtKind::ArenaRemove {\n                owner,\n                handle,\n                then_body,\n                else_body,\n                ..\n            } => {\n                if owner == parameter {\n                    effects.inspect_uses += 1;\n                }\n                collect_expr_effects(handle, parameter, UseMode::Consume, effects);\n                collect_statement_effects(then_body, parameter, effects);\n                collect_statement_effects(else_body, parameter, effects);\n            }\n            SyntaxStmtKind::Match { value, arms } => {\n",
)
replace_once(
    BORROW_INFERENCE,
    "        | SyntaxExprKind::SequenceNew { .. } => {}\n",
    "        | SyntaxExprKind::SequenceNew { .. }\n        | SyntaxExprKind::ArenaNew { .. } => {}\n",
)

# Move diagnostics classify arena owners distinctly; handles remain copy-like via SemanticType.
replace_once(
    OWNERSHIP,
    "        Some(SemanticType::Sequence(_)) => \"sequence\",\n        _ => \"record\",\n",
    "        Some(SemanticType::Sequence(_)) => \"sequence\",\n        Some(SemanticType::Arena(_)) => \"arena\",\n        _ => \"record\",\n",
)

# Codegen type spelling only; runtime helpers arrive with semantic implementation.
replace_once(
    CODEGEN,
    "        ValueType::Sequence(inner) => format!(\"Vec<{}>\", rust_type(inner)),\n",
    "        ValueType::Sequence(inner) => format!(\"Vec<{}>\", rust_type(inner)),\n        ValueType::Arena(inner) => format!(\"__EvoArena<{}>\", rust_type(inner)),\n        ValueType::Handle(inner) => format!(\"__EvoHandle<{}>\", rust_type(inner)),\n",
)
replace_once(
    CODEGEN,
    "        RecordType::Named(name) => generated_record_name(name),\n",
    "        RecordType::Named(name) => generated_record_name(name),\n        RecordType::Handle(name) => format!(\"__EvoHandle<{}>\", generated_record_name(name)),\n",
)

# Existing sequence test previously asserted that remove syntax did not exist. It is now contextual arena syntax.
seq_test = Path("crates/evo-parser/tests/sequence_syntax_v0.rs")
text = seq_test.read_text()
old = '''#[test]\nfn removal_surface_stays_absent_in_v0() {\n    let error = parse(&lex("items = seq int()\\nremove items, 0\\n").unwrap())\n        .expect_err("v0 must not expose removal syntax");\n    assert!(error.message.contains("expected '=' after binding name"));\n}\n'''
new = '''#[test]\nfn remove_remains_an_identifier_when_used_as_a_binding() {\n    parse_source("remove = 1\\nprint remove\\n");\n}\n'''
if text.count(old) != 1:
    raise SystemExit("sequence parser removal anchor drifted")
seq_test.write_text(text.replace(old, new, 1))

# New focused parser/formatter coverage.
Path("crates/evo-parser/tests/arena_syntax_v0.rs").write_text(r'''use evo_lexer::lex;
use evo_parser::{ExprKind, RecordFieldType, StmtKind, TypeName, parse};

fn parse_source(source: &str) -> evo_parser::Program {
    parse(&lex(source).expect("arena source should lex")).expect("arena source should parse")
}

#[test]
fn parses_contextual_arena_surface_and_handle_contracts() {
    let program = parse_source(
        "record Node\nnext handle Node\nend\nfn keep(h handle Node) handle Node\nreturn h\nend\nitems = arena Node()\ninsert items, Node(next = keep) as h\nlookup items, h as item\nprint item.next\nelse\nprint 0\nend\nremove items, h as removed\nprint removed.next\nelse\nprint 0\nend\n",
    );
    assert_eq!(program.records[0].fields[0].type_name, RecordFieldType::Handle("Node".to_owned()));
    assert_eq!(program.functions[0].parameters[0].type_name, TypeName::Handle(Box::new(TypeName::Named("Node".to_owned()))));
    assert_eq!(program.functions[0].return_type, TypeName::Handle(Box::new(TypeName::Named("Node".to_owned()))));
    let StmtKind::Bind { expr, .. } = &program.statements[0].kind else { panic!("expected arena binding"); };
    assert!(matches!(expr.kind, ExprKind::ArenaNew { .. }));
    assert!(matches!(program.statements[1].kind, StmtKind::ArenaInsert { .. }));
    assert!(matches!(program.statements[2].kind, StmtKind::SequenceLookup { .. }));
    assert!(matches!(program.statements[3].kind, StmtKind::ArenaRemove { .. }));
}

#[test]
fn sequence_can_store_typed_handles_without_general_generic_syntax() {
    let program = parse_source("record Node\nvalue int\nend\nfn edges(xs seq handle Node) seq handle Node\nreturn xs\nend\n");
    let expected = TypeName::Sequence(Box::new(TypeName::Handle(Box::new(TypeName::Named("Node".to_owned())))));
    assert_eq!(program.functions[0].parameters[0].type_name, expected);
}

#[test]
fn contextual_arena_words_remain_ordinary_identifiers_elsewhere() {
    let program = parse_source("fn arena(value int) int\nreturn value\nend\narena = 1\nhandle = arena(arena)\ninsert = handle\nremove = insert\nprint remove\n");
    assert_eq!(program.statements.len(), 5);
}

#[test]
fn checked_remove_requires_explicit_failure_branch() {
    let error = parse(&lex("items = arena int()\nremove items, h as value\nprint value\nend\n").unwrap())
        .expect_err("remove must expose failure");
    assert!(error.message.contains("explicit 'else'"));
}

#[test]
fn rejects_nested_or_reference_arena_payloads() {
    let nested = parse(&lex("items = arena seq int()\n").unwrap()).expect_err("nested payload must fail");
    assert!(nested.message.contains("nested"));
    let borrowed = parse(&lex("fn bad(items arena &Node) int\nreturn 1\nend\n").unwrap()).expect_err("reference payload must fail");
    assert!(borrowed.message.contains("type") || borrowed.message.contains("payload"));
}
''')

Path("crates/evo-formatter/tests/arena_syntax_v0.rs").write_text(r'''use evo_formatter::format_source;
use evo_lexer::lex;

#[test]
fn formats_contextual_arena_insert_lookup_remove_blocks_idempotently() {
    let source = "record Node\nnext handle Node\nend\nitems=arena Node()\ninsert items,Node(next=h)as h\nlookup items,h as item\nprint item.next\nelse\nprint 0\nend\nremove items,h as removed\nprint removed.next\nelse\nprint 0\nend\n";
    let expected = "record Node\n    next handle Node\nend\nitems = arena Node()\ninsert items, Node(next = h) as h\nlookup items, h as item\n    print item.next\nelse\n    print 0\nend\nremove items, h as removed\n    print removed.next\nelse\n    print 0\nend\n";
    let tokens = lex(source).expect("arena source should lex");
    let once = format_source(source, &tokens);
    assert_eq!(once, expected);
    let twice_tokens = lex(&once).expect("formatted arena source should lex");
    assert_eq!(format_source(&once, &twice_tokens), expected);
}
''')
