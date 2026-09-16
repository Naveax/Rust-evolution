from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one anchor, found {count}: {old[:140]!r}")
    p.write_text(text.replace(old, new, 1))


def replace_between(path: str, start: str, end: str, new: str, *, after: str | None = None) -> None:
    p = Path(path)
    text = p.read_text()
    origin = text.index(after) if after is not None else 0
    left = text.index(start, origin)
    right = text.index(end, left)
    p.write_text(text[:left] + new + text[right:])


LOWER = "crates/evo-lowering/src/lib.rs"
REFS = "crates/evo-lowering/src/reference_state.rs"
CODEGEN = "crates/evo-codegen-rust/src/lib.rs"

# Lowered IR grows only after syntax/type plumbing has passed workspace + clippy.
replace_once(
    LOWER,
    '''    SequenceLookup {
        owner: String,
        index: Expr,
        binding: String,
        binding_by_reference: bool,
        binding_used: bool,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
}''',
    '''    SequenceLookup {
        owner: String,
        index: Expr,
        binding: String,
        binding_by_reference: bool,
        binding_used: bool,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    ArenaInsert {
        owner: String,
        value: Expr,
        binding: String,
    },
    ArenaLookup {
        owner: String,
        handle: Expr,
        binding: String,
        binding_by_reference: bool,
        binding_used: bool,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    ArenaRemove {
        owner: String,
        handle: Expr,
        binding: String,
        binding_used: bool,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
}''',
)
replace_once(
    LOWER,
    '''    SequenceNew {
        element_type: ValueType,
    },
    Binary {''',
    '''    SequenceNew {
        element_type: ValueType,
    },
    ArenaNew {
        element_type: ValueType,
    },
    Binary {''',
)

replace_once(
    LOWER,
    '''        StmtKind::SequenceLookup {
            then_body,
            else_body,
            ..
        } => block_always_returns(then_body) && block_always_returns(else_body),
        StmtKind::Let { .. }
        | StmtKind::Assign { .. }
        | StmtKind::Print(_)
        | StmtKind::Repeat { .. }
        | StmtKind::SequenceAppend { .. } => false,''',
    '''        StmtKind::SequenceLookup {
            then_body,
            else_body,
            ..
        }
        | StmtKind::ArenaLookup {
            then_body,
            else_body,
            ..
        }
        | StmtKind::ArenaRemove {
            then_body,
            else_body,
            ..
        } => block_always_returns(then_body) && block_always_returns(else_body),
        StmtKind::Let { .. }
        | StmtKind::Assign { .. }
        | StmtKind::Print(_)
        | StmtKind::Repeat { .. }
        | StmtKind::SequenceAppend { .. }
        | StmtKind::ArenaInsert { .. } => false,''',
)

# Existing sequence success-scope machinery is owner-agnostic; rename it accordingly.
p = Path(LOWER)
text = p.read_text().replace(
    "lower_sequence_lookup_success_scope",
    "lower_checked_lookup_success_scope",
).replace(
    "sequence lookup child scope must be present after lowering",
    "checked lookup child scope must be present after lowering",
)
p.write_text(text)

