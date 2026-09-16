from pathlib import Path

# The adjacency-list fixture has: arena bind, insert, seq bind, append, then lookup.
test = Path("crates/evo-lowering/tests/arena_semantics_v0.rs")
text = test.read_text()
old = "&program.statements[3].kind else"
new = "&program.statements[4].kind else"
if text.count(old) != 1:
    raise SystemExit(f"arena semantics test lookup index anchor count: {text.count(old)}")
test.write_text(text.replace(old, new, 1))

# Avoid moving RecordType out of a borrowed field while detecting handle support.
codegen = Path("crates/evo-codegen-rust/src/lib.rs")
text = codegen.read_text()
old = "matches!(field.value_type, RecordType::Handle(_))"
new = "matches!(&field.value_type, RecordType::Handle(_))"
if text.count(old) != 1:
    raise SystemExit(f"arena support field match anchor count: {text.count(old)}")
codegen.write_text(text.replace(old, new, 1))

# The graph fixture stores typed Node handles, not integer handles.
compile_test = Path("crates/evo-codegen-rust/tests/arena_compile.rs")
text = compile_test.read_text()
old = 'assert!(rust.contains("Vec<__EvoHandle<i64>>"));'
new = 'assert!(rust.contains("Vec<__EvoHandle<__EvoRecord_Node>>"));'
if text.count(old) != 1:
    raise SystemExit(f"arena graph handle assertion anchor count: {text.count(old)}")
compile_test.write_text(text.replace(old, new, 1))
