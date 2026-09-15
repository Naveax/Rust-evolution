from pathlib import Path


def write(path, content):
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(content)

# Parser contract: no removal surface in v0 and bounded element types stay fail-closed.
parser_test = Path("crates/evo-parser/tests/sequence_syntax_v0.rs")
source = parser_test.read_text()
addition = r'''

#[test]
fn removal_surface_stays_absent_in_v0() {
    let error = parse(&lex("items = seq int()\nremove items, 0\n").unwrap())
        .expect_err("v0 must not expose removal syntax");
    assert!(error.message.contains("expected '=' after binding name"));
}

#[test]
fn reference_element_types_stay_outside_v0() {
    let error = parse(&lex("fn bad(items seq &Item) int\nreturn 1\nend\n").unwrap())
        .expect_err("reference sequence elements are outside v0");
    assert!(error.message.contains("function parameters") || error.message.contains("type"));
}
'''
if "fn removal_surface_stays_absent_in_v0" not in source:
    parser_test.write_text(source + addition)

write("crates/evo-lowering/tests/sequence_v0.rs", r'''use evo_lexer::lex;
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
    assert!(*mutable, "append requires the generated Vec binding to be mutable");
    assert!(matches!(
        expr.kind,
        ExprKind::SequenceNew {
            element_type: ValueType::Integer
        }
    ));
    assert!(matches!(program.statements[1].kind, StmtKind::SequenceAppend { .. }));
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
    assert!(bad_index.message.contains("lookup index must be an integer"));
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
    assert!(reinitialized.message.contains("cannot reinitialize sequence local"));
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
    assert!(reassign.message.contains("reassigning sequence lookup success binding"));

    let outside = lower_source(
        "items = seq int()\nappend items, 1\nlookup items, 0 as value\nprint value\nelse\nprint 0\nend\nprint value\n",
    )
    .expect_err("lookup binding must remain block-local");
    assert!(outside.message.contains("before definition or outside its scope"));
}
''')

