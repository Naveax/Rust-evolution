use evo_lexer::lex;
use evo_lowering::{ExprKind, StmtKind, ValueType, lower};
use evo_parser::parse;

fn lower_source(source: &str) -> Result<evo_lowering::Program, evo_lowering::LowerError> {
    let tokens = lex(source).expect("sequence source should lex");
    let syntax = parse(&tokens).expect("sequence source should parse");
    lower(&syntax)
}

#[test]
fn lowers_sequence_types_constructor_append_and_scalar_lookup() {
    let program = lower_source(
        "fn keep(items seq int) seq int\nreturn items\nend\nitems = seq int()\nappend items, 7\nlookup items, 0 as value\nprint value\nelse\nprint 0\nend\n",
    )
    .expect("bounded scalar sequence should lower");

    assert_eq!(
        program.functions[0].parameters[0].value_type,
        ValueType::Sequence(Box::new(ValueType::Integer))
    );
    assert_eq!(
        program.functions[0].return_type,
        ValueType::Sequence(Box::new(ValueType::Integer))
    );
    let StmtKind::Let { mutable, expr, .. } = &program.statements[0].kind else {
        panic!("expected sequence declaration");
    };
    assert!(
        *mutable,
        "append requires the generated Vec binding to be mutable"
    );
    assert!(matches!(
        expr.kind,
        ExprKind::SequenceNew {
            element_type: ValueType::Integer
        }
    ));
    assert!(matches!(
        program.statements[1].kind,
        StmtKind::SequenceAppend { .. }
    ));
    let StmtKind::SequenceLookup {
        binding_by_reference,
        binding_used,
        ..
    } = &program.statements[2].kind
    else {
        panic!("expected checked sequence lookup");
    };
    assert!(!binding_by_reference);
    assert!(*binding_used);
}

#[test]
fn append_requires_matching_element_type_and_lookup_requires_integer_index() {
    let mismatch = lower_source("items = seq int()\nappend items, true\n")
        .expect_err("append type mismatch must fail");
    assert!(mismatch.message.contains("expects int, found bool"));

    let bad_index = lower_source(
        "items = seq int()\nlookup items, true as value\nprint value\nelse\nprint 0\nend\n",
    )
    .expect_err("non-integer lookup index must fail");
    assert!(
        bad_index
            .message
            .contains("lookup index must be an integer")
    );
}

#[test]
fn append_moves_nominal_payload_without_implicit_clone() {
    let error = lower_source(
        "record Item\nvalue int\nend\nitem = Item(value = 1)\nitems = seq Item()\nappend items, item\nprint item.value\n",
    )
    .expect_err("record payload must move into the sequence");
    assert!(error.message.contains("moved record local"));
}

#[test]
fn live_record_element_reference_blocks_growth_move_and_reinitialization() {
    let grow = lower_source(
        "record Item\nvalue int\nend\nitems = seq Item()\nappend items, Item(value = 1)\nlookup items, 0 as item\nappend items, Item(value = 2)\nprint item.value\nelse\nprint 0\nend\n",
    )
    .expect_err("growth while element reference is live must fail");
    assert!(grow.message.contains("cannot grow sequence local"));
    assert!(grow.message.contains("immutable reference"));

    let moved = lower_source(
        "record Item\nvalue int\nend\nitems = seq Item()\nappend items, Item(value = 1)\nlookup items, 0 as item\nmoved = items\nprint item.value\nelse\nprint 0\nend\n",
    )
    .expect_err("move while element reference is live must fail");
    assert!(moved.message.contains("cannot move sequence local"));

    let reinitialized = lower_source(
        "record Item\nvalue int\nend\nitems = seq Item()\nappend items, Item(value = 1)\nlookup items, 0 as item\nitems = seq Item()\nprint item.value\nelse\nprint 0\nend\n",
    )
    .expect_err("reinitialization while element reference is live must fail");
    assert!(
        reinitialized
            .message
            .contains("cannot reinitialize sequence local")
    );
}

#[test]
fn final_reference_use_releases_sequence_for_growth_and_move() {
    lower_source(
        "record Item\nvalue int\nend\nitems = seq Item()\nappend items, Item(value = 1)\nlookup items, 0 as item\nprint item.value\nappend items, Item(value = 2)\nmoved = items\nprint 1\nelse\nprint 0\nend\n",
    )
    .expect("growth and move after final element-reference use should lower");
}

#[test]
fn unused_move_only_lookup_binding_does_not_pin_sequence() {
    let program = lower_source(
        "record Item\nvalue int\nend\nitems = seq Item()\nappend items, Item(value = 1)\nlookup items, 0 as item\nmoved = items\nprint 1\nelse\nprint 0\nend\n",
    )
    .expect("unused lookup reference must not extend the borrow");
    let StmtKind::SequenceLookup { binding_used, .. } = &program.statements[2].kind else {
        panic!("expected lookup");
    };
    assert!(!binding_used);
}

#[test]
fn record_and_shared_owner_lookup_bind_by_reference() {
    let records = lower_source(
        "record Item\nvalue int\nend\nitems = seq Item()\nappend items, Item(value = 1)\nlookup items, 0 as item\nprint item.value\nelse\nprint 0\nend\n",
    )
    .expect("record lookup should lower");
    let StmtKind::SequenceLookup {
        binding_by_reference,
        ..
    } = &records.statements[2].kind
    else {
        panic!("expected record lookup");
    };
    assert!(*binding_by_reference);

    let shared = lower_source(
        "record Item\nvalue int\nend\nowner = share Item(value = 1)\nitems = seq shared Item()\nappend items, owner\nlookup items, 0 as item\nprint item.value\nelse\nprint 0\nend\n",
    )
    .expect("shared-owner lookup should lower as a reference to Rc payload handle");
    let StmtKind::SequenceLookup {
        binding_by_reference,
        ..
    } = &shared.statements[3].kind
    else {
        panic!("expected shared-owner lookup");
    };
    assert!(*binding_by_reference);
}

#[test]
fn sequence_parameter_becomes_mutable_only_when_grown() {
    let program = lower_source(
        "record Item\nvalue int\nend\nfn add(items seq Item, item Item) seq Item\nappend items, item\nreturn items\nend\n",
    )
    .expect("sequence parameter growth should lower");
    assert!(program.functions[0].parameters[0].mutable);
    assert!(!program.functions[0].parameters[1].mutable);
}

#[test]
fn lookup_binding_cannot_reassign_or_escape_its_success_scope() {
    let reassign = lower_source(
        "items = seq int()\nappend items, 1\nlookup items, 0 as value\nvalue = 2\nprint value\nelse\nprint 0\nend\n",
    )
    .expect_err("lookup binding reassignment is outside v0");
    assert!(
        reassign
            .message
            .contains("reassigning sequence lookup success binding")
    );

    let outside = lower_source(
        "items = seq int()\nappend items, 1\nlookup items, 0 as value\nprint value\nelse\nprint 0\nend\nprint value\n",
    )
    .expect_err("lookup binding must remain block-local");
    assert!(
        outside
            .message
            .contains("before definition or outside its scope")
    );
}
