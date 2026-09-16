use evo_codegen_rust::generate_lowered_rust;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;
use std::fs;
use std::path::PathBuf;

#[test]
fn generational_arena_reference_is_independent_and_generated_runtime_has_no_hidden_costs() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let case_dir = manifest_dir.join("../../benchmarks/cases/generational-arena-v0");
    let evolution_source = fs::read_to_string(case_dir.join("evolution.evo"))
        .expect("arena benchmark Evolution source should be readable");
    let reference = fs::read_to_string(case_dir.join("reference.rs"))
        .expect("arena benchmark Rust reference should be readable");

    let tokens = lex(&evolution_source).expect("arena benchmark should lex");
    let syntax = parse(&tokens).expect("arena benchmark should parse");
    let lowered = lower(&syntax).expect("arena benchmark should lower");
    let generated = generate_lowered_rust(&lowered);

    assert!(generated.contains("struct __EvoHandle<T>"));
    assert!(generated.contains("struct __EvoArena<T>"));
    assert!(generated.contains("fn __evo_arena_insert<T>"));
    assert!(generated.contains("fn __evo_arena_get<T>"));
    assert!(generated.contains("fn __evo_arena_remove<T>"));
    assert!(generated.contains("slot.generation == u64::MAX"));
    assert!(generated.contains("checked_add(1).unwrap_or(0)"));

    for forbidden in [
        "unsafe",
        "RefCell",
        "Mutex",
        "RwLock",
        "HashMap",
        "Rc::clone",
        "Arc<",
    ] {
        assert!(
            !generated.contains(forbidden),
            "generated arena benchmark must not contain hidden runtime mechanism {forbidden:?}"
        );
    }

    assert!(reference.contains("struct RefArena<T>"));
    assert!(reference.contains("struct RefHandle<T>"));
    assert!(!reference.contains("__EvoArena"));
    assert!(!reference.contains("__EvoHandle"));
    assert_ne!(
        normalize_newlines(&reference),
        normalize_newlines(&generated),
        "production arena reference must remain independently authored rather than copied generated Rust"
    );
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}
