use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Classification {
    NoLifetimeRelation,
    OwnedSemanticsMustStayOwned,
    ElisionCandidateRequiresReferenceType,
    RequiresExplicitLifetimeRelation,
    UnsafeOrUnrepresentable,
}

impl Classification {
    const fn as_str(self) -> &'static str {
        match self {
            Self::NoLifetimeRelation => "NO-LIFETIME-RELATION",
            Self::OwnedSemanticsMustStayOwned => "OWNED-SEMANTICS-MUST-STAY-OWNED",
            Self::ElisionCandidateRequiresReferenceType => {
                "ELISION-CANDIDATE-REQUIRES-REFERENCE-TYPE"
            }
            Self::RequiresExplicitLifetimeRelation => "REQUIRES-EXPLICIT-LIFETIME-RELATION",
            Self::UnsafeOrUnrepresentable => "UNSAFE-OR-UNREPRESENTABLE",
        }
    }
}

#[derive(Debug)]
struct CaseSpec {
    name: &'static str,
    classification: Classification,
    possible_owner_sources: usize,
    result_escapes: bool,
    result_stored: bool,
    owner_overlap: &'static str,
    rust_lifetime_names_elided: bool,
    evolution_requires_reference_distinction: bool,
    expected_rust_compile: bool,
    reason: &'static str,
    rust_source: &'static str,
}

#[derive(Debug)]
struct Finding {
    spec: &'static CaseSpec,
    rust_compiled: bool,
    compile_expectation_matched: bool,
    stderr_summary: String,
}

