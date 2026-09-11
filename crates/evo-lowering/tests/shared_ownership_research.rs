use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Classification {
    OwnedControl,
    BorrowInstead,
    ExplicitSharedCandidate,
    RequiresInteriorMutabilityDesign,
    RequiresConcurrencyDesign,
    RequiresWeakCycleModel,
    ArenaIndexPreferred,
    RejectHiddenCostAmbiguous,
}

impl Classification {
    const fn as_str(self) -> &'static str {
        match self {
            Self::OwnedControl => "OWNED-CONTROL",
            Self::BorrowInstead => "BORROW-INSTEAD",
            Self::ExplicitSharedCandidate => "EXPLICIT-SHARED-CANDIDATE",
            Self::RequiresInteriorMutabilityDesign => "REQUIRES-INTERIOR-MUTABILITY-DESIGN",
            Self::RequiresConcurrencyDesign => "REQUIRES-CONCURRENCY-DESIGN",
            Self::RequiresWeakCycleModel => "REQUIRES-WEAK/CYCLE-MODEL",
            Self::ArenaIndexPreferred => "ARENA/INDEX-PREFERRED",
            Self::RejectHiddenCostAmbiguous => "REJECT-HIDDEN-COST/AMBIGUOUS",
        }
    }
}

#[derive(Debug)]
struct CaseSpec {
    name: &'static str,
    classification: Classification,
    ownership_model: &'static str,
    allocation_model: &'static str,
    ownership_operations: &'static str,
    current_evolution_expressible: bool,
    expected_rust_compile: bool,
    expected_stdout: Option<&'static str>,
    reason: &'static str,
    rust_source: &'static str,
}

#[derive(Debug)]
struct Finding {
    spec: &'static CaseSpec,
    rust_compiled: bool,
    compile_expectation_matched: bool,
    rust_ran: bool,
    runtime_expectation_matched: bool,
    stdout: String,
    stderr_summary: String,
}

