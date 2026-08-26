use evo_lexer::lex;
use evo_parser::{ExprKind, StmtKind, TypeName, parse};

fn parse_source(source: &str) -> evo_parser::Program {
    let tokens = lex(source).expect("record parser source should lex");
    parse(&tokens).expect("record parser source should parse")
}

#[test]
fn named_constructor_and_parenthesized_field_access_are_public_ast() {
    let program =
        parse_source("record Point\nx int\ny int\nend\np = Point(x = 1, y = 2)\nprint (p).x\n");

    let StmtKind::Bind { expr, .. } = &program.statements[0].kind else {
        panic!("expected binding");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Construct { name, fields } if name == "Point" && fields.len() == 2
    ));

    let StmtKind::Print(expr) = &program.statements[1].kind else {
        panic!("expected print");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::FieldAccess { field, .. } if field == "x"
    ));
}

#[test]
fn zero_field_call_stays_a_call_until_record_resolution() {
    let program = parse_source("value = Point()\n");
    let StmtKind::Bind { expr, .. } = &program.statements[0].kind else {
        panic!("expected binding");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Call { name, arguments } if name == "Point" && arguments.is_empty()
    ));
}

#[test]
fn named_types_are_exposed_in_function_signatures() {
    let program = parse_source(
        "record Point\nx int\nend\nfn identity(point Point) Point\nreturn point\nend\n",
    );
    assert_eq!(
        program.functions[0].parameters[0].type_name,
        TypeName::Named("Point".to_owned())
    );
    assert_eq!(
        program.functions[0].return_type,
        TypeName::Named("Point".to_owned())
    );
}