const CASES: &[CaseSpec] = &[
    CaseSpec {
        name: "borrowed-input-scalar-field-return",
        classification: Classification::NoLifetimeRelation,
        possible_owner_sources: 1,
        result_escapes: false,
        result_stored: false,
        owner_overlap: "none",
        rust_lifetime_names_elided: true,
        evolution_requires_reference_distinction: false,
        expected_rust_compile: true,
        reason: "the returned i64 is owned scalar data and carries no borrow relation",
        rust_source: r#"
struct Item { value: i64 }
fn read_value(item: &Item) -> i64 { item.value }
fn main() { let item = Item { value: 7 }; assert_eq!(read_value(&item), 7); }
"#,
    },
    CaseSpec {
        name: "borrowed-input-computed-scalar-return",
        classification: Classification::NoLifetimeRelation,
        possible_owner_sources: 1,
        result_escapes: false,
        result_stored: false,
        owner_overlap: "none",
        rust_lifetime_names_elided: true,
        evolution_requires_reference_distinction: false,
        expected_rust_compile: true,
        reason: "computed scalar output is independent owned data",
        rust_source: r#"
struct Item { value: i64 }
fn next_value(item: &Item) -> i64 { item.value + 1 }
fn main() { let item = Item { value: 7 }; assert_eq!(next_value(&item), 8); }
"#,
    },
    CaseSpec {
        name: "fresh-nominal-owned-return",
        classification: Classification::OwnedSemanticsMustStayOwned,
        possible_owner_sources: 0,
        result_escapes: true,
        result_stored: true,
        owner_overlap: "none",
        rust_lifetime_names_elided: false,
        evolution_requires_reference_distinction: false,
        expected_rust_compile: true,
        reason: "fresh construction is an owned result and must not be reinterpreted as a borrow",
        rust_source: r#"
struct Item { value: i64 }
fn fresh() -> Item { Item { value: 7 } }
fn main() { let item = fresh(); assert_eq!(item.value, 7); }
"#,
    },
    CaseSpec {
        name: "current-owned-nominal-identity-return",
        classification: Classification::OwnedSemanticsMustStayOwned,
        possible_owner_sources: 1,
        result_escapes: true,
        result_stored: true,
        owner_overlap: "ownership-transferred",
        rust_lifetime_names_elided: false,
        evolution_requires_reference_distinction: false,
        expected_rust_compile: true,
        reason: "an existing T -> T contract transfers ownership and cannot silently become &T",
        rust_source: r#"
struct Item { value: i64 }
fn identity(item: Item) -> Item { item }
fn main() { let item = identity(Item { value: 7 }); assert_eq!(item.value, 7); }
"#,
    },
    CaseSpec {
        name: "single-source-borrowed-whole-return",
        classification: Classification::ElisionCandidateRequiresReferenceType,
        possible_owner_sources: 1,
        result_escapes: true,
        result_stored: true,
        owner_overlap: "borrow-live",
        rust_lifetime_names_elided: true,
        evolution_requires_reference_distinction: true,
        expected_rust_compile: true,
        reason: "Rust elides the single input lifetime, but the caller still receives a borrowed result",
        rust_source: r#"
struct Item { value: i64 }
fn borrow_item(item: &Item) -> &Item { item }
fn main() { let item = Item { value: 7 }; let borrowed = borrow_item(&item); assert_eq!(borrowed.value, 7); }
"#,
    },
    CaseSpec {
        name: "single-source-borrowed-nested-field-return",
        classification: Classification::ElisionCandidateRequiresReferenceType,
        possible_owner_sources: 1,
        result_escapes: true,
        result_stored: true,
        owner_overlap: "borrow-live",
        rust_lifetime_names_elided: true,
        evolution_requires_reference_distinction: true,
        expected_rust_compile: true,
        reason: "a nested field borrow remains tied to the sole borrowed input and needs no named lifetime",
        rust_source: r#"
struct Inner { value: i64 }
struct Outer { inner: Inner }
fn borrow_inner(outer: &Outer) -> &Inner { &outer.inner }
fn main() { let outer = Outer { inner: Inner { value: 7 } }; let inner = borrow_inner(&outer); assert_eq!(inner.value, 7); }
"#,
    },
    CaseSpec {
        name: "single-source-borrowed-result-immediate-inspection",
        classification: Classification::ElisionCandidateRequiresReferenceType,
        possible_owner_sources: 1,
        result_escapes: true,
        result_stored: false,
        owner_overlap: "statement-local",
        rust_lifetime_names_elided: true,
        evolution_requires_reference_distinction: true,
        expected_rust_compile: true,
        reason: "the signature is still borrowed even when the caller immediately inspects the result",
        rust_source: r#"
struct Item { value: i64 }
fn borrow_item(item: &Item) -> &Item { item }
fn main() { let item = Item { value: 7 }; assert_eq!(borrow_item(&item).value, 7); }
"#,
    },
    CaseSpec {
        name: "single-source-borrowed-result-stored-local",
        classification: Classification::ElisionCandidateRequiresReferenceType,
        possible_owner_sources: 1,
        result_escapes: true,
        result_stored: true,
        owner_overlap: "borrow-live-across-statements",
        rust_lifetime_names_elided: true,
        evolution_requires_reference_distinction: true,
        expected_rust_compile: true,
        reason: "named lifetime syntax is unnecessary, but the caller must track a first-class borrowed value",
        rust_source: r#"
struct Item { value: i64 }
fn borrow_item(item: &Item) -> &Item { item }
fn main() { let item = Item { value: 7 }; let borrowed = borrow_item(&item); let x = borrowed.value; assert_eq!(x, 7); }
"#,
    },
    CaseSpec {
        name: "single-source-borrowed-result-owner-move-conflict",
        classification: Classification::ElisionCandidateRequiresReferenceType,
        possible_owner_sources: 1,
        result_escapes: true,
        result_stored: true,
        owner_overlap: "move-while-borrow-live",
        rust_lifetime_names_elided: true,
        evolution_requires_reference_distinction: true,
        expected_rust_compile: false,
        reason: "elision does not remove borrow checking; moving the owner while the stored borrow is live must fail",
        rust_source: r#"
struct Item { value: i64 }
fn borrow_item(item: &Item) -> &Item { item }
fn main() { let item = Item { value: 7 }; let borrowed = borrow_item(&item); let moved = item; assert_eq!(borrowed.value, 7); let _ = moved; }
"#,
    },
    CaseSpec {
        name: "single-source-borrowed-result-owner-reinit-conflict",
        classification: Classification::ElisionCandidateRequiresReferenceType,
        possible_owner_sources: 1,
        result_escapes: true,
        result_stored: true,
        owner_overlap: "reinit-while-borrow-live",
        rust_lifetime_names_elided: true,
        evolution_requires_reference_distinction: true,
        expected_rust_compile: false,
        reason: "reinitializing an owner while its stored borrow remains live must fail",
        rust_source: r#"
struct Item { value: i64 }
fn borrow_item(item: &Item) -> &Item { item }
fn main() { let mut item = Item { value: 7 }; let borrowed = borrow_item(&item); item = Item { value: 8 }; assert_eq!(borrowed.value, 7); }
"#,
    },
    CaseSpec {
        name: "two-inputs-return-first-without-explicit-relation",
        classification: Classification::RequiresExplicitLifetimeRelation,
        possible_owner_sources: 2,
        result_escapes: true,
        result_stored: true,
        owner_overlap: "ambiguous-signature-source",
        rust_lifetime_names_elided: false,
        evolution_requires_reference_distinction: true,
        expected_rust_compile: false,
        reason: "Rust elision is signature-based and cannot choose one of two input lifetimes even when the body returns the first",
        rust_source: r#"
struct Item { value: i64 }
fn first(a: &Item, _b: &Item) -> &Item { a }
fn main() {}
"#,
    },
    CaseSpec {
        name: "two-input-branch-selected-borrow",
        classification: Classification::RequiresExplicitLifetimeRelation,
        possible_owner_sources: 2,
        result_escapes: true,
        result_stored: true,
        owner_overlap: "branch-selects-owner",
        rust_lifetime_names_elided: false,
        evolution_requires_reference_distinction: true,
        expected_rust_compile: false,
        reason: "a branch may select different owners, so the output relationship cannot be elided from multiple input lifetimes",
        rust_source: r#"
struct Item { value: i64 }
fn choose(a: &Item, b: &Item, flag: bool) -> &Item { if flag { a } else { b } }
fn main() {}
"#,
    },
    CaseSpec {
        name: "single-source-forwarded-borrowed-return",
        classification: Classification::ElisionCandidateRequiresReferenceType,
        possible_owner_sources: 1,
        result_escapes: true,
        result_stored: true,
        owner_overlap: "borrow-forwarded",
        rust_lifetime_names_elided: true,
        evolution_requires_reference_distinction: true,
        expected_rust_compile: true,
        reason: "single-source forwarding remains elidable in Rust, but Evolution would need a real borrowed-result type before representing it",
        rust_source: r#"
struct Item { value: i64 }
fn borrow_item(item: &Item) -> &Item { item }
fn forward(item: &Item) -> &Item { borrow_item(item) }
fn main() { let item = Item { value: 7 }; assert_eq!(forward(&item).value, 7); }
"#,
    },
    CaseSpec {
        name: "single-source-recursive-borrowed-return",
        classification: Classification::ElisionCandidateRequiresReferenceType,
        possible_owner_sources: 1,
        result_escapes: true,
        result_stored: true,
        owner_overlap: "recursive-borrow-forwarding",
        rust_lifetime_names_elided: true,
        evolution_requires_reference_distinction: true,
        expected_rust_compile: true,
        reason: "Rust can still elide a recursive single-source signature; this does not make the borrowed result an owned value",
        rust_source: r#"
struct Item { value: i64 }
fn recurse(item: &Item, n: u32) -> &Item { if n == 0 { item } else { recurse(item, n - 1) } }
fn main() { let item = Item { value: 7 }; assert_eq!(recurse(&item, 2).value, 7); }
"#,
    },
    CaseSpec {
        name: "borrowed-return-from-temporary",
        classification: Classification::UnsafeOrUnrepresentable,
        possible_owner_sources: 0,
        result_escapes: true,
        result_stored: true,
        owner_overlap: "temporary-would-die",
        rust_lifetime_names_elided: false,
        evolution_requires_reference_distinction: true,
        expected_rust_compile: false,
        reason: "there is no caller-owned source lifetime to attach the returned reference to",
        rust_source: r#"
struct Item { value: i64 }
fn bad() -> &Item { &Item { value: 7 } }
fn main() {}
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
        .take(4)
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
    let binary = case_dir.join("case-bin");
    fs::write(&source, spec.rust_source)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", source.display()));

    let output = Command::new(rustc_path())
        .arg("--edition=2024")
        .arg("--crate-name")
        .arg("evo_lifetime_elision_case")
        .arg("--crate-type")
        .arg("bin")
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap_or_else(|error| panic!("failed to execute rustc for {}: {error}", spec.name));
    let rust_compiled = output.status.success();

    Finding {
        spec,
        rust_compiled,
        compile_expectation_matched: rust_compiled == spec.expected_rust_compile,
        stderr_summary: stderr_summary(&output.stderr),
    }
}

fn json_string(value: &str) -> String {
    format!("{value:?}")
}

fn csv_cell(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\"").replace('\n', "\\n"))
}