const CASES: &[CaseSpec] = &[
    CaseSpec {
        name: "owned-control",
        classification: Classification::OwnedControl,
        ownership_model: "single-owner-by-value",
        allocation_model: "stack-only",
        ownership_operations: "move",
        current_evolution_expressible: true,
        expected_rust_compile: true,
        expected_stdout: Some("7"),
        reason: "plain ownership remains the cheapest model when aliases are unnecessary",
        rust_source: r#"
struct Item { value: i64 }
fn consume(item: Item) -> i64 { item.value }
fn main() { println!("{}", consume(Item { value: 7 })); }
"#,
    },
    CaseSpec {
        name: "borrow-control",
        classification: Classification::BorrowInstead,
        ownership_model: "single-owner-shared-borrow",
        allocation_model: "stack-only",
        ownership_operations: "borrow",
        current_evolution_expressible: true,
        expected_rust_compile: true,
        expected_stdout: Some("14"),
        reason: "two read-only users do not require multiple owners when one owner outlives both borrows",
        rust_source: r#"
struct Item { value: i64 }
fn read(item: &Item) -> i64 { item.value }
fn main() { let item = Item { value: 7 }; println!("{}", read(&item) + read(&item)); }
"#,
    },
    CaseSpec {
        name: "rc-local-alias-clone-read-drop",
        classification: Classification::ExplicitSharedCandidate,
        ownership_model: "single-thread-reference-counted",
        allocation_model: "one Rc allocation",
        ownership_operations: "non-atomic strong-count increment/decrement",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("2 7 1"),
        reason: "two independently storable owners require a real shared-owner operation rather than a borrow",
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let alias = Rc::clone(&owner);
    let count_with_alias = Rc::strong_count(&owner);
    let value = alias.value;
    drop(alias);
    println!("{} {} {}", count_with_alias, value, Rc::strong_count(&owner));
}
"#,
    },
    CaseSpec {
        name: "rc-function-forwarding",
        classification: Classification::ExplicitSharedCandidate,
        ownership_model: "single-thread-reference-counted",
        allocation_model: "one Rc allocation",
        ownership_operations: "explicit handle clone followed by handle move/drop",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("2 7 1"),
        reason: "passing or returning a shared handle by value is distinct from passing the underlying nominal value",
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn forward(item: Rc<Item>) -> Rc<Item> { item }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let forwarded = forward(Rc::clone(&owner));
    let count_with_forwarded = Rc::strong_count(&owner);
    let value = forwarded.value;
    drop(forwarded);
    println!("{} {} {}", count_with_forwarded, value, Rc::strong_count(&owner));
}
"#,
    },
    CaseSpec {
        name: "rc-branch-repeat-clone-drop",
        classification: Classification::ExplicitSharedCandidate,
        ownership_model: "single-thread-reference-counted",
        allocation_model: "one Rc allocation",
        ownership_operations: "repeated explicit non-atomic handle clone/drop",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("21 1"),
        reason: "control-flow handle duplication has observable refcount work even when the payload is never deep-cloned",
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let mut sum = 0;
    for _ in 0..3 {
        let alias = Rc::clone(&owner);
        sum += alias.value;
        drop(alias);
    }
    println!("{} {}", sum, Rc::strong_count(&owner));
}
"#,
    },
    CaseSpec {
        name: "rc-reference-derived-retained-handle",
        classification: Classification::ExplicitSharedCandidate,
        ownership_model: "borrow-from-single-thread-shared-handle",
        allocation_model: "one Rc allocation",
        ownership_operations: "borrow plus independent handle clone/drop",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("7 1"),
        reason: "a reference derived from an Rc handle is still an ordinary borrow and remains tied to a live handle",
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let alias = Rc::clone(&owner);
    let borrowed: &Item = &owner;
    drop(alias);
    println!("{} {}", borrowed.value, Rc::strong_count(&owner));
}
"#,
    },
    CaseSpec {
        name: "rc-reference-derived-handle-move-conflict",
        classification: Classification::ExplicitSharedCandidate,
        ownership_model: "borrow-from-single-thread-shared-handle",
        allocation_model: "one Rc allocation",
        ownership_operations: "borrow then invalid move of borrowed handle",
        current_evolution_expressible: false,
        expected_rust_compile: false,
        expected_stdout: None,
        reason: "shared ownership does not erase ordinary borrow liveness for the particular handle used as the reference source",
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let borrowed: &Item = &owner;
    let moved = owner;
    println!("{}", borrowed.value);
    drop(moved);
}
"#,
    },
    CaseSpec {
        name: "arc-cross-thread",
        classification: Classification::RequiresConcurrencyDesign,
        ownership_model: "thread-safe-reference-counted",
        allocation_model: "one Arc allocation",
        ownership_operations: "atomic strong-count increment/decrement plus thread transfer",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("7 1"),
        reason: "cross-thread ownership requires Arc-like thread-safety semantics rather than silently upgrading an Rc-like handle",
        rust_source: r#"
use std::sync::Arc;
use std::thread;
struct Item { value: i64 }
fn main() {
    let owner = Arc::new(Item { value: 7 });
    let alias = Arc::clone(&owner);
    let value = thread::spawn(move || alias.value).join().expect("thread must join");
    println!("{} {}", value, Arc::strong_count(&owner));
}
"#,
    },
    CaseSpec {
        name: "rc-cross-thread-rejected",
        classification: Classification::RequiresConcurrencyDesign,
        ownership_model: "single-thread-reference-counted",
        allocation_model: "one Rc allocation",
        ownership_operations: "attempted thread transfer",
        current_evolution_expressible: false,
        expected_rust_compile: false,
        expected_stdout: None,
        reason: "Rc is intentionally not Send, proving one-thread and cross-thread shared ownership cannot be one implicit model",
        rust_source: r#"
use std::rc::Rc;
use std::thread;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    thread::spawn(move || println!("{}", owner.value)).join().unwrap();
}
"#,
    },
    CaseSpec {
        name: "rc-refcell-shared-mutation",
        classification: Classification::RequiresInteriorMutabilityDesign,
        ownership_model: "single-thread-reference-counted-plus-runtime-borrow-checking",
        allocation_model: "one Rc allocation",
        ownership_operations: "refcount operations plus RefCell runtime borrow state",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("8"),
        reason: "shared mutation is not a property of Rc itself and requires an explicit interior-mutability design",
        rust_source: r#"
use std::cell::RefCell;
use std::rc::Rc;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(RefCell::new(Item { value: 7 }));
    let alias = Rc::clone(&owner);
    alias.borrow_mut().value += 1;
    println!("{}", owner.borrow().value);
}
"#,
    },
    CaseSpec {
        name: "arc-mutex-shared-mutation",
        classification: Classification::RequiresConcurrencyDesign,
        ownership_model: "thread-safe-reference-counted-plus-lock",
        allocation_model: "one Arc allocation",
        ownership_operations: "atomic refcount operations plus Mutex lock/unlock",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("8"),
        reason: "cross-thread shared mutation adds synchronization semantics and cost beyond Arc handle ownership",
        rust_source: r#"
use std::sync::{Arc, Mutex};
use std::thread;
struct Item { value: i64 }
fn main() {
    let owner = Arc::new(Mutex::new(Item { value: 7 }));
    let alias = Arc::clone(&owner);
    thread::spawn(move || alias.lock().unwrap().value += 1).join().unwrap();
    println!("{}", owner.lock().unwrap().value);
}
"#,
    },
    CaseSpec {
        name: "rc-weak-cycle-break",
        classification: Classification::RequiresWeakCycleModel,
        ownership_model: "single-thread-reference-counted-plus-weak-edge",
        allocation_model: "two Rc allocations",
        ownership_operations: "strong/weak count maintenance",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("1 1"),
        reason: "reference counting alone cannot represent cycle-safe graph ownership; weak edges are a distinct semantic concept",
        rust_source: r#"
use std::cell::RefCell;
use std::rc::{Rc, Weak};
struct Node { parent: RefCell<Weak<Node>> }
fn main() {
    let parent = Rc::new(Node { parent: RefCell::new(Weak::new()) });
    let child = Rc::new(Node { parent: RefCell::new(Weak::new()) });
    *child.parent.borrow_mut() = Rc::downgrade(&parent);
    println!("{} {}", Rc::strong_count(&parent), Rc::weak_count(&parent));
}
"#,
    },
    CaseSpec {
        name: "arena-index-graph",
        classification: Classification::ArenaIndexPreferred,
        ownership_model: "container-owned-nodes-with-stable-indices",
        allocation_model: "Vec allocation",
        ownership_operations: "index copy instead of refcount operations",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("12"),
        reason: "some graphs can avoid per-edge reference counting by centralizing ownership in an arena-like container",
        rust_source: r#"
struct Node { value: i64, next: Option<usize> }
fn main() {
    let nodes = vec![
        Node { value: 5, next: Some(1) },
        Node { value: 7, next: None },
    ];
    let mut current = Some(0usize);
    let mut sum = 0;
    while let Some(index) = current {
        sum += nodes[index].value;
        current = nodes[index].next;
    }
    println!("{}", sum);
}
"#,
    },
    CaseSpec {
        name: "handle-clone-vs-deep-clone",
        classification: Classification::RejectHiddenCostAmbiguous,
        ownership_model: "shared-handle-versus-independent-deep-copy",
        allocation_model: "two Rc allocations after deep clone",
        ownership_operations: "one handle refcount increment plus one payload Clone and allocation",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("2 1 false"),
        reason: "handle cloning and payload cloning have materially different semantics and cost, so one ambiguous implicit clone operation is unacceptable",
        rust_source: r#"
use std::rc::Rc;
#[derive(Clone)]
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let handle_clone = Rc::clone(&owner);
    let deep_clone = Rc::new((*owner).clone());
    let _ = handle_clone.value + deep_clone.value;
    println!(
        "{} {} {}",
        Rc::strong_count(&owner),
        Rc::strong_count(&deep_clone),
        Rc::ptr_eq(&owner, &deep_clone)
    );
}
"#,
    },
];

