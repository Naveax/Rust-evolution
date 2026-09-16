from pathlib import Path


def replace(path: str, old: str, new: str, *, min_count: int = 1) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count < min_count:
        raise SystemExit(f"{path}: expected >= {min_count} matches, found {count}: {old[:100]!r}")
    p.write_text(text.replace(old, new))


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected 1 match, found {count}: {old[:100]!r}")
    p.write_text(text.replace(old, new, 1))

SUG = "crates/evo-lowering/src/source_suggestions.rs"
replace_once(
    SUG,
    "                        | RecordFieldType::String\n                        | RecordFieldType::Named(_) => None,\n",
    "                        | RecordFieldType::String\n                        | RecordFieldType::Named(_)\n                        | RecordFieldType::Handle(_) => None,\n",
)
replace_once(
    SUG,
    "                StmtKind::SequenceLookup {\n                    owner,\n                    index,\n                    binding,\n                    then_body,\n                    else_body,\n                } => {\n",
    "                StmtKind::ArenaInsert { owner, value, binding } => {\n                    if visible(scopes, owner).is_none() {\n                        let message = format!(\"use of local {owner:?} before definition or outside its scope\");\n                        register(&message, statement.span, owner, visible_names(scopes));\n                    }\n                    let _ = self.walk_expr(value, scopes);\n                    if visible(scopes, binding).is_none() {\n                        scopes.last_mut().expect(\"suggestion traversal always has a lexical scope\").insert(binding.clone(), None);\n                    }\n                }\n                StmtKind::ArenaRemove { owner, handle, binding, then_body, else_body } => {\n                    if visible(scopes, owner).is_none() {\n                        let message = format!(\"use of local {owner:?} before definition or outside its scope\");\n                        register(&message, statement.span, owner, visible_names(scopes));\n                    }\n                    let _ = self.walk_expr(handle, scopes);\n                    self.walk_child(then_body, scopes, Some((binding.clone(), None)));\n                    self.walk_child(else_body, scopes, None);\n                }\n                StmtKind::SequenceLookup {\n                    owner,\n                    index,\n                    binding,\n                    then_body,\n                    else_body,\n                } => {\n",
)
replace_once(
    SUG,
    "            | ExprKind::InputInt\n            | ExprKind::SequenceNew { .. } => None,\n",
    "            | ExprKind::InputInt\n            | ExprKind::SequenceNew { .. }\n            | ExprKind::ArenaNew { .. } => None,\n",
)
replace_once(
    SUG,
    "        TypeName::Sequence(_) | TypeName::Int | TypeName::Bool | TypeName::String => None,\n",
    "        TypeName::Sequence(_)\n        | TypeName::Arena(_)\n        | TypeName::Handle(_)\n        | TypeName::Int\n        | TypeName::Bool\n        | TypeName::String => None,\n",
)

ENUM_ENV = "crates/evo-lowering/src/enum_environment.rs"
replace_once(
    ENUM_ENV,
    "        SyntaxTypeName::Sequence(_) => Err(LowerError {\n            message: \"append-only sequence enum payloads are not supported in v0\".to_owned(),\n            span,\n        }),\n",
    "        SyntaxTypeName::Sequence(_) => Err(LowerError {\n            message: \"append-only sequence enum payloads are not supported in v0\".to_owned(),\n            span,\n        }),\n        SyntaxTypeName::Arena(_) | SyntaxTypeName::Handle(_) => Err(LowerError {\n            message: \"generational arena and handle enum payloads are not supported in v0\".to_owned(),\n            span,\n        }),\n",
)
replace(
    ENUM_ENV,
    "            SyntaxStmtKind::SequenceAppend { .. } | SyntaxStmtKind::SequenceLookup { .. } => {\n                return Err(LowerError {\n                    message: \"append-only sequences are not supported in enum-bearing programs in v0\".to_owned(),\n",
    "            SyntaxStmtKind::SequenceAppend { .. } | SyntaxStmtKind::SequenceLookup { .. } => {\n                return Err(LowerError {\n                    message: \"append-only sequences are not supported in enum-bearing programs in v0\".to_owned(),\n",
)
# Add a distinct fail-closed arena statement arm before Match.
replace_once(
    ENUM_ENV,
    "            SyntaxStmtKind::Match { value, arms } => {\n",
    "            SyntaxStmtKind::ArenaInsert { .. } | SyntaxStmtKind::ArenaRemove { .. } => {\n                return Err(LowerError {\n                    message: \"generational arenas are not supported in enum-bearing programs in v0\".to_owned(),\n                    span: statement.span,\n                });\n            }\n            SyntaxStmtKind::Match { value, arms } => {\n",
)
replace_once(
    ENUM_ENV,
    "            SyntaxExprKind::SequenceNew { .. } => Err(LowerError {\n                message: \"append-only sequences are not supported in enum-bearing programs in v0\".to_owned(),\n                span: expr.span,\n            }),\n",
    "            SyntaxExprKind::SequenceNew { .. } => Err(LowerError {\n                message: \"append-only sequences are not supported in enum-bearing programs in v0\".to_owned(),\n                span: expr.span,\n            }),\n            SyntaxExprKind::ArenaNew { .. } => Err(LowerError {\n                message: \"generational arenas are not supported in enum-bearing programs in v0\".to_owned(),\n                span: expr.span,\n            }),\n",
)
replace_once(
    ENUM_ENV,
    "        | SyntaxExprKind::SequenceNew { .. }\n        | SyntaxExprKind::Call { .. }\n",
    "        | SyntaxExprKind::SequenceNew { .. }\n        | SyntaxExprKind::ArenaNew { .. }\n        | SyntaxExprKind::Call { .. }\n",
)

