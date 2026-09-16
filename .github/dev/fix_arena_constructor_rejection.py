from pathlib import Path

path = Path("crates/evo-parser/src/lib.rs")
text = path.read_text()
old = '''    fn arena_constructor_starts_here(&self) -> bool {
        self.arena_element_type_width_at(self.index)
            .and_then(|width| self.tokens.get(self.index + width))
            .is_some_and(|token| matches!(token.kind, TokenKind::LParen))
    }
'''
new = '''    fn arena_constructor_starts_here(&self) -> bool {
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
if text.count(old) != 1:
    raise SystemExit(f"arena constructor lookahead anchor count: {text.count(old)}")
path.write_text(text.replace(old, new, 1))
