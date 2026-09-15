from pathlib import Path

path = Path("crates/evo-lowering/tests/borrow_inference_research.rs")
text = path.read_text()
old = '''            StmtKind::Match { value, arms } => {
                collect_expr_effects(value, parameter, UseMode::Consume, effects);
                for arm in arms {
                    collect_statement_effects(&arm.body, parameter, effects);
                }
            }
'''
new = '''            StmtKind::SequenceAppend { value, .. } => {
                collect_expr_effects(value, parameter, UseMode::Consume, effects);
            }
            StmtKind::SequenceLookup {
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
                collect_expr_effects(value, parameter, UseMode::Consume, effects);
                for arm in arms {
                    collect_statement_effects(&arm.body, parameter, effects);
                }
            }
'''
if text.count(old) != 1:
    raise SystemExit("borrow research statement anchor changed")
text = text.replace(old, new, 1)
old = '''        ExprKind::Integer(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::InputInt => {}
'''
new = '''        ExprKind::Integer(_)
        | ExprKind::String(_)
        | ExprKind::Bool(_)
        | ExprKind::InputInt
        | ExprKind::SequenceNew { .. } => {}
'''
if text.count(old) != 1:
    raise SystemExit("borrow research expression anchor changed")
path.write_text(text.replace(old, new, 1))