fn write_reports(
    findings: &[Finding],
    verdict: &str,
    out: &Path,
    git_sha: &str,
    rustc: &str,
) {
    fs::create_dir_all(out)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", out.display()));

    let mismatches = findings
        .iter()
        .filter(|item| !item.compile_expectation_matched)
        .count();
    let elision_candidates = findings
        .iter()
        .filter(|item| item.spec.classification == Classification::ElisionCandidateRequiresReferenceType)
        .count();
    let explicit_relations = findings
        .iter()
        .filter(|item| item.spec.classification == Classification::RequiresExplicitLifetimeRelation)
        .count();
    let unsafe_cases = findings
        .iter()
        .filter(|item| item.spec.classification == Classification::UnsafeOrUnrepresentable)
        .count();

    let mut csv = String::from("case,classification,possible_owner_sources,result_escapes,result_stored,owner_overlap,rust_lifetime_names_elided,evolution_requires_reference_distinction,expected_rust_compile,rust_compiled,compile_expectation_matched,reason,stderr_summary\n");
    for item in findings {
        writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            csv_cell(item.spec.name),
            csv_cell(item.spec.classification.as_str()),
            item.spec.possible_owner_sources,
            item.spec.result_escapes,
            item.spec.result_stored,
            csv_cell(item.spec.owner_overlap),
            item.spec.rust_lifetime_names_elided,
            item.spec.evolution_requires_reference_distinction,
            item.spec.expected_rust_compile,
            item.rust_compiled,
            item.compile_expectation_matched,
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
    writeln!(json, "  \"compile_expectation_mismatches\": {mismatches},").expect("writing JSON cannot fail");
    writeln!(json, "  \"elision_candidate_case_count\": {elision_candidates},").expect("writing JSON cannot fail");
    writeln!(json, "  \"explicit_lifetime_relation_case_count\": {explicit_relations},").expect("writing JSON cannot fail");
    writeln!(json, "  \"unsafe_or_unrepresentable_case_count\": {unsafe_cases},").expect("writing JSON cannot fail");
    writeln!(json, "  \"decision_basis\": \"REFERENCE-SURFACE-FIRST when useful single-source borrowed returns compile with Rust lifetime elision, but every such escaping result still requires a caller-visible borrowed/reference distinction; multiple-source and invalid-owner cases must fail closed\",").expect("writing JSON cannot fail");
    writeln!(json, "  \"cases\": [").expect("writing JSON cannot fail");
    for (index, item) in findings.iter().enumerate() {
        let comma = if index + 1 == findings.len() { "" } else { "," };
        writeln!(
            json,
            "    {{\"name\": {}, \"classification\": {}, \"possible_owner_sources\": {}, \"result_escapes\": {}, \"result_stored\": {}, \"owner_overlap\": {}, \"rust_lifetime_names_elided\": {}, \"evolution_requires_reference_distinction\": {}, \"expected_rust_compile\": {}, \"rust_compiled\": {}, \"compile_expectation_matched\": {}, \"reason\": {}, \"stderr_summary\": {}}}{comma}",
            json_string(item.spec.name),
            json_string(item.spec.classification.as_str()),
            item.spec.possible_owner_sources,
            item.spec.result_escapes,
            item.spec.result_stored,
            json_string(item.spec.owner_overlap),
            item.spec.rust_lifetime_names_elided,
            item.spec.evolution_requires_reference_distinction,
            item.spec.expected_rust_compile,
            item.rust_compiled,
            item.compile_expectation_matched,
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
    writeln!(markdown, "# Lifetime elision feasibility v0").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "- git_sha: `{git_sha}`").expect("writing Markdown cannot fail");
    writeln!(markdown, "- aggregate verdict: **{verdict}**").expect("writing Markdown cannot fail");
    writeln!(markdown, "- compile expectation mismatches: **{mismatches}**").expect("writing Markdown cannot fail");
    writeln!(markdown, "- elision-candidate/reference-type cases: **{elision_candidates}**").expect("writing Markdown cannot fail");
    writeln!(markdown, "- explicit-lifetime-relation cases: **{explicit_relations}**").expect("writing Markdown cannot fail");
    writeln!(markdown, "- unsafe/unrepresentable cases: **{unsafe_cases}**").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "```text\n{rustc}\n```").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "| Case | Classification | Owners | Escapes | Stored | Rust elides names | Evolution ref distinction | Rust compile | Expected | Match |")
        .expect("writing Markdown cannot fail");
    writeln!(markdown, "| --- | --- | ---: | --- | --- | --- | --- | --- | --- | --- |")
        .expect("writing Markdown cannot fail");
    for item in findings {
        writeln!(
            markdown,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            item.spec.name,
            item.spec.classification.as_str(),
            item.spec.possible_owner_sources,
            item.spec.result_escapes,
            item.spec.result_stored,
            item.spec.rust_lifetime_names_elided,
            item.spec.evolution_requires_reference_distinction,
            item.rust_compiled,
            item.spec.expected_rust_compile,
            item.compile_expectation_matched,
        )
        .expect("writing Markdown cannot fail");
    }
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "## Decision").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "Rust can omit named lifetimes for useful single-source borrowed-return signatures, including nested-field, forwarding and recursive single-source shapes. The caller-visible result is still a borrow, not an owned `T`, and stored borrowed results create real move/reinitialization conflicts. Multiple borrowed inputs do not receive a unique output lifetime from Rust's elision rules. Therefore the bounded result is **{verdict}** rather than silent borrowed-return inference.")
        .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "This report is research evidence only. It changes no Evolution syntax, accepted-program semantics, ownership behavior, generated Rust or runtime behavior.")
        .expect("writing Markdown cannot fail");
    fs::write(out.join("report.md"), &markdown)
        .unwrap_or_else(|error| panic!("failed to write report Markdown: {error}"));
    print!("{markdown}");
}

