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
    StrongRcDag,
    RejectStrongCycle,
    WeakEdgeCandidate,
    ArenaIndexCandidate,
    ParityControl,
    RejectHiddenCost,
    RequiresInteriorMutabilityDesign,
    RequiresConcurrencyDesign,
    SelfReferentialSeparate,
}

impl Classification {
    const fn as_str(self) -> &'static str {
        match self {
            Self::OwnedControl => "OWNED-CONTROL",
            Self::BorrowInstead => "BORROW-INSTEAD",
            Self::StrongRcDag => "STRONG-RC-DAG",
            Self::RejectStrongCycle => "REJECT-STRONG-CYCLE",
            Self::WeakEdgeCandidate => "WEAK-EDGE-CANDIDATE",
            Self::ArenaIndexCandidate => "ARENA-INDEX-CANDIDATE",
            Self::ParityControl => "PARITY-CONTROL",
            Self::RejectHiddenCost => "REJECT-HIDDEN-COST",
            Self::RequiresInteriorMutabilityDesign => "REQUIRES-INTERIOR-MUTABILITY-DESIGN",
            Self::RequiresConcurrencyDesign => "REQUIRES-CONCURRENCY-DESIGN",
            Self::SelfReferentialSeparate => "SELF-REFERENTIAL-SEPARATE",
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
    hidden_runtime_state: &'static str,
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
        name: "owned-tree-control",
        classification: Classification::OwnedControl,
        ownership_model: "single-owner-by-value-tree",
        allocation_model: "stack-only",
        ownership_operations: "move",
        hidden_runtime_state: "none",
        expected_rust_compile: true,
        expected_stdout: Some("12"),
        reason: "plain ownership remains the cheapest model when graph aliases are unnecessary",
        rust_source: r#"
struct Node { value: i64, child: Option<Box<Node>> }
fn sum(node: &Node) -> i64 {
    node.value + node.child.as_deref().map_or(0, sum)
}
fn main() {
    let tree = Node { value: 5, child: Some(Box::new(Node { value: 7, child: None })) };
    println!("{}", sum(&tree));
}
"#,
    },
    CaseSpec {
        name: "borrow-only-traversal",
        classification: Classification::BorrowInstead,
        ownership_model: "single-owner-shared-borrow-traversal",
        allocation_model: "stack-only",
        ownership_operations: "borrow",
        hidden_runtime_state: "none",
        expected_rust_compile: true,
        expected_stdout: Some("12"),
        reason: "read-only traversal of an existing owner does not require graph ownership machinery",
        rust_source: r#"
struct Node { value: i64 }
fn sum(left: &Node, right: &Node) -> i64 { left.value + right.value }
fn main() {
    let left = Node { value: 5 };
    let right = Node { value: 7 };
    println!("{}", sum(&left, &right));
}
"#,
    },
    CaseSpec {
        name: "rc-dag-explicit",
        classification: Classification::StrongRcDag,
        ownership_model: "single-thread-reference-counted-DAG",
        allocation_model: "two Rc node allocations",
        ownership_operations: "explicit strong clone and drop",
        hidden_runtime_state: "Rc strong/weak counters",
        expected_rust_compile: true,
        expected_stdout: Some("12"),
        reason: "a DAG may need independently storable owners without introducing a cycle",
        rust_source: r#"
use std::rc::Rc;
struct Node { value: i64, next: Option<Rc<Node>> }
fn main() {
    let tail = Rc::new(Node { value: 7, next: None });
    let head = Rc::new(Node { value: 5, next: Some(Rc::clone(&tail)) });
    let sum = head.value + head.next.as_ref().expect("tail").value;
    println!("{}", sum);
}
"#,
    },
    CaseSpec {
        name: "rc-strong-cycle",
        classification: Classification::RejectStrongCycle,
        ownership_model: "strong-only-reference-count-cycle",
        allocation_model: "two Rc node allocations",
        ownership_operations: "explicit strong clones form a cycle",
        hidden_runtime_state: "Rc strong/weak counters plus RefCell used only to wire the probe",
        expected_rust_compile: true,
        expected_stdout: Some("retained"),
        reason: "strong reference counting alone retains a cycle after external owners are dropped",
        rust_source: r#"
use std::cell::RefCell;
use std::rc::{Rc, Weak};
struct Node { next: RefCell<Option<Rc<Node>>> }
fn main() {
    let left = Rc::new(Node { next: RefCell::new(None) });
    let right = Rc::new(Node { next: RefCell::new(None) });
    *left.next.borrow_mut() = Some(Rc::clone(&right));
    *right.next.borrow_mut() = Some(Rc::clone(&left));
    let observer: Weak<Node> = Rc::downgrade(&left);
    drop(left);
    drop(right);
    println!("{}", if observer.upgrade().is_some() { "retained" } else { "released" });
}
"#,
    },
    CaseSpec {
        name: "weak-back-edge",
        classification: Classification::WeakEdgeCandidate,
        ownership_model: "strong-owner-plus-nonowning-weak-back-edge",
        allocation_model: "two Rc node allocations",
        ownership_operations: "explicit downgrade and upgrade",
        hidden_runtime_state: "Rc strong/weak counters",
        expected_rust_compile: true,
        expected_stdout: Some("5 1 1"),
        reason: "a weak edge can point back without becoming another strong owner",
        rust_source: r#"
use std::rc::{Rc, Weak};
struct Parent { value: i64 }
struct Child { parent: Weak<Parent> }
fn main() {
    let parent = Rc::new(Parent { value: 5 });
    let child = Rc::new(Child { parent: Rc::downgrade(&parent) });
    let value = child.parent.upgrade().expect("parent is live").value;
    println!("{} {} {}", value, Rc::strong_count(&parent), Rc::weak_count(&parent));
}
"#,
    },
    CaseSpec {
        name: "weak-upgrade-after-drop",
        classification: Classification::WeakEdgeCandidate,
        ownership_model: "strong-owner-plus-nonowning-weak-edge",
        allocation_model: "one Rc allocation",
        ownership_operations: "downgrade, final strong drop, upgrade attempt",
        hidden_runtime_state: "Rc strong/weak counters",
        expected_rust_compile: true,
        expected_stdout: Some("dead"),
        reason: "weak edges must expose dead targets rather than silently extending ownership",
        rust_source: r#"
use std::rc::Rc;
fn main() {
    let owner = Rc::new(7_i64);
    let weak = Rc::downgrade(&owner);
    drop(owner);
    println!("{}", if weak.upgrade().is_some() { "live" } else { "dead" });
}
"#,
    },
    CaseSpec {
        name: "strong-weak-counts",
        classification: Classification::WeakEdgeCandidate,
        ownership_model: "single-thread-strong-plus-weak-reference-counting",
        allocation_model: "one Rc allocation",
        ownership_operations: "strong clone plus weak downgrade",
        hidden_runtime_state: "Rc strong/weak counters",
        expected_rust_compile: true,
        expected_stdout: Some("1 1 2 1"),
        reason: "strong and weak edges have distinct caller-visible count operations",
        rust_source: r#"
use std::rc::Rc;
fn main() {
    let owner = Rc::new(7_i64);
    let _weak = Rc::downgrade(&owner);
    let before_strong = Rc::strong_count(&owner);
    let before_weak = Rc::weak_count(&owner);
    let alias = Rc::clone(&owner);
    println!("{} {} {} {}", before_strong, before_weak, Rc::strong_count(&owner), Rc::weak_count(&owner));
    drop(alias);
}
"#,
    },
    CaseSpec {
        name: "arena-index-dag",
        classification: Classification::ArenaIndexCandidate,
        ownership_model: "container-owned-nodes-with-index-edges",
        allocation_model: "one Vec allocation",
        ownership_operations: "copy usize edge",
        hidden_runtime_state: "none beyond container storage",
        expected_rust_compile: true,
        expected_stdout: Some("12"),
        reason: "centralized node ownership can avoid per-edge reference-count operations",
        rust_source: r#"
struct Node { value: i64, next: Option<usize> }
fn main() {
    let nodes = vec![Node { value: 5, next: Some(1) }, Node { value: 7, next: None }];
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
        name: "arena-index-cycle",
        classification: Classification::ArenaIndexCandidate,
        ownership_model: "container-owned-nodes-with-cyclic-index-edges",
        allocation_model: "one Vec allocation",
        ownership_operations: "copy usize edge",
        hidden_runtime_state: "none beyond container storage",
        expected_rust_compile: true,
        expected_stdout: Some("24"),
        reason: "index relationships may form cycles without creating ownership cycles",
        rust_source: r#"
struct Node { value: i64, next: usize }
fn main() {
    let nodes = vec![Node { value: 5, next: 1 }, Node { value: 7, next: 0 }];
    let mut current = 0usize;
    let mut sum = 0;
    for _ in 0..4 {
        sum += nodes[current].value;
        current = nodes[current].next;
    }
    println!("{}", sum);
}
"#,
    },
    CaseSpec {
        name: "arena-generation-stale",
        classification: Classification::ArenaIndexCandidate,
        ownership_model: "container-owned-generational-handle",
        allocation_model: "slot storage",
        ownership_operations: "copy index+generation and validate on access",
        hidden_runtime_state: "generation counter per reusable slot",
        expected_rust_compile: true,
        expected_stdout: Some("stale"),
        reason: "removable arena nodes require an explicit stale-handle policy instead of unchecked indices",
        rust_source: r#"
#[derive(Clone, Copy)]
struct Handle { index: usize, generation: u64 }
struct Slot { generation: u64, value: Option<i64> }
fn main() {
    let handle = Handle { index: 0, generation: 4 };
    let mut slots = vec![Slot { generation: 4, value: Some(7) }];
    let slot = &mut slots[handle.index];
    slot.value = None;
    slot.generation += 1;
    let current = &slots[handle.index];
    let live = current.generation == handle.generation && current.value.is_some();
    println!("{}", if live { "live" } else { "stale" });
}
"#,
    },
    CaseSpec {
        name: "traversal-parity",
        classification: Classification::ParityControl,
        ownership_model: "equivalent-Rc-DAG-and-arena-index-traversal",
        allocation_model: "controlled model-specific allocations",
        ownership_operations: "explicit Rc clone versus index copy",
        hidden_runtime_state: "Rc counters on one side only",
        expected_rust_compile: true,
        expected_stdout: Some("12 12"),
        reason: "candidate performance comparisons require equivalent graph semantics before comparing ownership cost",
        rust_source: r#"
use std::rc::Rc;
struct RcNode { value: i64, next: Option<Rc<RcNode>> }
struct ArenaNode { value: i64, next: Option<usize> }
fn main() {
    let tail = Rc::new(RcNode { value: 7, next: None });
    let head = Rc::new(RcNode { value: 5, next: Some(Rc::clone(&tail)) });
    let rc_sum = head.value + head.next.as_ref().expect("tail").value;
    let nodes = vec![ArenaNode { value: 5, next: Some(1) }, ArenaNode { value: 7, next: None }];
    let arena_sum = nodes[0].value + nodes[nodes[0].next.expect("tail")].value;
    println!("{} {}", rc_sum, arena_sum);
}
"#,
    },
    CaseSpec {
        name: "edge-vs-deep-clone",
        classification: Classification::RejectHiddenCost,
        ownership_model: "shared-handle-alias-versus-independent-payload-copy",
        allocation_model: "two Rc allocations after deep clone",
        ownership_operations: "one strong clone versus payload clone plus allocation",
        hidden_runtime_state: "Rc counters",
        expected_rust_compile: true,
        expected_stdout: Some("2 1 false"),
        reason: "edge/handle duplication and payload deep clone have different identity and cost",
        rust_source: r#"
use std::rc::Rc;
#[derive(Clone)]
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let alias = Rc::clone(&owner);
    let deep = Rc::new((*owner).clone());
    let _ = alias.value + deep.value;
    println!("{} {} {}", Rc::strong_count(&owner), Rc::strong_count(&deep), Rc::ptr_eq(&owner, &deep));
}
"#,
    },
    CaseSpec {
        name: "rc-refcell-contrast",
        classification: Classification::RequiresInteriorMutabilityDesign,
        ownership_model: "single-thread-reference-counted-plus-runtime-borrow-checking",
        allocation_model: "one Rc allocation",
        ownership_operations: "strong clone plus RefCell mutable borrow",
        hidden_runtime_state: "Rc counters plus RefCell borrow flag",
        expected_rust_compile: true,
        expected_stdout: Some("8"),
        reason: "shared mutation adds runtime borrow-state semantics and must remain a separate design",
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
        name: "arc-thread-contrast",
        classification: Classification::RequiresConcurrencyDesign,
        ownership_model: "atomic-reference-counted-cross-thread",
        allocation_model: "one Arc allocation",
        ownership_operations: "atomic strong clone plus thread transfer",
        hidden_runtime_state: "Arc atomic counters plus OS/runtime thread state",
        expected_rust_compile: true,
        expected_stdout: Some("7"),
        reason: "cross-thread ownership changes both capability and cost and cannot be a silent Rc upgrade",
        rust_source: r#"
use std::sync::Arc;
use std::thread;
fn main() {
    let owner = Arc::new(7_i64);
    let alias = Arc::clone(&owner);
    let value = thread::spawn(move || *alias).join().expect("thread joins");
    println!("{}", value);
}
"#,
    },
    CaseSpec {
        name: "self-reference-move-conflict",
        classification: Classification::SelfReferentialSeparate,
        ownership_model: "address-sensitive-borrow-then-owner-move",
        allocation_model: "stack-only String storage",
        ownership_operations: "borrow then invalid move",
        hidden_runtime_state: "none",
        expected_rust_compile: false,
        expected_stdout: None,
        reason: "address stability/self-reference is distinct from graph-cycle ownership and needs separate Pin-like research",
        rust_source: r#"
struct Node<'a> { owned: String, view: &'a str }
fn main() {
    let owned = String::from("hello");
    let view = owned.as_str();
    let node = Node { owned, view };
    println!("{} {}", node.owned, node.view);
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
        .take(6)
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
            let expected = spec.expected_stdout.unwrap_or_default();
            (
                run.status.success(),
                run.status.success() && stdout == expected,
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

fn write_reports(findings: &[Finding], verdict: &str, out: &Path, git_sha: &str, rustc: &str) {
    fs::create_dir_all(out)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", out.display()));

    let compile_mismatches = findings
        .iter()
        .filter(|finding| !finding.compile_expectation_matched)
        .count();
    let runtime_mismatches = findings
        .iter()
        .filter(|finding| !finding.runtime_expectation_matched)
        .count();
    let count = |classification| {
        findings
            .iter()
            .filter(|finding| finding.spec.classification == classification)
            .count()
    };

    let mut csv = String::from(
        "case,classification,ownership_model,allocation_model,ownership_operations,hidden_runtime_state,expected_rust_compile,rust_compiled,compile_expectation_matched,rust_ran,runtime_expectation_matched,stdout,reason,stderr_summary\n",
    );
    for finding in findings {
        writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            csv_cell(finding.spec.name),
            csv_cell(finding.spec.classification.as_str()),
            csv_cell(finding.spec.ownership_model),
            csv_cell(finding.spec.allocation_model),
            csv_cell(finding.spec.ownership_operations),
            csv_cell(finding.spec.hidden_runtime_state),
            finding.spec.expected_rust_compile,
            finding.rust_compiled,
            finding.compile_expectation_matched,
            finding.rust_ran,
            finding.runtime_expectation_matched,
            csv_cell(&finding.stdout),
            csv_cell(finding.spec.reason),
            csv_cell(&finding.stderr_summary),
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
    writeln!(json, "  \"weak_edge_candidate_count\": {},", count(Classification::WeakEdgeCandidate))
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"arena_index_candidate_count\": {},", count(Classification::ArenaIndexCandidate))
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"borrow_instead_count\": {},", count(Classification::BorrowInstead))
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"rejected_strong_cycle_count\": {},", count(Classification::RejectStrongCycle))
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"interior_mutability_boundary_count\": {},", count(Classification::RequiresInteriorMutabilityDesign))
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"concurrency_boundary_count\": {},", count(Classification::RequiresConcurrencyDesign))
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"self_referential_boundary_count\": {},", count(Classification::SelfReferentialSeparate))
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"decision_basis\": \"compare explicit weak edges and arena/index ownership as separate cost models; reject strong-only cycles and keep mutation/concurrency/self-reference split\",")
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"cases\": [").expect("writing JSON cannot fail");
    for (index, finding) in findings.iter().enumerate() {
        let comma = if index + 1 == findings.len() { "" } else { "," };
        writeln!(
            json,
            "    {{\"name\": {}, \"classification\": {}, \"ownership_model\": {}, \"allocation_model\": {}, \"ownership_operations\": {}, \"hidden_runtime_state\": {}, \"expected_rust_compile\": {}, \"rust_compiled\": {}, \"compile_expectation_matched\": {}, \"rust_ran\": {}, \"runtime_expectation_matched\": {}, \"stdout\": {}, \"reason\": {}, \"stderr_summary\": {}}}{comma}",
            json_string(finding.spec.name),
            json_string(finding.spec.classification.as_str()),
            json_string(finding.spec.ownership_model),
            json_string(finding.spec.allocation_model),
            json_string(finding.spec.ownership_operations),
            json_string(finding.spec.hidden_runtime_state),
            finding.spec.expected_rust_compile,
            finding.rust_compiled,
            finding.compile_expectation_matched,
            finding.rust_ran,
            finding.runtime_expectation_matched,
            json_string(&finding.stdout),
            json_string(finding.spec.reason),
            json_string(&finding.stderr_summary),
        )
        .expect("writing JSON cannot fail");
    }
    writeln!(json, "  ]").expect("writing JSON cannot fail");
    writeln!(json, "}}").expect("writing JSON cannot fail");
    fs::write(out.join("report.json"), json)
        .unwrap_or_else(|error| panic!("failed to write report JSON: {error}"));

    let mut markdown = String::new();
    writeln!(markdown, "# Cyclic graph ownership ergonomics v0 research").expect("markdown");
    writeln!(markdown).expect("markdown");
    writeln!(markdown, "- Git SHA: `{git_sha}`").expect("markdown");
    writeln!(markdown, "- Verdict: **{verdict}**").expect("markdown");
    writeln!(markdown, "- Cases: {}", findings.len()).expect("markdown");
    writeln!(markdown, "- Compile expectation mismatches: {compile_mismatches}").expect("markdown");
    writeln!(markdown, "- Runtime expectation mismatches: {runtime_mismatches}").expect("markdown");
    writeln!(markdown).expect("markdown");
    writeln!(markdown, "| Case | Classification | Compile matched | Runtime matched | Output |").expect("markdown");
    writeln!(markdown, "| --- | --- | --- | --- | --- |").expect("markdown");
    for finding in findings {
        writeln!(
            markdown,
            "| `{}` | `{}` | {} | {} | `{}` |",
            finding.spec.name,
            finding.spec.classification.as_str(),
            finding.compile_expectation_matched,
            finding.runtime_expectation_matched,
            finding.stdout.replace('|', "\\|")
        )
        .expect("markdown");
    }
    fs::write(out.join("report.md"), markdown)
        .unwrap_or_else(|error| panic!("failed to write Markdown report: {error}"));
}

