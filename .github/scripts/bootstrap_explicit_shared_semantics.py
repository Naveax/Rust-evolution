from pathlib import Path


def load(path: str) -> tuple[Path, str]:
    p = Path(path)
    return p, p.read_text()


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    assert count == 1, f"{label}: expected exactly one match, found {count}"
    return text.replace(old, new, 1)


def save(path: Path, text: str) -> None:
    path.write_text(text)


# Record semantic environment.
p, text = load("crates/evo-lowering/src/record_environment_records.rs")
text = replace_once(
    text,
    "    Record(String),\n    SharedRef(Box<SemanticType>),",
    "    Record(String),\n    SharedOwner(String),\n    SharedRef(Box<SemanticType>),",
    "SemanticType shared owner",
)
text = replace_once(
    text,
    "        !matches!(self, Self::Record(_))",
    "        !matches!(self, Self::Record(_) | Self::SharedOwner(_))",
    "shared owner move-only classification",
)
text = replace_once(
    text,
    "            SyntaxTypeName::SharedRef(inner) => self\n                .resolve_type_name(inner, span)\n                .map(|inner| SemanticType::SharedRef(Box::new(inner))),",
    """            SyntaxTypeName::SharedOwner(name) => {
                if self.indices.contains_key(name) {
                    Ok(SemanticType::SharedOwner(name.clone()))
                } else {
                    Err(LowerError {
                        message: format!(\"unknown record type {name:?} for shared owner\"),
                        span,
                    })
                }
            }
            SyntaxTypeName::SharedRef(inner) => self
                .resolve_type_name(inner, span)
                .map(|inner| SemanticType::SharedRef(Box::new(inner))),""",
    "resolve shared owner type",
)
text = replace_once(
    text,
    """        let record_name = match base_type {
            SemanticType::Record(name) => name,
            SemanticType::SharedRef(inner) => match inner.as_ref() {""",
    """        let record_name = match base_type {
            SemanticType::Record(name) | SemanticType::SharedOwner(name) => name,
            SemanticType::SharedRef(inner) => match inner.as_ref() {""",
    "shared owner field access",
)
text = replace_once(
    text,
    "        SemanticType::Record(name) => name.clone(),\n        SemanticType::SharedRef(inner) => format!(\"&{}\", semantic_type_label(inner)),",
    "        SemanticType::Record(name) => name.clone(),\n        SemanticType::SharedOwner(name) => format!(\"shared {name}\"),\n        SemanticType::SharedRef(inner) => format!(\"&{}\", semantic_type_label(inner)),",
    "shared owner type label",
)
save(p, text)

# Generic move state exposes the declared type even after a move, for source-native diagnostics.
p, text = load("crates/evo-lowering/src/move_state.rs")
text = replace_once(
    text,
    """    pub(super) fn forget(&mut self, name: &str) {
        let removed = self.bindings.remove(name);
        debug_assert!(removed.is_some());
    }

    pub(super) fn inspect(&self, name: &str) -> Result<T, MoveStateError> {""",
    """    pub(super) fn forget(&mut self, name: &str) {
        let removed = self.bindings.remove(name);
        debug_assert!(removed.is_some());
    }

    pub(super) fn value_type(&self, name: &str) -> Option<T> {
        self.bindings.get(name).map(|binding| binding.value_type.clone())
    }

    pub(super) fn inspect(&self, name: &str) -> Result<T, MoveStateError> {""",
    "move state value type accessor",
)
save(p, text)

