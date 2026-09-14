use evo_codegen_rust::generate_lowered_rust;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn generated_shared_owner_reinitialization_after_move_compiles() {
    let source = concat!(
        "record Item\nvalue int\nend\n",
        "owner = share Item(value = 1)\n",
        "moved = owner\n",
        "owner = share Item(value = 2)\n",
        "print moved.value\n",
        "print owner.value\n",
    );
    let tokens = lex(source).expect("shared-owner reinit compile source should lex");
    let syntax = parse(&tokens).expect("shared-owner reinit compile source should parse");
    let lowered = lower(&syntax).expect("shared-owner reinit compile source should lower");
    let rust = generate_lowered_rust(&lowered);

    assert!(rust.contains("let mut __evo_owner = std::rc::Rc::new"));
    assert!(rust.contains("let __evo_moved = __evo_owner;"));
    assert!(rust.contains("__evo_owner = std::rc::Rc::new"));

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/shared-owner-generated-rust")
        .join("move-reinitialize");
    if root.exists() {
        fs::remove_dir_all(&root).expect("stale generated-Rust test directory should be removable");
    }
    fs::create_dir_all(&root).expect("generated-Rust test directory should be creatable");
    let input = root.join("case.rs");
    let output = root.join(format!("case{}", std::env::consts::EXE_SUFFIX));
    fs::write(&input, &rust).expect("generated Rust should be writable");
    let result = Command::new("rustc")
        .arg("--edition=2024")
        .arg("--crate-name")
        .arg("evo_shared_owner_reinit_case")
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
}