write("crates/evo-codegen-rust/tests/sequence_v0.rs", r'''use evo_codegen_rust::{generate_lowered_rust, generate_lowered_rust_with_map};
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;

fn generate(source: &str) -> String {
    let tokens = lex(source).expect("sequence codegen source should lex");
    let syntax = parse(&tokens).expect("sequence codegen source should parse");
    let lowered = lower(&syntax).expect("sequence codegen source should lower");
    generate_lowered_rust(&lowered)
}

#[test]
fn scalar_sequence_codegen_is_direct_vec_push_and_checked_get() {
    let generated = generate(
        "items = seq int()\nappend items, 7\nlookup items, -1 as value\nprint value\nelse\nprint 0\nend\n",
    );
    assert!(generated.contains("let mut __evo_items = Vec::<i64>::new();"));
    assert!(generated.contains("__evo_items.push(7);"));
    assert!(generated.contains("usize::try_from((-1)).ok().and_then"));
    assert!(generated.contains("if let Some(&__evo_value)"));
    assert!(generated.contains("__evo_items.get(__evo_lookup_index)"));
    assert!(!generated.contains("unsafe"));
    assert!(!generated.contains("RefCell"));
    assert!(!generated.contains("Mutex"));
}

#[test]
fn record_lookup_keeps_plain_reference_and_allows_nll_growth_after_last_use() {
    let generated = generate(
        "record Item\nvalue int\nend\nitems = seq Item()\nappend items, Item(value = 1)\nlookup items, 0 as item\nprint item.value\nappend items, Item(value = 2)\nelse\nprint 0\nend\n",
    );
    assert!(generated.contains("Vec::<__EvoRecord_Item>::new()"));
    assert!(generated.contains("if let Some(__evo_item)"));
    assert!(generated.contains("(__evo_item).__evo_field_value"));
    assert!(!generated.contains("clone()"));
    assert!(!generated.contains("Rc::clone"));
}

#[test]
fn shared_owner_sequence_uses_vec_of_rc_without_hidden_clone() {
    let generated = generate(
        "record Item\nvalue int\nend\nowner = share Item(value = 9)\nitems = seq shared Item()\nappend items, owner\nlookup items, 0 as item\nprint item.value\nelse\nprint 0\nend\n",
    );
    assert!(generated.contains("Vec::<std::rc::Rc<__EvoRecord_Item>>::new()"));
    assert!(generated.contains("__evo_items.push(__evo_owner);"));
    assert!(generated.contains("if let Some(__evo_item)"));
    assert!(!generated.contains("Rc::clone"));
}

#[test]
fn sequence_parameter_codegen_marks_only_grown_parameter_mutable() {
    let generated = generate(
        "record Item\nvalue int\nend\nfn add(items seq Item, item Item) seq Item\nappend items, item\nreturn items\nend\n",
    );
    assert!(generated.contains(
        "fn __evo_fn_add(mut __evo_items: Vec<__EvoRecord_Item>, __evo_item: __EvoRecord_Item) -> Vec<__EvoRecord_Item>"
    ));
}

#[test]
fn unused_lookup_binding_codegen_avoids_unused_variable_and_retained_borrow() {
    let generated = generate(
        "record Item\nvalue int\nend\nitems = seq Item()\nappend items, Item(value = 1)\nlookup items, 0 as item\nmoved = items\nprint 1\nelse\nprint 0\nend\n",
    );
    assert!(generated.contains("if let Some(_)"));
    assert!(generated.contains("let __evo_moved = __evo_items;"));
    assert!(!generated.contains("Some(__evo_item)"));
}

#[test]
fn lookup_generated_lines_map_back_to_lookup_source_span() {
    let source = "items = seq int()\nappend items, 7\nlookup items, 0 as value\nprint value\nelse\nprint 0\nend\n";
    let tokens = lex(source).unwrap();
    let syntax = parse(&tokens).unwrap();
    let lowered = lower(&syntax).unwrap();
    let generated = generate_lowered_rust_with_map(&lowered);
    let line = generated
        .source
        .lines()
        .position(|line| line.contains("if let Some"))
        .unwrap()
        + 1;
    assert_eq!(generated.source_span_for_line(line).map(|span| span.line), Some(3));
}
''')

write("crates/evo-codegen-rust/tests/sequence_compile.rs", r'''use evo_codegen_rust::generate_lowered_rust;
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
    let output = Command::new(binary).output().expect("generated program should run");
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
''')

case = Path("benchmarks/cases/append-only-sequence-v0")
case.mkdir(parents=True, exist_ok=True)
write(case / "case.conf", "# Direct Vec<T> append + checked lookup parity gate.\nname=append-only-sequence-v0\nwarmup=3\nsamples=13\ntimeout_ms=5000\nmax_relative_mad=0.15\n")
write(case / "evolution.evo", "n = input_int\nitems = seq int()\ni = 0\nrepeat n\nappend items, i\ni = i + 1\nend\nsum = 0\ni = 0\nrepeat n\nlookup items, i as value\nsum = sum + value\nelse\nsum = sum + 0\nend\ni = i + 1\nend\nprint sum\n")
(case / "stdin.bin").write_bytes(b"2000000\n")
write(case / "expected.stdout", "1999999000000\n")
write(case / "reference.rs", r'''fn __evo_input_int() -> i64 {
    let mut __evo_input = String::new();
    std::io::stdin()
        .read_line(&mut __evo_input)
        .expect("failed to read integer input");
    __evo_input
        .trim()
        .parse::<i64>()
        .expect("expected signed integer input")
}

fn main() {
    let __evo_n = __evo_input_int();
    let mut __evo_items = Vec::<i64>::new();
    let mut __evo_i = 0;
    for _ in 0..__evo_n {
        __evo_items.push(__evo_i);
        __evo_i = (__evo_i + 1);
    }
    let mut __evo_sum = 0;
    __evo_i = 0;
    for _ in 0..__evo_n {
        if let Some(&__evo_value) = usize::try_from(__evo_i)
            .ok()
            .and_then(|__evo_lookup_index| __evo_items.get(__evo_lookup_index))
        {
            __evo_sum = (__evo_sum + __evo_value);
        } else {
            __evo_sum = (__evo_sum + 0);
        }
        __evo_i = (__evo_i + 1);
    }
    println!("{}", __evo_sum);
}
''')
write(case / "README.md", "# append-only-sequence-v0\n\nProduction parity gate for the first bounded collection slice. The Evolution program constructs `seq int`, grows it explicitly with `append`, and performs checked indexed lookup with an explicit failure branch. The Rust reference mirrors generated `Vec<i64>`/`push`/`get` code exactly so the benchmark measures abstraction overhead rather than different algorithms.\n")