# Replace the syntax lookup/fail-closed arena section with actual sequence+arena semantics.
lookup_block = r'''            SyntaxStmtKind::SequenceLookup {
                owner,
                index,
                binding,
                then_body,
                else_body,
            } => {
                let owner_binding = self.visible_binding(owner).ok_or_else(|| LowerError {
                    message: format!("use of local {owner:?} before definition or outside its scope"),
                    span: statement.span,
                })?;
                let inspected = self.move_tracker.inspect_value(owner, statement.span)?;
                debug_assert_eq!(lowered_value_type(&inspected), owner_binding.value_type);

                match &owner_binding.value_type {
                    ValueType::Sequence(element_type) => {
                        let (index, index_type) = self.lower_expr(index)?;
                        if index_type != ValueType::Integer {
                            return Err(LowerError {
                                message: "sequence lookup index must be an integer".to_owned(),
                                span: index.span,
                            });
                        }
                        if self.visible_binding(binding).is_some() {
                            return Err(LowerError {
                                message: format!(
                                    "sequence lookup success binding {binding:?} conflicts with an already-visible local"
                                ),
                                span: statement.span,
                            });
                        }
                        if Self::block_reassigns_name(then_body, binding) {
                            return Err(LowerError {
                                message: format!(
                                    "reassigning sequence lookup success binding {binding:?} is not supported in v0"
                                ),
                                span: statement.span,
                            });
                        }

                        let element_type = element_type.as_ref().clone();
                        let binding_by_reference = !matches!(
                            &element_type,
                            ValueType::Integer
                                | ValueType::Bool
                                | ValueType::String
                                | ValueType::Handle(_)
                        );
                        let binding_type = if binding_by_reference {
                            ValueType::SharedRef(Box::new(element_type.clone()))
                        } else {
                            element_type
                        };
                        let binding_used = Self::identifier_uses_in_statements(then_body)
                            .contains_key(binding);

                        let entry = self.move_tracker.clone();
                        self.move_tracker = entry.clone();
                        let then_body = self.lower_checked_lookup_success_scope(
                            owner,
                            binding,
                            binding_type,
                            binding_by_reference && binding_used,
                            statement.span,
                            then_body,
                        )?;
                        let then_exit = self.move_tracker.clone();

                        self.move_tracker = entry.clone();
                        let else_body = self.lower_child_scope(else_body)?;
                        let else_exit = self.move_tracker.clone();

                        let then_returns = block_always_returns(&then_body);
                        let else_returns = block_always_returns(&else_body);
                        let mut merged = entry;
                        match (then_returns, else_returns) {
                            (false, false) => merged.merge_if(&then_exit, &else_exit),
                            (true, false) => {
                                let continues = merged.merge_if_continuing(None, Some(&else_exit));
                                debug_assert!(continues);
                            }
                            (false, true) => {
                                let continues = merged.merge_if_continuing(Some(&then_exit), None);
                                debug_assert!(continues);
                            }
                            (true, true) => {
                                let continues = merged.merge_if_continuing(None, None);
                                debug_assert!(!continues);
                            }
                        }
                        self.move_tracker = merged;

                        StmtKind::SequenceLookup {
                            owner: owner.clone(),
                            index,
                            binding: binding.clone(),
                            binding_by_reference,
                            binding_used,
                            then_body,
                            else_body,
                        }
                    }
                    ValueType::Arena(element_type) => {
                        let (handle, handle_type) = self.lower_expr(index)?;
                        let ValueType::Handle(handle_element_type) = &handle_type else {
                            return Err(LowerError {
                                message: format!(
                                    "arena lookup requires handle {}; found {}",
                                    type_label(element_type),
                                    type_label(&handle_type)
                                ),
                                span: handle.span,
                            });
                        };
                        if handle_element_type.as_ref() != element_type.as_ref() {
                            return Err(LowerError {
                                message: format!(
                                    "arena lookup for {owner:?} expects handle {}, found handle {}",
                                    type_label(element_type),
                                    type_label(handle_element_type)
                                ),
                                span: handle.span,
                            });
                        }
                        if self.visible_binding(binding).is_some() {
                            return Err(LowerError {
                                message: format!(
                                    "arena lookup success binding {binding:?} conflicts with an already-visible local"
                                ),
                                span: statement.span,
                            });
                        }
                        if Self::block_reassigns_name(then_body, binding) {
                            return Err(LowerError {
                                message: format!(
                                    "reassigning arena lookup success binding {binding:?} is not supported in v0"
                                ),
                                span: statement.span,
                            });
                        }

                        let element_type = element_type.as_ref().clone();
                        let binding_by_reference = !matches!(
                            &element_type,
                            ValueType::Integer | ValueType::Bool | ValueType::String
                        );
                        let binding_type = if binding_by_reference {
                            ValueType::SharedRef(Box::new(element_type.clone()))
                        } else {
                            element_type
                        };
                        let binding_used = Self::identifier_uses_in_statements(then_body)
                            .contains_key(binding);

                        let entry = self.move_tracker.clone();
                        self.move_tracker = entry.clone();
                        let then_body = self.lower_checked_lookup_success_scope(
                            owner,
                            binding,
                            binding_type,
                            binding_by_reference && binding_used,
                            statement.span,
                            then_body,
                        )?;
                        let then_exit = self.move_tracker.clone();

                        self.move_tracker = entry.clone();
                        let else_body = self.lower_child_scope(else_body)?;
                        let else_exit = self.move_tracker.clone();

                        let then_returns = block_always_returns(&then_body);
                        let else_returns = block_always_returns(&else_body);
                        let mut merged = entry;
                        match (then_returns, else_returns) {
                            (false, false) => merged.merge_if(&then_exit, &else_exit),
                            (true, false) => {
                                let continues = merged.merge_if_continuing(None, Some(&else_exit));
                                debug_assert!(continues);
                            }
                            (false, true) => {
                                let continues = merged.merge_if_continuing(Some(&then_exit), None);
                                debug_assert!(continues);
                            }
                            (true, true) => {
                                let continues = merged.merge_if_continuing(None, None);
                                debug_assert!(!continues);
                            }
                        }
                        self.move_tracker = merged;

                        StmtKind::ArenaLookup {
                            owner: owner.clone(),
                            handle,
                            binding: binding.clone(),
                            binding_by_reference,
                            binding_used,
                            then_body,
                            else_body,
                        }
                    }
                    _ => {
                        return Err(LowerError {
                            message: format!(
                                "lookup requires a sequence or arena owner; found {}",
                                type_label(&owner_binding.value_type)
                            ),
                            span: statement.span,
                        });
                    }
                }
            }
            SyntaxStmtKind::ArenaInsert {
                owner,
                value,
                binding,
            } => {
                let owner_binding = self.visible_binding(owner).ok_or_else(|| LowerError {
                    message: format!("use of local {owner:?} before definition or outside its scope"),
                    span: statement.span,
                })?;
                let ValueType::Arena(element_type) = &owner_binding.value_type else {
                    return Err(LowerError {
                        message: format!(
                            "insert requires an arena owner; found {}",
                            type_label(&owner_binding.value_type)
                        ),
                        span: statement.span,
                    });
                };
                if self.visible_binding(binding).is_some() {
                    return Err(LowerError {
                        message: format!(
                            "arena insert handle binding {binding:?} conflicts with an already-visible local"
                        ),
                        span: statement.span,
                    });
                }
                self.reference_tracker.ensure_owner_operation_allowed(
                    owner,
                    "arena",
                    OwnerOperation::Insert,
                    statement.span,
                )?;
                let inspected = self.move_tracker.inspect_value(owner, statement.span)?;
                debug_assert_eq!(lowered_value_type(&inspected), owner_binding.value_type);
                let (value, actual_type) = self.lower_expr(value)?;
                if &actual_type != element_type.as_ref() {
                    return Err(LowerError {
                        message: format!(
                            "insert into arena {owner:?} expects {}, found {}",
                            type_label(element_type),
                            type_label(&actual_type)
                        ),
                        span: value.span,
                    });
                }
                let handle_type = ValueType::Handle(Box::new(element_type.as_ref().clone()));
                self.define_binding(binding.clone(), handle_type, statement.span.start);
                self.mutable_declarations.insert(owner_binding.declaration_start);
                StmtKind::ArenaInsert {
                    owner: owner.clone(),
                    value,
                    binding: binding.clone(),
                }
            }
            SyntaxStmtKind::ArenaRemove {
                owner,
                handle,
                binding,
                then_body,
                else_body,
            } => {
                let owner_binding = self.visible_binding(owner).ok_or_else(|| LowerError {
                    message: format!("use of local {owner:?} before definition or outside its scope"),
                    span: statement.span,
                })?;
                let ValueType::Arena(element_type) = &owner_binding.value_type else {
                    return Err(LowerError {
                        message: format!(
                            "remove requires an arena owner; found {}",
                            type_label(&owner_binding.value_type)
                        ),
                        span: statement.span,
                    });
                };
                self.reference_tracker.ensure_owner_operation_allowed(
                    owner,
                    "arena",
                    OwnerOperation::Remove,
                    statement.span,
                )?;
                let inspected = self.move_tracker.inspect_value(owner, statement.span)?;
                debug_assert_eq!(lowered_value_type(&inspected), owner_binding.value_type);
                let (handle, handle_type) = self.lower_expr(handle)?;
                let ValueType::Handle(handle_element_type) = &handle_type else {
                    return Err(LowerError {
                        message: format!(
                            "arena remove requires handle {}; found {}",
                            type_label(element_type),
                            type_label(&handle_type)
                        ),
                        span: handle.span,
                    });
                };
                if handle_element_type.as_ref() != element_type.as_ref() {
                    return Err(LowerError {
                        message: format!(
                            "arena remove for {owner:?} expects handle {}, found handle {}",
                            type_label(element_type),
                            type_label(handle_element_type)
                        ),
                        span: handle.span,
                    });
                }
                if self.visible_binding(binding).is_some() {
                    return Err(LowerError {
                        message: format!(
                            "arena remove success binding {binding:?} conflicts with an already-visible local"
                        ),
                        span: statement.span,
                    });
                }
                if Self::block_reassigns_name(then_body, binding) {
                    return Err(LowerError {
                        message: format!(
                            "reassigning arena remove success binding {binding:?} is not supported in v0"
                        ),
                        span: statement.span,
                    });
                }
                let binding_used = Self::identifier_uses_in_statements(then_body)
                    .contains_key(binding);

                let entry = self.move_tracker.clone();
                self.move_tracker = entry.clone();
                let then_body = self.lower_checked_lookup_success_scope(
                    owner,
                    binding,
                    element_type.as_ref().clone(),
                    false,
                    statement.span,
                    then_body,
                )?;
                let then_exit = self.move_tracker.clone();

                self.move_tracker = entry.clone();
                let else_body = self.lower_child_scope(else_body)?;
                let else_exit = self.move_tracker.clone();

                let then_returns = block_always_returns(&then_body);
                let else_returns = block_always_returns(&else_body);
                let mut merged = entry;
                match (then_returns, else_returns) {
                    (false, false) => merged.merge_if(&then_exit, &else_exit),
                    (true, false) => {
                        let continues = merged.merge_if_continuing(None, Some(&else_exit));
                        debug_assert!(continues);
                    }
                    (false, true) => {
                        let continues = merged.merge_if_continuing(Some(&then_exit), None);
                        debug_assert!(continues);
                    }
                    (true, true) => {
                        let continues = merged.merge_if_continuing(None, None);
                        debug_assert!(!continues);
                    }
                }
                self.move_tracker = merged;
                self.mutable_declarations.insert(owner_binding.declaration_start);

                StmtKind::ArenaRemove {
                    owner: owner.clone(),
                    handle,
                    binding: binding.clone(),
                    binding_used,
                    then_body,
                    else_body,
                }
            }
'''
replace_between(
    LOWER,
    "            SyntaxStmtKind::SequenceLookup {\n",
    "            SyntaxStmtKind::Match { .. } => {\n",
    lookup_block,
    after="fn lower_statement",
)