fn rustc_path() -> OsString {
    env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"))
}

fn rustc_version() -> String {
    let output = Command::new(rustc_path())
        .arg("-Vv")
        .output()
        .expect("failed to execute rustc -Vv");
    assert!(output.status.success(), "rustc -Vv failed");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn stderr_summary(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(5)
        .collect::<Vec<_>>()
        .join(" | ")
}

fn normalize_stdout(stdout: &[u8]) -> String {
    String::from_utf8_lossy(stdout).trim().to_owned()
}

fn run_case(spec: &'static CaseSpec, root: &Path) -> Finding {
    let case_dir = root.join(spec.name);
    if case_dir.exists() {
        fs::remove_dir_all(&case_dir)
            .unwrap_or_else(|error| panic!("failed to reset {}: {error}", case_dir.display()));
    }
    fs::create_dir_all(&case_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", case_dir.display()));
    let source = case_dir.join("case.rs");
    let binary = case_dir.join(format!("case-bin{}", env::consts::EXE_SUFFIX));
    fs::write(&source, spec.rust_source)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", source.display()));

    let compile = Command::new(rustc_path())
        .arg("--edition=2024")
        .arg("--crate-name")
        .arg("evo_shared_ownership_case")
        .arg("--crate-type")
        .arg("bin")
        .arg("-C")
        .arg("opt-level=3")
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap_or_else(|error| panic!("failed to execute rustc for {}: {error}", spec.name));
    let rust_compiled = compile.status.success();
    let compile_expectation_matched = rust_compiled == spec.expected_rust_compile;

    let (rust_ran, runtime_expectation_matched, stdout, runtime_stderr) =
        if rust_compiled && spec.expected_rust_compile {
            let run = Command::new(&binary)
                .output()
                .unwrap_or_else(|error| panic!("failed to execute {}: {error}", spec.name));
            let stdout = normalize_stdout(&run.stdout);
            let expected_stdout = spec.expected_stdout.unwrap_or_default();
            (
                run.status.success(),
                run.status.success() && stdout == expected_stdout,
                stdout,
                stderr_summary(&run.stderr),
            )
        } else {
            (false, !spec.expected_rust_compile && !rust_compiled, String::new(), String::new())
        };

    let compile_stderr = stderr_summary(&compile.stderr);
    let stderr_summary = if runtime_stderr.is_empty() {
        compile_stderr
    } else if compile_stderr.is_empty() {
        runtime_stderr
    } else {
        format!("compile: {compile_stderr} | runtime: {runtime_stderr}")
    };

    Finding {
        spec,
        rust_compiled,
        compile_expectation_matched,
        rust_ran,
        runtime_expectation_matched,
        stdout,
        stderr_summary,
    }
}

fn json_string(value: &str) -> String {
    format!("{value:?}")
}

fn csv_cell(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\"").replace('\n', "\\n"))
}

