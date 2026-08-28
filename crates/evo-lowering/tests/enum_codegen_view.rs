use evo_lexer::lex;
use evo_lowering::enum_codegen_view::{
    EnumCodegenExprKindView, EnumCodegenStmtKindView, EnumCodegenValueType,
};
use evo_lowering::lower;
use evo_parser::parse;

#[test]
fn codegen_view_borrows_structured_executable_enum_ir() {
    let source = "enum Inner\nA int\nend\nenum Wrapped\nNone\nSome Inner\nend\nrecord Holder\nvalue Wrapped\nend\nvalue = Wrapped.Some(Inner.A(1))\nmatch value\ncase Wrapped.None\nprint 0\ncase Wrapped.Some(x)\nmatch x\ncase Inner.A(y)\nprint y\nend\nend\n";
    let tokens = lex(source).expect("codegen view source should lex");
    let syntax = parse(&tokens).expect("codegen view source should parse");
    let program = lower(&syntax).expect("codegen view source should lower");
    let view = program
        .enum_codegen_view()
        .expect("enum program should expose a codegen view");

    assert_eq!(view.enums().len(), 2);
    let wrapped = view.enums().get(1).expect("Wrapped enum should exist");
    assert_eq!(wrapped.name(), "Wrapped");
    assert_eq!(wrapped.span().line, 4);
    assert_eq!(
        wrapped
            .variants()
            .get(1)
            .expect("Some variant should exist")
            .payload_type(),
        Some(EnumCodegenValueType::Enum("Inner"))
    );

    let holder = view.records().get(0).expect("Holder record should exist");
    assert_eq!(
        holder
            .fields()
            .get(0)
            .expect("Holder.value should exist")
            .value_type(),
        EnumCodegenValueType::Enum("Wrapped")
    );

    let constructor = view
        .statements()
        .get(0)
        .expect("constructor statement should exist");
    let EnumCodegenStmtKindView::Let { expr, .. } = constructor.kind() else {
        panic!("constructor should lower to a let");
    };
    let EnumCodegenExprKindView::EnumConstruct {
        enum_name,
        variant_name,
        payload,
        ..
    } = expr.kind()
    else {
        panic!("let expression should retain enum constructor identity");
    };
    assert_eq!(enum_name, "Wrapped");
    assert_eq!(variant_name, "Some");
    assert!(payload.is_some());

    let matched = view
        .statements()
        .get(1)
        .expect("match statement should exist");
    let EnumCodegenStmtKindView::Match {
        enum_name, arms, ..
    } = matched.kind()
    else {
        panic!("second statement should retain match identity");
    };
    assert_eq!(enum_name, "Wrapped");
    let some_arm = arms.get(1).expect("Some arm should exist");
    let binding = some_arm.binding().expect("Some arm should bind its payload");
    assert_eq!(binding.name(), "x");
    assert_eq!(binding.value_type(), EnumCodegenValueType::Enum("Inner"));
}
