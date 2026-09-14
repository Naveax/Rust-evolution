use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Classification {
    OwnedMutInstead,
    RefCellCandidate,
    ExplicitRcRefCellComposition,
    RequiresConcurrencyDesign,
    RejectHiddenCost,
}

impl Classification {
    const fn as_str(self) -> &'static str {
        match self {
            Self::OwnedMutInstead => "OWNED-MUT-INSTEAD",
            Self::RefCellCandidate => "REFCELL-CANDIDATE",
            Self::ExplicitRcRefCellComposition => "EXPLICIT-RC-REFCELL-COMPOSITION",
            Self::RequiresConcurrencyDesign => "REQUIRES-CONCURRENCY-DESIGN",
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
    dynamic_operations: &'static str,
    expected_run_success: bool,
    expected_stdout: Option<&'static str>,
    expected_stderr_contains: Option<&'static str>,
    reason: &'static str,
    rust_source: &'static str,
}

#[derive(Debug)]
struct Finding {
    spec: &'static CaseSpec,
    rust_compiled: bool,
    rust_ran_successfully: bool,
    expectation_matched: bool,
    stdout: String,
    stderr_summary: String,
}

const CASES: &[CaseSpec] = &[
    CaseSpec {
        name: "ordinary-exclusive-mutable-owner-control",
        classification: Classification::OwnedMutInstead,
        ownership_model: "single-owned-mutable-value",
        allocation_model: "stack-only",
        dynamic_operations: "none",
        expected_run_success: true,
        expected_stdout: Some("8"),
        expected_stderr_contains: None,
        reason: "plain exclusive mutation should remain cheaper when aliasing does not require runtime borrow state",
        rust_source: r#"
fn main() {
    let mut value = 7_i64;
    value += 1;
    println!("{}", value);
}
"#,
    },
    CaseSpec {
        name: "owned-refcell-immutable-borrow",
        classification: Classification::RefCellCandidate,
        ownership_model: "single-owner-runtime-borrow-checked-cell",
        allocation_model: "stack-only RefCell payload",
        dynamic_operations: "borrow shared; release guard",
        expected_run_success: true,
        expected_stdout: Some("7"),
        expected_stderr_contains: None,
        reason: "RefCell permits shared access through runtime borrow state without changing ownership",
        rust_source: r#"
use std::cell::RefCell;
fn main() {
    let cell = RefCell::new(7_i64);
    let guard = cell.borrow();
    println!("{}", *guard);
}
"#,
    },
    CaseSpec {
        name: "owned-refcell-mutable-borrow",
        classification: Classification::RefCellCandidate,
        ownership_model: "single-owner-runtime-borrow-checked-cell",
        allocation_model: "stack-only RefCell payload",
        dynamic_operations: "borrow exclusive; mutate; release guard",
        expected_run_success: true,
        expected_stdout: Some("8"),
        expected_stderr_contains: None,
        reason: "interior mutation is a runtime-checked operation distinct from an ordinary shared borrow",
        rust_source: r#"
use std::cell::RefCell;
fn main() {
    let cell = RefCell::new(7_i64);
    *cell.borrow_mut() += 1;
    println!("{}", cell.borrow());
}
"#,
    },
    CaseSpec {
        name: "sequential-mutable-borrows-succeed",
        classification: Classification::RefCellCandidate,
        ownership_model: "single-owner-runtime-borrow-checked-cell",
        allocation_model: "stack-only RefCell payload",
        dynamic_operations: "exclusive guard acquire/release twice",
        expected_run_success: true,
        expected_stdout: Some("9"),
        expected_stderr_contains: None,
        reason: "dynamic exclusivity is released when each guard ends",
        rust_source: r#"
use std::cell::RefCell;
fn main() {
    let cell = RefCell::new(7_i64);
    { *cell.borrow_mut() += 1; }
    { *cell.borrow_mut() += 1; }
    println!("{}", cell.borrow());
}
"#,
    },
    CaseSpec {
        name: "immutable-then-mutable-overlap-panics",
        classification: Classification::RefCellCandidate,
        ownership_model: "single-owner-runtime-borrow-checked-cell",
        allocation_model: "stack-only RefCell payload",
        dynamic_operations: "shared guard remains live; conflicting exclusive borrow",
        expected_run_success: false,
        expected_stdout: None,
        expected_stderr_contains: Some("BorrowMutError"),
        reason: "RefCell conflict is a runtime failure and must not be disguised as compile-time borrowing",
        rust_source: r#"
use std::cell::RefCell;
fn main() {
    let cell = RefCell::new(7_i64);
    let shared = cell.borrow();
    let _exclusive = cell.borrow_mut();
    println!("{}", *shared);
}
"#,
    },
    CaseSpec {
        name: "mutable-then-immutable-overlap-panics",
        classification: Classification::RefCellCandidate,
        ownership_model: "single-owner-runtime-borrow-checked-cell",
        allocation_model: "stack-only RefCell payload",
        dynamic_operations: "exclusive guard remains live; conflicting shared borrow",
        expected_run_success: false,
        expected_stdout: None,
        expected_stderr_contains: Some("BorrowError"),
        reason: "an active exclusive dynamic borrow rejects later shared access at runtime",
        rust_source: r#"
use std::cell::RefCell;
fn main() {
    let cell = RefCell::new(7_i64);
    let exclusive = cell.borrow_mut();
    let _shared = cell.borrow();
    println!("{}", *exclusive);
}
"#,
    },
    CaseSpec {
        name: "multiple-immutable-borrows-succeed",
        classification: Classification::RefCellCandidate,
        ownership_model: "single-owner-runtime-borrow-checked-cell",
        allocation_model: "stack-only RefCell payload",
        dynamic_operations: "two simultaneous shared guards",
        expected_run_success: true,
        expected_stdout: Some("14"),
        expected_stderr_contains: None,
        reason: "dynamic shared borrows may coexist exactly as RefCell permits",
        rust_source: r#"
use std::cell::RefCell;
fn main() {
    let cell = RefCell::new(7_i64);
    let left = cell.borrow();
    let right = cell.borrow();
    println!("{}", *left + *right);
}
"#,
    },
    CaseSpec {
        name: "try-borrow-conflict-returns-error",
        classification: Classification::RefCellCandidate,
        ownership_model: "single-owner-runtime-borrow-checked-cell",
        allocation_model: "stack-only RefCell payload",
        dynamic_operations: "exclusive guard plus fallible shared acquire",
        expected_run_success: true,
        expected_stdout: Some("true"),
        expected_stderr_contains: None,
        reason: "fallible dynamic borrowing exposes conflict without panic",
        rust_source: r#"
use std::cell::RefCell;
fn main() {
    let cell = RefCell::new(7_i64);
    let exclusive = cell.borrow_mut();
    println!("{}", cell.try_borrow().is_err());
    drop(exclusive);
}
"#,
    },
    CaseSpec {
        name: "try-borrow-mut-conflict-returns-error",
        classification: Classification::RefCellCandidate,
        ownership_model: "single-owner-runtime-borrow-checked-cell",
        allocation_model: "stack-only RefCell payload",
        dynamic_operations: "shared guard plus fallible exclusive acquire",
        expected_run_success: true,
        expected_stdout: Some("true"),
        expected_stderr_contains: None,
        reason: "fallible exclusive acquisition is semantically distinct from panicking borrow_mut",
        rust_source: r#"
use std::cell::RefCell;
fn main() {
    let cell = RefCell::new(7_i64);
    let shared = cell.borrow();
    println!("{}", cell.try_borrow_mut().is_err());
    drop(shared);
}
"#,
    },
    CaseSpec {
        name: "guard-drop-releases-runtime-state",
        classification: Classification::RefCellCandidate,
        ownership_model: "single-owner-runtime-borrow-checked-cell",
        allocation_model: "stack-only RefCell payload",
        dynamic_operations: "shared guard explicit drop then exclusive acquire",
        expected_run_success: true,
        expected_stdout: Some("8"),
        expected_stderr_contains: None,
        reason: "guard lifetime directly controls runtime borrow-state release",
        rust_source: r#"
use std::cell::RefCell;
fn main() {
    let cell = RefCell::new(7_i64);
    let shared = cell.borrow();
    assert_eq!(*shared, 7);
    drop(shared);
    *cell.borrow_mut() += 1;
    println!("{}", cell.borrow());
}
"#,
    },
    CaseSpec {
        name: "rc-refcell-duplicate-observes-mutation",
        classification: Classification::ExplicitRcRefCellComposition,
        ownership_model: "explicit-shared-owner-plus-runtime-borrow-checked-cell",
        allocation_model: "one Rc allocation containing RefCell",
        dynamic_operations: "Rc clone; exclusive dynamic borrow; shared dynamic borrow",
        expected_run_success: true,
        expected_stdout: Some("8 2"),
        expected_stderr_contains: None,
        reason: "shared ownership and interior mutation compose explicitly and have separate observable costs",
        rust_source: r#"
use std::cell::RefCell;
use std::rc::Rc;
fn main() {
    let owner = Rc::new(RefCell::new(7_i64));
    let alias = Rc::clone(&owner);
    *alias.borrow_mut() += 1;
    println!("{} {}", owner.borrow(), Rc::strong_count(&owner));
}
"#,
    },
    CaseSpec {
        name: "rc-refcell-drop-one-owner-retains-other",
        classification: Classification::ExplicitRcRefCellComposition,
        ownership_model: "explicit-shared-owner-plus-runtime-borrow-checked-cell",
        allocation_model: "one Rc allocation containing RefCell",
        dynamic_operations: "Rc clone/drop independent of dynamic borrow state",
        expected_run_success: true,
        expected_stdout: Some("7 1"),
        expected_stderr_contains: None,
        reason: "owner lifetime and RefCell borrow state remain separate dimensions",
        rust_source: r#"
use std::cell::RefCell;
use std::rc::Rc;
fn main() {
    let owner = Rc::new(RefCell::new(7_i64));
    let alias = Rc::clone(&owner);
    drop(owner);
    println!("{} {}", alias.borrow(), Rc::strong_count(&alias));
}
"#,
    },
    CaseSpec {
        name: "handle-duplication-vs-payload-deep-clone",
        classification: Classification::RejectHiddenCost,
        ownership_model: "shared-cell-handle-versus-independent-cell-copy",
        allocation_model: "one shared allocation versus second independent allocation",
        dynamic_operations: "Rc clone compared with payload copy plus allocation",
        expected_run_success: true,
        expected_stdout: Some("true false"),
        expected_stderr_contains: None,
        reason: "implicit clone would blur identity, allocation and mutation visibility",
        rust_source: r#"
use std::cell::RefCell;
use std::rc::Rc;
fn main() {
    let owner = Rc::new(RefCell::new(7_i64));
    let alias = Rc::clone(&owner);
    let deep = Rc::new(RefCell::new(*owner.borrow()));
    println!("{} {}", Rc::ptr_eq(&owner, &alias), Rc::ptr_eq(&owner, &deep));
}
"#,
    },
    CaseSpec {
        name: "arc-mutex-concurrency-boundary",
        classification: Classification::RequiresConcurrencyDesign,
        ownership_model: "atomic-shared-owner-plus-lock",
        allocation_model: "one Arc allocation containing Mutex",
        dynamic_operations: "atomic refcount; lock/unlock; thread transfer",
        expected_run_success: true,
        expected_stdout: Some("8"),
        expected_stderr_contains: None,
        reason: "cross-thread shared mutation is synchronization design, not a transparent RefCell upgrade",
        rust_source: r#"
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
        name: "ordinary-mut-control-rejects-hidden-refcell-cost",
        classification: Classification::OwnedMutInstead,
        ownership_model: "single-owned-mutable-value",
        allocation_model: "stack-only",
        dynamic_operations: "none",
        expected_run_success: true,
        expected_stdout: Some("10"),
        expected_stderr_contains: None,
        reason: "when exclusive ownership is available, inserting RefCell would add needless runtime checking",
        rust_source: r#"
fn bump(value: &mut i64) { *value += 1; }
fn main() {
    let mut value = 9_i64;
    bump(&mut value);
    println!("{}", value);
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
        .take(8)
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
    fs::write(&source, spec.rust_source)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", source.display()));

    let compile = Command::new(rustc_path())
        .arg("--edition=2024")
        .arg("--crate-name")
        .arg("evo_interior_mutability_case")
        .arg("-C")
        .arg("opt-level=3")
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap_or_else(|error| panic!("failed to execute rustc for {}: {error}", spec.name));

    if !compile.status.success() {
        return Finding {
            spec,
            rust_compiled: false,
            rust_ran_successfully: false,
            expectation_matched: false,
            stdout: String::new(),
            stderr_summary: summarize(&compile.stderr),
        };
    }

    let run = Command::new(&binary)
        .output()
        .unwrap_or_else(|error| panic!("failed to execute {}: {error}", spec.name));
    let stdout = String::from_utf8_lossy(&run.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&run.stderr);
    let stdout_matches = spec
        .expected_stdout
        .is_none_or(|expected| stdout == expected);
    let stderr_matches = spec
        .expected_stderr_contains
        .is_none_or(|expected| stderr.contains(expected));
    let expectation_matched =
        run.status.success() == spec.expected_run_success && stdout_matches && stderr_matches;

    Finding {
        spec,
        rust_compiled: true,
        rust_ran_successfully: run.status.success(),
        expectation_matched,
        stdout,
        stderr_summary: summarize(&run.stderr),
    }
}

fn json_string(value: &str) -> String {
    format!("{value:?}")
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
    writeln!(json, "  \"git_sha\": {},", json_string(git_sha)).expect("writing JSON cannot fail");
    writeln!(json, "  \"rustc_vv\": {},", json_string(rustc)).expect("writing JSON cannot fail");
    writeln!(json, "  \"verdict\": \"SPLIT-RESEARCH\",").expect("writing JSON cannot fail");
    writeln!(json, "  \"case_count\": {},", findings.len()).expect("writing JSON cannot fail");
    writeln!(json, "  \"expectation_mismatches\": {mismatches},")
        .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"owned_mut_instead_count\": {},",
        count(Classification::OwnedMutInstead)
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"refcell_candidate_count\": {},",
        count(Classification::RefCellCandidate)
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"rc_refcell_composition_count\": {},",
        count(Classification::ExplicitRcRefCellComposition)
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
        "  \"hidden_cost_rejection_count\": {},",
        count(Classification::RejectHiddenCost)
    )
    .expect("writing JSON cannot fail");
    writeln!(json, "  \"cases\": [").expect("writing JSON cannot fail");
    for (index, finding) in findings.iter().enumerate() {
        let comma = if index + 1 == findings.len() { "" } else { "," };
        writeln!(
            json,
            "    {{\"name\": {}, \"classification\": {}, \"ownership_model\": {}, \"allocation_model\": {}, \"dynamic_operations\": {}, \"rust_compiled\": {}, \"rust_ran_successfully\": {}, \"expectation_matched\": {}, \"stdout\": {}, \"stderr_summary\": {}, \"reason\": {}}}{comma}",
            json_string(finding.spec.name),
            json_string(finding.spec.classification.as_str()),
            json_string(finding.spec.ownership_model),
            json_string(finding.spec.allocation_model),
            json_string(finding.spec.dynamic_operations),
            finding.rust_compiled,
            finding.rust_ran_successfully,
            finding.expectation_matched,
            json_string(&finding.stdout),
            json_string(&finding.stderr_summary),
            json_string(finding.spec.reason),
        )
        .expect("writing JSON cannot fail");
    }
    writeln!(json, "  ]").expect("writing JSON cannot fail");
    writeln!(json, "}}").expect("writing JSON cannot fail");
    fs::write(out.join("report.json"), json).expect("report JSON should be writable");

    let mut markdown = String::new();
    writeln!(markdown, "# Interior mutability ergonomics v0 research")
        .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
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
        "| Case | Classification | Dynamic operations | Run success | Match |"
    )
    .expect("writing Markdown cannot fail");
    writeln!(markdown, "| --- | --- | --- | --- | --- |").expect("writing Markdown cannot fail");
    for finding in findings {
        writeln!(
            markdown,
            "| `{}` | {} | {} | {} | {} |",
            finding.spec.name,
            finding.spec.classification.as_str(),
            finding.spec.dynamic_operations,
            finding.rust_ran_successfully,
            finding.expectation_matched,
        )
        .expect("writing Markdown cannot fail");
    }
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "This research keeps ownership, dynamic borrow state and synchronization as separate cost/safety dimensions. It introduces no Evolution production syntax or runtime behavior.").expect("writing Markdown cannot fail");
    fs::write(out.join("report.md"), &markdown).expect("report Markdown should be writable");
    print!("{markdown}");
}

#[test]
#[ignore = "research evidence; dedicated workflow runs this exact test"]
fn interior_mutability_research_classifies_dynamic_borrow_boundaries() {
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
        "evo-interior-mutability-research-{}",
        std::process::id()
    ));
    if scratch.exists() {
        fs::remove_dir_all(&scratch).expect("stale research scratch should be removable");
    }
    fs::create_dir_all(&scratch).expect("research scratch should be creatable");

    let findings: Vec<_> = CASES.iter().map(|spec| run_case(spec, &scratch)).collect();
    assert!(CASES.len() >= 15);
    assert!(findings.iter().all(|finding| finding.expectation_matched));

    let out = env::var_os("EVO_INTERIOR_MUTABILITY_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/evo-interior-mutability-research")
        });
    let git_sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "unknown".to_owned());
    write_reports(&findings, &out, &git_sha, &rustc);

    fs::remove_dir_all(&scratch).expect("research scratch should be removable");
}