# Record/shared-owner move tracker diagnostics.
p, text = load("crates/evo-lowering/src/record_ownership.rs")
text = replace_once(
    text,
    """    pub(crate) fn inspect_value(&self, name: &str, span: Span) -> Result<SemanticType, LowerError> {
        self.state
            .inspect(name)
            .map_err(|error| record_read_error(name, span, error))
    }""",
    """    pub(crate) fn inspect_value(&self, name: &str, span: Span) -> Result<SemanticType, LowerError> {
        let value_type = self.state.value_type(name);
        self.state
            .inspect(name)
            .map_err(|error| record_read_error(name, span, value_type.as_ref(), error))
    }""",
    "inspect move diagnostic type",
)
text = replace_once(
    text,
    """    pub(crate) fn consume_value(
        &mut self,
        name: &str,
        span: Span,
    ) -> Result<SemanticType, LowerError> {
        self.state
            .consume(
                name,
                span,
                MoveReason::Direct,
                SemanticType::is_trivially_reusable_v0,
            )
            .map_err(|error| record_read_error(name, span, error))
    }""",
    """    pub(crate) fn consume_value(
        &mut self,
        name: &str,
        span: Span,
    ) -> Result<SemanticType, LowerError> {
        let value_type = self.state.value_type(name);
        self.state
            .consume(
                name,
                span,
                MoveReason::Direct,
                SemanticType::is_trivially_reusable_v0,
            )
            .map_err(|error| record_read_error(name, span, value_type.as_ref(), error))
    }""",
    "consume move diagnostic type",
)
text = replace_once(
    text,
    """                let message = format!(
                    "record local {name:?} is moved by repeat body and would be unavailable on a later iteration"
                );""",
    """                let kind = move_only_kind(self.state.value_type(&name).as_ref());
                let message = format!(
                    "{kind} local {name:?} is moved by repeat body and would be unavailable on a later iteration"
                );""",
    "repeat shared owner diagnostic",
)
text = replace_once(
    text,
    """        let base_type = self
            .state
            .inspect(base_name)
            .map_err(|error| record_read_error(base_name, span, error))?;""",
    """        let value_type = self.state.value_type(base_name);
        let base_type = self
            .state
            .inspect(base_name)
            .map_err(|error| record_read_error(base_name, span, value_type.as_ref(), error))?;""",
    "field access move diagnostic type",
)
text = replace_once(
    text,
    "fn record_read_error(name: &str, span: Span, error: MoveStateError) -> LowerError {",
    """fn move_only_kind(value_type: Option<&SemanticType>) -> &'static str {
    match value_type {
        Some(SemanticType::SharedOwner(_)) => "shared handle",
        _ => "record",
    }
}

fn record_read_error(
    name: &str,
    span: Span,
    value_type: Option<&SemanticType>,
    error: MoveStateError,
) -> LowerError {""",
    "record read diagnostic signature",
)
text = replace_once(
    text,
    """        MoveStateError::UnavailableBinding(provenance) => {
            let message = format!("use of moved record local {name:?}");""",
    """        MoveStateError::UnavailableBinding(provenance) => {
            let message = format!("use of moved {} local {name:?}", move_only_kind(value_type));""",
    "moved shared handle diagnostic",
)
save(p, text)

# Reference provenance keeps the same local identity, but diagnostics distinguish owner category.
p, text = load("crates/evo-lowering/src/reference_state.rs")
text = replace_once(
    text,
    """    pub(crate) fn ensure_owner_operation_allowed(
        &self,
        owner: &str,
        operation: OwnerOperation,
        span: Span,
    ) -> Result<(), LowerError> {""",
    """    pub(crate) fn ensure_owner_operation_allowed(
        &self,
        owner: &str,
        owner_kind: &str,
        operation: OwnerOperation,
        span: Span,
    ) -> Result<(), LowerError> {""",
    "reference owner kind parameter",
)
text = replace_once(
    text,
    """        let message = format!(
            "cannot {} record local {owner:?} while immutable reference {reference_name:?} is still live",
            operation.verb()
        );""",
    """        let message = format!(
            "cannot {} {owner_kind} local {owner:?} while immutable reference {reference_name:?} is still live",
            operation.verb()
        );""",
    "reference owner kind diagnostic",
)
save(p, text)

