use crate::LowerError;
use evo_lexer::Span;
use evo_parser::{
    Expr as SyntaxExpr, ExprKind as SyntaxExprKind, Program as SyntaxProgram, Stmt as SyntaxStmt,
    StmtKind as SyntaxStmtKind,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::super::{
    EnumEnvironment, ResolvedPayloadType,
    match_validation::{MatchEnvironment, ResolvedMatchBinding},
    ownership_state::{MoveState, MoveStateError},
};
use super::{EnumTypeEnvironment, resolve_signature_type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnershipUseMode {
    Inspect,
    Consume,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedOwnershipUse {
    pub(crate) name: String,
    pub(crate) value_type: ResolvedPayloadType,
    pub(crate) mode: OwnershipUseMode,
    pub(crate) span: Span,
}

#[derive(Debug)]
struct BranchResult {
    state: MoveState<ResolvedPayloadType>,
    continues: bool,
}

#[derive(Clone)]
struct OwnershipAnalyzer<'a, 'e> {
    environment: &'a EnumTypeEnvironment<'e>,
    matches: &'a MatchEnvironment,
    scopes: Vec<HashMap<String, ResolvedPayloadType>>,
    state: MoveState<ResolvedPayloadType>,
    uses: Rc<RefCell<Vec<ResolvedOwnershipUse>>>,
}

impl<'a, 'e> OwnershipAnalyzer<'a, 'e> {
    fn new(environment: &'a EnumTypeEnvironment<'e>, matches: &'a MatchEnvironment) -> Self {
        Self {
            environment,
            matches,
            scopes: vec![HashMap::new()],
            state: MoveState::default(),
            uses: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn visible_type(&self, name: &str) -> Option<&ResolvedPayloadType> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    fn define_new(&mut self, name: String, value_type: ResolvedPayloadType) {
        self.state.define(name.clone(), value_type.clone());
        self.scopes
            .last_mut()
            .expect("enum ownership always has a lexical scope")
            .insert(name, value_type);
    }

    fn validate_statements(&mut self, statements: &[SyntaxStmt]) -> Result<bool, LowerError> {
        for statement in statements {
            if !self.validate_statement(statement)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn validate_statement(&mut self, statement: &SyntaxStmt) -> Result<bool, LowerError> {
        match &statement.kind {
            SyntaxStmtKind::Bind { name, expr } => {
                let inferred = self.environment.infer_expr(expr, &self.scopes)?;
                self.use_expr(expr, OwnershipUseMode::Consume)?;
                if let Some(value_type) = inferred {
                    if self.visible_type(name).is_some() {
                        self.reinitialize(name, value_type, statement.span)?;
                    } else {
                        self.define_new(name.clone(), value_type);
                    }
                }
                Ok(true)
            }
            SyntaxStmtKind::Print(expr) => {
                self.use_expr(expr, OwnershipUseMode::Inspect)?;
                Ok(true)
            }
            SyntaxStmtKind::Return(expr) => {
                self.use_expr(expr, OwnershipUseMode::Consume)?;
                Ok(false)
            }
            SyntaxStmtKind::Repeat { count, body } => {
                self.use_expr(count, OwnershipUseMode::Inspect)?;
                let entry = self.state.clone();
                let body_result = self.run_child(body, None)?;

                // A terminal body never reaches a later iteration. The only conservative
                // continuation after repeat is the zero-iteration path, so retain entry state.
                if !body_result.continues {
                    self.state = entry;
                    return Ok(true);
                }

                let mut merged = entry.clone();
                match merged.merge_repeat(&body_result.state, is_reusable) {
                    Ok(()) => {
                        self.state = merged;
                        Ok(true)
                    }
                    Err(MoveStateError::RepeatWouldConsume) => Err(self.repeat_move_error(
                        &entry,
                        &body_result.state,
                        statement.span,
                    )),
                    Err(
                        MoveStateError::MissingBinding
                        | MoveStateError::UnavailableBinding
                        | MoveStateError::TypeMismatch,
                    ) => unreachable!("repeat ownership merge only reports later-iteration moves"),
                }
            }
            SyntaxStmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                self.use_expr(condition, OwnershipUseMode::Inspect)?;
                let then_result = self.run_child(then_body, None)?;
                let else_result = self.run_child(else_body, None)?;
                let mut merged = self.state.clone();
                let mut exits = Vec::with_capacity(2);
                if then_result.continues {
                    exits.push(&then_result.state);
                }
                if else_result.continues {
                    exits.push(&else_result.state);
                }
                let continues = merged.merge_continuing(exits);
                if continues {
                    self.state = merged;
                }
                Ok(continues)
            }
            SyntaxStmtKind::Match { value, arms } => {
                self.use_expr(value, OwnershipUseMode::Consume)?;
                let entry = self.state.clone();
                let resolved = self
                    .matches
                    .match_at(statement.span.start)
                    .expect("match ownership runs after resolved exhaustive match validation");
                debug_assert_eq!(resolved.arms.len(), arms.len());

                let mut arm_results = Vec::with_capacity(arms.len());
                for (arm, resolved_arm) in arms.iter().zip(&resolved.arms) {
                    arm_results.push(self.run_child(&arm.body, resolved_arm.binding.as_ref())?);
                }

                let mut merged = entry;
                let continues = merged.merge_continuing(
                    arm_results
                        .iter()
                        .filter(|result| result.continues)
                        .map(|result| &result.state),
                );
                debug_assert_eq!(!continues, resolved.all_arms_return);
                if continues {
                    self.state = merged;
                }
                Ok(continues)
            }
        }
    }

    fn run_child(
        &self,
        statements: &[SyntaxStmt],
        binding: Option<&ResolvedMatchBinding>,
    ) -> Result<BranchResult, LowerError> {
        let mut child = self.clone();
        child.scopes.push(HashMap::new());
        if let Some(binding) = binding {
            debug_assert!(child.visible_type(&binding.name).is_none());
            child.define_new(binding.name.clone(), binding.value_type.clone());
        }

        let continues = child.validate_statements(statements)?;
        let local_names: Vec<String> = child
            .scopes
            .last()
            .expect("child scope must exist while validating ownership")
            .keys()
            .cloned()
            .collect();
        for name in local_names {
            child.state.forget(&name);
        }
        child.scopes.pop();

        Ok(BranchResult {
            state: child.state,
            continues,
        })
    }

    fn use_expr(
        &mut self,
        expr: &SyntaxExpr,
        mode: OwnershipUseMode,
    ) -> Result<(), LowerError> {
        match &expr.kind {
            SyntaxExprKind::Integer(_)
            | SyntaxExprKind::String(_)
            | SyntaxExprKind::Bool(_)
            | SyntaxExprKind::InputInt => Ok(()),
            SyntaxExprKind::Identifier(name) => {
                let value_type = self.visible_type(name).cloned();
                let result = match mode {
                    OwnershipUseMode::Inspect => self.state.inspect(name),
                    OwnershipUseMode::Consume => self.state.consume(name, is_reusable),
                };
                match result {
                    Ok(_) => {
                        self.uses.borrow_mut().push(ResolvedOwnershipUse {
                            name: name.clone(),
                            value_type: value_type
                                .expect("successful ownership read must have a visible type"),
                            mode,
                            span: expr.span,
                        });
                        Ok(())
                    }
                    Err(error) => Err(self.read_error(name, expr.span, error)),
                }
            }
            SyntaxExprKind::Call { arguments, .. } => {
                for argument in arguments {
                    self.use_expr(argument, OwnershipUseMode::Consume)?;
                }
                Ok(())
            }
            SyntaxExprKind::Construct { fields, .. } => {
                for field in fields {
                    self.use_expr(&field.value, OwnershipUseMode::Consume)?;
                }
                Ok(())
            }
            SyntaxExprKind::EnumConstruct { arguments, .. } => {
                for argument in arguments {
                    self.use_expr(argument, OwnershipUseMode::Consume)?;
                }
                Ok(())
            }
            SyntaxExprKind::FieldAccess { base, field } => {
                self.use_expr(base, OwnershipUseMode::Inspect)?;
                if mode == OwnershipUseMode::Consume
                    && let Some(value_type) = self.environment.infer_expr(expr, &self.scopes)?
                    && !is_reusable(&value_type)
                {
                    return Err(LowerError {
                        message: format!(
                            "moving nominal field {field:?} out of an expression is not supported in Enums v0 ownership; no implicit clone is inserted"
                        ),
                        span: expr.span,
                    });
                }
                Ok(())
            }
            SyntaxExprKind::LogicalNot(inner) | SyntaxExprKind::UnaryMinus(inner) => {
                self.use_expr(inner, OwnershipUseMode::Consume)
            }
            SyntaxExprKind::Binary { left, right, .. } => {
                self.use_expr(left, OwnershipUseMode::Consume)?;
                self.use_expr(right, OwnershipUseMode::Consume)
            }
        }
    }

    fn reinitialize(
        &mut self,
        name: &str,
        value_type: ResolvedPayloadType,
        span: Span,
    ) -> Result<(), LowerError> {
        match self.state.reinitialize(name, value_type) {
            Ok(()) => Ok(()),
            Err(MoveStateError::MissingBinding) => Err(LowerError {
                message: format!("assignment to local {name:?} before definition"),
                span,
            }),
            Err(MoveStateError::TypeMismatch) => Err(LowerError {
                message: format!("cannot assign a different value type to existing local {name:?}"),
                span,
            }),
            Err(MoveStateError::UnavailableBinding | MoveStateError::RepeatWouldConsume) => {
                unreachable!("reinitialization only reports missing bindings or type mismatches")
            }
        }
    }

    fn read_error(&self, name: &str, span: Span, error: MoveStateError) -> LowerError {
        match error {
            MoveStateError::MissingBinding => LowerError {
                message: format!("use of local {name:?} before definition or outside its scope"),
                span,
            },
            MoveStateError::UnavailableBinding => LowerError {
                message: moved_local_message(name, self.visible_type(name)),
                span,
            },
            MoveStateError::TypeMismatch | MoveStateError::RepeatWouldConsume => {
                unreachable!("ownership reads only report missing or unavailable bindings")
            }
        }
    }

    fn repeat_move_error(
        &self,
        entry: &MoveState<ResolvedPayloadType>,
        body_exit: &MoveState<ResolvedPayloadType>,
        span: Span,
    ) -> LowerError {
        for name in entry.binding_names() {
            if entry.is_available(name) && !body_exit.is_available(name) {
                let kind = nominal_kind(self.visible_type(name));
                return LowerError {
                    message: format!(
                        "{kind} local {name:?} is moved by repeat body and would be unavailable on a later iteration"
                    ),
                    span,
                };
            }
        }
        unreachable!("repeat ownership merge reported an availability regression")
    }
}

pub(super) fn collect_enum_ownership(
    program: &SyntaxProgram,
    enums: &EnumEnvironment,
    matches: &MatchEnvironment,
) -> Result<Vec<ResolvedOwnershipUse>, LowerError> {
    let environment = EnumTypeEnvironment::collect(program, enums)?;
    let record_names: HashSet<&str> = program
        .records
        .iter()
        .map(|record| record.name.as_str())
        .collect();
    let mut uses = Vec::new();

    for function in &program.functions {
        let mut analyzer = OwnershipAnalyzer::new(&environment, matches);
        for parameter in &function.parameters {
            let value_type = resolve_signature_type(
                &parameter.type_name,
                &record_names,
                enums,
                parameter.span,
            )?;
            analyzer.define_new(parameter.name.clone(), value_type);
        }
        let _ = analyzer.validate_statements(&function.body)?;
        uses.extend(analyzer.uses.borrow().iter().cloned());
    }

    let mut top_level = OwnershipAnalyzer::new(&environment, matches);
    let _ = top_level.validate_statements(&program.statements)?;
    uses.extend(top_level.uses.borrow().iter().cloned());
    Ok(uses)
}

pub(super) fn validate_enum_ownership(
    program: &SyntaxProgram,
    enums: &EnumEnvironment,
    matches: &MatchEnvironment,
) -> Result<(), LowerError> {
    collect_enum_ownership(program, enums, matches).map(|_| ())
}

fn is_reusable(value_type: &ResolvedPayloadType) -> bool {
    matches!(
        value_type,
        ResolvedPayloadType::Integer | ResolvedPayloadType::Bool | ResolvedPayloadType::String
    )
}

fn nominal_kind(value_type: Option<&ResolvedPayloadType>) -> &'static str {
    match value_type {
        Some(ResolvedPayloadType::Enum(_)) => "enum",
        Some(ResolvedPayloadType::Record(_)) => "record",
        Some(
            ResolvedPayloadType::Integer | ResolvedPayloadType::Bool | ResolvedPayloadType::String,
        )
        | None => "move-only",
    }
}

fn moved_local_message(name: &str, value_type: Option<&ResolvedPayloadType>) -> String {
    format!("use of moved {} local {name:?}", nominal_kind(value_type))
}

#[cfg(test)]
mod tests {
    use super::super::super::{
        collect_enum_environment, match_validation::collect_match_environment,
    };
    use super::super::validate_enum_type_semantics;
    use super::{OwnershipUseMode, collect_enum_ownership, validate_enum_ownership};
    use evo_lexer::lex;
    use evo_parser::parse;

    fn validate(source: &str) -> Result<(), crate::LowerError> {
        let tokens = lex(source).expect("enum ownership source should lex");
        let program = parse(&tokens).expect("enum ownership source should parse");
        let enums = collect_enum_environment(&program)?;
        let matches = collect_match_environment(&program, &enums)?;
        validate_enum_type_semantics(&program, &enums)?;
        validate_enum_ownership(&program, &enums, &matches)
    }

    #[test]
    fn ownership_sidecar_records_validated_use_modes_types_and_spans() {
        let source = "enum Flag\nOff\nOn\nend\nn = 1\nvalue = Flag.On()\nprint n\nmoved = value\n";
        let tokens = lex(source).expect("ownership sidecar source should lex");
        let program = parse(&tokens).expect("ownership sidecar source should parse");
        let enums = collect_enum_environment(&program).expect("enum environment should resolve");
        let matches = collect_match_environment(&program, &enums)
            .expect("ownership sidecar source should resolve matches");
        validate_enum_type_semantics(&program, &enums)
            .expect("ownership sidecar source should type-check");
        let uses = collect_enum_ownership(&program, &enums, &matches)
            .expect("ownership sidecar source should validate ownership");

        assert_eq!(uses.len(), 2);
        assert_eq!(uses[0].name, "n");
        assert_eq!(uses[0].value_type, super::ResolvedPayloadType::Integer);
        assert_eq!(uses[0].mode, OwnershipUseMode::Inspect);
        assert_eq!(uses[0].span.line, 7);

        assert_eq!(uses[1].name, "value");
        assert_eq!(
            uses[1].value_type,
            super::ResolvedPayloadType::Enum("Flag".to_owned())
        );
        assert_eq!(uses[1].mode, OwnershipUseMode::Consume);
        assert_eq!(uses[1].span.line, 8);
    }

    #[test]
    fn enum_local_is_move_only_even_with_scalar_payload() {
        let error = validate(
            "enum MaybeInt\nNone\nSome int\nend\nvalue = MaybeInt.Some(1)\nfirst = value\nsecond = value\n",
        )
        .expect_err("second enum read should observe a move");
        assert!(error.message.contains("moved enum local \"value\""));
        assert_eq!(error.span.line, 7);
    }

    #[test]
    fn same_type_reinitialization_restores_enum_availability() {
        validate(
            "enum MaybeInt\nNone\nSome int\nend\nvalue = MaybeInt.Some(1)\nfirst = value\nvalue = MaybeInt.None()\nsecond = value\n",
        )
        .expect("same-type enum reinitialization should restore availability");
    }

    #[test]
    fn enum_function_argument_is_consumed_by_value() {
        let error = validate(
            "enum MaybeInt\nNone\nSome int\nend\nfn take(value MaybeInt) int\nreturn 0\nend\nvalue = MaybeInt.Some(1)\nfirst = take(value)\nsecond = take(value)\n",
        )
        .expect_err("enum argument should move into the first call");
        assert!(error.message.contains("moved enum local \"value\""));
        assert_eq!(error.span.line, 10);
    }

    #[test]
    fn exhaustive_match_consumes_owned_enum_scrutinee() {
        let error = validate(
            "enum MaybeInt\nNone\nSome int\nend\nvalue = MaybeInt.Some(1)\nmatch value\ncase MaybeInt.None\nprint 0\ncase MaybeInt.Some(x)\nprint x\nend\nagain = value\n",
        )
        .expect_err("whole enum should be unavailable after an owned match");
        assert!(error.message.contains("moved enum local \"value\""));
        assert_eq!(error.span.line, 12);
    }

    #[test]
    fn enum_payload_binding_is_move_only_inside_arm() {
        let error = validate(
            "enum Inner\nA\nend\nenum Wrapped\nNone\nSome Inner\nend\nvalue = Wrapped.Some(Inner.A())\nmatch value\ncase Wrapped.None\nprint 0\ncase Wrapped.Some(x)\nfirst = x\nsecond = x\nend\n",
        )
        .expect_err("enum payload binding should move on first by-value read");
        assert!(error.message.contains("moved enum local \"x\""));
        assert_eq!(error.span.line, 14);
    }

    #[test]
    fn record_payload_binding_is_move_only_inside_arm() {
        let error = validate(
            "record Item\nvalue int\nend\nenum Wrapped\nNone\nSome Item\nend\nfn use(value Wrapped) int\nmatch value\ncase Wrapped.None\nreturn 0\ncase Wrapped.Some(x)\nfirst = x\nsecond = x\nreturn 1\nend\nend\n",
        )
        .expect_err("record payload binding should move on first by-value read");
        assert!(error.message.contains("moved record local \"x\""));
        assert_eq!(error.span.line, 14);
    }

    #[test]
    fn scalar_payload_binding_remains_reusable() {
        validate(
            "enum MaybeInt\nNone\nSome int\nend\nvalue = MaybeInt.Some(1)\nmatch value\ncase MaybeInt.None\nprint 0\ncase MaybeInt.Some(x)\nfirst = MaybeInt.Some(x)\nsecond = MaybeInt.Some(x)\nend\n",
        )
        .expect("scalar payload binding should remain trivially reusable");
    }

    #[test]
    fn if_join_preserves_move_from_one_continuing_branch() {
        let error = validate(
            "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nif true\nfirst = value\nelse\nprint 0\nend\nsecond = value\n",
        )
        .expect_err("move on one continuing branch should poison the join");
        assert!(error.message.contains("moved enum local \"value\""));
        assert_eq!(error.span.line, 11);
    }

    #[test]
    fn terminal_if_branch_does_not_poison_continuing_ownership_state() {
        validate(
            "enum Flag\nOff\nOn\nend\nfn use(value Flag) int\nif true\nfirst = value\nreturn 1\nelse\nprint 0\nend\nsecond = value\nreturn 0\nend\n",
        )
        .expect("terminal branch move should not poison the continuing branch");
    }

    #[test]
    fn terminal_match_arm_does_not_poison_continuing_arm_state() {
        validate(
            "enum Flag\nOff\nOn\nend\nfn use(selector Flag, value Flag) int\nmatch selector\ncase Flag.Off\nfirst = value\nreturn 1\ncase Flag.On\nprint 0\nend\nsecond = value\nreturn 0\nend\n",
        )
        .expect("terminal match arm move should not poison the continuing arm");
    }

    #[test]
    fn repeat_rejects_enum_move_that_breaks_later_iteration() {
        let error = validate(
            "enum Flag\nOff\nOn\nend\nvalue = Flag.On()\nrepeat 2\nfirst = value\nend\n",
        )
        .expect_err("second repeat iteration would observe a moved enum");
        assert!(error.message.contains("enum local \"value\""));
        assert!(error.message.contains("later iteration"));
        assert_eq!(error.span.line, 6);
    }

    #[test]
    fn terminal_repeat_body_keeps_zero_iteration_continuation_state() {
        validate(
            "enum Flag\nOff\nOn\nend\nfn use(value Flag) int\nrepeat 2\nfirst = value\nreturn 1\nend\nsecond = value\nreturn 0\nend\n",
        )
        .expect("terminal repeat body must not poison the zero-iteration continuation");
    }
}
