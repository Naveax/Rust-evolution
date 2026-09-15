from pathlib import Path


def text(path):
    return Path(path).read_text()


def replace_once(path, old, new):
    p = Path(path)
    source = p.read_text()
    count = source.count(old)
    if count != 1:
        raise SystemExit(f"expected one semantic anchor in {path}, found {count}: {old[:140]!r}")
    p.write_text(source.replace(old, new, 1))


def insert_before(path, marker, addition):
    replace_once(path, marker, addition + marker)


def replace_between(path, start, end, replacement):
    p = Path(path)
    source = p.read_text()
    first = source.find(start)
    if first < 0 or source.find(start, first + 1) >= 0:
        raise SystemExit(f"non-unique/missing start marker in {path}: {start!r}")
    second = source.find(end, first + len(start))
    if second < 0:
        raise SystemExit(f"missing end marker in {path}: {end!r}")
    p.write_text(source[:first] + replacement + source[second:])

# Static semantic type: sequence is an owned, move-only Vec-shaped value.
p = "crates/evo-lowering/src/record_environment_records.rs"
replace_once(p,
'''    SharedOwner(String),
    SharedRef(Box<SemanticType>),
}''',
'''    SharedOwner(String),
    SharedRef(Box<SemanticType>),
    Sequence(Box<SemanticType>),
}''')
replace_once(p,
'''        !matches!(self, Self::Record(_) | Self::SharedOwner(_))''',
'''        !matches!(self, Self::Record(_) | Self::SharedOwner(_) | Self::Sequence(_))''')
replace_once(p,
'''            SyntaxTypeName::Sequence(_) => Err(LowerError {
                message: "append-only sequence semantic lowering is not implemented yet".to_owned(),
                span,
            }),''',
'''            SyntaxTypeName::Sequence(inner) => self
                .resolve_type_name(inner, span)
                .map(|inner| SemanticType::Sequence(Box::new(inner))),''')
replace_once(p,
'''            SemanticType::SharedRef(inner) => match inner.as_ref() {
                SemanticType::Record(name) => name,
                _ => {''',
'''            SemanticType::SharedRef(inner) => match inner.as_ref() {
                SemanticType::Record(name) | SemanticType::SharedOwner(name) => name,
                _ => {''')
replace_once(p,
'''        SemanticType::SharedRef(inner) => format!("&{}", semantic_type_label(inner)),
    }''',
'''        SemanticType::SharedRef(inner) => format!("&{}", semantic_type_label(inner)),
        SemanticType::Sequence(inner) => format!("seq {}", semantic_type_label(inner)),
    }''')

# Reallocation/growth is an owner operation that conflicts with live element references.
p = "crates/evo-lowering/src/reference_state.rs"
replace_once(p,
'''pub(crate) enum OwnerOperation {
    Move,
    Reinitialize,
}''',
'''pub(crate) enum OwnerOperation {
    Move,
    Reinitialize,
    Grow,
}''')
replace_once(p,
'''            Self::Reinitialize => "reinitialize",
        }''',
'''            Self::Reinitialize => "reinitialize",
            Self::Grow => "grow",
        }''')

p = "crates/evo-lowering/src/record_ownership.rs"
replace_once(p,
'''        Some(SemanticType::SharedOwner(_)) => "shared handle",
        _ => "record",''',
'''        Some(SemanticType::SharedOwner(_)) => "shared handle",
        Some(SemanticType::Sequence(_)) => "sequence",
        _ => "record",''')

# Lowered IR and analyzer semantics.
p = "crates/evo-lowering/src/lib.rs"
replace_once(p,
'''    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
}''',
'''    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    SequenceAppend {
        owner: String,
        value: Expr,
    },
    SequenceLookup {
        owner: String,
        index: Expr,
        binding: String,
        binding_by_reference: bool,
        binding_used: bool,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
}''')
replace_once(p,
'''    SharedAlloc(Box<Expr>),
    SharedDuplicate(Box<Expr>),
    Binary {''',
'''    SharedAlloc(Box<Expr>),
    SharedDuplicate(Box<Expr>),
    SequenceNew {
        element_type: ValueType,
    },
    Binary {''')
replace_once(p,
'''    SharedOwner(String),
    SharedRef(Box<ValueType>),
}''',
'''    SharedOwner(String),
    SharedRef(Box<ValueType>),
    Sequence(Box<ValueType>),
}''')
replace_once(p,
'''        StmtKind::Let { .. }
        | StmtKind::Assign { .. }
        | StmtKind::Print(_)
        | StmtKind::Repeat { .. } => false,''',
'''        StmtKind::SequenceLookup {
            then_body,
            else_body,
            ..
        } => block_always_returns(then_body) && block_always_returns(else_body),
        StmtKind::Let { .. }
        | StmtKind::Assign { .. }
        | StmtKind::Print(_)
        | StmtKind::Repeat { .. }
        | StmtKind::SequenceAppend { .. } => false,''')