# Arena construction becomes a direct typed expression.
replace_once(
    LOWER,
    '''            SyntaxExprKind::ArenaNew { .. } => {
                return Err(LowerError {
                    message: "generational arena construction is parsed, but arena semantic lowering is not enabled in this commit".to_owned(),
                    span: expr.span,
                });
            }
''',
    '''            SyntaxExprKind::ArenaNew { element_type } => {
                let element_type = self
                    .record_environment
                    .resolve_type_name(element_type, expr.span)?;
                let element_type = lowered_value_type(&element_type);
                (
                    ExprKind::ArenaNew {
                        element_type: element_type.clone(),
                    },
                    ValueType::Arena(Box::new(element_type)),
                )
            }
''',
)

# Apply mutability through arena branch blocks and classify arena insert as leaf mutation.
replace_once(
    LOWER,
    '''                StmtKind::SequenceLookup {
                    then_body,
                    else_body,
                    ..
                } => {
                    self.apply_mutability(then_body);
                    self.apply_mutability(else_body);
                }
                StmtKind::Assign { .. }
                | StmtKind::Print(_)
                | StmtKind::Return(_)
                | StmtKind::SequenceAppend { .. } => {}
''',
    '''                StmtKind::SequenceLookup {
                    then_body,
                    else_body,
                    ..
                }
                | StmtKind::ArenaLookup {
                    then_body,
                    else_body,
                    ..
                }
                | StmtKind::ArenaRemove {
                    then_body,
                    else_body,
                    ..
                } => {
                    self.apply_mutability(then_body);
                    self.apply_mutability(else_body);
                }
                StmtKind::Assign { .. }
                | StmtKind::Print(_)
                | StmtKind::Return(_)
                | StmtKind::SequenceAppend { .. }
                | StmtKind::ArenaInsert { .. } => {}
''',
)