fn write_reports(findings: &[Finding], verdict: &str, out: &Path, git_sha: &str, rustc: &str) {
    fs::create_dir_all(out)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", out.display()));

    let compile_mismatches = findings
        .iter()
        .filter(|item| !item.compile_expectation_matched)
        .count();
    let runtime_mismatches = findings
        .iter()
        .filter(|item| !item.runtime_expectation_matched)
        .count();
    let count = |classification| {
        findings
            .iter()
            .filter(|item| item.spec.classification == classification)
            .count()
    };

    let mut csv = String::from(
        "case,classification,ownership_model,allocation_model,ownership_operations,current_evolution_expressible,expected_rust_compile,rust_compiled,compile_expectation_matched,rust_ran,runtime_expectation_matched,stdout,reason,stderr_summary\n",
    );
    for item in findings {
        writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            csv_cell(item.spec.name),
            csv_cell(item.spec.classification.as_str()),
            csv_cell(item.spec.ownership_model),
            csv_cell(item.spec.allocation_model),
            csv_cell(item.spec.ownership_operations),
            item.spec.current_evolution_expressible,
            item.spec.expected_rust_compile,
            item.rust_compiled,
            item.compile_expectation_matched,
            item.rust_ran,
            item.runtime_expectation_matched,
            csv_cell(&item.stdout),
            csv_cell(item.spec.reason),
            csv_cell(&item.stderr_summary),
        )
        .expect("writing CSV to String cannot fail");
    }
    fs::write(out.join("raw-matrix.csv"), csv)
        .unwrap_or_else(|error| panic!("failed to write raw matrix: {error}"));

    let mut json = String::new();
    writeln!(json, "{{").expect("writing JSON cannot fail");
    writeln!(json, "  \"git_sha\": {},", json_string(git_sha)).expect("writing JSON cannot fail");
    writeln!(json, "  \"rustc_vv\": {},", json_string(rustc)).expect("writing JSON cannot fail");
    writeln!(json, "  \"verdict\": {},", json_string(verdict)).expect("writing JSON cannot fail");
    writeln!(json, "  \"case_count\": {},", findings.len()).expect("writing JSON cannot fail");
    writeln!(json, "  \"compile_expectation_mismatches\": {compile_mismatches},")
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"runtime_expectation_mismatches\": {runtime_mismatches},")
        .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"explicit_shared_candidate_count\": {},",
        count(Classification::ExplicitSharedCandidate)
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"borrow_instead_count\": {},",
        count(Classification::BorrowInstead)
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"interior_mutability_boundary_count\": {},",
        count(Classification::RequiresInteriorMutabilityDesign)
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"concurrency_boundary_count\": {},",
        count(Classification::RequiresConcurrencyDesign)
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"weak_cycle_boundary_count\": {},",
        count(Classification::RequiresWeakCycleModel)
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"arena_index_alternative_count\": {},",
        count(Classification::ArenaIndexPreferred)
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"hidden_cost_ambiguity_count\": {},",
        count(Classification::RejectHiddenCostAmbiguous)
    )
    .expect("writing JSON cannot fail");
    writeln!(json, "  \"decision_basis\": \"SPLIT-RESEARCH when explicit one-thread shared handles have useful representable Rust semantics, while borrowing, concurrency, interior mutability, weak-cycle handling and arena ownership remain materially distinct cost/safety models\",")
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"cases\": [").expect("writing JSON cannot fail");
    for (index, item) in findings.iter().enumerate() {
        let comma = if index + 1 == findings.len() { "" } else { "," };
        writeln!(
            json,
            "    {{\"name\": {}, \"classification\": {}, \"ownership_model\": {}, \"allocation_model\": {}, \"ownership_operations\": {}, \"current_evolution_expressible\": {}, \"expected_rust_compile\": {}, \"rust_compiled\": {}, \"compile_expectation_matched\": {}, \"rust_ran\": {}, \"runtime_expectation_matched\": {}, \"stdout\": {}, \"reason\": {}, \"stderr_summary\": {}}}{comma}",
            json_string(item.spec.name),
            json_string(item.spec.classification.as_str()),
            json_string(item.spec.ownership_model),
            json_string(item.spec.allocation_model),
            json_string(item.spec.ownership_operations),
            item.spec.current_evolution_expressible,
            item.spec.expected_rust_compile,
            item.rust_compiled,
            item.compile_expectation_matched,
            item.rust_ran,
            item.runtime_expectation_matched,
            json_string(&item.stdout),
            json_string(item.spec.reason),
            json_string(&item.stderr_summary),
        )
        .expect("writing JSON cannot fail");
    }
    writeln!(json, "  ]").expect("writing JSON cannot fail");
    writeln!(json, "}}").expect("writing JSON cannot fail");
    fs::write(out.join("report.json"), json)
        .unwrap_or_else(|error| panic!("failed to write report JSON: {error}"));

    let mut markdown = String::new();
    writeln!(markdown, "# Shared ownership ergonomics v0 research").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "- git_sha: `{git_sha}`").expect("writing Markdown cannot fail");
    writeln!(markdown, "- aggregate verdict: **{verdict}**").expect("writing Markdown cannot fail");
    writeln!(markdown, "- cases: **{}**", findings.len()).expect("writing Markdown cannot fail");
    writeln!(markdown, "- compile expectation mismatches: **{compile_mismatches}**")
        .expect("writing Markdown cannot fail");
    writeln!(markdown, "- runtime expectation mismatches: **{runtime_mismatches}**")
        .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "```text\n{rustc}\n```").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "| Case | Classification | Ownership model | Allocation | Ops | Evolution today | Compile | Run/output match |")
        .expect("writing Markdown cannot fail");
    writeln!(markdown, "| --- | --- | --- | --- | --- | --- | --- | --- |")
        .expect("writing Markdown cannot fail");
    for item in findings {
        writeln!(
            markdown,
            "| `{}` | {} | {} | {} | {} | {} | {}/{} | {} |",
            item.spec.name,
            item.spec.classification.as_str(),
            item.spec.ownership_model,
            item.spec.allocation_model,
            item.spec.ownership_operations,
            item.spec.current_evolution_expressible,
            item.rust_compiled,
            item.spec.expected_rust_compile,
            item.runtime_expectation_matched,
        )
        .expect("writing Markdown cannot fail");
    }
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "## Decision").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "The matrix deliberately refuses to collapse borrowing, `Rc`, `Arc`, interior mutability, synchronization, weak edges and arena ownership into one magic aliasing feature. If all pre-registered expectations hold, the research verdict is **{verdict}**: explicit one-thread shared ownership may justify a bounded successor, while concurrency, interior mutability and cycle/graph policy remain separate research/design work.")
        .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "This is research evidence only. It introduces no Evolution shared-ownership syntax, semantic type, allocation policy or runtime behavior.")
        .expect("writing Markdown cannot fail");
    fs::write(out.join("report.md"), &markdown)
        .unwrap_or_else(|error| panic!("failed to write report Markdown: {error}"));
    print!("{markdown}");
}

