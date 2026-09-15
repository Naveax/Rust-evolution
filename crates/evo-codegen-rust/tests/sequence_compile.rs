use evo_codegen_rust::generate_lowered_rust;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn generated(source: &str) -> String {
    let tokens = lex(source).expect("sequence compile source should lex");
    let syntax = parse(&tokens).expect("sequence compile source should parse");
    let lowered = lower(&syntax).expect("sequence compile source should lower");
    generate_lowered_rust(&lowered)
}

fn compile(label: &str, source: &str) -> (PathBuf, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/sequence-generated-rust")
        .join(label);
    if root.exists() {
        fs::remove_dir_all(&root).expect("stale sequence generated directory should be removable");
    }
    fs::create_dir_all(&root).expect("sequence generated directory should be creatable");
    let input = root.join("case.rs");
    let output = root.join(format!("case{}", std::env::consts::EXE_SUFFIX));
    let rust = generated(source);
    fs::write(&input, &rust).expect("generated Rust should be writable");
    let result = Command::new("rustc")
        .arg("--edition=2024")
        .arg("--crate-name")
        .arg("evo_sequence_codegen_case")
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
fn generated_scalar_checked_lookup_executes_in_bounds_negative_and_oob_paths() {
    let (binary, _) = compile(
        "checked-indices",
        "items = seq int()\nappend items, 7\nlookup items, 0 as value\nprint value\nelse\nprint 10\nend\nlookup items, -1 as value\nprint value\nelse\nprint 11\nend\nlookup items, 9 as value\nprint value\nelse\nprint 13\nend\n",
    );
    let output = Command::new(binary)
        .output()
        .expect("generated program should run");
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "7\n11\n13\n");
}

#[test]
fn generated_record_reference_releases_before_growth_and_move() {
    let _ = compile(
        "record-nll",
        "record Item\nvalue int\nend\nitems = seq Item()\nappend items, Item(value = 1)\nlookup items, 0 as item\nprint item.value\nappend items, Item(value = 2)\nmoved = items\nprint 1\nelse\nprint 0\nend\n",
    );
}

#[test]
fn generated_shared_owner_lookup_compiles_without_clone_scaffolding() {
    let (_, rust) = compile(
        "shared-owner",
        "record Item\nvalue int\nend\nowner = share Item(value = 9)\nitems = seq shared Item()\nappend items, owner\nlookup items, 0 as item\nprint item.value\nelse\nprint 0\nend\n",
    );
    assert!(!rust.contains("Rc::clone"));
    assert!(!rust.contains("unsafe"));
}
