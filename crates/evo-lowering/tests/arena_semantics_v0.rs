use evo_diagnostics::render_error;
use evo_lexer::lex;
use evo_lowering::{RecordType, StmtKind, ValueType, lower};
use evo_parser::parse;
use std::path::Path;

fn lower_source(source: &str) -> Result<evo_lowering::Program, evo_lowering::LowerError> {
    let tokens = lex(source).expect("arena semantic source should lex");
    let syntax = parse(&tokens).expect("arena semantic source should parse");
    lower(&syntax)
}

#[test]
fn lowers_arena_insert_lookup_remove_and_handle_contracts() {
    let program = lower_source(
        "fn keep(h handle int) handle int\nreturn h\nend\nitems = arena int()\ninsert items, 7 as h\nlookup items, h as value\nprint value\nelse\nprint 0\nend\nremove items, h as removed\nprint removed\nelse\nprint 0\nend\n",
    )
    .expect("bounded arena surface should lower");
    assert_eq!(
        program.functions[0].parameters[0].value_type,
        ValueType::Handle(Box::new(ValueType::Integer))
    );
    assert_eq!(
        program.functions[0].return_type,
        ValueType::Handle(Box::new(ValueType::Integer))
    );
    assert!(matches!(
        program.statements[1].kind,
        StmtKind::ArenaInsert { .. }
    ));
    assert!(matches!(
        program.statements[2].kind,
        StmtKind::ArenaLookup { .. }
    ));
    assert!(matches!(
        program.statements[3].kind,
        StmtKind::ArenaRemove { .. }
    ));
}

#[test]
fn rejects_wrong_handle_payload_type_before_codegen() {
    let error = lower_source(
        "record A\nvalue int\nend\nrecord B\nvalue int\nend\na = arena A()\nb = arena B()\ninsert b, B(value = 1) as hb\nlookup a, hb as item\nprint item.value\nelse\nprint 0\nend\n",
    )
    .expect_err("different handle payload types must fail statically");
    assert!(error.message.contains("expects handle A"));
    assert!(error.message.contains("handle B"));
}

#[test]
fn live_arena_element_reference_blocks_insert_remove_move_and_reinit() {
    for (label, body, needle) in [
        (
            "insert",
            "insert items, Item(value = 2) as h2\nprint item.value\n",
            "cannot insert into arena local",
        ),
        (
            "remove",
            "remove items, h as removed\nprint removed.value\nelse\nprint 0\nend\nprint item.value\n",
            "cannot remove from arena local",
        ),
        (
            "move",
            "moved = items\nprint item.value\n",
            "cannot move arena local",
        ),
        (
            "reinit",
            "items = arena Item()\nprint item.value\n",
            "cannot reinitialize arena local",
        ),
    ] {
        let source = format!(
            "record Item\nvalue int\nend\nitems = arena Item()\ninsert items, Item(value = 1) as h\nlookup items, h as item\n{body}else\nprint 0\nend\n"
        );
        let error = match lower_source(&source) {
            Ok(_) => panic!("{label} with live reference must fail"),
            Err(error) => error,
        };
        assert!(error.message.contains(needle), "{label}: {}", error.message);
    }
}

#[test]
fn final_use_release_allows_later_insert_and_remove() {
    lower_source(
        "record Item\nvalue int\nend\nitems = arena Item()\ninsert items, Item(value = 1) as h\nlookup items, h as item\nprint item.value\ninsert items, Item(value = 2) as h2\nremove items, h as removed\nprint removed.value\nelse\nprint 0\nend\nlookup items, h2 as second\nprint second.value\nelse\nprint 0\nend\nelse\nprint 0\nend\n",
    )
    .expect("owner mutations after final reference use should lower");
}

#[test]
fn handle_fields_are_fixed_size_graph_edges_not_recursive_records() {
    let program = lower_source("record Node\nnext handle Node\nend\n")
        .expect("self handle edge should be sized");
    assert_eq!(
        program.records[0].fields[0].value_type,
        RecordType::Handle("Node".to_owned())
    );
}

#[test]
fn sequence_handle_lookup_is_copy_like_not_borrowing() {
    let program = lower_source(
        "items = arena int()\ninsert items, 1 as h\nedges = seq handle int()\nappend edges, h\nlookup edges, 0 as copied\nlookup items, copied as value\nprint value\nelse\nprint 0\nend\nelse\nprint 0\nend\n",
    )
    .expect("handle adjacency storage should lower");
    let StmtKind::SequenceLookup {
        binding_by_reference,
        ..
    } = &program.statements[4].kind
    else {
        panic!("expected sequence handle lookup");
    };
    assert!(
        !binding_by_reference,
        "handles must copy without pinning the sequence"
    );
}

#[test]
fn live_arena_reference_diagnostic_keeps_lookup_as_related_location() {
    let source = "record Item\nvalue int\nend\nitems = arena Item()\ninsert items, Item(value = 1) as h\nlookup items, h as item\ninsert items, Item(value = 2) as h2\nprint item.value\nelse\nprint 0\nend\n";
    let error = lower_source(source).expect_err("insert with live arena element reference must fail");
    let rendered = render_error(
        Path::new("arena-related.evo"),
        source,
        &error.message,
        error.span,
    );
    assert!(rendered.contains("cannot insert into arena local \"items\""));
    assert!(rendered.contains(" --> arena-related.evo:7:1"));
    assert!(rendered.contains("note: immutable reference \"item\" was created here"));
    assert!(rendered.contains(" --> arena-related.evo:6:1"));
}