#[test]
#[ignore = "research evidence; dedicated workflow runs this exact test"]
fn shared_ownership_research_classifies_explicit_cost_boundaries() {
    let rustc = rustc_version();
    if env::var_os("EVO_REQUIRE_PINNED_RUSTC").is_some() {
        assert!(
            rustc
                .lines()
                .next()
                .is_some_and(|line| line.contains("rustc 1.98.0")),
            "research workflow must use pinned Rust 1.98.0, got: {rustc}"
        );
    }

    let scratch = env::temp_dir().join(format!(
        "evo-shared-ownership-research-{}",
        std::process::id()
    ));
    if scratch.exists() {
        fs::remove_dir_all(&scratch)
            .unwrap_or_else(|error| panic!("failed to reset {}: {error}", scratch.display()));
    }
    fs::create_dir_all(&scratch)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", scratch.display()));

    let findings: Vec<Finding> = CASES.iter().map(|spec| run_case(spec, &scratch)).collect();
    let compile_mismatches = findings
        .iter()
        .filter(|item| !item.compile_expectation_matched)
        .count();
    let runtime_mismatches = findings
        .iter()
        .filter(|item| !item.runtime_expectation_matched)
        .count();
    let count = |classification| {
        findings
            .iter()
            .filter(|item| item.spec.classification == classification)
            .count()
    };
    let compile_rejections = findings
        .iter()
        .filter(|item| !item.spec.expected_rust_compile && !item.rust_compiled)
        .count();

    let split_boundary_complete = count(Classification::ExplicitSharedCandidate) >= 4
        && count(Classification::BorrowInstead) >= 1
        && count(Classification::RequiresInteriorMutabilityDesign) >= 1
        && count(Classification::RequiresConcurrencyDesign) >= 3
        && count(Classification::RequiresWeakCycleModel) >= 1
        && count(Classification::ArenaIndexPreferred) >= 1
        && count(Classification::RejectHiddenCostAmbiguous) >= 1
        && compile_rejections >= 2;

    let verdict = if compile_mismatches == 0 && runtime_mismatches == 0 && split_boundary_complete {
        "SPLIT-RESEARCH"
    } else if compile_mismatches == 0
        && runtime_mismatches == 0
        && count(Classification::ExplicitSharedCandidate) >= 2
    {
        "IMPLEMENT-CANDIDATE"
    } else {
        "DEFER"
    };

    let git_sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "local".to_owned());
    let out = env::var_os("EVO_SHARED_OWNERSHIP_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("evo-shared-ownership-research")
        });
    write_reports(&findings, verdict, &out, &git_sha, &rustc);

    assert_eq!(compile_mismatches, 0, "all Rust compile expectations must match");
    assert_eq!(runtime_mismatches, 0, "all successful probes must match expected output");
    assert!(
        count(Classification::ExplicitSharedCandidate) >= 4,
        "need multiple useful one-thread explicit shared-owner cases"
    );
    assert!(
        count(Classification::BorrowInstead) >= 1,
        "must preserve borrowing as the cheaper model where one owner is sufficient"
    );
    assert!(
        count(Classification::RequiresConcurrencyDesign) >= 3,
        "Rc/Arc and synchronized cross-thread behavior must remain distinct"
    );
    assert!(
        count(Classification::RequiresInteriorMutabilityDesign) >= 1,
        "shared mutation must remain a separate interior-mutability boundary"
    );
    assert!(
        count(Classification::RequiresWeakCycleModel) >= 1,
        "cycle-safe reference counting must expose weak-edge semantics"
    );
    assert!(
        count(Classification::ArenaIndexPreferred) >= 1,
        "graph ownership must compare a non-refcounted arena/index alternative"
    );
    assert!(
        count(Classification::RejectHiddenCostAmbiguous) >= 1,
        "handle clone and deep clone ambiguity must be rejected"
    );
    assert!(compile_rejections >= 2, "need fail-closed Rust ownership/thread-safety probes");
    assert_eq!(verdict, "SPLIT-RESEARCH");
}
