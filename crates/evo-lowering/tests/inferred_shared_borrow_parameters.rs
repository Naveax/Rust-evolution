use evo_lexer::lex;
use evo_lowering::{ExprKind, ParameterPassingMode, StmtKind, lower};
use evo_parser::parse;

fn lower_source(source: &str) -> evo_lowering::Program {
    let tokens = lex(source).expect("source should lex");
    let syntax = parse(&tokens).expect("source should parse");
    lower(&syntax).expect("source should lower")
}

#[test]
fn read_only_nominal_parameter_is_shared_and_local_can_be_read_twice() {
    let program = lower_source(
        "record Item\nvalue int\nend\nfn read_value(item Item) int\nreturn item.value\nend\nitem = Item(value = 7)\nfirst = read_value(item)\nsecond = read_value(item)\nprint first + second\n",
    );

    assert_eq!(
        program.functions[0].parameters[0].passing_mode,
        ParameterPassingMode::SharedBorrow
    );

    for statement in &program.statements[1..=2] {
        let StmtKind::Let { expr, .. } = &statement.kind else {
            panic!("expected call binding");
        };
        let ExprKind::Call { argument_modes, .. } = &expr.kind else {
            panic!("expected lowered call");
        };
        assert_eq!(argument_modes, &[ParameterPassingMode::SharedBorrow]);
    }
}

#[test]
fn shared_read_can_be_followed_by_owned_move() {
    let program = lower_source(
        "record Item\nvalue int\nend\nfn read_value(item Item) int\nreturn item.value\nend\nfn consume(item Item) Item\nreturn item\nend\nitem = Item(value = 7)\nprint read_value(item)\nmoved = consume(item)\n",
    );

    assert_eq!(
        program.functions[0].parameters[0].passing_mode,
        ParameterPassingMode::SharedBorrow
    );
    assert_eq!(
        program.functions[1].parameters[0].passing_mode,
        ParameterPassingMode::Owned
    );

    let StmtKind::Let { expr, .. } = &program.statements[2].kind else {
        panic!("expected owned consume binding");
    };
    let ExprKind::Call { argument_modes, .. } = &expr.kind else {
        panic!("expected consume call");
    };
    assert_eq!(argument_modes, &[ParameterPassingMode::Owned]);
}

#[test]
fn forwarding_classification_remains_deliberately_non_transitive() {
    let program = lower_source(
        "record Item\nvalue int\nend\nfn read_value(item Item) int\nreturn item.value\nend\nfn forward(item Item) int\nreturn read_value(item)\nend\n",
    );

    assert_eq!(
        program.functions[0].parameters[0].passing_mode,
        ParameterPassingMode::SharedBorrow
    );
    assert_eq!(
        program.functions[1].parameters[0].passing_mode,
        ParameterPassingMode::Owned
    );

    let StmtKind::Return(expr) = &program.functions[1].body[0].kind else {
        panic!("expected forwarding return");
    };
    let ExprKind::Call { argument_modes, .. } = &expr.kind else {
        panic!("expected forwarding call");
    };
    assert_eq!(argument_modes, &[ParameterPassingMode::SharedBorrow]);
}

#[test]
fn owned_return_and_reinitialization_stay_owned() {
    let program = lower_source(
        "record Item\nvalue int\nend\nfn identity(item Item) Item\nreturn item\nend\nfn replace(item Item) int\nitem = Item(value = 9)\nreturn item.value\nend\n",
    );

    assert_eq!(
        program.functions[0].parameters[0].passing_mode,
        ParameterPassingMode::Owned
    );
    assert_eq!(
        program.functions[1].parameters[0].passing_mode,
        ParameterPassingMode::Owned
    );
}

#[test]
fn scalar_parameters_never_enter_shared_borrow_inference() {
    let program = lower_source("fn read(value int) int\nreturn value\nend\n");
    assert_eq!(
        program.functions[0].parameters[0].passing_mode,
        ParameterPassingMode::Owned
    );
}
