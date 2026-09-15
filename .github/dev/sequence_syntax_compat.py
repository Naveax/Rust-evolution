from pathlib import Path


def replace_once(path, old, new):
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one compat anchor in {path}, found {count}: {old[:120]!r}")
    p.write_text(text.replace(old, new, 1))


def replace_all(path, old, new, minimum=1):
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count < minimum:
        raise SystemExit(f"expected compat anchors in {path}, found {count}: {old[:120]!r}")
    p.write_text(text.replace(old, new))

unsupported_stmt = '''            SyntaxStmtKind::SequenceAppend { .. } | SyntaxStmtKind::SequenceLookup { .. } => {
                return Err(LowerError {
                    message: "append-only sequences are not supported in enum-bearing programs in v0".to_owned(),
                    span: statement.span,
                });
            }
'''
unsupported_expr = '''            SyntaxExprKind::SequenceNew { .. } => Err(LowerError {
                message: "append-only sequences are not supported in enum-bearing programs in v0".to_owned(),
                span: expr.span,
            }),
'''

# Source suggestion traversal must understand the new parser surface even before semantic lowering.
p = "crates/evo-lowering/src/source_suggestions.rs"
replace_once(p,
'''                StmtKind::Match { value, arms } => {''',
'''                StmtKind::SequenceAppend { owner, value } => {
                    if visible(scopes, owner).is_none() {
                        let message = format!("use of local {owner:?} before definition or outside its scope");
                        register(&message, statement.span, owner, visible_names(scopes));
                    }
                    let _ = self.walk_expr(value, scopes);
                }
                StmtKind::SequenceLookup {
                    owner,
                    index,
                    binding,
                    then_body,
                    else_body,
                } => {
                    if visible(scopes, owner).is_none() {
                        let message = format!("use of local {owner:?} before definition or outside its scope");
                        register(&message, statement.span, owner, visible_names(scopes));
                    }
                    let _ = self.walk_expr(index, scopes);
                    self.walk_child(then_body, scopes, Some((binding.clone(), None)));
                    self.walk_child(else_body, scopes, None);
                }
                StmtKind::Match { value, arms } => {''')
replace_once(p,
'''            ExprKind::Integer(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::InputInt => {
                None
            }''',
'''            ExprKind::Integer(_)
            | ExprKind::String(_)
            | ExprKind::Bool(_)
            | ExprKind::InputInt
            | ExprKind::SequenceNew { .. } => None,''')
replace_once(p,
'''        TypeName::SharedRef(inner) => named_type(inner),
        TypeName::Int | TypeName::Bool | TypeName::String => None,''',
'''        TypeName::SharedRef(inner) => named_type(inner),
        TypeName::Sequence(_) | TypeName::Int | TypeName::Bool | TypeName::String => None,''')

# Record-only environment remains fail-closed until the semantic slice lands.
p = "crates/evo-lowering/src/record_environment_records.rs"
replace_once(p,
'''            SyntaxTypeName::SharedRef(inner) => self
                .resolve_type_name(inner, span)
                .map(|inner| SemanticType::SharedRef(Box::new(inner))),''',
'''            SyntaxTypeName::SharedRef(inner) => self
                .resolve_type_name(inner, span)
                .map(|inner| SemanticType::SharedRef(Box::new(inner))),
            SyntaxTypeName::Sequence(_) => Err(LowerError {
                message: "append-only sequence semantic lowering is not implemented yet".to_owned(),
                span,
            }),''')

# Enum declaration/shape validation explicitly rejects sequence surface.
p = "crates/evo-lowering/src/enum_environment.rs"
replace_once(p,
'''        SyntaxTypeName::SharedRef(_) | SyntaxTypeName::SharedOwner(_) => Err(LowerError {
            message: "immutable reference enum payloads are not supported in v0".to_owned(),
            span,
        }),''',
'''        SyntaxTypeName::SharedRef(_) | SyntaxTypeName::SharedOwner(_) => Err(LowerError {
            message: "immutable reference enum payloads are not supported in v0".to_owned(),
            span,
        }),
        SyntaxTypeName::Sequence(_) => Err(LowerError {
            message: "append-only sequence enum payloads are not supported in v0".to_owned(),
            span,
        }),''')
