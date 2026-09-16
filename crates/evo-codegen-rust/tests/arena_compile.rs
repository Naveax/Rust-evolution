use evo_codegen_rust::generate_lowered_rust;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn generated(source: &str) -> String {
    let tokens = lex(source).expect("arena compile source should lex");
    let syntax = parse(&tokens).expect("arena compile source should parse");
    let lowered = lower(&syntax).expect("arena compile source should lower");
    generate_lowered_rust(&lowered)
}

fn compile(label: &str, source: &str) -> (PathBuf, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/arena-generated-rust")
        .join(label);
    if root.exists() {
        fs::remove_dir_all(&root).expect("stale arena generated directory should be removable");
    }
    fs::create_dir_all(&root).expect("arena generated directory should be creatable");
    let input = root.join("case.rs");
    let output = root.join(format!("case{}", std::env::consts::EXE_SUFFIX));
    let rust = generated(source);
    fs::write(&input, &rust).expect("generated Rust should be writable");
    let result = Command::new("rustc")
        .arg("--edition=2024")
        .arg("--crate-name")
        .arg("evo_arena_codegen_case")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .output()
        .expect("rustc should execute");
    assert!(
        result.status.success(),
        "generated Rust failed to compile:\n{rust}\n\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    (output, rust)
}

#[test]
fn generated_arena_rejects_wrong_arena_and_stale_handles_and_reuses_generation() {
    let source = "a = arena int()\nb = arena int()\ninsert a, 10 as h\ninsert b, 20 as hb\nlookup b, h as wrong\nprint 999\nelse\nprint 1\nend\nremove a, h as removed\nprint removed\nelse\nprint 0\nend\ninsert a, 30 as fresh\nlookup a, h as stale\nprint 999\nelse\nprint 2\nend\nlookup a, fresh as value\nprint value\nelse\nprint 0\nend\n";
    let (binary, rust) = compile("identity-stale-reuse", source);
    let output = Command::new(binary)
        .output()
        .expect("generated arena program should run");
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n10\n2\n30\n");
    assert!(rust.contains("struct __EvoHandle<T>"));
    assert!(rust.contains("generation += 1"));
    assert!(rust.contains("slot.retired = true"));
    assert!(rust.contains("checked_add(1).unwrap_or(0)"));
    assert!(!rust.contains("unsafe"));
    assert!(!rust.contains("RefCell"));
    assert!(!rust.contains("Mutex"));
    assert!(!rust.contains("HashMap"));
}

#[test]
fn generated_handle_graph_storage_compiles_without_refcount_scaffolding() {
    let (_, rust) = compile(
        "handle-sequence",
        "items = arena int()\ninsert items, 7 as h\nedges = seq handle int()\nappend edges, h\nlookup edges, 0 as copied\nlookup items, copied as value\nprint value\nelse\nprint 0\nend\nelse\nprint 0\nend\n",
    );
    assert!(rust.contains("Vec::<__EvoHandle<i64>>::new()"));
    assert!(!rust.contains("Rc::clone"));
    assert!(!rust.contains("Arc<"));
}

#[test]
fn generated_record_lookup_borrows_payload_and_releases_before_mutation() {
    let _ = compile(
        "record-nll",
        "record Item\nvalue int\nend\nitems = arena Item()\ninsert items, Item(value = 1) as h\nlookup items, h as item\nprint item.value\ninsert items, Item(value = 2) as h2\nremove items, h as removed\nprint removed.value\nelse\nprint 0\nend\nlookup items, h2 as second\nprint second.value\nelse\nprint 0\nend\nelse\nprint 0\nend\n",
    );
}
