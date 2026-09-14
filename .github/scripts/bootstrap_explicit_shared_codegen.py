from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    assert count == 1, f"{label}: expected exactly one match, found {count}"
    p.write_text(text.replace(old, new, 1))


# Preserve the semantic distinction between &Record and a payload reference derived from Rc<Record>.
replace_once(
    "crates/evo-lowering/src/lib.rs",
    """    SharedBorrow(Box<Expr>),
    SharedAlloc(Box<Expr>),""",
    """    SharedBorrow(Box<Expr>),
    SharedOwnerBorrow(Box<Expr>),
    SharedAlloc(Box<Expr>),""",
    "lowered shared-owner payload borrow node",
)
replace_once(
    "crates/evo-lowering/src/lib.rs",
    """            SyntaxExprKind::SharedBorrow(inner) => {
                let (inner, inner_type) = self.lower_reference_target(inner, expr.span)?;
                let ValueType::Record(name) = inner_type else {
                    unreachable!("reference target validation returns a nominal record type")
                };
                (
                    ExprKind::SharedBorrow(Box::new(inner)),
                    ValueType::SharedRef(Box::new(ValueType::Record(name))),
                )
            }""",
    """            SyntaxExprKind::SharedBorrow(inner) => {
                let shared_owner_payload = if let SyntaxExprKind::Identifier(name) = &inner.kind {
                    self.visible_binding(name).is_some_and(|binding| {
                        matches!(binding.value_type, ValueType::SharedOwner(_))
                    })
                } else {
                    false
                };
                let (inner, inner_type) = self.lower_reference_target(inner, expr.span)?;
                let ValueType::Record(name) = inner_type else {
                    unreachable!("reference target validation returns a nominal record type")
                };
                let kind = if shared_owner_payload {
                    ExprKind::SharedOwnerBorrow(Box::new(inner))
                } else {
                    ExprKind::SharedBorrow(Box::new(inner))
                };
                (
                    kind,
                    ValueType::SharedRef(Box::new(ValueType::Record(name))),
                )
            }""",
    "direct shared-owner payload borrow lowering",
)

# Direct safe Rust Rc codegen. No wrapper object or hidden clone path is introduced.
replace_once(
    "crates/evo-codegen-rust/src/lib.rs",
    """        ValueType::Record(name) => generated_record_name(name),
        ValueType::SharedRef(inner) => format!("&{}", rust_type(inner)),""",
    """        ValueType::Record(name) => generated_record_name(name),
        ValueType::SharedOwner(name) => {
            format!("std::rc::Rc<{}>", generated_record_name(name))
        }
        ValueType::SharedRef(inner) => format!("&{}", rust_type(inner)),""",
    "Rc shared-owner type",
)
replace_once(
    "crates/evo-codegen-rust/src/lib.rs",
    """        ExprKind::InputInt => "__evo_input_int()".to_owned(),
        ExprKind::LogicalNot(inner) => format!("(!{})", render_expr(inner)),
    ExprKind::UnaryMinus(inner) => format!("(-{})", render_expr(inner)),
    ExprKind::SharedBorrow(inner) => format!("&({})", render_expr(inner)),
    ExprKind::Binary { left, op, right } => format!(""",
    """        ExprKind::InputInt => "__evo_input_int()".to_owned(),
        ExprKind::LogicalNot(inner) => format!("(!{})", render_expr(inner)),
        ExprKind::UnaryMinus(inner) => format!("(-{})", render_expr(inner)),
        ExprKind::SharedBorrow(inner) => format!("&({})", render_expr(inner)),
        ExprKind::SharedOwnerBorrow(inner) => {
            format!("std::rc::Rc::as_ref(&{})", render_expr(inner))
        }
        ExprKind::SharedAlloc(inner) => format!("std::rc::Rc::new({})", render_expr(inner)),
        ExprKind::SharedDuplicate(inner) => {
            format!("std::rc::Rc::clone(&({}))", render_expr(inner))
        }
        ExprKind::Binary { left, op, right } => format!(""",
    "shared owner expression codegen",
)
replace_once(
    "crates/evo-codegen-rust/src/lib.rs",
    """        ExprKind::LogicalNot(inner)
    | ExprKind::UnaryMinus(inner)
    | ExprKind::SharedBorrow(inner) => expr_uses_input_int(inner),""",
    """        ExprKind::LogicalNot(inner)
        | ExprKind::UnaryMinus(inner)
        | ExprKind::SharedBorrow(inner)
        | ExprKind::SharedOwnerBorrow(inner)
        | ExprKind::SharedAlloc(inner)
        | ExprKind::SharedDuplicate(inner) => expr_uses_input_int(inner),""",
    "shared owner input traversal",
)