replace_once(p,
'''        ValueType::SharedRef(inner) => {
            SemanticType::SharedRef(Box::new(semantic_type(inner)))
        }
    }''',
'''        ValueType::SharedRef(inner) => {
            SemanticType::SharedRef(Box::new(semantic_type(inner)))
        }
        ValueType::Sequence(inner) => SemanticType::Sequence(Box::new(semantic_type(inner))),
    }''')
replace_once(p,
'''        SemanticType::SharedRef(inner) => {
            ValueType::SharedRef(Box::new(lowered_value_type(inner)))
        }
    }''',
'''        SemanticType::SharedRef(inner) => {
            ValueType::SharedRef(Box::new(lowered_value_type(inner)))
        }
        SemanticType::Sequence(inner) => ValueType::Sequence(Box::new(lowered_value_type(inner))),
    }''')
replace_once(p,
'''        ValueType::SharedRef(inner) => format!("&{}", type_label(inner)),
    }''',
'''        ValueType::SharedRef(inner) => format!("&{}", type_label(inner)),
        ValueType::Sequence(inner) => format!("seq {}", type_label(inner)),
    }''')
replace_once(p,
'''                        ValueType::SharedOwner(_) => self.reference_tracker.ensure_owner_operation_allowed(
                            name,
                            "shared handle",
                            OwnerOperation::Reinitialize,
                            statement.span,
                        )?,
                        _ => {}''',
'''                        ValueType::SharedOwner(_) => self.reference_tracker.ensure_owner_operation_allowed(
                            name,
                            "shared handle",
                            OwnerOperation::Reinitialize,
                            statement.span,
                        )?,
                        ValueType::Sequence(_) => self.reference_tracker.ensure_owner_operation_allowed(
                            name,
                            "sequence",
                            OwnerOperation::Reinitialize,
                            statement.span,
                        )?,
                        _ => {}''')
replace_once(p,
'''            SyntaxStmtKind::SequenceAppend { .. } | SyntaxStmtKind::SequenceLookup { .. } => {
                return Err(LowerError {
                    message: "append-only sequence syntax is parsed, but sequence semantic lowering is not implemented yet".to_owned(),
                    span: statement.span,
                });
            }
''',
'''            SyntaxStmtKind::SequenceAppend { owner, value } => {
                let owner_binding = self.visible_binding(owner).ok_or_else(|| LowerError {
                    message: format!("use of local {owner:?} before definition or outside its scope"),
                    span: statement.span,
                })?;
                let ValueType::Sequence(element_type) = &owner_binding.value_type else {
                    return Err(LowerError {
                        message: format!(
                            "append requires a sequence owner; found {}",
                            type_label(&owner_binding.value_type)
                        ),
                        span: statement.span,
                    });
                };
                self.reference_tracker.ensure_owner_operation_allowed(
                    owner,
                    "sequence",
                    OwnerOperation::Grow,
                    statement.span,
                )?;
                let inspected = self.move_tracker.inspect_value(owner, statement.span)?;
                debug_assert_eq!(lowered_value_type(&inspected), owner_binding.value_type);
                let (value, actual_type) = self.lower_expr(value)?;
                if &actual_type != element_type.as_ref() {
                    return Err(LowerError {
                        message: format!(
                            "append to sequence {owner:?} expects {}, found {}",
                            type_label(element_type),
                            type_label(&actual_type)
                        ),
                        span: value.span,
                    });
                }
                self.mutable_declarations.insert(owner_binding.declaration_start);
                StmtKind::SequenceAppend {
                    owner: owner.clone(),
                    value,
                }
            }
            SyntaxStmtKind::SequenceLookup {
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
                let ValueType::Sequence(element_type) = &owner_binding.value_type else {
                    return Err(LowerError {
                        message: format!(
                            "lookup requires a sequence owner; found {}",
                            type_label(&owner_binding.value_type)
                        ),
                        span: statement.span,
                    });
                };
                let inspected = self.move_tracker.inspect_value(owner, statement.span)?;
                debug_assert_eq!(lowered_value_type(&inspected), owner_binding.value_type);
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
                    element_type,
                    ValueType::Integer | ValueType::Bool | ValueType::String
                );
                let binding_type = if binding_by_reference {
                    ValueType::SharedRef(Box::new(element_type))
                } else {
                    element_type
                };
                let binding_used = Self::identifier_uses_in_statements(then_body)
                    .contains_key(binding);

                let entry = self.move_tracker.clone();
                self.move_tracker = entry.clone();
                let then_body = self.lower_sequence_lookup_success_scope(
                    owner,
                    binding,
                    binding_type,
                    binding_by_reference && binding_used,
                    statement.span.start,
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
''')
# Whole-sequence values cannot be printed as scalars.
replace_once(p,
'''            &expression_type,
            ValueType::Record(_) | ValueType::SharedOwner(_) | ValueType::SharedRef(_)
        ) {''',
'''            &expression_type,
            ValueType::Record(_)
                | ValueType::SharedOwner(_)
                | ValueType::SharedRef(_)
                | ValueType::Sequence(_)
        ) {''')