# Main lowering IR and semantics.
p, text = load("crates/evo-lowering/src/lib.rs")
text = replace_once(
    text,
    "    SharedBorrow(Box<Expr>),\n    Binary {",
    "    SharedBorrow(Box<Expr>),\n    SharedAlloc(Box<Expr>),\n    SharedDuplicate(Box<Expr>),\n    Binary {",
    "lowered shared operation IR",
)
text = replace_once(
    text,
    "    Record(String),\n    SharedRef(Box<ValueType>),",
    "    Record(String),\n    SharedOwner(String),\n    SharedRef(Box<ValueType>),",
    "ValueType shared owner",
)
text = replace_once(
    text,
    "        ValueType::Record(name) => SemanticType::Record(name.clone()),\n        ValueType::SharedRef(inner) => {",
    "        ValueType::Record(name) => SemanticType::Record(name.clone()),\n        ValueType::SharedOwner(name) => SemanticType::SharedOwner(name.clone()),\n        ValueType::SharedRef(inner) => {",
    "semantic shared owner conversion",
)
text = replace_once(
    text,
    "        SemanticType::Record(name) => ValueType::Record(name.clone()),\n        SemanticType::SharedRef(inner) => {",
    "        SemanticType::Record(name) => ValueType::Record(name.clone()),\n        SemanticType::SharedOwner(name) => ValueType::SharedOwner(name.clone()),\n        SemanticType::SharedRef(inner) => {",
    "lowered shared owner conversion",
)
text = replace_once(
    text,
    "        ValueType::Record(name) => name.clone(),\n        ValueType::SharedRef(inner) => format!(\"&{}\", type_label(inner)),",
    "        ValueType::Record(name) => name.clone(),\n        ValueType::SharedOwner(name) => format!(\"shared {name}\"),\n        ValueType::SharedRef(inner) => format!(\"&{}\", type_label(inner)),",
    "shared owner lower type label",
)
text = replace_once(
    text,
    """                    if matches!(&binding.value_type, ValueType::Record(_)) {
                        self.reference_tracker.ensure_owner_operation_allowed(
                            name,
                            OwnerOperation::Reinitialize,
                            statement.span,
                        )?;
                    }""",
    """                    match &binding.value_type {
                        ValueType::Record(_) => self.reference_tracker.ensure_owner_operation_allowed(
                            name,
                            "record",
                            OwnerOperation::Reinitialize,
                            statement.span,
                        )?,
                        ValueType::SharedOwner(_) => self.reference_tracker.ensure_owner_operation_allowed(
                            name,
                            "shared handle",
                            OwnerOperation::Reinitialize,
                            statement.span,
                        )?,
                        _ => {}
                    }""",
    "reinitialize live reference guard",
)
text = replace_once(
    text,
    "ValueType::Record(_) | ValueType::SharedRef(_)",
    "ValueType::Record(_) | ValueType::SharedOwner(_) | ValueType::SharedRef(_)",
    "print whole shared owner rejection",
)
text = replace_once(
    text,
    """            SyntaxExprKind::Identifier(name) => {
                if self
                    .visible_binding(name)
                    .is_some_and(|binding| matches!(binding.value_type, ValueType::Record(_)))
                {
                    self.reference_tracker.ensure_owner_operation_allowed(
                        name,
                        OwnerOperation::Move,
                        expr.span,
                    )?;
                }
                let value_type = self.move_tracker.consume_value(name, expr.span)?;""",
    """            SyntaxExprKind::Identifier(name) => {
                if let Some(binding) = self.visible_binding(name) {
                    match binding.value_type {
                        ValueType::Record(_) => self.reference_tracker.ensure_owner_operation_allowed(
                            name,
                            "record",
                            OwnerOperation::Move,
                            expr.span,
                        )?,
                        ValueType::SharedOwner(_) => self.reference_tracker.ensure_owner_operation_allowed(
                            name,
                            "shared handle",
                            OwnerOperation::Move,
                            expr.span,
                        )?,
                        _ => {}
                    }
                }
                let value_type = self.move_tracker.consume_value(name, expr.span)?;""",
    "move live reference guard",
)
shared_borrow_block = """            SyntaxExprKind::SharedBorrow(inner) => {
                let (inner, inner_type) = self.lower_reference_target(inner, expr.span)?;
                let ValueType::Record(name) = inner_type else {
                    unreachable!("reference target validation returns a nominal record type")
                };
                (
                    ExprKind::SharedBorrow(Box::new(inner)),
                    ValueType::SharedRef(Box::new(ValueType::Record(name))),
                )
            }
"""
shared_ops_block = shared_borrow_block + """            SyntaxExprKind::SharedAlloc(inner) => {
                let (inner, inner_type) = self.lower_expr(inner)?;
                let ValueType::Record(name) = inner_type else {
                    return Err(LowerError {
                        message: format!(
                            "share requires an owned record value in v0; found {}",
                            type_label(&inner_type)
                        ),
                        span: expr.span,
                    });
                };
                (
                    ExprKind::SharedAlloc(Box::new(inner)),
                    ValueType::SharedOwner(name),
                )
            }
            SyntaxExprKind::SharedDuplicate(inner) => {
                let (inner, inner_type) = if let SyntaxExprKind::Identifier(name) = &inner.kind {
                    let value_type = self.move_tracker.inspect_value(name, inner.span)?;
                    (
                        Expr {
                            kind: ExprKind::Local(name.clone()),
                            span: inner.span,
                        },
                        lowered_value_type(&value_type),
                    )
                } else {
                    self.lower_expr(inner)?
                };
                let ValueType::SharedOwner(name) = inner_type else {
                    return Err(LowerError {
                        message: format!(
                            "dup requires a shared owner in v0; found {}",
                            type_label(&inner_type)
                        ),
                        span: expr.span,
                    });
                };
                (
                    ExprKind::SharedDuplicate(Box::new(inner)),
                    ValueType::SharedOwner(name),
                )
            }
"""
text = replace_once(text, shared_borrow_block, shared_ops_block, "shared operation lowering")
text = replace_once(
    text,
    """        match value_type {
            ValueType::Record(_) => Ok((expr, value_type)),
            ValueType::SharedRef(_) => Err(LowerError {""",
    """        match value_type {
            ValueType::Record(_) => Ok((expr, value_type)),
            ValueType::SharedOwner(name) => Ok((expr, ValueType::Record(name))),
            ValueType::SharedRef(_) => Err(LowerError {""",
    "shared owner reference target",
)
text = replace_once(
    text,
    "            ValueType::Integer | ValueType::Bool | ValueType::String => Err(LowerError {",
    "            ValueType::Integer | ValueType::Bool | ValueType::String => Err(LowerError {",
    "reference scalar arm anchor",
)
text = replace_once(
    text,
    """            | SyntaxExprKind::LogicalNot(_)
            | SyntaxExprKind::UnaryMinus(_)
            | SyntaxExprKind::Binary { .. } => Ok(None),""",
    """            | SyntaxExprKind::LogicalNot(_)
            | SyntaxExprKind::UnaryMinus(_)
            | SyntaxExprKind::SharedAlloc(_)
            | SyntaxExprKind::SharedDuplicate(_)
            | SyntaxExprKind::Binary { .. } => Ok(None),""",
    "reference provenance shared operations",
)
text = replace_once(
    text,
    """                    ValueType::Record(_) => Ok(ReferenceProvenance::local_owner(
                        name.clone(),
                        origin_span,
                    )),
                    ValueType::SharedRef(_) => self""",
    """                    ValueType::Record(_) | ValueType::SharedOwner(_) => Ok(
                        ReferenceProvenance::local_owner(name.clone(), origin_span),
                    ),
                    ValueType::SharedRef(_) => self""",
    "shared handle reference provenance",
)
text = replace_once(
    text,
    """        | SyntaxExprKind::UnaryMinus(base)
        | SyntaxExprKind::SharedBorrow(base) => {
            Self::collect_expr_identifier_uses(base, uses);
        }""",
    """        | SyntaxExprKind::UnaryMinus(base)
        | SyntaxExprKind::SharedBorrow(base)
        | SyntaxExprKind::SharedAlloc(base)
        | SyntaxExprKind::SharedDuplicate(base) => {
            Self::collect_expr_identifier_uses(base, uses);
        }""",
    "identifier liveness shared operations",
)
# Equality on either move-only nominal category stays unsupported.
text = text.replace(
    "matches!(&left_type, ValueType::Record(_))",
    "matches!(&left_type, ValueType::Record(_) | ValueType::SharedOwner(_))",
)
text = text.replace(
    "matches!(&right_type, ValueType::Record(_))",
    "matches!(&right_type, ValueType::Record(_) | ValueType::SharedOwner(_))",
)
save(p, text)