# Diagnostics distinguish arena-exclusive mutations from Vec growth.
replace_once(
    REFS,
    '''pub(crate) enum OwnerOperation {
    Move,
    Reinitialize,
    Grow,
}''',
    '''pub(crate) enum OwnerOperation {
    Move,
    Reinitialize,
    Grow,
    Insert,
    Remove,
}''',
)
replace_once(
    REFS,
    '''            Self::Move => "move",
            Self::Reinitialize => "reinitialize",
            Self::Grow => "grow",
''',
    '''            Self::Move => "move",
            Self::Reinitialize => "reinitialize",
            Self::Grow => "grow",
            Self::Insert => "insert into",
            Self::Remove => "remove from",
''',
)

# Codegen: runtime support is emitted only for programs that actually mention arena/handle types.
replace_once(
    CODEGEN,
    '''    fn generate(mut self, program: &Program) -> GeneratedRust {
        for record in &program.records {''',
    '''    fn generate(mut self, program: &Program) -> GeneratedRust {
        if program_uses_arena_support(program) {
            self.push_unmapped(arena_runtime_support());
        }

        for record in &program.records {''',
)

arena_stmt_codegen = r'''            StmtKind::ArenaInsert {
                owner,
                value,
                binding,
            } => {
                self.push_mapped_line(
                    format!(
                        "{padding}let {} = __evo_arena_insert(&mut {}, {});\n",
                        generated_identifier(binding),
                        generated_identifier(owner),
                        render_expr(value)
                    ),
                    statement.span,
                );
            }
            StmtKind::ArenaLookup {
                owner,
                handle,
                binding,
                binding_by_reference,
                binding_used,
                then_body,
                else_body,
            } => {
                let pattern = if !binding_used {
                    "_".to_owned()
                } else if *binding_by_reference {
                    generated_identifier(binding)
                } else {
                    format!("&{}", generated_identifier(binding))
                };
                self.push_mapped_line(
                    format!(
                        "{padding}if let Some({pattern}) = __evo_arena_get(&{}, {}) {{\n",
                        generated_identifier(owner),
                        render_expr(handle)
                    ),
                    statement.span,
                );
                for statement in then_body {
                    self.write_statement(statement, indent + 1);
                }
                self.push_mapped_line(format!("{padding}}} else {{\n"), statement.span);
                for statement in else_body {
                    self.write_statement(statement, indent + 1);
                }
                self.push_mapped_line(format!("{padding}}}\n"), statement.span);
            }
            StmtKind::ArenaRemove {
                owner,
                handle,
                binding,
                binding_used,
                then_body,
                else_body,
            } => {
                let pattern = if *binding_used {
                    generated_identifier(binding)
                } else {
                    "_".to_owned()
                };
                self.push_mapped_line(
                    format!(
                        "{padding}if let Some({pattern}) = __evo_arena_remove(&mut {}, {}) {{\n",
                        generated_identifier(owner),
                        render_expr(handle)
                    ),
                    statement.span,
                );
                for statement in then_body {
                    self.write_statement(statement, indent + 1);
                }
                self.push_mapped_line(format!("{padding}}} else {{\n"), statement.span);
                for statement in else_body {
                    self.write_statement(statement, indent + 1);
                }
                self.push_mapped_line(format!("{padding}}}\n"), statement.span);
            }
'''
replace_once(
    CODEGEN,
    '''            StmtKind::If {
                condition,
                then_body,
                else_body,
            } => {''',
    arena_stmt_codegen + '''            StmtKind::If {
                condition,
                then_body,
                else_body,
            } => {''',
)

