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
    StrongRcCycleLeak,
    WeakEdgeCandidate,
    ArenaIndexCandidate,
    RequiresInteriorMutabilityDesign,
    RequiresConcurrencyDesign,
    SelfReferentialSeparate,
    RejectHiddenCost,
}

impl Classification {
    const fn as_str(self) -> &'static str {
        match self {
            Self::OwnedControl => "OWNED-CONTROL",
            Self::BorrowInstead => "BORROW-INSTEAD",
            Self::StrongRcCycleLeak => "STRONG-RC-CYCLE-LEAK",
            Self::WeakEdgeCandidate => "WEAK-EDGE-CANDIDATE",
            Self::ArenaIndexCandidate => "ARENA/INDEX-CANDIDATE",
            Self::RequiresInteriorMutabilityDesign => "REQUIRES-INTERIOR-MUTABILITY-DESIGN",
            Self::RequiresConcurrencyDesign => "REQUIRES-CONCURRENCY-DESIGN",
            Self::SelfReferentialSeparate => "SELF-REFERENTIAL-SEPARATE",
            Self::RejectHiddenCost => "REJECT-HIDDEN-COST",
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
        name: "acyclic-owned-tree-control",
        classification: Classification::OwnedControl,
        ownership_model: "single-owner-by-value-tree",
        allocation_model: "Box allocations only where tree edges request indirection",
        ownership_operations: "ordinary move/drop",
        expected_rust_compile: true,
        expected_stdout: Some("12"),
        reason: "acyclic trees do not require shared graph ownership machinery",
        rust_source: r#"
struct Node { value: i64, child: Option<Box<Node>> }
fn main() {
    let root = Node { value: 5, child: Some(Box::new(Node { value: 7, child: None })) };
    println!("{}", root.value + root.child.as_ref().unwrap().value);
}
"#,
    },
    CaseSpec {
        name: "borrow-only-graph-traversal-control",
        classification: Classification::BorrowInstead,
        ownership_model: "container-owned-nodes-borrowed-traversal",
        allocation_model: "one Vec allocation",
        ownership_operations: "shared borrows plus copied indices",
        expected_rust_compile: true,
        expected_stdout: Some("12"),
        reason: "read-only traversal needs no additional owner when the container outlives the traversal",
        rust_source: r#"
struct Node { value: i64, next: Option<usize> }
fn sum(nodes: &[Node], mut current: Option<usize>) -> i64 {
    let mut total = 0;
    while let Some(index) = current {
        total += nodes[index].value;
        current = nodes[index].next;
    }
    total
}
fn main() {
    let nodes = vec![Node { value: 5, next: Some(1) }, Node { value: 7, next: None }];
    println!("{}", sum(&nodes, Some(0)));
}
"#,
    },
    CaseSpec {
        name: "strong-rc-dag-explicit-duplication",
        classification: Classification::WeakEdgeCandidate,
        ownership_model: "single-thread-reference-counted-dag",
        allocation_model: "two Rc node allocations",
        ownership_operations: "explicit non-atomic strong-count increment/decrement",
        expected_rust_compile: true,
        expected_stdout: Some("2 12"),
        reason: "Rc handles represent shared DAG ownership but every extra owner has explicit count work",
        rust_source: r#"
use std::rc::Rc;
struct Node { value: i64 }
fn main() {
    let tail = Rc::new(Node { value: 7 });
    let edge = Rc::clone(&tail);
    let head = Rc::new(Node { value: 5 });
    println!("{} {}", Rc::strong_count(&tail), head.value + edge.value);
}
"#,
    },
    CaseSpec {
        name: "strong-rc-cycle-retains-counts",
        classification: Classification::StrongRcCycleLeak,
        ownership_model: "strong-reference-counted-cycle",
        allocation_model: "two Rc allocations plus RefCell edge slots",
        ownership_operations: "strong-count increments create a cycle that cannot reach zero by ordinary drop",
        expected_rust_compile: true,
        expected_stdout: Some("2 2"),
        reason: "strong Rc edges alone leak cycles; the count behavior is observable and must never be sold as cycle safety",
        rust_source: r#"
use std::cell::RefCell;
use std::rc::Rc;
struct Node { next: RefCell<Option<Rc<Node>>> }
fn main() {
    let left = Rc::new(Node { next: RefCell::new(None) });
    let right = Rc::new(Node { next: RefCell::new(None) });
    *left.next.borrow_mut() = Some(Rc::clone(&right));
    *right.next.borrow_mut() = Some(Rc::clone(&left));
    println!("{} {}", Rc::strong_count(&left), Rc::strong_count(&right));
}
"#,
    },
    CaseSpec {
        name: "weak-back-edge-breaks-cycle",
        classification: Classification::WeakEdgeCandidate,
        ownership_model: "strong-forward-weak-back-edge",
        allocation_model: "two Rc allocations",
        ownership_operations: "explicit Rc downgrade plus weak-count maintenance",
        expected_rust_compile: true,
        expected_stdout: Some("1 1 true"),
        reason: "Weak is a distinct non-owning edge kind that breaks a reference-count cycle explicitly",
        rust_source: r#"
use std::cell::RefCell;
use std::rc::{Rc, Weak};
struct Node { parent: RefCell<Weak<Node>> }
fn main() {
    let parent = Rc::new(Node { parent: RefCell::new(Weak::new()) });
    let child = Rc::new(Node { parent: RefCell::new(Weak::new()) });
    *child.parent.borrow_mut() = Rc::downgrade(&parent);
    let upgraded = child.parent.borrow().upgrade().is_some();
    println!("{} {} {}", Rc::strong_count(&parent), Rc::weak_count(&parent), upgraded);
}
"#,
    },
    CaseSpec {
        name: "weak-upgrade-after-final-strong-drop",
        classification: Classification::WeakEdgeCandidate,
        ownership_model: "weak-non-owning-edge",
        allocation_model: "one Rc allocation and one weak handle",
        ownership_operations: "downgrade, final strong drop, failed upgrade",
        expected_rust_compile: true,
        expected_stdout: Some("false"),
        reason: "dead targets are represented explicitly by failed Weak upgrade rather than dangling references",
        rust_source: r#"
use std::rc::Rc;
fn main() {
    let owner = Rc::new(7_i64);
    let weak = Rc::downgrade(&owner);
    drop(owner);
    println!("{}", weak.upgrade().is_some());
}
"#,
    },
    CaseSpec {
        name: "strong-weak-count-lifecycle",
        classification: Classification::WeakEdgeCandidate,
        ownership_model: "reference-counted-strong-and-weak-handles",
        allocation_model: "one Rc allocation",
        ownership_operations: "clone, downgrade, drop strong, drop weak",
        expected_rust_compile: true,
        expected_stdout: Some("2 1 1 1"),
        reason: "strong and weak edges have different count operations that must remain caller-visible",
        rust_source: r#"
use std::rc::Rc;
fn main() {
    let owner = Rc::new(7_i64);
    let alias = Rc::clone(&owner);
    let weak = Rc::downgrade(&owner);
    let a = Rc::strong_count(&owner);
    let b = Rc::weak_count(&owner);
    drop(alias);
    println!("{} {} {} {}", a, b, Rc::strong_count(&owner), Rc::weak_count(&owner));
    drop(weak);
}
"#,
    },
    CaseSpec {
        name: "arena-index-dag",
        classification: Classification::ArenaIndexCandidate,
        ownership_model: "container-owned-nodes-with-copyable-indices",
        allocation_model: "one Vec allocation",
        ownership_operations: "index copies; no per-edge refcount update",
        expected_rust_compile: true,
        expected_stdout: Some("12"),
        reason: "arena ownership can represent DAG edges without reference-count traffic",
        rust_source: r#"
struct Node { value: i64, next: Option<usize> }
fn main() {
    let nodes = vec![Node { value: 5, next: Some(1) }, Node { value: 7, next: None }];
    println!("{}", nodes[0].value + nodes[nodes[0].next.unwrap()].value);
}
"#,
    },
    CaseSpec {
        name: "arena-index-cycle",
        classification: Classification::ArenaIndexCandidate,
        ownership_model: "container-owned-cyclic-graph-with-indices",
        allocation_model: "one Vec allocation",
        ownership_operations: "index copies; bounded traversal over cyclic edges",
        expected_rust_compile: true,
        expected_stdout: Some("24"),
        reason: "a container can own cyclic graph nodes while edges remain ordinary copied indices",
        rust_source: r#"
struct Node { value: i64, next: usize }
fn main() {
    let nodes = vec![Node { value: 5, next: 1 }, Node { value: 7, next: 0 }];
    let mut index = 0usize;
    let mut sum = 0;
    for _ in 0..4 { sum += nodes[index].value; index = nodes[index].next; }
    println!("{}", sum);
}
"#,
    },
    CaseSpec {
        name: "arena-stale-index-safe-boundary",
        classification: Classification::ArenaIndexCandidate,
        ownership_model: "container-owned-nodes-with-checked-index",
        allocation_model: "one Vec allocation",
        ownership_operations: "checked lookup after removal",
        expected_rust_compile: true,
        expected_stdout: Some("false"),
        reason: "plain indices need an explicit stale-target policy; checked lookup can fail safely without fabricating validity",
        rust_source: r#"
fn main() {
    let mut nodes = vec![5_i64, 7_i64];
    let stale = 1usize;
    nodes.pop();
    println!("{}", nodes.get(stale).is_some());
}
"#,
    },
    CaseSpec {
        name: "candidate-traversal-parity",
        classification: Classification::ArenaIndexCandidate,
        ownership_model: "equivalent-rc-and-index-read-only-traversal",
        allocation_model: "Rc nodes versus one Vec",
        ownership_operations: "equivalent bounded traversal result",
        expected_rust_compile: true,
        expected_stdout: Some("12 12"),
        reason: "candidate comparisons must first prove equivalent traversal results before cost comparisons are meaningful",
        rust_source: r#"
use std::rc::Rc;
struct RcNode { value: i64, next: Option<Rc<RcNode>> }
struct IndexNode { value: i64, next: Option<usize> }
fn main() {
    let tail = Rc::new(RcNode { value: 7, next: None });
    let head = Rc::new(RcNode { value: 5, next: Some(Rc::clone(&tail)) });
    let rc_sum = head.value + head.next.as_ref().unwrap().value;
    let nodes = vec![IndexNode { value: 5, next: Some(1) }, IndexNode { value: 7, next: None }];
    let index_sum = nodes[0].value + nodes[nodes[0].next.unwrap()].value;
    println!("{} {}", rc_sum, index_sum);
}
"#,
    },
    CaseSpec {
        name: "edge-duplication-vs-payload-deep-clone",
        classification: Classification::RejectHiddenCost,
        ownership_model: "shared-edge-versus-independent-payload-copy",
        allocation_model: "one versus two Rc allocations",
        ownership_operations: "Rc clone compared with payload Clone plus allocation",
        expected_rust_compile: true,
        expected_stdout: Some("true false"),
        reason: "edge/handle duplication and payload cloning have different identity and allocation semantics",
        rust_source: r#"
use std::rc::Rc;
#[derive(Clone)]
struct Node { value: i64 }
fn main() {
    let owner = Rc::new(Node { value: 7 });
    let edge = Rc::clone(&owner);
    let deep = Rc::new((*owner).clone());
    let _ = edge.value + deep.value;
    println!("{} {}", Rc::ptr_eq(&owner, &edge), Rc::ptr_eq(&owner, &deep));
}
"#,
    },
    CaseSpec {
        name: "rc-refcell-mutation-boundary",
        classification: Classification::RequiresInteriorMutabilityDesign,
        ownership_model: "Rc-plus-runtime-borrow-checked-mutation",
        allocation_model: "one Rc allocation",
        ownership_operations: "refcount plus RefCell dynamic borrow state",
        expected_rust_compile: true,
        expected_stdout: Some("8"),
        reason: "shared mutation is a separate runtime-borrow-checking design, not a weak-edge feature",
        rust_source: r#"
use std::cell::RefCell;
use std::rc::Rc;
fn main() {
    let owner = Rc::new(RefCell::new(7_i64));
    let alias = Rc::clone(&owner);
    *alias.borrow_mut() += 1;
    println!("{}", owner.borrow());
}
"#,
    },
    CaseSpec {
        name: "arc-graph-concurrency-boundary",
        classification: Classification::RequiresConcurrencyDesign,
        ownership_model: "atomic-reference-counted-cross-thread-owner",
        allocation_model: "one Arc allocation",
        ownership_operations: "atomic count operations plus thread transfer",
        expected_rust_compile: true,
        expected_stdout: Some("7"),
        reason: "cross-thread graph ownership requires Arc/Send/Sync design and must not be an implicit Rc upgrade",
        rust_source: r#"
use std::sync::Arc;
use std::thread;
fn main() {
    let owner = Arc::new(7_i64);
    let alias = Arc::clone(&owner);
    println!("{}", thread::spawn(move || *alias).join().unwrap());
}
"#,
    },
    CaseSpec {
        name: "self-referential-local-rejected",
        classification: Classification::SelfReferentialSeparate,
        ownership_model: "address-sensitive-self-reference",
        allocation_model: "stack local",
        ownership_operations: "attempt to store a reference to self",
        expected_rust_compile: false,
        expected_stdout: None,
        reason: "self-referential address-sensitive values are a separate design problem from weak edges and arena indices",
        rust_source: r#"
struct Node<'a> { next: Option<&'a Node<'a>> }
fn main() {
    let mut node = Node { next: None };
    node.next = Some(&node);
    let _ = node.next.is_some();
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
        .arg("evo_cyclic_graph_ownership_case")
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
            (
                false,
                !spec.expected_rust_compile && !rust_compiled,
                String::new(),
                String::new(),
            )
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

fn write_reports(findings: &[Finding], out: &Path, git_sha: &str, rustc: &str) {
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
        "case,classification,ownership_model,allocation_model,ownership_operations,expected_rust_compile,rust_compiled,compile_expectation_matched,rust_ran,runtime_expectation_matched,stdout,reason,stderr_summary\n",
    );
    for item in findings {
        writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            csv_cell(item.spec.name),
            csv_cell(item.spec.classification.as_str()),
            csv_cell(item.spec.ownership_model),
            csv_cell(item.spec.allocation_model),
            csv_cell(item.spec.ownership_operations),
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
    writeln!(json, "  \"verdict\": \"SPLIT-RESEARCH\",").expect("writing JSON cannot fail");
    writeln!(json, "  \"case_count\": {},", findings.len()).expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"compile_expectation_mismatches\": {compile_mismatches},"
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"runtime_expectation_mismatches\": {runtime_mismatches},"
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"weak_edge_candidate_count\": {},",
        count(Classification::WeakEdgeCandidate)
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"arena_index_candidate_count\": {},",
        count(Classification::ArenaIndexCandidate)
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
        "  \"strong_cycle_leak_count\": {},",
        count(Classification::StrongRcCycleLeak)
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
        "  \"self_referential_separate_count\": {},",
        count(Classification::SelfReferentialSeparate)
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"hidden_cost_rejection_count\": {},",
        count(Classification::RejectHiddenCost)
    )
    .expect("writing JSON cannot fail");
    writeln!(json, "  \"cases\": [").expect("writing JSON cannot fail");
    for (index, item) in findings.iter().enumerate() {
        let comma = if index + 1 == findings.len() { "" } else { "," };
        writeln!(
            json,
            "    {{\"name\": {}, \"classification\": {}, \"ownership_model\": {}, \"allocation_model\": {}, \"ownership_operations\": {}, \"expected_rust_compile\": {}, \"rust_compiled\": {}, \"compile_expectation_matched\": {}, \"rust_ran\": {}, \"runtime_expectation_matched\": {}, \"stdout\": {}, \"reason\": {}, \"stderr_summary\": {}}}{comma}",
            json_string(item.spec.name),
            json_string(item.spec.classification.as_str()),
            json_string(item.spec.ownership_model),
            json_string(item.spec.allocation_model),
            json_string(item.spec.ownership_operations),
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
    writeln!(markdown, "# Cyclic graph ownership ergonomics v0 research")
        .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "- git_sha: `{git_sha}`").expect("writing Markdown cannot fail");
    writeln!(markdown, "- aggregate verdict: **SPLIT-RESEARCH**")
        .expect("writing Markdown cannot fail");
    writeln!(markdown, "- cases: **{}**", findings.len()).expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "- compile expectation mismatches: **{compile_mismatches}**"
    )
    .expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "- runtime expectation mismatches: **{runtime_mismatches}**"
    )
    .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "```text\n{rustc}\n```").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "| Case | Classification | Ownership model | Allocation | Ops | Compile | Run/output match |").expect("writing Markdown cannot fail");
    writeln!(markdown, "| --- | --- | --- | --- | --- | --- | --- |")
        .expect("writing Markdown cannot fail");
    for item in findings {
        writeln!(
            markdown,
            "| `{}` | {} | {} | {} | {} | {}/{} | {} |",
            item.spec.name,
            item.spec.classification.as_str(),
            item.spec.ownership_model,
            item.spec.allocation_model,
            item.spec.ownership_operations,
            item.rust_compiled,
            item.spec.expected_rust_compile,
            item.runtime_expectation_matched,
        )
        .expect("writing Markdown cannot fail");
    }
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "## Boundary").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "This matrix keeps strong `Rc` cycles, explicit `Weak` edges, arena/index ownership, shared mutation, cross-thread ownership, payload cloning and self-reference as separate cost/safety models. It introduces no Evolution syntax or runtime behavior.").expect("writing Markdown cannot fail");
    fs::write(out.join("report.md"), &markdown)
        .unwrap_or_else(|error| panic!("failed to write report Markdown: {error}"));
    print!("{markdown}");
}

