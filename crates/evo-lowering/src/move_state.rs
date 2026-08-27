use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MoveStateError {
    MissingBinding,
    UnavailableBinding,
    TypeMismatch,
    RepeatWouldConsume,
}

#[derive(Debug, Clone)]
struct MoveBinding<T> {
    value_type: T,
    available: bool,
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
                available: true,
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
        if !binding.available {
            return Err(MoveStateError::UnavailableBinding);
        }
        Ok(binding.value_type.clone())
    }

    pub(super) fn consume(
        &mut self,
        name: &str,
        is_reusable: impl FnOnce(&T) -> bool,
    ) -> Result<T, MoveStateError> {
        let binding = self
            .bindings
            .get_mut(name)
            .ok_or(MoveStateError::MissingBinding)?;
        if !binding.available {
            return Err(MoveStateError::UnavailableBinding);
        }
        if !is_reusable(&binding.value_type) {
            binding.available = false;
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
        binding.available = true;
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
            binding.available = exits.iter().all(|exit| {
                let exit_binding = exit
                    .bindings
                    .get(name)
                    .expect("move-state branch is forked from the same visible bindings");
                debug_assert!(binding.value_type == exit_binding.value_type);
                exit_binding.available
            });
        }
        true
    }

    pub(super) fn merge_repeat(
        &mut self,
        body_exit: &Self,
        is_reusable: impl Fn(&T) -> bool,
    ) -> Result<(), MoveStateError> {
        for (name, binding) in &mut self.bindings {
            let body_binding = body_exit
                .bindings
                .get(name)
                .expect("repeat move state is forked from the same visible bindings");
            debug_assert!(binding.value_type == body_binding.value_type);

            if binding.available
                && !body_binding.available
                && !is_reusable(&binding.value_type)
            {
                return Err(MoveStateError::RepeatWouldConsume);
            }

            binding.available = binding.available && body_binding.available;
        }
        Ok(())
    }

    pub(super) fn binding_names(&self) -> impl Iterator<Item = &str> {
        self.bindings.keys().map(String::as_str)
    }

    pub(super) fn is_available(&self, name: &str) -> bool {
        self.bindings
            .get(name)
            .is_some_and(|binding| binding.available)
    }
}

#[cfg(test)]
mod tests {
    use super::{MoveState, MoveStateError};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum ValueType {
        Scalar,
        Nominal(&'static str),
    }

    fn reusable(value_type: &ValueType) -> bool {
        matches!(value_type, ValueType::Scalar)
    }

    #[test]
    fn consume_distinguishes_reusable_and_move_only_values() {
        let mut state = MoveState::default();
        state.define("count".to_owned(), ValueType::Scalar);
        state.define("item".to_owned(), ValueType::Nominal("Item"));

        assert_eq!(state.consume("count", reusable), Ok(ValueType::Scalar));
        assert_eq!(state.consume("count", reusable), Ok(ValueType::Scalar));
        assert_eq!(
            state.consume("item", reusable),
            Ok(ValueType::Nominal("Item"))
        );
        assert_eq!(
            state.consume("item", reusable),
            Err(MoveStateError::UnavailableBinding)
        );
    }

    #[test]
    fn reinitialization_requires_exact_type_and_restores_availability() {
        let mut state = MoveState::default();
        state.define("item".to_owned(), ValueType::Nominal("Item"));
        state
            .consume("item", reusable)
            .expect("first move should succeed");
        assert_eq!(
            state.reinitialize("item", ValueType::Nominal("Other")),
            Err(MoveStateError::TypeMismatch)
        );
        state
            .reinitialize("item", ValueType::Nominal("Item"))
            .expect("same-type reinitialization should restore availability");
        assert_eq!(
            state.consume("item", reusable),
            Ok(ValueType::Nominal("Item"))
        );
    }

    #[test]
    fn continuing_branch_merge_requires_availability_on_every_continuing_exit() {
        let mut entry = MoveState::default();
        entry.define("item".to_owned(), ValueType::Nominal("Item"));

        let mut first = entry.clone();
        first
            .consume("item", reusable)
            .expect("one branch may move item");
        let second = entry.clone();
        let third = entry.clone();

        assert!(entry.merge_continuing([&first, &second, &third]));
        assert_eq!(
            entry.inspect("item"),
            Err(MoveStateError::UnavailableBinding)
        );
    }

    #[test]
    fn terminal_branches_are_omitted_from_continuing_merge() {
        let mut entry = MoveState::default();
        entry.define("item".to_owned(), ValueType::Nominal("Item"));

        let continuing = entry.clone();
        let mut terminal = entry.clone();
        terminal
            .consume("item", reusable)
            .expect("terminal branch may consume item");

        assert!(entry.merge_continuing([&continuing]));
        assert_eq!(
            entry.consume("item", reusable),
            Ok(ValueType::Nominal("Item"))
        );
        assert!(!entry.merge_continuing(std::iter::empty()));
    }

    #[test]
    fn repeat_rejects_a_move_that_breaks_a_later_iteration() {
        let mut entry = MoveState::default();
        entry.define("item".to_owned(), ValueType::Nominal("Item"));
        let mut body_exit = entry.clone();
        body_exit
            .consume("item", reusable)
            .expect("first body iteration may move item");

        assert_eq!(
            entry.merge_repeat(&body_exit, reusable),
            Err(MoveStateError::RepeatWouldConsume)
        );
    }
}