replace_once(
    CODEGEN,
    '''        ExprKind::SequenceNew { element_type } => {
            format!("Vec::<{}>::new()", rust_type(element_type))
        }
''',
    '''        ExprKind::SequenceNew { element_type } => {
            format!("Vec::<{}>::new()", rust_type(element_type))
        }
        ExprKind::ArenaNew { element_type } => {
            format!("__evo_arena_new::<{}>()", rust_type(element_type))
        }
''',
)

# Input-int traversal must remain exhaustive for the new IR.
replace_once(
    CODEGEN,
    '''        StmtKind::SequenceAppend { value, .. } => expr_uses_input_int(value),
        StmtKind::SequenceLookup {
            index,
            then_body,
            else_body,
            ..
        } => {
            expr_uses_input_int(index)
                || then_body.iter().any(statement_uses_input_int)
                || else_body.iter().any(statement_uses_input_int)
        }
''',
    '''        StmtKind::SequenceAppend { value, .. } | StmtKind::ArenaInsert { value, .. } => {
            expr_uses_input_int(value)
        }
        StmtKind::SequenceLookup {
            index,
            then_body,
            else_body,
            ..
        } => {
            expr_uses_input_int(index)
                || then_body.iter().any(statement_uses_input_int)
                || else_body.iter().any(statement_uses_input_int)
        }
        StmtKind::ArenaLookup {
            handle,
            then_body,
            else_body,
            ..
        }
        | StmtKind::ArenaRemove {
            handle,
            then_body,
            else_body,
            ..
        } => {
            expr_uses_input_int(handle)
                || then_body.iter().any(statement_uses_input_int)
                || else_body.iter().any(statement_uses_input_int)
        }
''',
)
replace_once(
    CODEGEN,
    '''        | ExprKind::Local(_)
        | ExprKind::SequenceNew { .. } => false,
''',
    '''        | ExprKind::Local(_)
        | ExprKind::SequenceNew { .. }
        | ExprKind::ArenaNew { .. } => false,
''',
)