#[test]
#[ignore = "research evidence; dedicated workflow runs this exact test"]
fn cyclic_graph_ownership_research_classifies_weak_and_arena_boundaries() {
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
        "evo-cyclic-graph-ownership-research-{}",
        std::process::id()
    ));
    if scratch.exists() {
        fs::remove_dir_all(&scratch).expect("stale research scratch should be removable");
    }
    fs::create_dir_all(&scratch).expect("research scratch should be creatable");

    let findings: Vec<_> = CASES.iter().map(|spec| run_case(spec, &scratch)).collect();
    let compile_mismatches = findings
        .iter()
        .filter(|item| !item.compile_expectation_matched)
        .count();
    let runtime_mismatches = findings
        .iter()
        .filter(|item| !item.runtime_expectation_matched)
        .count();
    assert_eq!(compile_mismatches, 0, "Rust compile expectations changed");
    assert_eq!(runtime_mismatches, 0, "Rust runtime expectations changed");
    assert!(CASES.len() >= 15);

    let out = env::var_os("EVO_CYCLIC_GRAPH_OWNERSHIP_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/evo-cyclic-graph-ownership-research")
        });
    let git_sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "unknown".to_owned());
    write_reports(&findings, &out, &git_sha, &rustc);

    fs::remove_dir_all(&scratch).expect("research scratch should be removable");
}