replace_once(p,
'''            SyntaxStmtKind::Match { value, arms } => {''', unsupported_stmt + '''            SyntaxStmtKind::Match { value, arms } => {''')
replace_once(p,
'''        SyntaxExprKind::Call { arguments, .. } => {''', unsupported_expr + '''        SyntaxExprKind::Call { arguments, .. } => {''')
replace_once(p,
'''        SyntaxExprKind::Identifier(_)
        | SyntaxExprKind::Call { .. }''',
'''        SyntaxExprKind::Identifier(_)
        | SyntaxExprKind::SequenceNew { .. }
        | SyntaxExprKind::Call { .. }''')

# Enum typing rejects sequence constructs/contracts before ownership/executable promotion.
p = "crates/evo-lowering/src/enum_constructor_typing.rs"
replace_once(p,
'''            SyntaxExprKind::Call { name, arguments } => {''', unsupported_expr + '''            SyntaxExprKind::Call { name, arguments } => {''')
replace_once(p,
'''            SyntaxStmtKind::Match { value, arms } => {''', unsupported_stmt + '''            SyntaxStmtKind::Match { value, arms } => {''')
replace_once(p,
'''        SyntaxTypeName::SharedRef(_) | SyntaxTypeName::SharedOwner(_) => Err(LowerError {
            message: "immutable reference semantic lowering is not implemented yet".to_owned(),
            span,
        }),''',
'''        SyntaxTypeName::SharedRef(_) | SyntaxTypeName::SharedOwner(_) => Err(LowerError {
            message: "immutable reference semantic lowering is not implemented yet".to_owned(),
            span,
        }),
        SyntaxTypeName::Sequence(_) => Err(LowerError {
            message: "append-only sequence function contracts are not supported in enum-bearing programs in v0".to_owned(),
            span,
        }),''')

# Enum ownership passes are unreachable for sequence syntax after validation; keep explicit errors.
for p in [
    "crates/evo-lowering/src/enum_borrow_ownership.rs",
    "crates/evo-lowering/src/enum_ownership.rs",
]:
    replace_once(p,
    '''            SyntaxStmtKind::Match { value, arms } => {''', unsupported_stmt + '''            SyntaxStmtKind::Match { value, arms } => {''')
    replace_once(p,
    '''            SyntaxExprKind::SharedBorrow(_)
            | SyntaxExprKind::SharedAlloc(_)
            | SyntaxExprKind::SharedDuplicate(_) => Err(LowerError {''',
    '''            SyntaxExprKind::SequenceNew { .. } => Err(LowerError {
                message: "append-only sequences are not supported in enum-bearing programs in v0".to_owned(),
                span: expr.span,
            }),
            SyntaxExprKind::SharedBorrow(_)
            | SyntaxExprKind::SharedAlloc(_)
            | SyntaxExprKind::SharedDuplicate(_) => Err(LowerError {''')

p = "crates/evo-lowering/src/enum_static_semantics.rs"
replace_once(p,
'''            SyntaxExprKind::SharedBorrow(_) => Err(LowerError {''', unsupported_expr + '''            SyntaxExprKind::SharedBorrow(_) => Err(LowerError {''')
replace_once(p,
'''            SyntaxStmtKind::Match { value, arms } => {''', unsupported_stmt + '''            SyntaxStmtKind::Match { value, arms } => {''')
replace_once(p,
'''        SyntaxStmtKind::Bind { .. }
        | SyntaxStmtKind::Print(_)
        | SyntaxStmtKind::Repeat { .. } => false,''',
'''        SyntaxStmtKind::Bind { .. }
        | SyntaxStmtKind::Print(_)
        | SyntaxStmtKind::Repeat { .. }
        | SyntaxStmtKind::SequenceAppend { .. }
        | SyntaxStmtKind::SequenceLookup { .. } => false,''')
replace_once(p,
'''        TypeName::SharedOwner(_) => Err(LowerError {
            message: "explicit shared-owner function contracts are not supported in enum-bearing programs in v0"
                .to_owned(),
            span,
        }),''',
'''        TypeName::SharedOwner(_) => Err(LowerError {
            message: "explicit shared-owner function contracts are not supported in enum-bearing programs in v0"
                .to_owned(),
            span,
        }),
        TypeName::Sequence(_) => Err(LowerError {
            message: "append-only sequence function contracts are not supported in enum-bearing programs in v0".to_owned(),
            span,
        }),''')