ENUM_TYPING = "crates/evo-lowering/src/enum_constructor_typing.rs"
replace_once(
    ENUM_TYPING,
    "            SyntaxExprKind::SequenceNew { .. } => Err(LowerError {\n                message: \"append-only sequences are not supported in enum-bearing programs in v0\".to_owned(),\n                span: expr.span,\n            }),\n",
    "            SyntaxExprKind::SequenceNew { .. } => Err(LowerError {\n                message: \"append-only sequences are not supported in enum-bearing programs in v0\".to_owned(),\n                span: expr.span,\n            }),\n            SyntaxExprKind::ArenaNew { .. } => Err(LowerError {\n                message: \"generational arenas are not supported in enum-bearing programs in v0\".to_owned(),\n                span: expr.span,\n            }),\n",
)
replace_once(
    ENUM_TYPING,
    "            SyntaxStmtKind::Match { value, arms } => {\n                validate_match(value, arms, environment, scopes)?;\n            }\n",
    "            SyntaxStmtKind::ArenaInsert { .. } | SyntaxStmtKind::ArenaRemove { .. } => {\n                return Err(LowerError {\n                    message: \"generational arenas are not supported in enum-bearing programs in v0\".to_owned(),\n                    span: statement.span,\n                });\n            }\n            SyntaxStmtKind::Match { value, arms } => {\n                validate_match(value, arms, environment, scopes)?;\n            }\n",
)
replace_once(
    ENUM_TYPING,
    "        SyntaxRecordFieldType::Named(name) => Err(LowerError {\n            message: format!(\"unknown nominal type {name:?}\"),\n            span,\n        }),\n",
    "        SyntaxRecordFieldType::Named(name) => Err(LowerError {\n            message: format!(\"unknown nominal type {name:?}\"),\n            span,\n        }),\n        SyntaxRecordFieldType::Handle(_) => Err(LowerError {\n            message: \"generational handle record fields are not supported in enum-bearing programs in v0\".to_owned(),\n            span,\n        }),\n",
)
replace_once(
    ENUM_TYPING,
    "        SyntaxTypeName::Sequence(_) => Err(LowerError {\n            message: \"append-only sequence function contracts are not supported in enum-bearing programs in v0\".to_owned(),\n            span,\n        }),\n",
    "        SyntaxTypeName::Sequence(_) => Err(LowerError {\n            message: \"append-only sequence function contracts are not supported in enum-bearing programs in v0\".to_owned(),\n            span,\n        }),\n        SyntaxTypeName::Arena(_) | SyntaxTypeName::Handle(_) => Err(LowerError {\n            message: \"generational arena and handle function contracts are not supported in enum-bearing programs in v0\".to_owned(),\n            span,\n        }),\n",
)

