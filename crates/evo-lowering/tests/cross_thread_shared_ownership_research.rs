use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
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

struct CaseSpec {
    name: &'static str,
    classification: Classification,
    ownership_model: &'static str,
    operations: &'static str,
    expected_compile: bool,
    expected_stdout: Option<&'static str>,
    source: &'static str,
}

struct Finding {
    spec: &'static CaseSpec,
    compiled: bool,
    ran: bool,
    expectation_matched: bool,
    stdout: String,
    stderr: String,
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
    let worker = thread::spawn(move || *worker_owner).join().unwrap();
    println!("{} {} {}", worker, *owner, Arc::strong_count(&owner));
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
impl Drop for Item { fn drop(&mut self) { DROPS.fetch_add(1, Ordering::SeqCst); } }
fn main() {
    { let owner = Arc::new(Item); let alias = Arc::clone(&owner); drop(alias); drop(owner); }
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
    thread::spawn(move || { alias.fetch_add(1, Ordering::SeqCst); }).join().unwrap();
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
    let _ = alias.0 + deep.0;
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

fn run_case(spec: &'static CaseSpec, root: &Path) -> Finding {
    let dir = root.join(spec.name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let source = dir.join("case.rs");
    let binary = dir.join(format!("case{}", env::consts::EXE_SUFFIX));
    fs::write(&source, spec.source).unwrap();
    let compile = Command::new(rustc_path())
        .arg("--edition=2024")
        .arg("-C")
        .arg("opt-level=3")
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    let compiled = compile.status.success();
    if compiled && spec.expected_compile {
        let run = Command::new(&binary).output().unwrap();
        let stdout = String::from_utf8_lossy(&run.stdout).trim().to_owned();
        let expected = spec.expected_stdout.unwrap_or_default();
        Finding {
            spec,
            compiled,
            ran: run.status.success(),
            expectation_matched: run.status.success() && stdout == expected,
            stdout,
            stderr: String::from_utf8_lossy(&run.stderr).trim().to_owned(),
        }
    } else {
        Finding {
            spec,
            compiled,
            ran: false,
            expectation_matched: compiled == spec.expected_compile,
            stdout: String::new(),
            stderr: String::from_utf8_lossy(&compile.stderr)
                .lines()
                .take(4)
                .collect::<Vec<_>>()
                .join(" | "),
        }
    }
}

#[test]
#[ignore = "research evidence; dedicated workflow should run this exact test"]
fn cross_thread_shared_ownership_research_classifies_arc_boundaries() {
    let version = Command::new(rustc_path()).arg("-Vv").output().unwrap();
    let version = String::from_utf8_lossy(&version.stdout).trim().to_owned();
    if env::var_os("EVO_REQUIRE_PINNED_RUSTC").is_some() {
        assert!(version.lines().next().is_some_and(|line| line.contains("rustc 1.98.0")));
    }
    let root = env::temp_dir().join("evo-cross-thread-shared-ownership-v0");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let findings = CASES.iter().map(|spec| run_case(spec, &root)).collect::<Vec<_>>();
    assert!(findings.iter().all(|finding| finding.expectation_matched));

    if let Some(out) = env::var_os("EVO_CROSS_THREAD_SHARED_OWNERSHIP_RESEARCH_OUT") {
        let out = Path::new(&out);
        fs::create_dir_all(out).unwrap();
        let git_sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "unknown".to_owned());
        let count = |class| findings.iter().filter(|f| f.spec.classification == class).count();
        let mut json = String::new();
        writeln!(json, "{{").unwrap();
        writeln!(json, "  \"git_sha\": {git_sha:?},").unwrap();
        writeln!(json, "  \"verdict\": \"SPLIT-RESEARCH\",").unwrap();
        writeln!(json, "  \"case_count\": {},", findings.len()).unwrap();
        writeln!(json, "  \"expectation_mismatches\": {},", findings.iter().filter(|f| !f.expectation_matched).count()).unwrap();
        writeln!(json, "  \"arc_candidate_count\": {},", count(Classification::ArcCandidate)).unwrap();
        writeln!(json, "  \"rc_owned_instead_count\": {},", count(Classification::RcOwnedInstead)).unwrap();
        writeln!(json, "  \"send_sync_boundary_count\": {},", count(Classification::RequiresSendSyncDesign)).unwrap();
        writeln!(json, "  \"synchronization_boundary_count\": {},", count(Classification::RequiresSynchronizationDesign)).unwrap();
        writeln!(json, "  \"weak_cycle_boundary_count\": {},", count(Classification::RequiresWeakCycleModel)).unwrap();
        writeln!(json, "  \"hidden_cost_rejection_count\": {},", count(Classification::RejectHiddenCost)).unwrap();
        writeln!(json, "  \"rustc_vv\": {version:?}").unwrap();
        writeln!(json, "}}").unwrap();
        fs::write(out.join("report.json"), json).unwrap();

        let mut markdown = String::from("# Cross-thread shared ownership research\n\n");
        writeln!(markdown, "- git_sha: `{git_sha}`").unwrap();
        writeln!(markdown, "- verdict: **SPLIT-RESEARCH**").unwrap();
        writeln!(markdown, "- cases: {}", findings.len()).unwrap();
        for finding in &findings {
            writeln!(markdown, "- `{}`: {} | compiled={} | ran={} | stdout=`{}` | ops={}", finding.spec.name, finding.spec.classification.as_str(), finding.compiled, finding.ran, finding.stdout, finding.spec.operations).unwrap();
            if !finding.stderr.is_empty() {
                writeln!(markdown, "  - stderr: `{}`", finding.stderr.replace('`', "'" )).unwrap();
            }
        }
        fs::write(out.join("report.md"), markdown).unwrap();
    }
}