# Sequence identifiers are move-only owners and must respect live element references.
replace_once(p,
'''                        ValueType::SharedOwner(_) => self.reference_tracker.ensure_owner_operation_allowed(
                            name,
                            "shared handle",
                            OwnerOperation::Move,
                            expr.span,
                        )?,
                        _ => {}''',
'''                        ValueType::SharedOwner(_) => self.reference_tracker.ensure_owner_operation_allowed(
                            name,
                            "shared handle",
                            OwnerOperation::Move,
                            expr.span,
                        )?,
                        ValueType::Sequence(_) => self.reference_tracker.ensure_owner_operation_allowed(
                            name,
                            "sequence",
                            OwnerOperation::Move,
                            expr.span,
                        )?,
                        _ => {}''')
replace_once(p,
'''            SyntaxExprKind::SequenceNew { .. } => {
                return Err(LowerError {
                    message: "sequence construction syntax is parsed, but sequence semantic lowering is not implemented yet".to_owned(),
                    span: expr.span,
                });
            }
''',
'''            SyntaxExprKind::SequenceNew { element_type } => {
                let element_type = self
                    .record_environment
                    .resolve_type_name(element_type, expr.span)?;
                let element_type = lowered_value_type(&element_type);
                (
                    ExprKind::SequenceNew {
                        element_type: element_type.clone(),
                    },
                    ValueType::Sequence(Box::new(element_type)),
                )
            }
''')
replace_once(p,
'''                    && matches!(&left_type, ValueType::Record(_) | ValueType::SharedOwner(_))''',
'''                    && matches!(
                        &left_type,
                        ValueType::Record(_) | ValueType::SharedOwner(_) | ValueType::Sequence(_)
                    )''')
replace_once(p,
'''                        if matches!(&right_type, ValueType::Record(_) | ValueType::SharedOwner(_)) {''',
'''                        if matches!(
                            &right_type,
                            ValueType::Record(_) | ValueType::SharedOwner(_) | ValueType::Sequence(_)
                        ) {''')
replace_once(p,
'''            ValueType::Integer | ValueType::Bool | ValueType::String => Err(LowerError {''',
'''            ValueType::Integer | ValueType::Bool | ValueType::String | ValueType::Sequence(_) => Err(LowerError {''')
replace_once(p,
'''                    ValueType::Integer | ValueType::Bool | ValueType::String => {
                        Err(LowerError {''',
'''                    ValueType::Integer
                    | ValueType::Bool
                    | ValueType::String
                    | ValueType::Sequence(_) => {
                        Err(LowerError {''')
# Helpers for scoped checked lookup binding and its no-reassignment v0 rule.
insert_before(p,
'''    fn lower_expr(&mut self, expr: &SyntaxExpr) -> Result<(Expr, ValueType), LowerError> {''',
'''    fn lower_sequence_lookup_success_scope(
        &mut self,
        owner: &str,
        binding: &str,
        binding_type: ValueType,
        track_reference: bool,
        declaration_start: usize,
        statements: &[SyntaxStmt],
    ) -> Result<Vec<Stmt>, LowerError> {
        self.scopes.push(HashMap::new());
        self.define_binding(binding.to_owned(), binding_type, declaration_start);
        if track_reference {
            self.reference_tracker.define_reference(
                binding.to_owned(),
                ReferenceProvenance::local_owner(owner.to_owned(), statements.first().map_or(
                    Span {
                        start: declaration_start,
                        end: declaration_start,
                        line: 1,
                        column: 1,
                    },
                    |statement| statement.span,
                )),
            );
        }
        let result = self.lower_statements(statements);
        let locals = self
            .scopes
            .pop()
            .expect("sequence lookup child scope must be present after lowering");
        for name in locals.keys() {
            self.move_tracker.forget(name);
            self.reference_tracker.forget(name);
        }
        result
    }

    fn block_reassigns_name(statements: &[SyntaxStmt], target: &str) -> bool {
        statements.iter().any(|statement| match &statement.kind {
            SyntaxStmtKind::Bind { name, .. } => name == target,
            SyntaxStmtKind::Repeat { body, .. } => Self::block_reassigns_name(body, target),
            SyntaxStmtKind::If {
                then_body,
                else_body,
                ..
            }
            | SyntaxStmtKind::SequenceLookup {
                then_body,
                else_body,
                ..
            } => {
                Self::block_reassigns_name(then_body, target)
                    || Self::block_reassigns_name(else_body, target)
            }
            SyntaxStmtKind::Match { arms, .. } => arms
                .iter()
                .any(|arm| Self::block_reassigns_name(&arm.body, target)),
            SyntaxStmtKind::Print(_)
            | SyntaxStmtKind::Return(_)
            | SyntaxStmtKind::SequenceAppend { .. } => false,
        })
    }

''')
# Remove the synthetic fallback span above in favor of the lookup statement span propagated by caller.
replace_once(p,
'''                    statement.span.start,
                    then_body,
                )?;''',
'''                    statement.span,
                    then_body,
                )?;''')
