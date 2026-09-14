from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    assert count == 1, f"{label}: expected exactly one match, found {count}"
    p.write_text(text.replace(old, new, 1))


# Normalize the alias-specific type pattern after the broad exhaustive-match pass.
for path in [
    "crates/evo-lowering/src/enum_environment.rs",
    "crates/evo-lowering/src/enum_constructor_typing.rs",
]:
    p = Path(path)
    text = p.read_text()
    malformed = (
        "SyntaxTypeName::SharedRef(_) | TypeName::SharedOwner(_) | "
        "SyntaxTypeName::SharedOwner(_)"
    )
    count = text.count(malformed)
    assert count >= 1, f"{path}: expected malformed alias pattern from first pass"
    text = text.replace(
        malformed,
        "SyntaxTypeName::SharedRef(_) | SyntaxTypeName::SharedOwner(_)",
    )
    p.write_text(text)

replace_once(
    "crates/evo-lowering/src/source_suggestions.rs",
    "            ExprKind::SharedBorrow(inner) => self.walk_expr(inner, scopes),",
    """            ExprKind::SharedBorrow(inner)
            | ExprKind::SharedAlloc(inner)
            | ExprKind::SharedDuplicate(inner) => self.walk_expr(inner, scopes),""",
    "source suggestion shared expression traversal",
)
replace_once(
    "crates/evo-lowering/src/source_suggestions.rs",
    """        TypeName::Named(name) => Some(name),
        TypeName::SharedRef(inner) => named_type(inner),
        TypeName::Int | TypeName::Bool | TypeName::String => None,""",
    """        TypeName::Named(name) | TypeName::SharedOwner(name) => Some(name),
        TypeName::SharedRef(inner) => named_type(inner),
        TypeName::Int | TypeName::Bool | TypeName::String => None,""",
    "source suggestion shared owner type",
)
replace_once(
    "crates/evo-lowering/src/enum_ir.rs",
    """        SyntaxExprKind::FieldAccess { base, .. }
        | SyntaxExprKind::LogicalNot(base)
        | SyntaxExprKind::UnaryMinus(base)
        | SyntaxExprKind::SharedBorrow(base) => collect_constructor_expr(base, environment, lowered),""",
    """        SyntaxExprKind::FieldAccess { base, .. }
        | SyntaxExprKind::LogicalNot(base)
        | SyntaxExprKind::UnaryMinus(base)
        | SyntaxExprKind::SharedBorrow(base)
        | SyntaxExprKind::SharedAlloc(base)
        | SyntaxExprKind::SharedDuplicate(base) => {
            collect_constructor_expr(base, environment, lowered);
        }""",
    "enum IR shared expression traversal",
)