#[test]
#[ignore = "research evidence; dedicated workflow runs this exact test"]
fn lifetime_elision_research_classifies_single_source_borrowed_returns() {
    let rustc = rustc_version();
    if env::var_os("EVO_REQUIRE_PINNED_RUSTC").is_some() {
        assert!(
            rustc.lines().next().is_some_and(|line| line.contains("rustc 1.98.0")),
            "research workflow must use pinned Rust 1.98.0, got: {rustc}"
        );
    }

    let scratch = env::temp_dir().join(format!(
        "evo-lifetime-elision-research-{}",
        std::process::id()
    ));
    if scratch.exists() {
        fs::remove_dir_all(&scratch)
            .unwrap_or_else(|error| panic!("failed to reset {}: {error}", scratch.display()));
    }
    fs::create_dir_all(&scratch)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", scratch.display()));

    let findings: Vec<Finding> = CASES.iter().map(|spec| run_case(spec, &scratch)).collect();
    let mismatch_count = findings
        .iter()
        .filter(|item| !item.compile_expectation_matched)
        .count();
    let successful_single_source_elisions = findings
        .iter()
        .filter(|item| {
            item.spec.classification == Classification::ElisionCandidateRequiresReferenceType
                && item.spec.expected_rust_compile
                && item.rust_compiled
                && item.spec.rust_lifetime_names_elided
                && item.spec.evolution_requires_reference_distinction
        })
        .count();
    let overlap_rejections = findings
        .iter()
        .filter(|item| {
            item.spec.classification == Classification::ElisionCandidateRequiresReferenceType
                && !item.spec.expected_rust_compile
                && !item.rust_compiled
        })
        .count();
    let explicit_relation_rejections = findings
        .iter()
        .filter(|item| {
            item.spec.classification == Classification::RequiresExplicitLifetimeRelation
                && !item.rust_compiled
        })
        .count();
    let unsafe_rejections = findings
        .iter()
        .filter(|item| {
            item.spec.classification == Classification::UnsafeOrUnrepresentable
                && !item.rust_compiled
        })
        .count();
    let all_borrowed_results_require_reference_distinction = findings
        .iter()
        .filter(|item| item.spec.classification == Classification::ElisionCandidateRequiresReferenceType)
        .all(|item| item.spec.evolution_requires_reference_distinction);

    let verdict = if mismatch_count == 0
        && successful_single_source_elisions >= 4
        && overlap_rejections >= 2
        && explicit_relation_rejections >= 2
        && unsafe_rejections >= 1
        && all_borrowed_results_require_reference_distinction
    {
        "REFERENCE-SURFACE-FIRST"
    } else if successful_single_source_elisions > 0 && mismatch_count == 0 {
        "ELISION-CANDIDATE"
    } else if explicit_relation_rejections > 0 && mismatch_count == 0 {
        "EXPLICIT-LIFETIME-RELATION-FIRST"
    } else {
        "DEFER"
    };

    let git_sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "local".to_owned());
    let out = env::var_os("EVO_LIFETIME_ELISION_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("evo-lifetime-elision-research")
        });
    write_reports(&findings, verdict, &out, &git_sha, &rustc);

    assert_eq!(mismatch_count, 0, "all pre-registered Rust compile expectations must match");
    assert!(successful_single_source_elisions >= 4, "need several useful single-source elision cases");
    assert!(overlap_rejections >= 2, "stored borrowed results must retain move/reinit conflict enforcement");
    assert!(explicit_relation_rejections >= 2, "multiple-owner lifetime relationships must fail closed without an explicit relation");
    assert!(unsafe_rejections >= 1, "temporary-derived borrowed returns must fail closed");
    assert!(all_borrowed_results_require_reference_distinction, "escaping borrowed results must never be conflated with owned T results");
    assert_eq!(verdict, "REFERENCE-SURFACE-FIRST");
}