write("crates/evo-bench/tests/append_only_sequence_reference.rs", r'''use evo_codegen_rust::generate_lowered_rust;
use evo_lexer::lex;
use evo_lowering::lower;
use evo_parser::parse;
use std::fs;
use std::path::PathBuf;

#[test]
fn append_only_sequence_reference_matches_generated_rust_exactly() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let case_dir = manifest_dir.join("../../benchmarks/cases/append-only-sequence-v0");
    let evolution_source = fs::read_to_string(case_dir.join("evolution.evo"))
        .expect("sequence benchmark Evolution source should be readable");
    let reference = fs::read_to_string(case_dir.join("reference.rs"))
        .expect("sequence benchmark Rust reference should be readable");
    let tokens = lex(&evolution_source).expect("sequence benchmark should lex");
    let syntax = parse(&tokens).expect("sequence benchmark should parse");
    let lowered = lower(&syntax).expect("sequence benchmark should lower");
    let generated = generate_lowered_rust(&lowered);
    assert_eq!(normalize_newlines(&reference), normalize_newlines(&generated));
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}
''')

write(".github/workflows/append-only-sequence-performance.yml", r'''name: Append-only sequence performance

on:
  pull_request:
    paths:
      - 'benchmarks/cases/append-only-sequence-v0/**'
      - 'crates/evo-parser/**'
      - 'crates/evo-lowering/**'
      - 'crates/evo-codegen-rust/**'
      - 'crates/evo-bench/tests/append_only_sequence_reference.rs'
      - '.github/workflows/append-only-sequence-performance.yml'
  push:
    branches:
      - main
    paths:
      - 'benchmarks/cases/append-only-sequence-v0/**'
      - 'crates/evo-parser/**'
      - 'crates/evo-lowering/**'
      - 'crates/evo-codegen-rust/**'
      - 'crates/evo-bench/tests/append_only_sequence_reference.rs'
      - '.github/workflows/append-only-sequence-performance.yml'
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: true

jobs:
  vec-parity:
    name: ubuntu-24.04 / Rust 1.98.0
    runs-on: ubuntu-24.04
    steps:
      - name: Checkout
        uses: actions/checkout@v5

      - name: Install pinned Rust toolchain
        run: rustup toolchain install 1.98.0 --profile minimal --component rustfmt --component clippy

      - name: Format check
        run: cargo fmt --all -- --check

      - name: Append-only sequence focused tests
        run: |
          cargo test -p evo-parser --test sequence_syntax_v0
          cargo test -p evo-lowering --test sequence_v0
          cargo test -p evo-codegen-rust --test sequence_v0
          cargo test -p evo-codegen-rust --test sequence_compile
          cargo test -p evo-bench --test append_only_sequence_reference

      - name: Append-only sequence Vec parity gate
        run: cargo run -p evo-bench -- run benchmarks/cases/append-only-sequence-v0 --out target/evo-bench-append-only-sequence

      - name: Upload append-only sequence report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: evo-bench-append-only-sequence-ubuntu-24.04
          path: target/evo-bench-append-only-sequence/
          if-no-files-found: warn
''')