# Runtime support and detection live before the existing input helper detection.
replace_once(
    CODEGEN,
    '''fn program_uses_input_int(program: &Program) -> bool {''',
    r'''fn arena_runtime_support() -> &'static str {
    concat!(
        "struct __EvoHandle<T> {\n",
        "    arena: u64,\n",
        "    index: usize,\n",
        "    generation: u64,\n",
        "    _marker: std::marker::PhantomData<fn() -> T>,\n",
        "}\n",
        "impl<T> Copy for __EvoHandle<T> {}\n",
        "impl<T> Clone for __EvoHandle<T> {\n",
        "    fn clone(&self) -> Self { *self }\n",
        "}\n",
        "struct __EvoSlot<T> {\n",
        "    generation: u64,\n",
        "    value: Option<T>,\n",
        "    retired: bool,\n",
        "}\n",
        "struct __EvoArena<T> {\n",
        "    id: u64,\n",
        "    slots: Vec<__EvoSlot<T>>,\n",
        "    free: Vec<usize>,\n",
        "}\n",
        "std::thread_local! {\n",
        "    static __EVO_NEXT_ARENA_ID: std::cell::Cell<u64> = const { std::cell::Cell::new(1) };\n",
        "}\n",
        "fn __evo_next_arena_id() -> u64 {\n",
        "    __EVO_NEXT_ARENA_ID.with(|next| {\n",
        "        let id = next.get();\n",
        "        if id == 0 { panic!(\"arena identity exhausted\"); }\n",
        "        next.set(id.checked_add(1).unwrap_or(0));\n",
        "        id\n",
        "    })\n",
        "}\n",
        "fn __evo_arena_new<T>() -> __EvoArena<T> {\n",
        "    __EvoArena { id: __evo_next_arena_id(), slots: Vec::new(), free: Vec::new() }\n",
        "}\n",
        "fn __evo_arena_insert<T>(arena: &mut __EvoArena<T>, value: T) -> __EvoHandle<T> {\n",
        "    if let Some(index) = arena.free.pop() {\n",
        "        let slot = &mut arena.slots[index];\n",
        "        debug_assert!(!slot.retired && slot.value.is_none());\n",
        "        slot.value = Some(value);\n",
        "        return __EvoHandle { arena: arena.id, index, generation: slot.generation, _marker: std::marker::PhantomData };\n",
        "    }\n",
        "    let index = arena.slots.len();\n",
        "    arena.slots.push(__EvoSlot { generation: 0, value: Some(value), retired: false });\n",
        "    __EvoHandle { arena: arena.id, index, generation: 0, _marker: std::marker::PhantomData }\n",
        "}\n",
        "fn __evo_arena_get<T>(arena: &__EvoArena<T>, handle: __EvoHandle<T>) -> Option<&T> {\n",
        "    if handle.arena != arena.id { return None; }\n",
        "    arena.slots.get(handle.index)\n",
        "        .filter(|slot| !slot.retired && slot.generation == handle.generation)\n",
        "        .and_then(|slot| slot.value.as_ref())\n",
        "}\n",
        "fn __evo_arena_remove<T>(arena: &mut __EvoArena<T>, handle: __EvoHandle<T>) -> Option<T> {\n",
        "    if handle.arena != arena.id { return None; }\n",
        "    let slot = arena.slots.get_mut(handle.index)?;\n",
        "    if slot.retired || slot.generation != handle.generation { return None; }\n",
        "    let value = slot.value.take()?;\n",
        "    if slot.generation == u64::MAX {\n",
        "        slot.retired = true;\n",
        "    } else {\n",
        "        slot.generation += 1;\n",
        "        arena.free.push(handle.index);\n",
        "    }\n",
        "    Some(value)\n",
        "}\n\n",
    )
}

fn value_type_uses_arena_support(value_type: &ValueType) -> bool {
    match value_type {
        ValueType::Arena(_) | ValueType::Handle(_) => true,
        ValueType::SharedRef(inner) | ValueType::Sequence(inner) => {
            value_type_uses_arena_support(inner)
        }
        ValueType::Integer
        | ValueType::Bool
        | ValueType::String
        | ValueType::Record(_)
        | ValueType::SharedOwner(_) => false,
    }
}

fn expr_uses_arena_support(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::ArenaNew { .. } => true,
        ExprKind::SequenceNew { element_type } => value_type_uses_arena_support(element_type),
        ExprKind::Call { arguments, .. } => arguments.iter().any(expr_uses_arena_support),
        ExprKind::Construct { fields, .. } => fields
            .iter()
            .any(|field| expr_uses_arena_support(&field.value)),
        ExprKind::FieldAccess { base, .. }
        | ExprKind::LogicalNot(base)
        | ExprKind::UnaryMinus(base)
        | ExprKind::SharedBorrow(base)
        | ExprKind::SharedOwnerBorrow(base)
        | ExprKind::SharedAlloc(base)
        | ExprKind::SharedDuplicate(base) => expr_uses_arena_support(base),
        ExprKind::Binary { left, right, .. } => {
            expr_uses_arena_support(left) || expr_uses_arena_support(right)
        }
        ExprKind::Integer(_)
        | ExprKind::String(_)
        | ExprKind::Bool(_)
        | ExprKind::Local(_)
        | ExprKind::InputInt => false,
    }
}

fn statement_uses_arena_support(statement: &Stmt) -> bool {
    match &statement.kind {
        StmtKind::ArenaInsert { .. }
        | StmtKind::ArenaLookup { .. }
        | StmtKind::ArenaRemove { .. } => true,
        StmtKind::Let { expr, .. }
        | StmtKind::Assign { expr, .. }
        | StmtKind::Print(expr)
        | StmtKind::Return(expr) => expr_uses_arena_support(expr),
        StmtKind::Repeat { count, body } => {
            expr_uses_arena_support(count) || body.iter().any(statement_uses_arena_support)
        }
        StmtKind::SequenceAppend { value, .. } => expr_uses_arena_support(value),
        StmtKind::SequenceLookup {
            index,
            then_body,
            else_body,
            ..
        } => {
            expr_uses_arena_support(index)
                || then_body.iter().any(statement_uses_arena_support)
                || else_body.iter().any(statement_uses_arena_support)
        }
        StmtKind::If {
            condition,
            then_body,
            else_body,
        } => {
            expr_uses_arena_support(condition)
                || then_body.iter().any(statement_uses_arena_support)
                || else_body.iter().any(statement_uses_arena_support)
        }
    }
}

fn program_uses_arena_support(program: &Program) -> bool {
    program
        .records
        .iter()
        .flat_map(|record| &record.fields)
        .any(|field| matches!(field.value_type, RecordType::Handle(_)))
        || program.functions.iter().any(|function| {
            value_type_uses_arena_support(&function.return_type)
                || function
                    .parameters
                    .iter()
                    .any(|parameter| value_type_uses_arena_support(&parameter.value_type))
                || function.body.iter().any(statement_uses_arena_support)
        })
        || program.statements.iter().any(statement_uses_arena_support)
}

fn program_uses_input_int(program: &Program) -> bool {''',
)

