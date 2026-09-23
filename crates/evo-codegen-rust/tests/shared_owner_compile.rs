use evo_codegen_rust::generate_lowered_rust;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn generated(source: &str) -> String {
    let tokens = lex(source).expect("shared-owner compile source should lex");
    let syntax = parse(&tokens).expect("shared-owner compile source should parse");
    let lowered = lower(&syntax).expect("shared-owner compile source should lower");
    generate_lowered_rust(&lowered)
}

fn assert_rustc_accepts(label: &str, source: &str) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/shared-owner-generated-rust")
        .join(label);
    if root.exists() {
        fs::remove_dir_all(&root).expect("stale generated-Rust test directory should be removable");
    }
    fs::create_dir_all(&root).expect("generated-Rust test directory should be creatable");
    let input = root.join("case.rs");
    let output = root.join(format!("case{}", std::env::consts::EXE_SUFFIX));
    let rust = generated(source);
    fs::write(&input, &rust).expect("generated Rust should be writable");
    let result = Command::new("rustc")
        .arg("--edition=2024")
        .arg("--crate-name")
        .arg("evo_shared_owner_codegen_case")
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

#[test]
fn generated_rc_create_duplicate_and_reads_compile_as_safe_rust() {
    assert_rustc_accepts(
        "create-duplicate-read",
        concat!(
            "record Item\nvalue int\nend\n",
            "owner = share Item(value = 7)\n",
            "alias = dup owner\n",
            "print owner.value\n",
            "print alias.value\n",
        ),
    );
}

#[test]
fn generated_payload_reference_from_rc_compiles_with_payload_type() {
    assert_rustc_accepts(
        "payload-reference",
        concat!(
            "record Item\nvalue int\nend\n",
            "fn read(item &Item) int\nreturn item.value\nend\n",
            "owner = share Item(value = 7)\n",
            "r = &owner\n",
            "print read(r)\n",
        ),
    );
}

#[test]
fn generated_duplicate_then_by_value_forward_compiles() {
    assert_rustc_accepts(
        "duplicate-forward",
        concat!(
            "record Item\nvalue int\nend\n",
            "fn forward(item shared Item) shared Item\nreturn item\nend\n",
            "owner = share Item(value = 7)\n",
            "forwarded = forward(dup owner)\n",
            "print owner.value\n",
            "print forwarded.value\n",
        ),
    );
}

#[test]
fn generated_live_weak_upgrade_compiles_as_safe_rust() {
    assert_rustc_accepts(
        "weak-live-upgrade",
        concat!(
            "record Item\nvalue int\nend\n",
            "owner = share Item(value = 7)\n",
            "edge = downgrade owner\n",
            "upgrade edge as live\n",
            "print live.value\n",
            "else\n",
            "print 0\n",
            "end\n",
            "print owner.value\n",
        ),
    );
}

#[test]
fn generated_dead_weak_upgrade_branch_compiles_as_safe_rust() {
    assert_rustc_accepts(
        "weak-dead-upgrade",
        concat!(
            "record Item\nvalue int\nend\n",
            "fn consume(item shared Item) int\nreturn item.value\nend\n",
            "owner = share Item(value = 7)\n",
            "edge = downgrade owner\n",
            "print consume(owner)\n",
            "upgrade edge as live\n",
            "print live.value\n",
            "else\n",
            "print 0\n",
            "end\n",
        ),
    );
}
