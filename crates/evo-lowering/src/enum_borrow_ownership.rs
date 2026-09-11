use crate::{LowerError, ParameterPassingMode};
use evo_diagnostics::{clear_related_location, set_related_location};
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
    ownership_state::{MoveReason, MoveState, MoveStateError},
};
use super::ownership::{OwnershipUseMode, ResolvedOwnershipUse};
use super::{EnumTypeEnvironment, resolve_signature_type};

#[derive(Debug)]
struct BranchResult {
    state: MoveState<ResolvedPayloadType>,
    continues: bool,
}

#[derive(Clone)]
struct OwnershipAnalyzer<'a, 'e> {
    environment: &'a EnumTypeEnvironment<'e>,
    matches: &'a MatchEnvironment,
    parameter_modes: &'a HashMap<String, Vec<ParameterPassingMode>>,
    scopes: Vec<HashMap<String, ResolvedPayloadType>>,
    state: MoveState<ResolvedPayloadType>,
    uses: Rc<RefCell<Vec<ResolvedOwnershipUse>>>,
}

impl<'a, 'e> OwnershipAnalyzer<'a, 'e> {
    fn new(
        environment: &'a EnumTypeEnvironment<'e>,
        matches: &'a MatchEnvironment,
        parameter_modes: &'a HashMap<String, Vec<ParameterPassingMode>>,
    ) -> Self {
        Self {
            environment,
            matches,
            parameter_modes,
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
                self.use_expr(expr, OwnershipUseMode::Consume, MoveReason::Direct)?;
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
                self.use_expr(expr, OwnershipUseMode::Inspect, MoveReason::Direct)?;
                Ok(true)
            }
            SyntaxStmtKind::Return(expr) => {
                self.use_expr(expr, OwnershipUseMode::Consume, MoveReason::Return)?;
                Ok(false)
            }
            SyntaxStmtKind::Repeat { count, body } => {
                self.use_expr(count, OwnershipUseMode::Inspect, MoveReason::Direct)?;
                let entry = self.state.clone();
                let body_result = self.run_child(body, None)?;

                if !body_result.continues {
                    self.state = entry;
                    return Ok(true);
                }

                let mut merged = entry;
                match merged.merge_repeat(&body_result.state, is_reusable) {
                    Ok(()) => {
                        self.state = merged;
                        Ok(true)
                    }
                    Err(MoveStateError::RepeatWouldConsume { name, provenance }) => {
                        let kind = nominal_kind(self.visible_type(&name));
                        let message = format!(
                            "{kind} local {name:?} is moved by repeat body and would be unavailable on a later iteration"
                        );
                        set_related_location(
                            &message,
                            statement.span,
                            provenance.reason.note(),
                            provenance.span,
                        );
                        Err(LowerError {
                            message,
                            span: statement.span,
                        })
                    }
                    Err(
                        MoveStateError::MissingBinding
                        | MoveStateError::UnavailableBinding(_)
                        | MoveStateError::TypeMismatch,
                    ) => unreachable!("repeat ownership merge only reports later-iteration moves"),
                }
            }
            SyntaxStmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                self.use_expr(condition, OwnershipUseMode::Inspect, MoveReason::Direct)?;
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
                self.use_expr(value, OwnershipUseMode::Consume, MoveReason::MatchScrutinee)?;
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
        reason: MoveReason,
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
                    OwnershipUseMode::Consume => {
                        self.state.consume(name, expr.span, reason, is_reusable)
                    }
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
            SyntaxExprKind::Call { name, arguments } => {
                let modes = self.parameter_modes.get(name);
                for (index, argument) in arguments.iter().enumerate() {
                    let passing_mode = modes
                        .and_then(|candidate| candidate.get(index))
                        .copied()
                        .unwrap_or(ParameterPassingMode::Owned);
                    let ownership_mode = if passing_mode == ParameterPassingMode::SharedBorrow
                        && matches!(argument.kind, SyntaxExprKind::Identifier(_))
                    {
                        OwnershipUseMode::Inspect
                    } else {
                        OwnershipUseMode::Consume
                    };
                    self.use_expr(argument, ownership_mode, MoveReason::FunctionArgument)?;
                }
                Ok(())
            }
            SyntaxExprKind::Construct { fields, .. } => {
                for field in fields {
                    self.use_expr(&field.value, OwnershipUseMode::Consume, reason)?;
                }
                Ok(())
            }
            SyntaxExprKind::EnumConstruct { arguments, .. } => {
                for argument in arguments {
                    self.use_expr(argument, OwnershipUseMode::Consume, reason)?;
                }
                Ok(())
            }
            SyntaxExprKind::FieldAccess { base, field } => {
                self.use_expr(base, OwnershipUseMode::Inspect, MoveReason::Direct)?;
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
                self.use_expr(inner, OwnershipUseMode::Consume, reason)
            }
            SyntaxExprKind::SharedBorrow(_) => Err(LowerError {
                message: "immutable reference ownership lowering is not implemented yet".to_owned(),
                span: expr.span,
            }),
            SyntaxExprKind::Binary { left, right, .. } => {
                self.use_expr(left, OwnershipUseMode::Consume, reason)?;
                self.use_expr(right, OwnershipUseMode::Consume, reason)
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
            Err(
                MoveStateError::UnavailableBinding(_)
                | MoveStateError::RepeatWouldConsume { .. },
            ) => unreachable!("reinitialization only reports missing bindings or type mismatches"),
        }
    }

    fn read_error(&self, name: &str, span: Span, error: MoveStateError) -> LowerError {
        match error {
            MoveStateError::MissingBinding => LowerError {
                message: format!("use of local {name:?} before definition or outside its scope"),
                span,
            },
            MoveStateError::UnavailableBinding(provenance) => {
                let message = moved_local_message(name, self.visible_type(name));
                set_related_location(
                    &message,
                    span,
                    provenance.reason.note(),
                    provenance.span,
                );
                LowerError { message, span }
            }
            MoveStateError::TypeMismatch | MoveStateError::RepeatWouldConsume { .. } => {
                unreachable!("ownership reads only report missing or unavailable bindings")
            }
        }
    }
}

pub(super) fn collect_enum_ownership(
    program: &SyntaxProgram,
    enums: &EnumEnvironment,
    matches: &MatchEnvironment,
    parameter_modes: &HashMap<String, Vec<ParameterPassingMode>>,
) -> Result<Vec<ResolvedOwnershipUse>, LowerError> {
    clear_related_location();
    let environment = EnumTypeEnvironment::collect(program, enums)?;
    let record_names: HashSet<&str> = program
        .records
        .iter()
        .map(|record| record.name.as_str())
        .collect();
    let mut uses = Vec::new();

    for function in &program.functions {
        let mut analyzer = OwnershipAnalyzer::new(&environment, matches, parameter_modes);
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

    let mut top_level = OwnershipAnalyzer::new(&environment, matches, parameter_modes);
    let _ = top_level.validate_statements(&program.statements)?;
    uses.extend(top_level.uses.borrow().iter().cloned());
    Ok(uses)
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