#[test]
#[ignore = "research probe invokes pinned rustc and writes evidence artifacts"]
fn cyclic_graph_ownership_research_classifies_models() {
    let rustc = rustc_version();
    if env::var_os("EVO_REQUIRE_PINNED_RUSTC").is_some() {
        assert!(
            rustc.contains("rustc 1.98.0"),
            "research workflow requires pinned Rust 1.98.0; found {rustc}"
        );
    }

    let git_sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "local-unknown".to_owned());
    let out = env::var_os("EVO_CYCLIC_GRAPH_OWNERSHIP_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../../target/evo-cyclic-graph-ownership-research"));
    let cases_root = out.join("cases");
    let findings: Vec<_> = CASES
        .iter()
        .map(|spec| run_case(spec, &cases_root))
        .collect();

    let compile_mismatches = findings
        .iter()
        .filter(|finding| !finding.compile_expectation_matched)
        .count();
    let runtime_mismatches = findings
        .iter()
        .filter(|finding| !finding.runtime_expectation_matched)
        .count();
    let verdict = if compile_mismatches == 0 && runtime_mismatches == 0 {
        "SPLIT-CANDIDATES"
    } else {
        "EVIDENCE-MISMATCH"
    };

    write_reports(&findings, verdict, &out, &git_sha, &rustc);

    assert_eq!(CASES.len(), 15);
    assert_eq!(compile_mismatches, 0, "compile expectations must match");
    assert_eq!(runtime_mismatches, 0, "runtime expectations must match");
    assert_eq!(verdict, "SPLIT-CANDIDATES");
    assert_eq!(
        findings
            .iter()
            .filter(|finding| finding.spec.classification == Classification::WeakEdgeCandidate)
            .count(),
        3
    );
    assert_eq!(
        findings
            .iter()
            .filter(|finding| finding.spec.classification == Classification::ArenaIndexCandidate)
            .count(),
        3
    );
}
