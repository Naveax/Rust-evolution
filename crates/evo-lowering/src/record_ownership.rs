use crate::{
    LowerError,
    record_environment::{RecordEnvironment, SemanticType},
};
use evo_lexer::Span;

mod move_state {
    include!("move_state.rs");
}

use move_state::{MoveState, MoveStateError};

#[derive(Debug, Clone, Default)]
pub(crate) struct MoveTracker {
    state: MoveState<SemanticType>,
}

impl MoveTracker {
    pub(crate) fn define(&mut self, name: String, value_type: SemanticType) {
        self.state.define(name, value_type);
    }

    pub(crate) fn forget(&mut self, name: &str) {
        self.state.forget(name);
    }

    pub(crate) fn inspect_value(&self, name: &str, span: Span) -> Result<SemanticType, LowerError> {
        self.state
            .inspect(name)
            .map_err(|error| record_read_error(name, span, error))
    }

    pub(crate) fn consume_value(
        &mut self,
        name: &str,
        span: Span,
    ) -> Result<SemanticType, LowerError> {
        self.state
            .consume(name, SemanticType::is_trivially_reusable_v0)
            .map_err(|error| record_read_error(name, span, error))
    }

    pub(crate) fn reinitialize(
        &mut self,
        name: &str,
        value_type: SemanticType,
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

    pub(crate) fn merge_if(&mut self, then_exit: &Self, else_exit: &Self) {
        let has_continuation = self.merge_if_continuing(Some(then_exit), Some(else_exit));
        debug_assert!(has_continuation);
    }

    pub(crate) fn merge_if_continuing(
        &mut self,
        then_exit: Option<&Self>,
        else_exit: Option<&Self>,
    ) -> bool {
        self.state.merge_continuing(
            [then_exit, else_exit]
                .into_iter()
                .flatten()
                .map(|tracker| &tracker.state),
        )
    }

    pub(crate) fn merge_repeat(&mut self, body_exit: &Self, span: Span) -> Result<(), LowerError> {
        match self.state.merge_repeat(
            &body_exit.state,
            SemanticType::is_trivially_reusable_v0,
        ) {
            Ok(()) => Ok(()),
            Err(MoveStateError::RepeatWouldConsume) => Err(LowerError {
                message: repeat_move_message(self, body_exit),
                span,
            }),
            Err(
                MoveStateError::MissingBinding
                | MoveStateError::UnavailableBinding
                | MoveStateError::TypeMismatch,
            ) => unreachable!("repeat merge only reports a move that breaks later iterations"),
        }
    }

    pub(crate) fn access_field(
        &self,
        records: &RecordEnvironment,
        base_name: &str,
        field_name: &str,
        span: Span,
    ) -> Result<SemanticType, LowerError> {
        let base_type = self
            .state
            .inspect(base_name)
            .map_err(|error| record_read_error(base_name, span, error))?;

        let field_type = records.field_type(&base_type, field_name, span)?;
        if !field_type.is_trivially_reusable_v0() {
            return Err(LowerError {
                message: format!(
                    "moving record-valued field {field_name:?} out of local {base_name:?} is not supported in Records v0; no implicit clone is inserted"
                ),
                span,
            });
        }
        Ok(field_type)
    }
}

fn record_read_error(name: &str, span: Span, error: MoveStateError) -> LowerError {
    match error {
        MoveStateError::MissingBinding => LowerError {
            message: format!("use of local {name:?} before definition or outside its scope"),
            span,
        },
        MoveStateError::UnavailableBinding => LowerError {
            message: format!("use of moved record local {name:?}"),
            span,
        },
        MoveStateError::TypeMismatch | MoveStateError::RepeatWouldConsume => {
            unreachable!("record reads only report missing or unavailable bindings")
        }
    }
}

fn repeat_move_message(entry: &MoveTracker, body_exit: &MoveTracker) -> String {
    // Preserve the existing source-native diagnostic without leaking generic move-state
    // machinery into user-facing Records v0 behavior. Find the first binding that is
    // available on entry but unavailable after one body iteration.
    for name in entry.binding_names() {
        if entry.is_available(name) && !body_exit.is_available(name) {
            return format!(
                "record local {name:?} is moved by repeat body and would be unavailable on a later iteration"
            );
        }
    }
    unreachable!("repeat merge reported a move-only availability regression")
}

impl MoveTracker {
    fn binding_names(&self) -> impl Iterator<Item = &str> {
        self.state.binding_names()
    }

    fn is_available(&self, name: &str) -> bool {
        self.state.is_available(name)
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
    fn inspection_checks_availability_without_consuming() {
        let mut tracker = MoveTracker::default();
        tracker.define("point".to_owned(), SemanticType::Record("Point".to_owned()));
        assert_eq!(
            tracker
                .inspect_value("point", span(1))
                .expect("inspection should see available record"),
            SemanticType::Record("Point".to_owned())
        );
        tracker
            .consume_value("point", span(2))
            .expect("inspection must not consume record");
        let error = tracker
            .inspect_value("point", span(3))
            .expect_err("inspection after a move must fail");
        assert!(error.message.contains("moved record local"));
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
        assert!(error.message.contains("record-valued field"));
        assert!(error.message.contains("no implicit clone"));
    }
}
