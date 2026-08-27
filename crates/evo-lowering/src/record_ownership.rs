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

#[derive(Debug, Default)]
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
