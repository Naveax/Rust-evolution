use evo_lexer::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MoveReason {
    Direct,
    FunctionArgument,
    Return,
    MatchScrutinee,
    ControlFlowMerge,
    RepeatBody,
}

impl MoveReason {
    pub(super) const fn note(self) -> &'static str {
        match self {
            Self::Direct => "value was moved here",
            Self::FunctionArgument => "value was moved into this function argument",
            Self::Return => "value was moved by this return",
            Self::MatchScrutinee => "value was moved into this match",
            Self::ControlFlowMerge => "a continuing control-flow path moved the value here",
            Self::RepeatBody => "repeat body moves the value here",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MoveProvenance {
    pub(super) span: Span,
    pub(super) reason: MoveReason,
}

impl MoveProvenance {
    const fn with_reason(self, reason: MoveReason) -> Self {
        Self {
            span: self.span,
            reason,
        }
    }

    const fn source_order_key(self) -> (usize, usize, usize, usize, MoveReason) {
        (
            self.span.start,
            self.span.end,
            self.span.line,
            self.span.column,
            self.reason,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum MoveStateError {
    MissingBinding,
    UnavailableBinding(MoveProvenance),
    TypeMismatch,
    RepeatWouldConsume {
        name: String,
        provenance: MoveProvenance,
    },
}

#[derive(Debug, Clone)]
struct MoveBinding<T> {
    value_type: T,
    provenance: Option<MoveProvenance>,
}

#[derive(Debug, Clone)]
pub(super) struct MoveState<T> {
    bindings: HashMap<String, MoveBinding<T>>,
}

impl<T> Default for MoveState<T> {
    fn default() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }
}

impl<T: Clone + Eq> MoveState<T> {
    pub(super) fn define(&mut self, name: String, value_type: T) {
        let previous = self.bindings.insert(
            name,
            MoveBinding {
                value_type,
                provenance: None,
            },
        );
        debug_assert!(previous.is_none());
    }

    pub(super) fn forget(&mut self, name: &str) {
        let removed = self.bindings.remove(name);
        debug_assert!(removed.is_some());
    }

    pub(super) fn inspect(&self, name: &str) -> Result<T, MoveStateError> {
        let binding = self
            .bindings
            .get(name)
            .ok_or(MoveStateError::MissingBinding)?;
        if let Some(provenance) = binding.provenance {
            return Err(MoveStateError::UnavailableBinding(provenance));
        }
        Ok(binding.value_type.clone())
    }

    pub(super) fn consume(
        &mut self,
        name: &str,
        span: Span,
        reason: MoveReason,
        is_reusable: impl FnOnce(&T) -> bool,
    ) -> Result<T, MoveStateError> {
        let binding = self
            .bindings
            .get_mut(name)
            .ok_or(MoveStateError::MissingBinding)?;
        if let Some(provenance) = binding.provenance {
            return Err(MoveStateError::UnavailableBinding(provenance));
        }
        if !is_reusable(&binding.value_type) {
            binding.provenance = Some(MoveProvenance { span, reason });
        }
        Ok(binding.value_type.clone())
    }

    pub(super) fn reinitialize(
        &mut self,
        name: &str,
        value_type: T,
    ) -> Result<(), MoveStateError> {
        let binding = self
            .bindings
            .get_mut(name)
            .ok_or(MoveStateError::MissingBinding)?;
        if binding.value_type != value_type {
            return Err(MoveStateError::TypeMismatch);
        }
        binding.provenance = None;
        Ok(())
    }

    pub(super) fn merge_continuing<'a>(
        &mut self,
        exits: impl IntoIterator<Item = &'a Self>,
    ) -> bool
    where
        T: 'a,
    {
        let exits: Vec<&Self> = exits.into_iter().collect();
        if exits.is_empty() {
            return false;
        }

        for (name, binding) in &mut self.bindings {
            let mut unavailable = Vec::new();
            for exit in &exits {
                let exit_binding = exit
                    .bindings
                    .get(name)
                    .expect("move-state branch is forked from the same visible bindings");
                debug_assert!(binding.value_type == exit_binding.value_type);
                if let Some(provenance) = exit_binding.provenance {
                    unavailable.push(provenance);
                }
            }

            if unavailable.is_empty() {
                binding.provenance = None;
                continue;
            }

            if let Some(entry_provenance) = binding.provenance
                && unavailable
                    .iter()
                    .all(|provenance| *provenance == entry_provenance)
            {
                binding.provenance = Some(entry_provenance);
                continue;
            }

            let selected = unavailable
                .into_iter()
                .min_by_key(|provenance| provenance.source_order_key())
                .expect("unavailable provenance list is non-empty");
            binding.provenance = Some(selected.with_reason(MoveReason::ControlFlowMerge));
        }
        true
    }

    pub(super) fn merge_repeat(
        &mut self,
        body_exit: &Self,
        is_reusable: impl Fn(&T) -> bool,
    ) -> Result<(), MoveStateError> {
        let violation = self
            .bindings
            .iter()
            .filter_map(|(name, binding)| {
                let body_binding = body_exit
                    .bindings
                    .get(name)
                    .expect("repeat move state is forked from the same visible bindings");
                debug_assert!(binding.value_type == body_binding.value_type);

                if binding.provenance.is_none() && !is_reusable(&binding.value_type) {
                    body_binding
                        .provenance
                        .map(|provenance| (name.as_str(), provenance))
                } else {
                    None
                }
            })
            .min_by(|(left_name, left), (right_name, right)| {
                left.source_order_key()
                    .cmp(&right.source_order_key())
                    .then_with(|| left_name.cmp(right_name))
            });

        if let Some((name, provenance)) = violation {
            return Err(MoveStateError::RepeatWouldConsume {
                name: name.to_owned(),
                provenance: provenance.with_reason(MoveReason::RepeatBody),
            });
        }

        for (name, binding) in &mut self.bindings {
            let body_binding = body_exit
                .bindings
                .get(name)
                .expect("repeat move state is forked from the same visible bindings");
            debug_assert!(binding.value_type == body_binding.value_type);

            // A repeat may execute zero times. A value already unavailable on entry therefore
            // remains unavailable even when the body would reinitialize it.
            if binding.provenance.is_none() {
                binding.provenance = body_binding.provenance;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{MoveReason, MoveState, MoveStateError};
    use evo_lexer::Span;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum ValueType {
        Scalar,
        Nominal(&'static str),
    }

    fn reusable(value_type: &ValueType) -> bool {
        matches!(value_type, ValueType::Scalar)
    }

    const fn span(line: usize) -> Span {
        Span {
            start: line * 10,
            end: line * 10 + 1,
            line,
            column: 1,
        }
    }

    #[test]
    fn consume_distinguishes_reusable_values_and_retains_move_provenance() {
        let mut state = MoveState::default();
        state.define("count".to_owned(), ValueType::Scalar);
        state.define("item".to_owned(), ValueType::Nominal("Item"));

        assert_eq!(
            state.consume("count", span(1), MoveReason::Direct, reusable),
            Ok(ValueType::Scalar)
        );
        assert_eq!(
            state.consume("count", span(2), MoveReason::Direct, reusable),
            Ok(ValueType::Scalar)
        );
        assert_eq!(
            state.consume("item", span(3), MoveReason::Direct, reusable),
            Ok(ValueType::Nominal("Item"))
        );

        let error = state
            .consume("item", span(4), MoveReason::Direct, reusable)
            .expect_err("second move-only consume must fail");
        let MoveStateError::UnavailableBinding(provenance) = error else {
            panic!("expected unavailable binding");
        };
        assert_eq!(provenance.span, span(3));
        assert_eq!(provenance.reason, MoveReason::Direct);
    }

    #[test]
    fn reinitialization_requires_exact_type_and_clears_stale_provenance() {
        let mut state = MoveState::default();
        state.define("item".to_owned(), ValueType::Nominal("Item"));
        state
            .consume("item", span(1), MoveReason::Direct, reusable)
            .expect("first move should succeed");
        assert_eq!(
            state.reinitialize("item", ValueType::Nominal("Other")),
            Err(MoveStateError::TypeMismatch)
        );
        state
            .reinitialize("item", ValueType::Nominal("Item"))
            .expect("same-type reinitialization should restore availability");
        state
            .consume("item", span(5), MoveReason::Return, reusable)
            .expect("move after reinitialization should succeed");
        let error = state.inspect("item").expect_err("item was moved again");
        let MoveStateError::UnavailableBinding(provenance) = error else {
            panic!("expected unavailable binding");
        };
        assert_eq!(provenance.span, span(5));
        assert_eq!(provenance.reason, MoveReason::Return);
    }

    #[test]
    fn continuing_merge_chooses_deterministic_source_order_provenance() {
        let mut entry = MoveState::default();
        entry.define("item".to_owned(), ValueType::Nominal("Item"));

        let mut later = entry.clone();
        later
            .consume("item", span(7), MoveReason::Direct, reusable)
            .expect("later branch may move item");
        let mut earlier = entry.clone();
        earlier
            .consume("item", span(3), MoveReason::FunctionArgument, reusable)
            .expect("earlier branch may move item");

        assert!(entry.merge_continuing([&later, &earlier]));
        let error = entry.inspect("item").expect_err("merged item must be unavailable");
        let MoveStateError::UnavailableBinding(provenance) = error else {
            panic!("expected unavailable binding");
        };
        assert_eq!(provenance.span, span(3));
        assert_eq!(provenance.reason, MoveReason::ControlFlowMerge);
    }

    #[test]
    fn terminal_branches_are_omitted_from_continuing_merge() {
        let mut entry = MoveState::default();
        entry.define("item".to_owned(), ValueType::Nominal("Item"));

        let continuing = entry.clone();
        let mut terminal = entry.clone();
        terminal
            .consume("item", span(2), MoveReason::Direct, reusable)
            .expect("terminal branch may consume item");

        assert!(entry.merge_continuing([&continuing]));
        assert_eq!(
            entry.consume("item", span(3), MoveReason::Direct, reusable),
            Ok(ValueType::Nominal("Item"))
        );
        assert!(!entry.merge_continuing(std::iter::empty()));
    }

    #[test]
    fn repeat_reports_body_move_with_binding_and_repeat_reason() {
        let mut entry = MoveState::default();
        entry.define("item".to_owned(), ValueType::Nominal("Item"));
        let mut body_exit = entry.clone();
        body_exit
            .consume("item", span(2), MoveReason::Direct, reusable)
            .expect("first body iteration may move item");

        let error = entry
            .merge_repeat(&body_exit, reusable)
            .expect_err("later repeat iteration would observe the move");
        let MoveStateError::RepeatWouldConsume { name, provenance } = error else {
            panic!("expected repeat move error");
        };
        assert_eq!(name, "item");
        assert_eq!(provenance.span, span(2));
        assert_eq!(provenance.reason, MoveReason::RepeatBody);
    }

    #[test]
    fn repeat_violation_selection_is_deterministic_by_source_then_binding_name() {
        let mut entry = MoveState::default();
        entry.define("later".to_owned(), ValueType::Nominal("Item"));
        entry.define("earlier".to_owned(), ValueType::Nominal("Item"));

        let mut body_exit = entry.clone();
        body_exit
            .consume("later", span(8), MoveReason::Direct, reusable)
            .expect("later value may move");
        body_exit
            .consume("earlier", span(4), MoveReason::Direct, reusable)
            .expect("earlier value may move");

        let error = entry
            .merge_repeat(&body_exit, reusable)
            .expect_err("repeat must report one deterministic violation");
        let MoveStateError::RepeatWouldConsume { name, provenance } = error else {
            panic!("expected repeat move error");
        };
        assert_eq!(name, "earlier");
        assert_eq!(provenance.span, span(4));
        assert_eq!(provenance.reason, MoveReason::RepeatBody);
    }
}
