use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Classification {
    ArcCandidate,
    RcOwnedInstead,
    RequiresSendSyncDesign,
    RequiresSynchronizationDesign,
    RequiresWeakCycleModel,
    RejectHiddenCost,
}

impl Classification {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ArcCandidate => "ARC-CANDIDATE",
            Self::RcOwnedInstead => "RC/OWNED-INSTEAD",
            Self::RequiresSendSyncDesign => "REQUIRES-SEND-SYNC-CAPABILITY-DESIGN",
            Self::RequiresSynchronizationDesign => "REQUIRES-SYNCHRONIZATION-DESIGN",
            Self::RequiresWeakCycleModel => "REQUIRES-WEAK-CYCLE-MODEL",
            Self::RejectHiddenCost => "REJECT-HIDDEN-COST",
        }
    }
}

#[derive(Debug)]
struct CaseSpec {
    name: &'static str,
    classification: Classification,
    ownership_model: &'static str,
    operations: &'static str,
    expected_compile: bool,
    expected_stdout: Option<&'static str>,
    source: &'static str,
}

#[derive(Debug)]
struct Finding {
    spec: &'static CaseSpec,
    compiled: bool,
    ran: bool,
    expectation_matched: bool,
    stdout: String,
    stderr_summary: String,
}

const CASES: &[CaseSpec] = &[
    CaseSpec {
        name: "owned-thread-transfer-control",
        classification: Classification::RcOwnedInstead,
        ownership_model: "single-owned-Send-value",
        operations: "one move into worker",
        expected_compile: true,
        expected_stdout: Some("7"),
        source: r#"
use std::thread;
fn main() {
    let value = 7_i64;
    println!("{}", thread::spawn(move || value).join().unwrap());
}
"#,
    },
    CaseSpec {
        name: "rc-cross-thread-rejected",
        classification: Classification::RequiresSendSyncDesign,
        ownership_model: "single-thread-reference-counted",
        operations: "attempted Rc move into worker",
        expected_compile: false,
        expected_stdout: None,
        source: r#"
use std::rc::Rc;
use std::thread;
fn main() {
    let owner = Rc::new(7_i64);
    thread::spawn(move || println!("{}", owner)).join().unwrap();
}
"#,
    },
    CaseSpec {
        name: "arc-clone-read-join",
        classification: Classification::ArcCandidate,
        ownership_model: "atomic-reference-counted-immutable",
        operations: "one Arc clone, worker move, read, join",
        expected_compile: true,
        expected_stdout: Some("7 1"),
        source: r#"
use std::sync::Arc;
use std::thread;
fn main() {
    let owner = Arc::new(7_i64);
    let alias = Arc::clone(&owner);
    let value = thread::spawn(move || *alias).join().unwrap();
    println!("{} {}", value, Arc::strong_count(&owner));
}
"#,
    },
    CaseSpec {
        name: "arc-strong-count-lifecycle",
        classification: Classification::ArcCandidate,
        ownership_model: "atomic-reference-counted-immutable",
        operations: "clone and atomic strong-count decrement",
        expected_compile: true,
        expected_stdout: Some("2 1"),
        source: r#"
use std::sync::Arc;
fn main() {
    let owner = Arc::new(7_i64);
    let alias = Arc::clone(&owner);
    let before = Arc::strong_count(&owner);
    drop(alias);
    println!("{} {}", before, Arc::strong_count(&owner));
}
"#,
    },
    CaseSpec {
        name: "arc-by-value-worker-forward",
        classification: Classification::ArcCandidate,
        ownership_model: "atomic-reference-counted-immutable",
        operations: "Arc handle move through helper into worker",
        expected_compile: true,
        expected_stdout: Some("9"),
        source: r#"
use std::sync::Arc;
use std::thread;
fn forward(value: Arc<i64>) -> Arc<i64> { value }
fn main() {
    let owner = Arc::new(9_i64);
    println!("{}", thread::spawn(move || *forward(owner)).join().unwrap());
}
"#,
    },
    CaseSpec {
        name: "arc-duplicate-before-worker-retains-caller",
        classification: Classification::ArcCandidate,
        ownership_model: "atomic-reference-counted-immutable",
        operations: "explicit Arc clone before worker transfer",
        expected_compile: true,
        expected_stdout: Some("7 7 1"),
        source: r#"
use std::sync::Arc;
use std::thread;
fn main() {
    let owner = Arc::new(7_i64);
    let worker_owner = Arc::clone(&owner);
    let worker_value = thread::spawn(move || *worker_owner).join().unwrap();
    println!("{} {} {}", worker_value, *owner, Arc::strong_count(&owner));
}
"#,
    },
    CaseSpec {
        name: "arc-final-owner-drop",
        classification: Classification::ArcCandidate,
        ownership_model: "atomic-reference-counted-immutable",
        operations: "final strong owner drop destroys payload",
        expected_compile: true,
        expected_stdout: Some("1"),
        source: r#"
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
static DROPS: AtomicUsize = AtomicUsize::new(0);
struct Item;
impl Drop for Item {
    fn drop(&mut self) {
        DROPS.fetch_add(1, Ordering::SeqCst);
    }
}
fn main() {
    {
        let owner = Arc::new(Item);
        let alias = Arc::clone(&owner);
        drop(alias);
        drop(owner);
    }
    println!("{}", DROPS.load(Ordering::SeqCst));
}
"#,
    },
    CaseSpec {
        name: "arc-local-reference-scope",
        classification: Classification::ArcCandidate,
        ownership_model: "borrow-from-Arc-within-one-thread",
        operations: "ordinary shared reference tied to local Arc handle",
        expected_compile: true,
        expected_stdout: Some("7"),
        source: r#"
use std::sync::Arc;
fn main() {
    let owner = Arc::new(7_i64);
    let borrowed: &i64 = &owner;
    println!("{}", borrowed);
}
"#,
    },
    CaseSpec {
        name: "arc-non-send-payload-rejected",
        classification: Classification::RequiresSendSyncDesign,
        ownership_model: "Arc-around-non-Send-non-Sync-payload",
        operations: "attempted thread transfer",
        expected_compile: false,
        expected_stdout: None,
        source: r#"
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
fn main() {
    let owner = Arc::new(Rc::new(7_i64));
    thread::spawn(move || println!("{}", **owner)).join().unwrap();
}
"#,
    },
    CaseSpec {
        name: "arc-mutex-is-synchronization",
        classification: Classification::RequiresSynchronizationDesign,
        ownership_model: "Arc-plus-Mutex-shared-mutation",
        operations: "atomic refcount plus lock/unlock",
        expected_compile: true,
        expected_stdout: Some("8"),
        source: r#"
use std::sync::{Arc, Mutex};
use std::thread;
fn main() {
    let owner = Arc::new(Mutex::new(7_i64));
    let alias = Arc::clone(&owner);
    thread::spawn(move || *alias.lock().unwrap() += 1).join().unwrap();
    println!("{}", *owner.lock().unwrap());
}
"#,
    },
    CaseSpec {
        name: "arc-rwlock-is-synchronization",
        classification: Classification::RequiresSynchronizationDesign,
        ownership_model: "Arc-plus-RwLock-shared-mutation",
        operations: "atomic refcount plus read/write lock state",
        expected_compile: true,
        expected_stdout: Some("8"),
        source: r#"
use std::sync::{Arc, RwLock};
use std::thread;
fn main() {
    let owner = Arc::new(RwLock::new(7_i64));
    let alias = Arc::clone(&owner);
    thread::spawn(move || *alias.write().unwrap() += 1).join().unwrap();
    println!("{}", *owner.read().unwrap());
}
"#,
    },
    CaseSpec {
        name: "arc-atomic-payload-is-explicit-shared-state",
        classification: Classification::RequiresSynchronizationDesign,
        ownership_model: "Arc-plus-atomic-payload",
        operations: "atomic refcount plus payload atomic fetch_add/load",
        expected_compile: true,
        expected_stdout: Some("8"),
        source: r#"
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::thread;
fn main() {
    let owner = Arc::new(AtomicI64::new(7));
    let alias = Arc::clone(&owner);
    thread::spawn(move || {
        alias.fetch_add(1, Ordering::SeqCst);
    })
    .join()
    .unwrap();
    println!("{}", owner.load(Ordering::SeqCst));
}
"#,
    },
    CaseSpec {
        name: "arc-weak-is-separate-edge-model",
        classification: Classification::RequiresWeakCycleModel,
        ownership_model: "Arc-strong-plus-Weak-non-owning-edge",
        operations: "downgrade and upgrade",
        expected_compile: true,
        expected_stdout: Some("true false"),
        source: r#"
use std::sync::Arc;
fn main() {
    let owner = Arc::new(7_i64);
    let weak = Arc::downgrade(&owner);
    let live = weak.upgrade().is_some();
    drop(owner);
    println!("{} {}", live, weak.upgrade().is_some());
}
"#,
    },
    CaseSpec {
        name: "arc-handle-clone-vs-payload-clone",
        classification: Classification::RejectHiddenCost,
        ownership_model: "shared-owner-handle-versus-independent-payload-copy",
        operations: "one Arc clone versus payload clone plus allocation",
        expected_compile: true,
        expected_stdout: Some("true false"),
        source: r#"
use std::sync::Arc;
#[derive(Clone)]
struct Item(i64);
fn main() {
    let owner = Arc::new(Item(7));
    let alias = Arc::clone(&owner);
    let deep = Arc::new((*owner).clone());
    let _sum = alias.0 + deep.0;
    println!("{} {}", Arc::ptr_eq(&owner, &alias), Arc::ptr_eq(&owner, &deep));
}
"#,
    },
    CaseSpec {
        name: "single-thread-atomic-cost-unneeded",
        classification: Classification::RcOwnedInstead,
        ownership_model: "single-thread-alias-control",
        operations: "Rc clone without atomic count operations",
        expected_compile: true,
        expected_stdout: Some("14"),
        source: r#"
use std::rc::Rc;
fn main() {
    let owner = Rc::new(7_i64);
    let alias = Rc::clone(&owner);
    println!("{}", *owner + *alias);
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

fn summarize(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(6)
        .collect::<Vec<_>>()
        .join(" | ")
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
    fs::write(&source, spec.source)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", source.display()));

    let compile = Command::new(rustc_path())
        .arg("--edition=2024")
        .arg("--crate-name")
        .arg("evo_cross_thread_shared_ownership_case")
        .arg("-C")
        .arg("opt-level=3")
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap_or_else(|error| panic!("failed to execute rustc for {}: {error}", spec.name));
    let compiled = compile.status.success();

    if !compiled {
        return Finding {
            spec,
            compiled: false,
            ran: false,
            expectation_matched: !spec.expected_compile,
            stdout: String::new(),
            stderr_summary: summarize(&compile.stderr),
        };
    }

    if !spec.expected_compile {
        return Finding {
            spec,
            compiled: true,
            ran: false,
            expectation_matched: false,
            stdout: String::new(),
            stderr_summary: String::new(),
        };
    }

    let run = Command::new(&binary)
        .output()
        .unwrap_or_else(|error| panic!("failed to execute {}: {error}", spec.name));
    let stdout = String::from_utf8_lossy(&run.stdout).trim().to_owned();
    let expectation_matched = run.status.success()
        && spec
            .expected_stdout
            .is_none_or(|expected| stdout == expected);

    Finding {
        spec,
        compiled: true,
        ran: run.status.success(),
        expectation_matched,
        stdout,
        stderr_summary: summarize(&run.stderr),
    }
}

fn write_reports(findings: &[Finding], out: &Path, git_sha: &str, rustc: &str) {
    fs::create_dir_all(out)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", out.display()));
    let mismatches = findings
        .iter()
        .filter(|finding| !finding.expectation_matched)
        .count();
    let count = |classification| {
        findings
            .iter()
            .filter(|finding| finding.spec.classification == classification)
            .count()
    };

    let mut json = String::new();
    writeln!(json, "{{").expect("writing JSON cannot fail");
    writeln!(json, "  \"git_sha\": {git_sha:?},").expect("writing JSON cannot fail");
    writeln!(json, "  \"verdict\": \"SPLIT-RESEARCH\",").expect("writing JSON cannot fail");
    writeln!(json, "  \"case_count\": {},", findings.len()).expect("writing JSON cannot fail");
    writeln!(json, "  \"expectation_mismatches\": {mismatches},")
        .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"arc_candidate_count\": {},",
        count(Classification::ArcCandidate)
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"rc_owned_instead_count\": {},",
        count(Classification::RcOwnedInstead)
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"send_sync_boundary_count\": {},",
        count(Classification::RequiresSendSyncDesign)
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"synchronization_boundary_count\": {},",
        count(Classification::RequiresSynchronizationDesign)
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
        "  \"hidden_cost_rejection_count\": {},",
        count(Classification::RejectHiddenCost)
    )
    .expect("writing JSON cannot fail");
    writeln!(json, "  \"rustc_vv\": {rustc:?}").expect("writing JSON cannot fail");
    writeln!(json, "}}").expect("writing JSON cannot fail");
    fs::write(out.join("report.json"), json).expect("report JSON should be writable");

    let mut markdown = String::from("# Cross-thread shared ownership research\n\n");
    writeln!(markdown, "- git_sha: `{git_sha}`").expect("writing Markdown cannot fail");
    writeln!(markdown, "- aggregate verdict: **SPLIT-RESEARCH**")
        .expect("writing Markdown cannot fail");
    writeln!(markdown, "- cases: **{}**", findings.len()).expect("writing Markdown cannot fail");
    writeln!(markdown, "- expectation mismatches: **{mismatches}**")
        .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "```text\n{rustc}\n```").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "| Case | Classification | Ownership model | Operations | Compiled | Ran | Match |"
    )
    .expect("writing Markdown cannot fail");
    writeln!(markdown, "| --- | --- | --- | --- | --- | --- | --- |")
        .expect("writing Markdown cannot fail");
    for finding in findings {
        writeln!(
            markdown,
            "| `{}` | {} | {} | {} | {} | {} | {} |",
            finding.spec.name,
            finding.spec.classification.as_str(),
            finding.spec.ownership_model,
            finding.spec.operations,
            finding.compiled,
            finding.ran,
            finding.expectation_matched,
        )
        .expect("writing Markdown cannot fail");
        if !finding.stderr_summary.is_empty() {
            writeln!(
                markdown,
                "  - `{}` stderr: `{}`",
                finding.spec.name,
                finding.stderr_summary.replace('`', "'")
            )
            .expect("writing Markdown cannot fail");
        }
    }
    fs::write(out.join("report.md"), &markdown).expect("report Markdown should be writable");
    print!("{markdown}");
}

#[test]
#[ignore = "research evidence; dedicated workflow should run this exact test"]
fn cross_thread_shared_ownership_research_classifies_arc_boundaries() {
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
        "evo-cross-thread-shared-ownership-research-{}",
        std::process::id()
    ));
    if scratch.exists() {
        fs::remove_dir_all(&scratch).expect("stale research scratch should be removable");
    }
    fs::create_dir_all(&scratch).expect("research scratch should be creatable");

    let findings: Vec<_> = CASES.iter().map(|spec| run_case(spec, &scratch)).collect();
    assert_eq!(CASES.len(), 15);
    assert!(findings.iter().all(|finding| finding.expectation_matched));

    let out = env::var_os("EVO_CROSS_THREAD_SHARED_OWNERSHIP_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/evo-cross-thread-shared-ownership-research")
        });
    let git_sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "unknown".to_owned());
    write_reports(&findings, &out, &git_sha, &rustc);

    fs::remove_dir_all(&scratch).expect("research scratch should be removable");
}