replace_once(p,
'''        declaration_start: usize,
        statements: &[SyntaxStmt],''',
'''        lookup_span: Span,
        statements: &[SyntaxStmt],''')
replace_once(p,
'''        self.define_binding(binding.to_owned(), binding_type, declaration_start);
        if track_reference {
            self.reference_tracker.define_reference(
                binding.to_owned(),
                ReferenceProvenance::local_owner(owner.to_owned(), statements.first().map_or(
                    Span {
                        start: declaration_start,
                        end: declaration_start,
                        line: 1,
                        column: 1,
                    },
                    |statement| statement.span,
                )),
            );
        }''',
'''        self.define_binding(binding.to_owned(), binding_type, lookup_span.start);
        if track_reference {
            self.reference_tracker.define_reference(
                binding.to_owned(),
                ReferenceProvenance::local_owner(owner.to_owned(), lookup_span),
            );
        }''')
# Mutability and input/ref collectors recurse through sequence lookup.
replace_once(p,
'''                StmtKind::Assign { .. } | StmtKind::Print(_) | StmtKind::Return(_) => {}''',
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
                | StmtKind::SequenceAppend { .. } => {}''')

# Rust code generation maps directly to Vec<T>, push and checked get.
p = "crates/evo-codegen-rust/src/lib.rs"
replace_once(p,
'''            StmtKind::If {
                condition,
                then_body,
                else_body,
            } => {''',
'''            StmtKind::SequenceAppend { owner, value } => {
                self.push_mapped_line(
                    format!(
                        "{padding}{}.push({});\\n",
                        generated_identifier(owner),
                        render_expr(value)
                    ),
                    statement.span,
                );
            }
            StmtKind::SequenceLookup {
                owner,
                index,
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
                        "{padding}if let Some({pattern}) = usize::try_from({}).ok().and_then(|__evo_lookup_index| {}.get(__evo_lookup_index)) {{\\n",
                        render_expr(index),
                        generated_identifier(owner)
                    ),
                    statement.span,
                );
                for statement in then_body {
                    self.write_statement(statement, indent + 1);
                }
                self.push_mapped_line(format!("{padding}}} else {{\\n"), statement.span);
                for statement in else_body {
                    self.write_statement(statement, indent + 1);
                }
                self.push_mapped_line(format!("{padding}}}\\n"), statement.span);
            }
            StmtKind::If {
                condition,
                then_body,
                else_body,
            } => {''')
replace_once(p,
'''        ValueType::SharedRef(inner) => format!("&{}", rust_type(inner)),
    }''',
'''        ValueType::SharedRef(inner) => format!("&{}", rust_type(inner)),
        ValueType::Sequence(inner) => format!("Vec<{}>", rust_type(inner)),
    }''')
replace_once(p,
'''        ExprKind::SharedDuplicate(inner) => {
            format!("std::rc::Rc::clone(&({}))", render_expr(inner))
        }
        ExprKind::Binary {''',
'''        ExprKind::SharedDuplicate(inner) => {
            format!("std::rc::Rc::clone(&({}))", render_expr(inner))
        }
        ExprKind::SequenceNew { element_type } => {
            format!("Vec::<{}>::new()", rust_type(element_type))
        }
        ExprKind::Binary {''')
replace_once(p,
'''        StmtKind::If {
            condition,
            then_body,
            else_body,
        } => {
            expr_uses_input_int(condition)''',
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
        StmtKind::If {
            condition,
            then_body,
            else_body,
        } => {
            expr_uses_input_int(condition)''')
replace_once(p,
'''        ExprKind::Integer(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::Local(_) => {
            false
        }''',
'''        ExprKind::Integer(_)
        | ExprKind::String(_)
        | ExprKind::Bool(_)
        | ExprKind::Local(_)
        | ExprKind::SequenceNew { .. } => false,''')