# Focused semantic tests.
Path("crates/evo-lowering/tests/arena_semantics_v0.rs").write_text(r'''use evo_lexer::lex;
use evo_lowering::{RecordType, StmtKind, ValueType, lower};
use evo_parser::parse;

fn lower_source(source: &str) -> Result<evo_lowering::Program, evo_lowering::LowerError> {
    let tokens = lex(source).expect("arena semantic source should lex");
    let syntax = parse(&tokens).expect("arena semantic source should parse");
    lower(&syntax)
}

#[test]
fn lowers_arena_insert_lookup_remove_and_handle_contracts() {
    let program = lower_source(
        "fn keep(h handle int) handle int\nreturn h\nend\nitems = arena int()\ninsert items, 7 as h\nlookup items, h as value\nprint value\nelse\nprint 0\nend\nremove items, h as removed\nprint removed\nelse\nprint 0\nend\n",
    )
    .expect("bounded arena surface should lower");
    assert_eq!(
        program.functions[0].parameters[0].value_type,
        ValueType::Handle(Box::new(ValueType::Integer))
    );
    assert_eq!(
        program.functions[0].return_type,
        ValueType::Handle(Box::new(ValueType::Integer))
    );
    assert!(matches!(program.statements[1].kind, StmtKind::ArenaInsert { .. }));
    assert!(matches!(program.statements[2].kind, StmtKind::ArenaLookup { .. }));
    assert!(matches!(program.statements[3].kind, StmtKind::ArenaRemove { .. }));
}

#[test]
fn rejects_wrong_handle_payload_type_before_codegen() {
    let error = lower_source(
        "record A\nvalue int\nend\nrecord B\nvalue int\nend\na = arena A()\nb = arena B()\ninsert b, B(value = 1) as hb\nlookup a, hb as item\nprint item.value\nelse\nprint 0\nend\n",
    )
    .expect_err("different handle payload types must fail statically");
    assert!(error.message.contains("expects handle A"));
    assert!(error.message.contains("handle B"));
}

#[test]
fn live_arena_element_reference_blocks_insert_remove_move_and_reinit() {
    for (label, body, needle) in [
        (
            "insert",
            "insert items, Item(value = 2) as h2\nprint item.value\n",
            "cannot insert into arena local",
        ),
        (
            "remove",
            "remove items, h as removed\nprint removed.value\nelse\nprint 0\nend\nprint item.value\n",
            "cannot remove from arena local",
        ),
        (
            "move",
            "moved = items\nprint item.value\n",
            "cannot move arena local",
        ),
        (
            "reinit",
            "items = arena Item()\nprint item.value\n",
            "cannot reinitialize arena local",
        ),
    ] {
        let source = format!(
            "record Item\nvalue int\nend\nitems = arena Item()\ninsert items, Item(value = 1) as h\nlookup items, h as item\n{body}else\nprint 0\nend\n"
        );
        let error = lower_source(&source).unwrap_err_or_else(|_| panic!("{label} with live reference must fail"));
        assert!(error.message.contains(needle), "{label}: {}", error.message);
    }
}

trait ResultExt<T, E> {
    fn unwrap_err_or_else(self, ok: impl FnOnce(T) -> E) -> E;
}
impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn unwrap_err_or_else(self, ok: impl FnOnce(T) -> E) -> E {
        match self { Ok(value) => ok(value), Err(error) => error }
    }
}

#[test]
fn final_use_release_allows_later_insert_and_remove() {
    lower_source(
        "record Item\nvalue int\nend\nitems = arena Item()\ninsert items, Item(value = 1) as h\nlookup items, h as item\nprint item.value\ninsert items, Item(value = 2) as h2\nremove items, h as removed\nprint removed.value\nelse\nprint 0\nend\nlookup items, h2 as second\nprint second.value\nelse\nprint 0\nend\nelse\nprint 0\nend\n",
    )
    .expect("owner mutations after final reference use should lower");
}

#[test]
fn handle_fields_are_fixed_size_graph_edges_not_recursive_records() {
    let program = lower_source("record Node\nnext handle Node\nend\n").expect("self handle edge should be sized");
    assert_eq!(program.records[0].fields[0].value_type, RecordType::Handle("Node".to_owned()));
}

#[test]
fn sequence_handle_lookup_is_copy_like_not_borrowing() {
    let program = lower_source(
        "items = arena int()\ninsert items, 1 as h\nedges = seq handle int()\nappend edges, h\nlookup edges, 0 as copied\nlookup items, copied as value\nprint value\nelse\nprint 0\nend\nelse\nprint 0\nend\n",
    )
    .expect("handle adjacency storage should lower");
    let StmtKind::SequenceLookup { binding_by_reference, .. } = &program.statements[3].kind else {
        panic!("expected sequence handle lookup");
    };
    assert!(!binding_by_reference, "handles must copy without pinning the sequence");
}
''')