# Match metadata walkers never receive sequence statements after enum validation; traverse neither.
p = "crates/evo-lowering/src/enum_match_validation.rs"
replace_once(p,
'''            SyntaxStmtKind::Bind { .. }
            | SyntaxStmtKind::Print(_)
            | SyntaxStmtKind::Return(_) => {}''',
'''            SyntaxStmtKind::Bind { .. }
            | SyntaxStmtKind::Print(_)
            | SyntaxStmtKind::Return(_)
            | SyntaxStmtKind::SequenceAppend { .. }
            | SyntaxStmtKind::SequenceLookup { .. } => {}''')
replace_once(p,
'''        SyntaxStmtKind::Bind { .. }
        | SyntaxStmtKind::Print(_)
        | SyntaxStmtKind::Repeat { .. } => false,''',
'''        SyntaxStmtKind::Bind { .. }
        | SyntaxStmtKind::Print(_)
        | SyntaxStmtKind::Repeat { .. }
        | SyntaxStmtKind::SequenceAppend { .. }
        | SyntaxStmtKind::SequenceLookup { .. } => false,''')

p = "crates/evo-lowering/src/enum_match_sidecar.rs"
replace_once(p,
'''            SyntaxStmtKind::Bind { .. }
            | SyntaxStmtKind::Print(_)
            | SyntaxStmtKind::Return(_) => {}''',
'''            SyntaxStmtKind::Bind { .. }
            | SyntaxStmtKind::Print(_)
            | SyntaxStmtKind::Return(_)
            | SyntaxStmtKind::SequenceAppend { .. }
            | SyntaxStmtKind::SequenceLookup { .. } => {}''')

p = "crates/evo-lowering/src/enum_ir.rs"
replace_all(p,
'''            SyntaxStmtKind::Bind { .. }
            | SyntaxStmtKind::Print(_)
            | SyntaxStmtKind::Return(_) => {}''',
'''            SyntaxStmtKind::Bind { .. }
            | SyntaxStmtKind::Print(_)
            | SyntaxStmtKind::Return(_)
            | SyntaxStmtKind::SequenceAppend { .. }
            | SyntaxStmtKind::SequenceLookup { .. } => {}''', minimum=1)
replace_once(p,
'''        SyntaxExprKind::Integer(_)
        | SyntaxExprKind::String(_)
        | SyntaxExprKind::Bool(_)
        | SyntaxExprKind::Identifier(_)
        | SyntaxExprKind::InputInt => {}''',
'''        SyntaxExprKind::Integer(_)
        | SyntaxExprKind::String(_)
        | SyntaxExprKind::Bool(_)
        | SyntaxExprKind::Identifier(_)
        | SyntaxExprKind::InputInt
        | SyntaxExprKind::SequenceNew { .. } => {}''')
# The constructor walker has a Match arm rather than a terminal no-op statement arm.
replace_once(p,
'''            SyntaxStmtKind::Match { value, arms } => {
                collect_constructor_expr(value, environment, lowered);''',
'''            SyntaxStmtKind::SequenceAppend { .. } | SyntaxStmtKind::SequenceLookup { .. } => {}
            SyntaxStmtKind::Match { value, arms } => {
                collect_constructor_expr(value, environment, lowered);''')

p = "crates/evo-lowering/src/enum_executable_ir.rs"
replace_once(p,
'''            SyntaxStmtKind::Match { value, arms } => {''',
'''            SyntaxStmtKind::SequenceAppend { .. } | SyntaxStmtKind::SequenceLookup { .. } => {
                unreachable!("sequence syntax is rejected before enum executable IR promotion")
            }
            SyntaxStmtKind::Match { value, arms } => {''')
replace_once(p,
'''            SyntaxExprKind::SharedBorrow(_)
            | SyntaxExprKind::SharedAlloc(_)
            | SyntaxExprKind::SharedDuplicate(_) => unreachable!(''',
'''            SyntaxExprKind::SequenceNew { .. } => {
                unreachable!("sequence syntax is rejected before enum executable IR promotion")
            }
            SyntaxExprKind::SharedBorrow(_)
            | SyntaxExprKind::SharedAlloc(_)
            | SyntaxExprKind::SharedDuplicate(_) => unreachable!(''')
replace_once(p,
'''            TypeName::SharedRef(_) | TypeName::SharedOwner(_) => unreachable!(''',
'''            TypeName::Sequence(_) => {
                unreachable!("sequence types are rejected before enum executable IR promotion")
            }
            TypeName::SharedRef(_) | TypeName::SharedOwner(_) => unreachable!(''')