# Common enum validators reject arena syntax before promotion. Keep their later matches exhaustive.
for path in [
    "crates/evo-lowering/src/enum_borrow_ownership.rs",
    "crates/evo-lowering/src/enum_ownership.rs",
]:
    replace(
        path,
        "            SyntaxStmtKind::SequenceAppend { .. } | SyntaxStmtKind::SequenceLookup { .. } => {\n",
        "            SyntaxStmtKind::SequenceAppend { .. } | SyntaxStmtKind::SequenceLookup { .. } => {\n",
    )
    p = Path(path)
    text = p.read_text()
    anchor = "            SyntaxStmtKind::Match { value, arms } => {\n"
    if anchor in text and "SyntaxStmtKind::ArenaInsert" not in text:
        text = text.replace(
            anchor,
            "            SyntaxStmtKind::ArenaInsert { .. } | SyntaxStmtKind::ArenaRemove { .. } => {\n                Err(LowerError {\n                    message: \"generational arenas are not supported in enum-bearing programs in v0\".to_owned(),\n                    span: statement.span,\n                })\n            }\n" + anchor,
            1,
        )
    expr_anchor = "            SyntaxExprKind::SequenceNew { .. } => Err(LowerError {\n"
    if expr_anchor in text and "SyntaxExprKind::ArenaNew" not in text:
        pos = text.index(expr_anchor)
        # Insert a separate arena arm immediately before the sequence arm.
        text = text[:pos] + "            SyntaxExprKind::ArenaNew { .. } => Err(LowerError {\n                message: \"generational arenas are not supported in enum-bearing programs in v0\".to_owned(),\n                span: expr.span,\n            }),\n" + text[pos:]
    p.write_text(text)

MATCH_VALID = "crates/evo-lowering/src/enum_match_validation.rs"
replace(
    MATCH_VALID,
    "            | SyntaxStmtKind::SequenceAppend { .. }\n            | SyntaxStmtKind::SequenceLookup { .. } => {}\n",
    "            | SyntaxStmtKind::SequenceAppend { .. }\n            | SyntaxStmtKind::SequenceLookup { .. }\n            | SyntaxStmtKind::ArenaInsert { .. }\n            | SyntaxStmtKind::ArenaRemove { .. } => {}\n",
)
replace(
    MATCH_VALID,
    "        | SyntaxStmtKind::SequenceAppend { .. }\n        | SyntaxStmtKind::SequenceLookup { .. } => false,\n",
    "        | SyntaxStmtKind::SequenceAppend { .. }\n        | SyntaxStmtKind::SequenceLookup { .. }\n        | SyntaxStmtKind::ArenaInsert { .. }\n        | SyntaxStmtKind::ArenaRemove { .. } => false,\n",
)

MATCH_SIDECAR = "crates/evo-lowering/src/enum_match_sidecar.rs"
replace_once(
    MATCH_SIDECAR,
    "            | SyntaxStmtKind::SequenceAppend { .. }\n            | SyntaxStmtKind::SequenceLookup { .. } => {}\n",
    "            | SyntaxStmtKind::SequenceAppend { .. }\n            | SyntaxStmtKind::SequenceLookup { .. }\n            | SyntaxStmtKind::ArenaInsert { .. }\n            | SyntaxStmtKind::ArenaRemove { .. } => {}\n",
)

ENUM_IR = "crates/evo-lowering/src/enum_ir.rs"
replace(
    ENUM_IR,
    "            | SyntaxStmtKind::SequenceAppend { .. }\n            | SyntaxStmtKind::SequenceLookup { .. } => {}\n",
    "            | SyntaxStmtKind::SequenceAppend { .. }\n            | SyntaxStmtKind::SequenceLookup { .. }\n            | SyntaxStmtKind::ArenaInsert { .. }\n            | SyntaxStmtKind::ArenaRemove { .. } => {}\n",
)
replace(
    ENUM_IR,
    "            SyntaxStmtKind::SequenceAppend { .. } | SyntaxStmtKind::SequenceLookup { .. } => {}\n",
    "            SyntaxStmtKind::SequenceAppend { .. }\n            | SyntaxStmtKind::SequenceLookup { .. }\n            | SyntaxStmtKind::ArenaInsert { .. }\n            | SyntaxStmtKind::ArenaRemove { .. } => {}\n",
)
replace_once(
    ENUM_IR,
    "        | SyntaxExprKind::InputInt\n        | SyntaxExprKind::SequenceNew { .. } => {}\n",
    "        | SyntaxExprKind::InputInt\n        | SyntaxExprKind::SequenceNew { .. }\n        | SyntaxExprKind::ArenaNew { .. } => {}\n",
)
replace_once(
    ENUM_IR,
    "        SyntaxRecordFieldType::Named(name) => SchemaType::Record(name.clone()),\n",
    "        SyntaxRecordFieldType::Named(name) => SchemaType::Record(name.clone()),\n        SyntaxRecordFieldType::Handle(_) => unreachable!(\"arena handle fields are rejected before enum IR promotion\"),\n",
)