# Fix the one helper in the focused test without depending on unstable Result conveniences.
test_path = Path("crates/evo-lowering/tests/arena_semantics_v0.rs")
text = test_path.read_text().replace(
    '        let error = lower_source(&source).unwrap_err_or_else(|_| panic!("{label} with live reference must fail"));\n',
    '        let error = match lower_source(&source) {\n            Ok(_) => panic!("{label} with live reference must fail"),\n            Err(error) => error,\n        };\n',
).replace(
    '''\ntrait ResultExt<T, E> {
    fn unwrap_err_or_else(self, ok: impl FnOnce(T) -> E) -> E;
}
impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn unwrap_err_or_else(self, ok: impl FnOnce(T) -> E) -> E {
        match self { Ok(value) => ok(value), Err(error) => error }
    }
}\n''',
    "\n",
)
test_path.write_text(text)

Path("crates/evo-codegen-rust/tests/arena_compile.rs").write_text(r'''use evo_codegen_rust::generate_lowered_rust;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn generated(source: &str) -> String {
    let tokens = lex(source).expect("arena compile source should lex");
    let syntax = parse(&tokens).expect("arena compile source should parse");
    let lowered = lower(&syntax).expect("arena compile source should lower");
    generate_lowered_rust(&lowered)
}

fn compile(label: &str, source: &str) -> (PathBuf, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/arena-generated-rust")
        .join(label);
    if root.exists() {
        fs::remove_dir_all(&root).expect("stale arena generated directory should be removable");
    }
    fs::create_dir_all(&root).expect("arena generated directory should be creatable");
    let input = root.join("case.rs");
    let output = root.join(format!("case{}", std::env::consts::EXE_SUFFIX));
    let rust = generated(source);
    fs::write(&input, &rust).expect("generated Rust should be writable");
    let result = Command::new("rustc")
        .arg("--edition=2024")
        .arg("--crate-name")
        .arg("evo_arena_codegen_case")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .output()
        .expect("rustc should execute");
    assert!(
        result.status.success(),
        "generated Rust failed to compile:\n{rust}\n\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    (output, rust)
}

#[test]
fn generated_arena_rejects_wrong_arena_and_stale_handles_and_reuses_generation() {
    let source = "a = arena int()\nb = arena int()\ninsert a, 10 as h\ninsert b, 20 as hb\nlookup b, h as wrong\nprint 999\nelse\nprint 1\nend\nremove a, h as removed\nprint removed\nelse\nprint 0\nend\ninsert a, 30 as fresh\nlookup a, h as stale\nprint 999\nelse\nprint 2\nend\nlookup a, fresh as value\nprint value\nelse\nprint 0\nend\n";
    let (binary, rust) = compile("identity-stale-reuse", source);
    let output = Command::new(binary).output().expect("generated arena program should run");
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n10\n2\n30\n");
    assert!(rust.contains("struct __EvoHandle<T>"));
    assert!(rust.contains("generation += 1"));
    assert!(rust.contains("slot.retired = true"));
    assert!(rust.contains("checked_add(1).unwrap_or(0)"));
    assert!(!rust.contains("unsafe"));
    assert!(!rust.contains("RefCell"));
    assert!(!rust.contains("Mutex"));
    assert!(!rust.contains("HashMap"));
}

#[test]
fn generated_handle_graph_storage_compiles_without_refcount_scaffolding() {
    let (_, rust) = compile(
        "handle-sequence",
        "items = arena int()\ninsert items, 7 as h\nedges = seq handle int()\nappend edges, h\nlookup edges, 0 as copied\nlookup items, copied as value\nprint value\nelse\nprint 0\nend\nelse\nprint 0\nend\n",
    );
    assert!(rust.contains("Vec<__EvoHandle<i64>>"));
    assert!(!rust.contains("Rc::clone"));
    assert!(!rust.contains("Arc<"));
}

#[test]
fn generated_record_lookup_borrows_payload_and_releases_before_mutation() {
    let _ = compile(
        "record-nll",
        "record Item\nvalue int\nend\nitems = arena Item()\ninsert items, Item(value = 1) as h\nlookup items, h as item\nprint item.value\ninsert items, Item(value = 2) as h2\nremove items, h as removed\nprint removed.value\nelse\nprint 0\nend\nlookup items, h2 as second\nprint second.value\nelse\nprint 0\nend\nelse\nprint 0\nend\n",
    );
}
''')
