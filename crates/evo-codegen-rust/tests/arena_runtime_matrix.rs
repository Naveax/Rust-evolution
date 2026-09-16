use evo_codegen_rust::generate_lowered_rust;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn generated(source: &str) -> String {
    let tokens = lex(source).expect("arena runtime matrix source should lex");
    let syntax = parse(&tokens).expect("arena runtime matrix source should parse");
    let lowered = lower(&syntax).expect("arena runtime matrix source should lower");
    generate_lowered_rust(&lowered)
}

fn arena_runtime_prefix() -> String {
    let rust = generated("items = arena int()\n");
    let (prefix, _) = rust
        .split_once("fn main() {")
        .expect("generated arena Rust should contain main");
    prefix.to_owned()
}

fn compile_rust(label: &str, rust: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/arena-runtime-matrix")
        .join(label);
    if root.exists() {
        fs::remove_dir_all(&root)
            .expect("stale arena runtime matrix directory should be removable");
    }
    fs::create_dir_all(&root).expect("arena runtime matrix directory should be creatable");
    let input = root.join("case.rs");
    let output = root.join(format!("case{}", std::env::consts::EXE_SUFFIX));
    fs::write(&input, rust).expect("arena runtime matrix Rust should be writable");
    let result = Command::new("rustc")
        .arg("--edition=2024")
        .arg("--crate-name")
        .arg("evo_arena_runtime_matrix_case")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .output()
        .expect("rustc should execute");
    assert!(
        result.status.success(),
        "arena runtime matrix Rust failed to compile:\n{rust}\n\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    output
}

#[test]
fn generated_runtime_retires_generation_max_keeps_three_word_handle_and_drops_once() {
    let prefix = arena_runtime_prefix();
    let rust = format!(
        r#"{prefix}
std::thread_local! {{
    static __TEST_DROPS: std::cell::Cell<usize> = const {{ std::cell::Cell::new(0) }};
}}

struct __TestDropProbe;

impl Drop for __TestDropProbe {{
    fn drop(&mut self) {{
        __TEST_DROPS.with(|drops| drops.set(drops.get() + 1));
    }}
}}

fn main() {{
    assert_eq!(std::mem::size_of::<usize>(), std::mem::size_of::<u64>());
    assert_eq!(
        std::mem::size_of::<__EvoHandle<i64>>(),
        3 * std::mem::size_of::<usize>()
    );

    let mut retired = __evo_arena_new::<i64>();
    let initial = __evo_arena_insert(&mut retired, 7);
    retired.slots[initial.index].generation = u64::MAX;
    let max_handle = __EvoHandle {{
        arena: initial.arena,
        index: initial.index,
        generation: u64::MAX,
        _marker: std::marker::PhantomData,
    }};
    assert_eq!(__evo_arena_remove(&mut retired, max_handle), Some(7));
    assert!(retired.slots[max_handle.index].retired);
    assert!(retired.free.is_empty());
    assert!(__evo_arena_get(&retired, max_handle).is_none());
    let fresh = __evo_arena_insert(&mut retired, 9);
    assert_ne!(fresh.index, max_handle.index);
    assert_eq!(__evo_arena_get(&retired, fresh), Some(&9));

    let mut drops = __evo_arena_new::<__TestDropProbe>();
    let first = __evo_arena_insert(&mut drops, __TestDropProbe);
    let _second = __evo_arena_insert(&mut drops, __TestDropProbe);
    let removed = __evo_arena_remove(&mut drops, first).expect("first payload should remove");
    __TEST_DROPS.with(|count| assert_eq!(count.get(), 0));
    drop(removed);
    __TEST_DROPS.with(|count| assert_eq!(count.get(), 1));
    drop(drops);
    __TEST_DROPS.with(|count| assert_eq!(count.get(), 2));
    println!("ok");
}}
"#
    );
    let binary = compile_rust("generation-retire-drop-size", &rust);
    let output = Command::new(binary)
        .output()
        .expect("generation retirement/drop matrix should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "ok\n");
}

#[test]
fn generated_runtime_arena_id_exhaustion_fails_closed() {
    let prefix = arena_runtime_prefix();
    let rust = format!(
        r#"{prefix}
fn main() {{
    __EVO_NEXT_ARENA_ID.with(|next| next.set(u64::MAX));
    let last = __evo_arena_new::<i64>();
    assert_eq!(last.id, u64::MAX);
    let _must_fail = __evo_arena_new::<i64>();
}}
"#
    );
    let binary = compile_rust("arena-id-exhaustion", &rust);
    let output = Command::new(binary)
        .output()
        .expect("arena-id exhaustion matrix should run");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("arena identity exhausted"),
        "arena-id exhaustion must fail closed with the generated runtime diagnostic"
    );
}

#[test]
fn generated_shared_owner_lookup_borrows_without_hidden_rc_clone() {
    let rust = generated(
        "record Item\nvalue int\nend\nowner = share Item(value = 9)\nitems = arena shared Item()\ninsert items, owner as h\nlookup items, h as item\nprint item.value\nelse\nprint 0\nend\n",
    );
    assert!(rust.contains("std::rc::Rc::new"));
    assert!(!rust.contains("Rc::clone"));
    assert!(!rust.contains("unsafe"));
    let binary = compile_rust("shared-owner-no-clone", &rust);
    let output = Command::new(binary)
        .output()
        .expect("shared-owner arena matrix should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "9\n");
}
