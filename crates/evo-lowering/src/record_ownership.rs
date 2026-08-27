use crate::{
    LowerError,
    record_environment::{RecordEnvironment, SemanticType},
};
use evo_lexer::Span;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct MoveBinding {
    value_type: SemanticType,
    available: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MoveTracker {
    bindings: HashMap<String, MoveBinding>,
}

impl MoveTracker {
    pub(crate) fn define(&mut self, name: String, value_type: SemanticType) {
        self.bindings.insert(
            name,
            MoveBinding {
                value_type,
                available: true,
            },
        );
    }

    pub(crate) fn consume_value(
        &mut self,
        name: &str,
        span: Span,
    ) -> Result<SemanticType, LowerError> {
        let binding = self.bindings.get_mut(name).ok_or_else(|| LowerError {
            message: format!("use of local {name:?} before definition or outside its scope"),
            span,
        })?;

        if !binding.available {
            return Err(LowerError {
                message: format!("use of moved record local {name:?}"),
                span,
            });
        }

        if !binding.value_type.is_trivially_reusable_v0() {
            binding.available = false;
        }
        Ok(binding.value_type.clone())
    }

    pub(crate) fn reinitialize(
        &mut self,
        name: &str,
        value_type: SemanticType,
        span: Span,
    ) -> Result<(), LowerError> {
        let binding = self.bindings.get_mut(name).ok_or_else(|| LowerError {
            message: format!("assignment to local {name:?} before definition"),
            span,
        })?;
        if binding.value_type != value_type {
            return Err(LowerError {
                message: format!("cannot assign a different value type to existing local {name:?}"),
                span,
            });
        }
        binding.available = true;
        Ok(())
    }

    pub(crate) fn merge_if(&mut self, then_exit: &Self, else_exit: &Self) {
        let has_continuation = self.merge_if_continuing(Some(then_exit), Some(else_exit));
        debug_assert!(has_continuation);
    }

    pub(crate) fn merge_if_continuing(
        &mut self,
        then_exit: Option<&Self>,
        else_exit: Option<&Self>,
    ) -> bool {
        match (then_exit, else_exit) {
            (None, None) => false,
            (Some(exit), None) | (None, Some(exit)) => {
                for (name, binding) in &mut self.bindings {
                    let exit_binding = exit
                        .bindings
                        .get(name)
                        .expect("branch tracker is forked from the same visible bindings");
                    debug_assert_eq!(binding.value_type, exit_binding.value_type);
                    binding.available = exit_binding.available;
                }
                true
            }
            (Some(then_exit), Some(else_exit)) => {
                for (name, binding) in &mut self.bindings {
                    let then_binding = then_exit
                        .bindings
                        .get(name)
                        .expect("branch trackers are forked from the same visible bindings");
                    let else_binding = else_exit
                        .bindings
                        .get(name)
                        .expect("branch trackers are forked from the same visible bindings");
                    debug_assert_eq!(binding.value_type, then_binding.value_type);
                    debug_assert_eq!(binding.value_type, else_binding.value_type);
                    binding.available = then_binding.available && else_binding.available;
                }
                true
            }
        }
    }

    pub(crate) fn merge_repeat(&mut self, body_exit: &Self, span: Span) -> Result<(), LowerError> {
        for (name, binding) in &mut self.bindings {
            let body_binding = body_exit
                .bindings
                .get(name)
                .expect("repeat tracker is forked from the same visible bindings");
            debug_assert_eq!(binding.value_type, body_binding.value_type);

            if binding.available
                && !body_binding.available
                && !binding.value_type.is_trivially_reusable_v0()
            {
                return Err(LowerError {
                    message: format!(
                        "record local {name:?} is moved by repeat body and would be unavailable on a later iteration"
                    ),
                    span,
                });
            }

            binding.available = binding.available && body_binding.available;
        }
        Ok(())
    }

    pub(crate) fn access_field(
        &self,
        records: &RecordEnvironment,
        base_name: &str,
        field_name: &str,
        span: Span,
    ) -> Result<SemanticType, LowerError> {
        let binding = self.bindings.get(base_name).ok_or_else(|| LowerError {
            message: format!("use of local {base_name:?} before definition or outside its scope"),
            span,
        })?;
        if !binding.available {
            return Err(LowerError {
                message: format!("use of moved record local {base_name:?}"),
                span,
            });
        }

        let field_type = records.field_type(&binding.value_type, field_name, span)?;
        if !field_type.is_trivially_reusable_v0() {
            return Err(LowerError {
                message: format!(
                    "moving non-reusable record field {field_name:?} out of local {base_name:?} is not supported in Records v0; no implicit clone is inserted"
                ),
                span,
            });
        }
        Ok(field_type)
    }
}

#[cfg(test)]
mod tests {
    use super::MoveTracker;
    use crate::record_environment::{SemanticType, collect_record_environment};
    use evo_lexer::{Span, lex};
    use evo_parser::parse;

    fn span(line: usize) -> Span {
        Span {
            start: 0,
            end: 1,
            line,
            column: 1,
        }
    }

    #[test]
    fn scalar_values_remain_reusable_but_records_move() {
        let mut tracker = MoveTracker::default();
        tracker.define("count".to_owned(), SemanticType::Integer);
        tracker.define("point".to_owned(), SemanticType::Record("Point".to_owned()));

        tracker
            .consume_value("count", span(1))
            .expect("first scalar read");
        tracker
            .consume_value("count", span(2))
            .expect("scalar reuse");
        tracker
            .consume_value("point", span(3))
            .expect("first record move");
        let error = tracker
            .consume_value("point", span(4))
            .expect_err("record reuse after move must fail");
        assert!(error.message.contains("moved record local"));
        assert_eq!(error.span.line, 4);
    }

    #[test]
    fn explicit_reinitialization_restores_moved_record_local() {
        let mut tracker = MoveTracker::default();
        let point = SemanticType::Record("Point".to_owned());
        tracker.define("point".to_owned(), point.clone());
        tracker.consume_value("point", span(1)).expect("first move");
        tracker
            .reinitialize("point", point, span(2))
            .expect("same-type reinitialization should restore availability");
        tracker
            .consume_value("point", span(3))
            .expect("move after reinitialization");
    }

    #[test]
    fn if_merge_requires_record_to_be_available_on_both_paths() {
        let mut entry = MoveTracker::default();
        entry.define("point".to_owned(), SemanticType::Record("Point".to_owned()));

        let mut then_exit = entry.clone();
        then_exit
            .consume_value("point", span(2))
            .expect("then branch may move point");
        let else_exit = entry.clone();

        entry.merge_if(&then_exit, &else_exit);
        let error = entry
            .consume_value("point", span(3))
            .expect_err("move on either branch must make merged value unavailable");
        assert!(error.message.contains("moved record local"));
    }

    #[test]
    fn if_merge_ignores_terminal_branch_state() {
        let mut entry = MoveTracker::default();
        entry.define("point".to_owned(), SemanticType::Record("Point".to_owned()));

        let mut terminal_then = entry.clone();
        terminal_then
            .consume_value("point", span(2))
            .expect("terminal branch may consume point before returning");
        let continuing_else = entry.clone();

        assert!(entry.merge_if_continuing(None, Some(&continuing_else)));
        entry
            .consume_value("point", span(3))
            .expect("terminal branch must not poison continuing state");
    }

    #[test]
    fn if_merge_preserves_move_on_only_continuing_branch() {
        let mut entry = MoveTracker::default();
        entry.define("point".to_owned(), SemanticType::Record("Point".to_owned()));

        let mut continuing_then = entry.clone();
        continuing_then
            .consume_value("point", span(2))
            .expect("continuing branch moves point");

        assert!(entry.merge_if_continuing(Some(&continuing_then), None));
        let error = entry
            .consume_value("point", span(3))
            .expect_err("move on the only continuing branch must remain visible");
        assert!(error.message.contains("moved record local"));
    }

    #[test]
    fn if_merge_reports_no_state_when_both_branches_terminate() {
        let mut entry = MoveTracker::default();
        entry.define("point".to_owned(), SemanticType::Record("Point".to_owned()));
        assert!(!entry.merge_if_continuing(None, None));
    }

    #[test]
    fn if_merge_accepts_definite_reinitialization_on_both_paths() {
        let point = SemanticType::Record("Point".to_owned());
        let mut entry = MoveTracker::default();
        entry.define("point".to_owned(), point.clone());
        entry
            .consume_value("point", span(1))
            .expect("point starts moved before branch");

        let mut then_exit = entry.clone();
        then_exit
            .reinitialize("point", point.clone(), span(2))
            .expect("then branch reinitializes point");
        let mut else_exit = entry.clone();
        else_exit
            .reinitialize("point", point, span(3))
            .expect("else branch reinitializes point");

        entry.merge_if(&then_exit, &else_exit);
        entry
            .consume_value("point", span(4))
            .expect("both branches definitely restore point");
    }

    #[test]
    fn branch_local_bindings_do_not_leak_through_merge() {
        let mut entry = MoveTracker::default();
        entry.define("point".to_owned(), SemanticType::Record("Point".to_owned()));

        let mut then_exit = entry.clone();
        then_exit.define("temporary".to_owned(), SemanticType::Integer);
        let else_exit = entry.clone();
        entry.merge_if(&then_exit, &else_exit);

        let error = entry
            .consume_value("temporary", span(5))
            .expect_err("branch-local binding must not escape merge");
        assert!(error.message.contains("outside its scope"));
    }

    #[test]
    fn repeat_rejects_record_move_that_breaks_later_iterations() {
        let mut entry = MoveTracker::default();
        entry.define("point".to_owned(), SemanticType::Record("Point".to_owned()));
        let mut body_exit = entry.clone();
        body_exit
            .consume_value("point", span(2))
            .expect("first iteration move is locally valid");

        let error = entry
            .merge_repeat(&body_exit, span(1))
            .expect_err("later repeat iteration would observe moved record");
        assert!(error.message.contains("later iteration"));
    }

    #[test]
    fn repeat_allows_move_when_body_reinitializes_before_next_iteration() {
        let point = SemanticType::Record("Point".to_owned());
        let mut entry = MoveTracker::default();
        entry.define("point".to_owned(), point.clone());
        let mut body_exit = entry.clone();
        body_exit
            .consume_value("point", span(2))
            .expect("body moves point");
        body_exit
            .reinitialize("point", point, span(3))
            .expect("body restores point before next iteration");

        entry
            .merge_repeat(&body_exit, span(1))
            .expect("repeat body is safe for another iteration");
        entry
            .consume_value("point", span(4))
            .expect("point remains available after repeat");
    }

    #[test]
    fn repeat_reinitialization_cannot_restore_previously_moved_value_when_loop_may_skip() {
        let point = SemanticType::Record("Point".to_owned());
        let mut entry = MoveTracker::default();
        entry.define("point".to_owned(), point.clone());
        entry
            .consume_value("point", span(1))
            .expect("point is moved before repeat");
        let mut body_exit = entry.clone();
        body_exit
            .reinitialize("point", point, span(2))
            .expect("body may restore point when repeat executes");

        entry
            .merge_repeat(&body_exit, span(2))
            .expect("merge itself is valid");
        let error = entry
            .consume_value("point", span(3))
            .expect_err("zero-iteration path keeps point moved");
        assert!(error.message.contains("moved record local"));
    }

    #[test]
    fn scalar_field_read_does_not_move_whole_record() {
        let source = "record Point\nx int\nend\n";
        let tokens = lex(source).expect("record source should lex");
        let program = parse(&tokens).expect("record source should parse");
        let records =
            collect_record_environment(&program).expect("record environment should build");

        let mut tracker = MoveTracker::default();
        tracker.define("point".to_owned(), SemanticType::Record("Point".to_owned()));
        assert_eq!(
            tracker
                .access_field(&records, "point", "x", span(1))
                .expect("copy-like scalar field should be readable"),
            SemanticType::Integer
        );
        tracker
            .consume_value("point", span(2))
            .expect("whole record should remain available");
    }

    #[test]
    fn nested_record_field_move_is_rejected_without_clone() {
        let source = "record Inner\nvalue int\nend\nrecord Outer\ninner Inner\nend\n";
        let tokens = lex(source).expect("record source should lex");
        let program = parse(&tokens).expect("record source should parse");
        let records =
            collect_record_environment(&program).expect("record environment should build");

        let mut tracker = MoveTracker::default();
        tracker.define("outer".to_owned(), SemanticType::Record("Outer".to_owned()));
        let error = tracker
            .access_field(&records, "outer", "inner", span(2))
            .expect_err("partial move is explicitly unsupported in v0");
        assert!(error.message.contains("no implicit clone"));
    }
}
