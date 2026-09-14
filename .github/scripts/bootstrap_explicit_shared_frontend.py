from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    assert count == 1, f"{label}: expected exactly one match, found {count}"
    return text.replace(old, new, 1)


parser = Path("crates/evo-parser/src/lib.rs")
text = parser.read_text()

text = replace_once(
    text,
    "    Named(String),\n    SharedRef(Box<TypeName>),",
    "    Named(String),\n    SharedOwner(String),\n    SharedRef(Box<TypeName>),",
    "TypeName shared owner variant",
)
text = replace_once(
    text,
    "    SharedBorrow(Box<Expr>),\n    Binary {",
    "    SharedBorrow(Box<Expr>),\n    SharedAlloc(Box<Expr>),\n    SharedDuplicate(Box<Expr>),\n    Binary {",
    "ExprKind shared operations",
)

record_old = """    fn parse_record_field_type(&mut self) -> Result<RecordFieldType, ParseError> {
        let token = self.advance();"""
record_new = """    fn parse_record_field_type(&mut self) -> Result<RecordFieldType, ParseError> {
        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"shared\")
            && self
                .tokens
                .get(self.index + 1)
                .is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_)))
        {
            return Err(self.error_here(\"shared-owner record fields are not supported in v0\"));
        }
        let token = self.advance();"""
text = replace_once(text, record_old, record_new, "record field guard")

enum_old = """                if matches!(self.current().kind, TokenKind::Ampersand) {
                    return Err(self
                        .error_here(\"immutable reference enum payloads are not supported in v0\"));
                }
                Some(self.parse_owned_type_name()?)"""
enum_new = """                if matches!(self.current().kind, TokenKind::Ampersand) {
                    return Err(self
                        .error_here(\"immutable reference enum payloads are not supported in v0\"));
                }
                if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"shared\")
                    && self
                        .tokens
                        .get(self.index + 1)
                        .is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_)))
                {
                    return Err(self
                        .error_here(\"shared-owner enum payloads are not supported in v0\"));
                }
                Some(self.parse_owned_type_name()?)"""
text = replace_once(text, enum_old, enum_new, "enum payload guard")

type_pattern = re.compile(
    r"    fn parse_type_name\(&mut self\) -> Result<TypeName, ParseError> \{.*?\n    \}\n\n(?=    fn parse_owned_type_name)",
    re.S,
)
type_replacement = """    fn parse_type_name(&mut self) -> Result<TypeName, ParseError> {
        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == \"shared\") {
            let next = self.tokens.get(self.index + 1);
            if let Some(token) = next {
                match &token.kind {
                    TokenKind::Identifier(name) => {
                        let name = name.clone();
                        self.advance();
                        self.advance();
                        return Ok(TypeName::SharedOwner(name));
                    }
                    TokenKind::TypeInt | TokenKind::TypeBool | TokenKind::TypeString => {
                        return Err(ParseError {
                            message: \"shared-owner types require a nominal record type in v0\"
                                .to_owned(),
                            span: self.current().span.join(token.span),
                        });
                    }
                    _ => {}
                }
            }
        }

        if !matches!(self.current().kind, TokenKind::Ampersand) {
            return self.parse_owned_type_name();
        }

        let marker = self.advance().span;
        let token = self.advance();
        match token.kind {
            TokenKind::Identifier(name) if name == \"mut\" => Err(ParseError {
                message: \"mutable references are not supported in v0\".to_owned(),
                span: marker.join(token.span),
            }),
            TokenKind::Identifier(name) => Ok(TypeName::SharedRef(Box::new(TypeName::Named(name)))),
            TokenKind::Ampersand => Err(ParseError {
                message: \"nested immutable reference types are not supported in v0\".to_owned(),
                span: marker.join(token.span),
            }),
            TokenKind::TypeInt | TokenKind::TypeBool | TokenKind::TypeString => Err(ParseError {
                message: \"immutable reference types require a nominal type in v0\".to_owned(),
                span: marker.join(token.span),
            }),
            _ => Err(ParseError {
                message: \"expected nominal type name after '&'\".to_owned(),
                span: marker.join(token.span),
            }),
        }
    }

"""
text, count = type_pattern.subn(type_replacement, text, count=1)
assert count == 1, f"parse_type_name: expected one match, found {count}"

unary_old = """        if matches!(self.current().kind, TokenKind::Ampersand) {
            let start = self.advance().span;
            if matches!(self.current().kind, TokenKind::Ampersand) {
                return Err(
                    self.error_here(\"nested immutable borrow expressions are not supported in v0\")
                );
            }
            let expr = self.parse_unary()?;
            let span = start.join(expr.span);
            return Ok(Expr {
                kind: ExprKind::SharedBorrow(Box::new(expr)),
                span,
            });
        }
        self.parse_postfix()"""
unary_new = """        if matches!(self.current().kind, TokenKind::Ampersand) {
            let start = self.advance().span;
            if matches!(self.current().kind, TokenKind::Ampersand) {
                return Err(
                    self.error_here(\"nested immutable borrow expressions are not supported in v0\")
                );
            }
            let expr = self.parse_unary()?;
            let span = start.join(expr.span);
            return Ok(Expr {
                kind: ExprKind::SharedBorrow(Box::new(expr)),
                span,
            });
        }
        let shared_operation = match &self.current().kind {
            TokenKind::Identifier(name)
                if matches!(name.as_str(), \"share\" | \"dup\")
                    && self
                        .tokens
                        .get(self.index + 1)
                        .is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_))) =>
            {
                Some(name.clone())
            }
            _ => None,
        };
        if let Some(operation) = shared_operation {
            let start = self.advance().span;
            let expr = self.parse_unary()?;
            let span = start.join(expr.span);
            let kind = if operation == \"share\" {
                ExprKind::SharedAlloc(Box::new(expr))
            } else {
                ExprKind::SharedDuplicate(Box::new(expr))
            };
            return Ok(Expr { kind, span });
        }
        self.parse_postfix()"""
text = replace_once(text, unary_old, unary_new, "parse_unary contextual operations")
parser.write_text(text)

formatter = Path("crates/evo-formatter/src/lib.rs")
ftext = formatter.read_text()
marker = "    if matches!(current, TokenKind::LParen) {\n"
assert ftext.count(marker) == 1, f"formatter insertion point count={ftext.count(marker)}"
ftext = ftext.replace(
    marker,
    """    if matches!(current, TokenKind::Identifier(_))
        && matches!(previous, TokenKind::RParen)
    {
        return true;
    }

    if matches!(current, TokenKind::LParen) {
""",
    1,
)
formatter.write_text(ftext)
