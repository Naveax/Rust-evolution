use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Classification {
    OwnBorrowInstead,
    CellCandidate,
    RefCellCandidate,
    ExplicitCompositionCandidate,
    RequiresGuardSurfaceResearch,
    RequiresConcurrencyDesign,
    RejectHiddenRuntimeCost,
}

impl Classification {
    const fn as_str(self) -> &'static str {
        match self {
            Self::OwnBorrowInstead => "OWN/BORROW-INSTEAD",
            Self::CellCandidate => "CELL-CANDIDATE",
            Self::RefCellCandidate => "REFCELL-CANDIDATE",
            Self::ExplicitCompositionCandidate => "EXPLICIT-COMPOSITION-CANDIDATE",
            Self::RequiresGuardSurfaceResearch => "REQUIRES-GUARD-SURFACE-RESEARCH",
            Self::RequiresConcurrencyDesign => "REQUIRES-CONCURRENCY-DESIGN",
            Self::RejectHiddenRuntimeCost => "REJECT-HIDDEN-RUNTIME-COST",
        }
    }
}

#[derive(Debug)]
struct CaseSpec {
    name: &'static str,
    classification: Classification,
    ownership_model: &'static str,
    allocation_model: &'static str,
    runtime_borrow_state: &'static str,
    dynamic_failure_model: &'static str,
    operations: &'static str,
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
        name: "owned-mutable-local",
        classification: Classification::OwnBorrowInstead,
        ownership_model: "single-owner-mutable-local",
        allocation_model: "stack-only",
        runtime_borrow_state: "none",
        dynamic_failure_model: "none",
        operations: "direct mutation",
        current_evolution_expressible: true,
        expected_rust_compile: true,
        expected_stdout: Some("8"),
        reason: "one-owner mutation needs no interior-mutability runtime state",
        rust_source: r#"
fn main() {
    let mut value = 7_i64;
    value += 1;
    println!("{}", value);
}
"#,
    },
    CaseSpec {
        name: "exclusive-mut-borrow",
        classification: Classification::OwnBorrowInstead,
        ownership_model: "exclusive-mutable-borrow",
        allocation_model: "stack-only",
        runtime_borrow_state: "none",
        dynamic_failure_model: "compile-time exclusivity",
        operations: "&mut borrow",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("8"),
        reason: "compile-time exclusive mutation is cheaper than dynamic borrow checking",
        rust_source: r#"
fn increment(value: &mut i64) { *value += 1; }
fn main() {
    let mut value = 7_i64;
    increment(&mut value);
    println!("{}", value);
}
"#,
    },
    CaseSpec {
        name: "cell-get-set",
        classification: Classification::CellCandidate,
        ownership_model: "shared-reference-cell-copy-in-copy-out",
        allocation_model: "stack-only",
        runtime_borrow_state: "Cell stored value, no borrow guard",
        dynamic_failure_model: "none",
        operations: "get/set",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("8"),
        reason: "copy-like scalar interior mutation can avoid RefCell guard machinery",
        rust_source: r#"
use std::cell::Cell;
fn main() {
    let value = Cell::new(7_i64);
    value.set(value.get() + 1);
    println!("{}", value.get());
}
"#,
    },
    CaseSpec {
        name: "cell-shared-aliases",
        classification: Classification::CellCandidate,
        ownership_model: "multiple-shared-references-to-cell",
        allocation_model: "stack-only",
        runtime_borrow_state: "Cell stored value, no borrow guard",
        dynamic_failure_model: "none",
        operations: "shared aliases perform get/set",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("9"),
        reason: "Cell permits explicit copy-like mutation through shared references without borrow guards",
        rust_source: r#"
use std::cell::Cell;
fn main() {
    let value = Cell::new(7_i64);
    let left = &value;
    let right = &value;
    left.set(left.get() + 1);
    right.set(right.get() + 1);
    println!("{}", value.get());
}
"#,
    },
    CaseSpec {
        name: "refcell-immutable-borrow",
        classification: Classification::RefCellCandidate,
        ownership_model: "single-owner-runtime-checked-shared-borrow",
        allocation_model: "stack-only",
        runtime_borrow_state: "RefCell borrow flag and Ref guard",
        dynamic_failure_model: "panic or fallible error on conflict",
        operations: "borrow guard",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("7"),
        reason: "RefCell immutable access creates a runtime-tracked guard rather than an ordinary untracked reference",
        rust_source: r#"
use std::cell::RefCell;
fn main() {
    let value = RefCell::new(7_i64);
    let read = value.borrow();
    println!("{}", *read);
}
"#,
    },
    CaseSpec {
        name: "refcell-mutable-borrow",
        classification: Classification::RefCellCandidate,
        ownership_model: "single-owner-runtime-checked-exclusive-borrow",
        allocation_model: "stack-only",
        runtime_borrow_state: "RefCell borrow flag and RefMut guard",
        dynamic_failure_model: "panic or fallible error on conflict",
        operations: "borrow_mut guard",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("8"),
        reason: "RefCell mutation is safe because exclusivity is checked dynamically and represented by a guard",
        rust_source: r#"
use std::cell::RefCell;
fn main() {
    let value = RefCell::new(7_i64);
    let mut write = value.borrow_mut();
    *write += 1;
    drop(write);
    println!("{}", *value.borrow());
}
"#,
    },
    CaseSpec {
        name: "guard-drop-then-mutable",
        classification: Classification::RefCellCandidate,
        ownership_model: "sequential-runtime-borrow-guards",
        allocation_model: "stack-only",
        runtime_borrow_state: "RefCell borrow flag",
        dynamic_failure_model: "guard scoped",
        operations: "borrow, explicit guard drop, borrow_mut",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("8"),
        reason: "the end of a dynamic guard is semantically relevant to when later mutation becomes legal",
        rust_source: r#"
use std::cell::RefCell;
fn main() {
    let value = RefCell::new(7_i64);
    let read = value.borrow();
    let previous = *read;
    drop(read);
    *value.borrow_mut() = previous + 1;
    println!("{}", *value.borrow());
}
"#,
    },
    CaseSpec {
        name: "overlap-immutable-mutable",
        classification: Classification::RequiresGuardSurfaceResearch,
        ownership_model: "overlapping-runtime-shared-and-exclusive-borrows",
        allocation_model: "stack-only",
        runtime_borrow_state: "RefCell borrow flag",
        dynamic_failure_model: "runtime panic on borrow_mut conflict",
        operations: "borrow then conflicting borrow_mut",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("borrow-conflict"),
        reason: "a surface that exposes RefCell semantics must make dynamic overlap failure explicit",
        rust_source: r#"
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
fn main() {
    let value = RefCell::new(7_i64);
    let read = value.borrow();
    let failed = catch_unwind(AssertUnwindSafe(|| {
        let _write = value.borrow_mut();
    })).is_err();
    drop(read);
    println!("{}", if failed { "borrow-conflict" } else { "missed" });
}
"#,
    },
    CaseSpec {
        name: "overlap-mutable-mutable",
        classification: Classification::RequiresGuardSurfaceResearch,
        ownership_model: "overlapping-runtime-exclusive-borrows",
        allocation_model: "stack-only",
        runtime_borrow_state: "RefCell borrow flag",
        dynamic_failure_model: "runtime panic on second borrow_mut",
        operations: "borrow_mut then conflicting borrow_mut",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("borrow-conflict"),
        reason: "a second dynamic exclusive borrow must fail rather than silently alias mutably",
        rust_source: r#"
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
fn main() {
    let value = RefCell::new(7_i64);
    let write = value.borrow_mut();
    let failed = catch_unwind(AssertUnwindSafe(|| {
        let _second = value.borrow_mut();
    })).is_err();
    drop(write);
    println!("{}", if failed { "borrow-conflict" } else { "missed" });
}
"#,
    },
    CaseSpec {
        name: "try-borrow-path",
        classification: Classification::RefCellCandidate,
        ownership_model: "fallible-runtime-exclusive-borrow",
        allocation_model: "stack-only",
        runtime_borrow_state: "RefCell borrow flag",
        dynamic_failure_model: "explicit Result error",
        operations: "borrow then try_borrow_mut",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("blocked"),
        reason: "RefCell has a non-panicking explicit failure path that a future surface may prefer",
        rust_source: r#"
use std::cell::RefCell;
fn main() {
    let value = RefCell::new(7_i64);
    let _read = value.borrow();
    let blocked = value.try_borrow_mut().is_err();
    println!("{}", if blocked { "blocked" } else { "missed" });
}
"#,
    },
    CaseSpec {
        name: "rc-refcell-shared-mutation",
        classification: Classification::ExplicitCompositionCandidate,
        ownership_model: "single-thread-reference-counted-plus-runtime-borrow-checking",
        allocation_model: "one Rc allocation",
        runtime_borrow_state: "Rc counters plus RefCell borrow flag",
        dynamic_failure_model: "panic or fallible error on borrow conflict",
        operations: "Rc clone plus borrow_mut",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("8"),
        reason: "shared ownership and interior mutability are two explicit composable costs",
        rust_source: r#"
use std::cell::RefCell;
use std::rc::Rc;
fn main() {
    let owner = Rc::new(RefCell::new(7_i64));
    let alias = Rc::clone(&owner);
    *alias.borrow_mut() += 1;
    println!("{}", *owner.borrow());
}
"#,
    },
    CaseSpec {
        name: "rc-refcell-alias-survives",
        classification: Classification::ExplicitCompositionCandidate,
        ownership_model: "reference-counted-owner-lifetime-plus-runtime-borrow-state",
        allocation_model: "one Rc allocation",
        runtime_borrow_state: "Rc counters plus RefCell borrow flag",
        dynamic_failure_model: "panic or fallible error on borrow conflict",
        operations: "Rc clone, source handle drop, borrow_mut through alias",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("8"),
        reason: "owner-handle lifetime and dynamic borrow state remain independent semantic dimensions",
        rust_source: r#"
use std::cell::RefCell;
use std::rc::Rc;
fn main() {
    let owner = Rc::new(RefCell::new(7_i64));
    let alias = Rc::clone(&owner);
    drop(owner);
    *alias.borrow_mut() += 1;
    println!("{}", *alias.borrow());
}
"#,
    },
    CaseSpec {
        name: "alias-vs-deep-clone",
        classification: Classification::RejectHiddenRuntimeCost,
        ownership_model: "shared-cell-alias-versus-independent-payload-copy",
        allocation_model: "two Rc allocations after deep copy",
        runtime_borrow_state: "two independent RefCell borrow flags",
        dynamic_failure_model: "none in this sequence",
        operations: "Rc handle clone versus payload clone+allocation",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("8 7 false"),
        reason: "sharing mutable identity and deep-cloning state have materially different semantics and cost",
        rust_source: r#"
use std::cell::RefCell;
use std::rc::Rc;
#[derive(Clone)]
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(RefCell::new(Item { value: 7 }));
    let alias = Rc::clone(&owner);
    let deep = Rc::new(RefCell::new(owner.borrow().clone()));
    alias.borrow_mut().value += 1;
    println!("{} {} {}", owner.borrow().value, deep.borrow().value, Rc::ptr_eq(&owner, &deep));
}
"#,
    },
    CaseSpec {
        name: "arc-mutex-contrast",
        classification: Classification::RequiresConcurrencyDesign,
        ownership_model: "atomic-reference-counted-plus-lock",
        allocation_model: "one Arc allocation",
        runtime_borrow_state: "Mutex lock state plus Arc atomics",
        dynamic_failure_model: "blocking/poisoning lock semantics",
        operations: "Arc clone plus Mutex lock",
        current_evolution_expressible: false,
        expected_rust_compile: true,
        expected_stdout: Some("8"),
        reason: "thread-safe shared mutation adds atomics and lock semantics beyond one-thread interior mutability",
        rust_source: r#"
use std::sync::{Arc, Mutex};
use std::thread;
fn main() {
    let owner = Arc::new(Mutex::new(7_i64));
    let alias = Arc::clone(&owner);
    thread::spawn(move || *alias.lock().expect("lock") += 1).join().expect("join");
    println!("{}", *owner.lock().expect("lock"));
}
"#,
    },
    CaseSpec {
        name: "plain-borrow-alternative",
        classification: Classification::OwnBorrowInstead,
        ownership_model: "single-owner-two-shared-borrows",
        allocation_model: "stack-only",
        runtime_borrow_state: "none",
        dynamic_failure_model: "none",
        operations: "ordinary shared borrows",
        current_evolution_expressible: true,
        expected_rust_compile: true,
        expected_stdout: Some("14"),
        reason: "read-only aliases should stay ordinary borrows instead of paying interior-mutability state",
        rust_source: r#"
fn read(value: &i64) -> i64 { *value }
fn main() {
    let value = 7_i64;
    println!("{}", read(&value) + read(&value));
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
        .arg("evo_interior_mutability_case")
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
        "case,classification,ownership_model,allocation_model,runtime_borrow_state,dynamic_failure_model,operations,current_evolution_expressible,expected_rust_compile,rust_compiled,compile_expectation_matched,rust_ran,runtime_expectation_matched,stdout,reason,stderr_summary\n",
    );
    for finding in findings {
        writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            csv_cell(finding.spec.name),
            csv_cell(finding.spec.classification.as_str()),
            csv_cell(finding.spec.ownership_model),
            csv_cell(finding.spec.allocation_model),
            csv_cell(finding.spec.runtime_borrow_state),
            csv_cell(finding.spec.dynamic_failure_model),
            csv_cell(finding.spec.operations),
            finding.spec.current_evolution_expressible,
            finding.spec.expected_rust_compile,
            finding.rust_compiled,
            finding.compile_expectation_matched,
            finding.rust_ran,
            finding.runtime_expectation_matched,
            csv_cell(&finding.stdout),
            csv_cell(finding.spec.reason),
            csv_cell(&finding.stderr_summary),
        )
        .expect("writing CSV cannot fail");
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
    writeln!(json, "  \"own_borrow_instead_count\": {},", count(Classification::OwnBorrowInstead))
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"cell_candidate_count\": {},", count(Classification::CellCandidate))
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"refcell_candidate_count\": {},", count(Classification::RefCellCandidate))
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"composition_candidate_count\": {},", count(Classification::ExplicitCompositionCandidate))
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"guard_surface_boundary_count\": {},", count(Classification::RequiresGuardSurfaceResearch))
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"concurrency_boundary_count\": {},", count(Classification::RequiresConcurrencyDesign))
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"hidden_runtime_cost_reject_count\": {},", count(Classification::RejectHiddenRuntimeCost))
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"decision_basis\": \"keep Cell, RefCell guard semantics, Rc composition, and lock-based concurrency as explicit separate cost models\",")
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"cases\": [").expect("writing JSON cannot fail");
    for (index, finding) in findings.iter().enumerate() {
        let comma = if index + 1 == findings.len() { "" } else { "," };
        writeln!(
            json,
            "    {{\"name\": {}, \"classification\": {}, \"ownership_model\": {}, \"allocation_model\": {}, \"runtime_borrow_state\": {}, \"dynamic_failure_model\": {}, \"operations\": {}, \"current_evolution_expressible\": {}, \"expected_rust_compile\": {}, \"rust_compiled\": {}, \"compile_expectation_matched\": {}, \"rust_ran\": {}, \"runtime_expectation_matched\": {}, \"stdout\": {}, \"reason\": {}, \"stderr_summary\": {}}}{comma}",
            json_string(finding.spec.name),
            json_string(finding.spec.classification.as_str()),
            json_string(finding.spec.ownership_model),
            json_string(finding.spec.allocation_model),
            json_string(finding.spec.runtime_borrow_state),
            json_string(finding.spec.dynamic_failure_model),
            json_string(finding.spec.operations),
            finding.spec.current_evolution_expressible,
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
    writeln!(markdown, "# Interior mutability ergonomics v0 research").expect("markdown");
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
fn interior_mutability_research_classifies_models() {
    let rustc = rustc_version();
    if env::var_os("EVO_REQUIRE_PINNED_RUSTC").is_some() {
        assert!(
            rustc.contains("rustc 1.98.0"),
            "research workflow requires pinned Rust 1.98.0; found {rustc}"
        );
    }

    let git_sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "local-unknown".to_owned());
    let out = env::var_os("EVO_INTERIOR_MUTABILITY_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../../target/evo-interior-mutability-research"));
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
            .filter(|finding| finding.spec.classification == Classification::CellCandidate)
            .count(),
        2
    );
    assert_eq!(
        findings
            .iter()
            .filter(|finding| finding.spec.classification == Classification::RefCellCandidate)
            .count(),
        4
    );
    assert_eq!(
        findings
            .iter()
            .filter(|finding| {
                finding.spec.classification == Classification::ExplicitCompositionCandidate
            })
            .count(),
        2
    );
}
