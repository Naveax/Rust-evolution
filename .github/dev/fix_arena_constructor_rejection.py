from pathlib import Path

path = Path("crates/evo-parser/src/lib.rs")
text = path.read_text()

old_constructor = '''    fn arena_constructor_starts_here(&self) -> bool {
        self.arena_element_type_width_at(self.index)
            .and_then(|width| self.tokens.get(self.index + width))
            .is_some_and(|token| matches!(token.kind, TokenKind::LParen))
    }
'''
new_constructor = '''    fn arena_constructor_starts_here(&self) -> bool {
        if matches!(self.current().kind, TokenKind::Ampersand)
            || matches!(
                &self.current().kind,
                TokenKind::Identifier(name) if matches!(name.as_str(), "arena" | "handle" | "seq")
            )
        {
            return true;
        }
        self.arena_element_type_width_at(self.index)
            .and_then(|width| self.tokens.get(self.index + width))
            .is_some_and(|token| matches!(token.kind, TokenKind::LParen))
    }
'''
if text.count(old_constructor) != 1:
    raise SystemExit(f"arena constructor lookahead anchor count: {text.count(old_constructor)}")
text = text.replace(old_constructor, new_constructor, 1)

old_sequence_constructor = '''    fn sequence_constructor_starts_here(&self) -> bool {
        self.sequence_element_type_width_at(self.index)
            .and_then(|width| self.tokens.get(self.index + width))
            .is_some_and(|token| matches!(token.kind, TokenKind::LParen))
    }
'''
new_sequence_constructor = '''    fn sequence_constructor_starts_here(&self) -> bool {
        if matches!(self.current().kind, TokenKind::Ampersand)
            || matches!(&self.current().kind, TokenKind::Identifier(name) if name == "seq")
        {
            return true;
        }
        self.sequence_element_type_width_at(self.index)
            .and_then(|width| self.tokens.get(self.index + width))
            .is_some_and(|token| matches!(token.kind, TokenKind::LParen))
    }
'''
if text.count(old_sequence_constructor) != 1:
    raise SystemExit(
        f"sequence constructor lookahead anchor count: {text.count(old_sequence_constructor)}"
    )
text = text.replace(old_sequence_constructor, new_sequence_constructor, 1)

old_type_start = '''    fn arena_element_type_starts_at(&self, index: usize) -> bool {
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
'''
new_type_start = '''    fn arena_element_type_starts_at(&self, index: usize) -> bool {
        match self.tokens.get(index).map(|token| &token.kind) {
            Some(TokenKind::TypeInt | TokenKind::TypeBool | TokenKind::TypeString) => true,
            Some(TokenKind::Identifier(name)) if name == "shared" => self
                .tokens
                .get(index + 1)
                .is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_))),
            Some(TokenKind::Identifier(_)) | Some(TokenKind::Ampersand) => true,
            _ => false,
        }
    }
'''
if text.count(old_type_start) != 1:
    raise SystemExit(f"arena type-start anchor count: {text.count(old_type_start)}")
path.write_text(text.replace(old_type_start, new_type_start, 1))
