from pathlib import Path

path = Path("crates/evo-lowering/tests/borrow_inference_research.rs")
text = path.read_text()
old_stmt = '''            StmtKind::SequenceLookup {
                index,
                then_body,
                else_body,
                ..
            } => {
                collect_expr_effects(index, parameter, UseMode::Consume, effects);
                collect_statement_effects(then_body, parameter, effects);
                collect_statement_effects(else_body, parameter, effects);
            }
            StmtKind::Match { value, arms } => {
'''
new_stmt = '''            StmtKind::SequenceLookup {
                index,
                then_body,
                else_body,
                ..
            } => {
                collect_expr_effects(index, parameter, UseMode::Consume, effects);
                collect_statement_effects(then_body, parameter, effects);
                collect_statement_effects(else_body, parameter, effects);
            }
            StmtKind::ArenaInsert { value, .. } => {
                collect_expr_effects(value, parameter, UseMode::Consume, effects);
            }
            StmtKind::ArenaRemove {
                handle,
                then_body,
                else_body,
                ..
            } => {
                collect_expr_effects(handle, parameter, UseMode::Consume, effects);
                collect_statement_effects(then_body, parameter, effects);
                collect_statement_effects(else_body, parameter, effects);
            }
            StmtKind::Match { value, arms } => {
'''
if text.count(old_stmt) != 1:
    raise SystemExit(f"borrow research statement anchor count: {text.count(old_stmt)}")
text = text.replace(old_stmt, new_stmt, 1)
old_expr = '''        | ExprKind::InputInt
        | ExprKind::SequenceNew { .. } => {}
'''
new_expr = '''        | ExprKind::InputInt
        | ExprKind::SequenceNew { .. }
        | ExprKind::ArenaNew { .. } => {}
'''
if text.count(old_expr) != 1:
    raise SystemExit(f"borrow research expr anchor count: {text.count(old_expr)}")
path.write_text(text.replace(old_expr, new_expr, 1))