EXEC_IR = "crates/evo-lowering/src/enum_executable_ir.rs"
replace_once(
    EXEC_IR,
    "            SyntaxStmtKind::SequenceAppend { .. } | SyntaxStmtKind::SequenceLookup { .. } => {\n                unreachable!(\"sequence syntax is rejected before enum executable IR promotion\")\n            }\n",
    "            SyntaxStmtKind::SequenceAppend { .. } | SyntaxStmtKind::SequenceLookup { .. } => {\n                unreachable!(\"sequence syntax is rejected before enum executable IR promotion\")\n            }\n            SyntaxStmtKind::ArenaInsert { .. } | SyntaxStmtKind::ArenaRemove { .. } => {\n                unreachable!(\"arena syntax is rejected before enum executable IR promotion\")\n            }\n",
)
replace_once(
    EXEC_IR,
    "            SyntaxExprKind::SequenceNew { .. } => {\n                unreachable!(\"sequence syntax is rejected before enum executable IR promotion\")\n            }\n",
    "            SyntaxExprKind::SequenceNew { .. } => {\n                unreachable!(\"sequence syntax is rejected before enum executable IR promotion\")\n            }\n            SyntaxExprKind::ArenaNew { .. } => {\n                unreachable!(\"arena syntax is rejected before enum executable IR promotion\")\n            }\n",
)
replace_once(
    EXEC_IR,
    "            TypeName::Sequence(_) => {\n                unreachable!(\"sequence types are rejected before enum executable IR promotion\")\n            }\n",
    "            TypeName::Sequence(_) => {\n                unreachable!(\"sequence types are rejected before enum executable IR promotion\")\n            }\n            TypeName::Arena(_) | TypeName::Handle(_) => {\n                unreachable!(\"arena types are rejected before enum executable IR promotion\")\n            }\n",
)

STATIC = "crates/evo-lowering/src/enum_static_semantics.rs"
replace_once(
    STATIC,
    "                    evo_parser::RecordFieldType::Named(name) => {\n                        return Err(LowerError {\n                            message: format!(\"unknown nominal type {name:?}\"),\n                            span: field.span,\n                        });\n                    }\n",
    "                    evo_parser::RecordFieldType::Named(name) => {\n                        return Err(LowerError {\n                            message: format!(\"unknown nominal type {name:?}\"),\n                            span: field.span,\n                        });\n                    }\n                    evo_parser::RecordFieldType::Handle(_) => {\n                        return Err(LowerError {\n                            message: \"generational handle record fields are not supported in enum-bearing programs in v0\".to_owned(),\n                            span: field.span,\n                        });\n                    }\n",
)
replace_once(
    STATIC,
    "            SyntaxExprKind::SequenceNew { .. } => Err(LowerError {\n                message: \"append-only sequences are not supported in enum-bearing programs in v0\".to_owned(),\n                span: expr.span,\n            }),\n",
    "            SyntaxExprKind::SequenceNew { .. } => Err(LowerError {\n                message: \"append-only sequences are not supported in enum-bearing programs in v0\".to_owned(),\n                span: expr.span,\n            }),\n            SyntaxExprKind::ArenaNew { .. } => Err(LowerError {\n                message: \"generational arenas are not supported in enum-bearing programs in v0\".to_owned(),\n                span: expr.span,\n            }),\n",
)
# Add arena statement rejection before each Match arm in static statement validators.
p = Path(STATIC)
text = p.read_text()
match_anchor = "            SyntaxStmtKind::Match { value, arms } => {\n"
count = text.count(match_anchor)
if count < 1:
    raise SystemExit("enum_static_semantics: match statement anchor missing")
text = text.replace(
    match_anchor,
    "            SyntaxStmtKind::ArenaInsert { .. } | SyntaxStmtKind::ArenaRemove { .. } => {\n                return Err(LowerError {\n                    message: \"generational arenas are not supported in enum-bearing programs in v0\".to_owned(),\n                    span: statement.span,\n                });\n            }\n" + match_anchor,
)
text = text.replace(
    "        | SyntaxStmtKind::SequenceLookup { .. } => false,\n",
    "        | SyntaxStmtKind::SequenceLookup { .. }\n        | SyntaxStmtKind::ArenaInsert { .. }\n        | SyntaxStmtKind::ArenaRemove { .. } => false,\n",
)
text = text.replace(
    "        TypeName::Sequence(_) => Err(LowerError {\n",
    "        TypeName::Arena(_) | TypeName::Handle(_) => Err(LowerError {\n            message: \"generational arena and handle types are not supported in enum-bearing programs in v0\".to_owned(),\n            span,\n        }),\n        TypeName::Sequence(_) => Err(LowerError {\n",
)
p.write_text(text)