# Borrow inference keeps explicit shared-owner parameters by value and merely traverses new expressions.
p, text = load("crates/evo-lowering/src/borrow_inference.rs")
text = replace_once(
    text,
    """        SyntaxExprKind::LogicalNot(inner)
        | SyntaxExprKind::UnaryMinus(inner)
        | SyntaxExprKind::SharedBorrow(inner) => {""",
    """        SyntaxExprKind::LogicalNot(inner)
        | SyntaxExprKind::UnaryMinus(inner)
        | SyntaxExprKind::SharedBorrow(inner)
        | SyntaxExprKind::SharedAlloc(inner)
        | SyntaxExprKind::SharedDuplicate(inner) => {""",
    "borrow inference shared expressions",
)
save(p, text)

# Keep enum fail-closed machinery exhaustive over the new frontend nodes.
for path in [
    "crates/evo-lowering/src/enum_environment.rs",
    "crates/evo-lowering/src/enum_constructor_typing.rs",
    "crates/evo-lowering/src/enum_static_semantics.rs",
    "crates/evo-lowering/src/enum_borrow_ownership.rs",
    "crates/evo-lowering/src/enum_ownership.rs",
    "crates/evo-lowering/src/enum_executable_ir.rs",
]:
    p, text = load(path)
    text = text.replace(
        "SyntaxTypeName::SharedRef(_)",
        "SyntaxTypeName::SharedRef(_) | SyntaxTypeName::SharedOwner(_)",
    )
    text = text.replace(
        "TypeName::SharedRef(_)",
        "TypeName::SharedRef(_) | TypeName::SharedOwner(_)",
    )
    text = text.replace(
        "SyntaxExprKind::SharedBorrow(base)",
        "SyntaxExprKind::SharedBorrow(base)\n        | SyntaxExprKind::SharedAlloc(base)\n        | SyntaxExprKind::SharedDuplicate(base)",
    )
    text = text.replace(
        "SyntaxExprKind::SharedBorrow(inner)",
        "SyntaxExprKind::SharedBorrow(inner)\n            | SyntaxExprKind::SharedAlloc(inner)\n            | SyntaxExprKind::SharedDuplicate(inner)",
    )
    text = text.replace(
        "SyntaxExprKind::SharedBorrow(_)",
        "SyntaxExprKind::SharedBorrow(_)\n            | SyntaxExprKind::SharedAlloc(_)\n            | SyntaxExprKind::SharedDuplicate(_)",
    )
    save(p, text)
