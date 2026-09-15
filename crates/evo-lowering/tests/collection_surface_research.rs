use evo_lexer::{TokenKind, lex};
use evo_parser::{TypeName, parse};
use std::cell::Cell;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    AppendOnlyCandidate,
    CheckedGrowthCandidate,
    RemovalIdentityRisk,
    HoleRemovalCandidate,
    SurfaceConstraint,
    CompositionControl,
}

impl Class {
    const fn label(self) -> &'static str {
        match self {
            Self::AppendOnlyCandidate => "APPEND-ONLY-CANDIDATE",
            Self::CheckedGrowthCandidate => "CHECKED-GROWTH-CANDIDATE",
            Self::RemovalIdentityRisk => "REMOVAL-IDENTITY-RISK",
            Self::HoleRemovalCandidate => "HOLE-REMOVAL-CANDIDATE",
            Self::SurfaceConstraint => "SURFACE-CONSTRAINT",
            Self::CompositionControl => "COMPOSITION-CONTROL",
        }
    }
}

#[derive(Debug)]
struct Finding {
    name: &'static str,
    class: Class,
    matched: bool,
    reason: &'static str,
}

fn finding(name: &'static str, class: Class, matched: bool, reason: &'static str) -> Finding {
    Finding {
        name,
        class,
        matched,
        reason,
    }
}

#[derive(Debug)]
struct CompileCase {
    name: &'static str,
    expected_compile: bool,
    source: &'static str,
    reason: &'static str,
}

#[derive(Debug)]
struct CompileFinding {
    name: &'static str,
    expected_compile: bool,
    compiled: bool,
    matched: bool,
    reason: &'static str,
    stderr_summary: String,
}

const COMPILE_CASES: &[CompileCase] = &[
    CompileCase {
        name: "element-borrow-blocks-push-while-live",
        expected_compile: false,
        source: r#"
fn main() {
    let mut items = vec![10_i64];
    let first = &items[0];
    items.push(20);
    println!("{}", first);
}
"#,
        reason: "growing the container while a live element reference exists must be rejected",
    },
    CompileCase {
        name: "push-after-final-element-reference-use",
        expected_compile: true,
        source: r#"
fn main() {
    let mut items = vec![10_i64];
    let first = &items[0];
    println!("{}", first);
    items.push(20);
    assert_eq!(items.len(), 2);
}
"#,
        reason: "bounded last-use analysis permits later growth after the element reference is dead",
    },
    CompileCase {
        name: "container-move-blocked-while-element-reference-live",
        expected_compile: false,
        source: r#"
fn main() {
    let items = vec![String::from("a")];
    let first = &items[0];
    let moved = items;
    println!("{}", first);
    drop(moved);
}
"#,
        reason: "moving the owning container while a live element reference exists must be rejected",
    },
    CompileCase {
        name: "move-only-payload-not-moved-through-shared-reference",
        expected_compile: false,
        source: r#"
struct Item { value: String }
fn main() {
    let items = vec![Item { value: String::from("a") }];
    let first = &items[0];
    let moved = first.value;
    drop(moved);
}
"#,
        reason: "reading through a shared element reference must not implicitly move a move-only payload",
    },
];

#[derive(Debug)]
struct SurfaceCandidate {
    name: &'static str,
    type_example: &'static str,
    constructor_example: &'static str,
    new_lexer_tokens: usize,
    contextual_identifiers: usize,
    general_generics_required: bool,
    explicit_allocation_surface: bool,
    compatibility: &'static str,
}

const SURFACE_CANDIDATES: &[SurfaceCandidate] = &[
    SurfaceCandidate {
        name: "CONTEXTUAL-SEQUENCE-PREFIX",
        type_example: "seq Item",
        constructor_example: "seq Item()",
        new_lexer_tokens: 0,
        contextual_identifiers: 1,
        general_generics_required: false,
        explicit_allocation_surface: true,
        compatibility: "fits-existing-contextual-type-constructor-pattern-but-consumes-seq-in-type-leading-position",
    },
    SurfaceCandidate {
        name: "BRACKET-SEQUENCE",
        type_example: "[Item]",
        constructor_example: "[]",
        new_lexer_tokens: 2,
        contextual_identifiers: 0,
        general_generics_required: false,
        explicit_allocation_surface: true,
        compatibility: "uses-currently-invalid-punctuation-but-risks-array-or-slice-semantics-confusion",
    },
    SurfaceCandidate {
        name: "GENERIC-LIKE-VEC",
        type_example: "Vec(Item)",
        constructor_example: "Vec()",
        new_lexer_tokens: 0,
        contextual_identifiers: 1,
        general_generics_required: true,
        explicit_allocation_surface: true,
        compatibility: "collides-with-current-nominal-call-shaped-identifier-space-and-implies-broader-generics",
    },
];

fn rustc_path() -> std::ffi::OsString {
    env::var_os("RUSTC").unwrap_or_else(|| "rustc".into())
}

fn rustc_version() -> String {
    let output = Command::new(rustc_path())
        .arg("-Vv")
        .output()
        .expect("rustc -Vv must run");
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

fn run_compile_case(spec: &CompileCase, root: &Path) -> CompileFinding {
    let case_dir = root.join(spec.name);
    if case_dir.exists() {
        fs::remove_dir_all(&case_dir)
            .unwrap_or_else(|error| panic!("failed to reset {}: {error}", case_dir.display()));
    }
    fs::create_dir_all(&case_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", case_dir.display()));
    let source = case_dir.join("case.rs");
    let binary = case_dir.join("case-bin");
    fs::write(&source, spec.source)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", source.display()));

    let output = Command::new(rustc_path())
        .arg("--edition=2024")
        .arg("--crate-name")
        .arg("evo_collection_surface_case")
        .arg("--crate-type")
        .arg("bin")
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap_or_else(|error| panic!("failed to execute rustc for {}: {error}", spec.name));
    let compiled = output.status.success();

    CompileFinding {
        name: spec.name,
        expected_compile: spec.expected_compile,
        compiled,
        matched: compiled == spec.expected_compile,
        reason: spec.reason,
        stderr_summary: stderr_summary(&output.stderr),
    }
}

fn csv_cell(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\"").replace('\n', "\\n"))
}

fn json_string(value: &str) -> String {
    format!("{value:?}")
}

fn is_identifier(source: &str, expected: &str) -> bool {
    match lex(source) {
        Ok(tokens) => matches!(
            tokens.as_slice(),
            [
                evo_lexer::Token {
                    kind: TokenKind::Identifier(name),
                    ..
                },
                evo_lexer::Token {
                    kind: TokenKind::Eof,
                    ..
                }
            ] if name == expected
        ),
        Err(_) => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn write_reports(
    findings: &[Finding],
    compile_findings: &[CompileFinding],
    rustc_vv: &str,
    git_sha: &str,
    out: &Path,
    verdict: &str,
    recommended_surface: &str,
    brackets_currently_invalid: bool,
    seq_identifier_available: bool,
    bounded_type_constructor_precedent: bool,
) {
    fs::create_dir_all(out)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", out.display()));

    let count = |class| {
        findings
            .iter()
            .filter(|finding| finding.class == class)
            .count()
    };
    let expectation_mismatches = findings.iter().filter(|finding| !finding.matched).count()
        + compile_findings
            .iter()
            .filter(|finding| !finding.matched)
            .count();

    let mut json = String::new();
    writeln!(json, "{{").unwrap();
    writeln!(json, "  \"git_sha\": {},", json_string(git_sha)).unwrap();
    writeln!(json, "  \"rustc_vv\": {},", json_string(rustc_vv)).unwrap();
    writeln!(json, "  \"verdict\": {},", json_string(verdict)).unwrap();
    writeln!(
        json,
        "  \"recommended_surface\": {},",
        json_string(recommended_surface)
    )
    .unwrap();
    writeln!(json, "  \"case_count\": {},", findings.len()).unwrap();
    writeln!(
        json,
        "  \"compile_boundary_case_count\": {},",
        compile_findings.len()
    )
    .unwrap();
    writeln!(
        json,
        "  \"expectation_mismatches\": {expectation_mismatches},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"append_only_candidate_count\": {},",
        count(Class::AppendOnlyCandidate)
    )
    .unwrap();
    writeln!(
        json,
        "  \"checked_growth_candidate_count\": {},",
        count(Class::CheckedGrowthCandidate)
    )
    .unwrap();
    writeln!(
        json,
        "  \"removal_identity_risk_count\": {},",
        count(Class::RemovalIdentityRisk)
    )
    .unwrap();
    writeln!(
        json,
        "  \"hole_removal_candidate_count\": {},",
        count(Class::HoleRemovalCandidate)
    )
    .unwrap();
    writeln!(
        json,
        "  \"surface_constraint_count\": {},",
        count(Class::SurfaceConstraint)
    )
    .unwrap();
    writeln!(
        json,
        "  \"composition_control_count\": {},",
        count(Class::CompositionControl)
    )
    .unwrap();
    writeln!(
        json,
        "  \"brackets_currently_invalid\": {brackets_currently_invalid},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"seq_identifier_available\": {seq_identifier_available},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"bounded_type_constructor_precedent\": {bounded_type_constructor_precedent},"
    )
    .unwrap();
    writeln!(json, "  \"cases\": [").unwrap();
    for (index, item) in findings.iter().enumerate() {
        let comma = if index + 1 == findings.len() { "" } else { "," };
        writeln!(
            json,
            "    {{\"name\": {}, \"classification\": {}, \"matched\": {}, \"reason\": {}}}{comma}",
            json_string(item.name),
            json_string(item.class.label()),
            item.matched,
            json_string(item.reason)
        )
        .unwrap();
    }
    writeln!(json, "  ],").unwrap();
    writeln!(json, "  \"compile_boundaries\": [").unwrap();
    for (index, item) in compile_findings.iter().enumerate() {
        let comma = if index + 1 == compile_findings.len() {
            ""
        } else {
            ","
        };
        writeln!(
            json,
            "    {{\"name\": {}, \"expected_compile\": {}, \"compiled\": {}, \"matched\": {}, \"reason\": {}, \"stderr_summary\": {}}}{comma}",
            json_string(item.name),
            item.expected_compile,
            item.compiled,
            item.matched,
            json_string(item.reason),
            json_string(&item.stderr_summary)
        )
        .unwrap();
    }
    writeln!(json, "  ]").unwrap();
    writeln!(json, "}}").unwrap();
    fs::write(out.join("report.json"), json).expect("write JSON report");

    let mut markdown = String::new();
    writeln!(markdown, "# Collection surface v0 research").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "- git_sha: `{git_sha}`").unwrap();
    writeln!(markdown, "- verdict: **{verdict}**").unwrap();
    writeln!(
        markdown,
        "- recommended surface family: **{recommended_surface}**"
    )
    .unwrap();
    writeln!(markdown, "- runtime/surface cases: **{}**", findings.len()).unwrap();
    writeln!(
        markdown,
        "- compile-boundary cases: **{}**",
        compile_findings.len()
    )
    .unwrap();
    writeln!(
        markdown,
        "- expectation mismatches: **{expectation_mismatches}**"
    )
    .unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "```text\n{rustc_vv}\n```").unwrap();
    writeln!(markdown).unwrap();
    writeln!(
        markdown,
        "Append-only growth preserves numeric index identity and supports checked lookup, exclusive mutation, container move/drop, and explicit payload ownership with ordinary safe Rust storage. General removal is not accepted in this slice because shifting removal silently rebinds old numeric indices; hole-preserving removal is a distinct storage policy that must remain explicit rather than hidden behind a generic sequence operation."
    )
    .unwrap();
    fs::write(out.join("report.md"), markdown).expect("write Markdown report");

    let mut csv = String::from("kind,name,classification_or_surface,matched_or_tokens,detail\n");
    for item in findings {
        writeln!(
            csv,
            "runtime,{},{},{},{}",
            csv_cell(item.name),
            csv_cell(item.class.label()),
            item.matched,
            csv_cell(item.reason)
        )
        .unwrap();
    }
    for item in compile_findings {
        writeln!(
            csv,
            "compile,{},{},{},{}",
            csv_cell(item.name),
            csv_cell(if item.expected_compile {
                "EXPECT-COMPILE"
            } else {
                "EXPECT-REJECT"
            }),
            item.matched,
            csv_cell(item.reason)
        )
        .unwrap();
    }
    for surface in SURFACE_CANDIDATES {
        let detail = format!(
            "type={}; constructor={}; contextual_identifiers={}; generic_infrastructure={}; explicit_allocation={}; compatibility={}",
            surface.type_example,
            surface.constructor_example,
            surface.contextual_identifiers,
            surface.general_generics_required,
            surface.explicit_allocation_surface,
            surface.compatibility
        );
        writeln!(
            csv,
            "surface,{},{},{},{}",
            csv_cell(surface.name),
            csv_cell(surface.compatibility),
            surface.new_lexer_tokens,
            csv_cell(&detail)
        )
        .unwrap();
    }
    fs::write(out.join("surface-matrix.csv"), csv).expect("write CSV report");
}

#[test]
#[ignore = "research evidence; dedicated workflow runs this exact test"]
fn collection_surface_research_classifies_bounded_indexed_storage() {
    let rustc_vv = rustc_version();
    if env::var_os("EVO_REQUIRE_PINNED_RUSTC").is_some() {
        assert!(
            rustc_vv
                .lines()
                .next()
                .is_some_and(|line| line.contains("rustc 1.98.0")),
            "research requires Rust 1.98.0, got {rustc_vv}"
        );
    }

    let mut findings = Vec::new();

    let empty: Vec<i64> = Vec::new();
    findings.push(finding(
        "create-empty-storage",
        Class::AppendOnlyCandidate,
        empty.is_empty(),
        "empty owned storage is explicit and has no payload",
    ));

    let non_empty = Vec::from([5_i64, 7_i64]);
    findings.push(finding(
        "create-non-empty-storage",
        Class::AppendOnlyCandidate,
        non_empty.len() == 2 && non_empty[0] == 5 && non_empty[1] == 7,
        "non-empty owned storage preserves insertion order",
    ));

    let mut appended = Vec::with_capacity(1);
    appended.extend([5_i64, 7_i64]);
    findings.push(finding(
        "append-and-checked-read",
        Class::CheckedGrowthCandidate,
        appended.first() == Some(&5) && appended.get(1) == Some(&7),
        "growth and checked lookup map directly to ordinary Vec operations",
    ));
    findings.push(finding(
        "checked-out-of-bounds",
        Class::CheckedGrowthCandidate,
        appended.get(99).is_none(),
        "out-of-bounds checked lookup is observable rather than unchecked pointer access",
    ));

    *appended.get_mut(1).expect("existing element") = 8;
    findings.push(finding(
        "exclusive-container-mutation",
        Class::CheckedGrowthCandidate,
        appended.get(1) == Some(&8),
        "exclusive container ownership permits direct safe element mutation without RefCell or locks",
    ));

    let moved = appended;
    findings.push(finding(
        "container-move-preserves-payload",
        Class::AppendOnlyCandidate,
        moved.as_slice() == [5, 8],
        "moving the container transfers ownership without cloning payloads",
    ));

    struct DropProbe(Rc<Cell<usize>>);
    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = Rc::new(Cell::new(0usize));
    {
        let owned = Vec::from([DropProbe(Rc::clone(&drops)), DropProbe(Rc::clone(&drops))]);
        assert_eq!(owned.len(), 2);
    }
    findings.push(finding(
        "container-drop-destroys-payloads",
        Class::AppendOnlyCandidate,
        drops.get() == 2,
        "container destruction deterministically drops live payloads",
    ));

    let mut stable = Vec::from([10_i64, 20_i64]);
    let first_index = 0usize;
    let second_index = 1usize;
    for value in 0_i64..256 {
        stable.push(1000 + value);
    }
    findings.push(finding(
        "append-only-indices-remain-stable-across-growth",
        Class::AppendOnlyCandidate,
        stable.get(first_index) == Some(&10) && stable.get(second_index) == Some(&20),
        "physical Vec reallocation does not change logical numeric indices when elements are only appended",
    ));

    let mut shifting = vec![10_i64, 20_i64, 30_i64];
    let remembered_index = 1usize;
    let remembered_value = shifting[remembered_index];
    let removed = shifting.remove(0);
    findings.push(finding(
        "shifting-removal-rebinds-old-index",
        Class::RemovalIdentityRisk,
        removed == 10 && remembered_value == 20 && shifting.get(remembered_index) == Some(&30),
        "ordinary shifting removal can silently make an old numeric index name a different payload",
    ));

    let mut holed = Vec::from([Some(10_i64), Some(20_i64), Some(30_i64)]);
    let removed = holed[1].take();
    findings.push(finding(
        "hole-removal-preserves-other-indices",
        Class::HoleRemovalCandidate,
        removed == Some(20) && holed[1].is_none() && holed[2] == Some(30),
        "hole-preserving removal avoids index shifting but introduces an explicit occupied-vs-empty storage policy",
    ));

    #[derive(Debug, PartialEq, Eq)]
    struct MoveOnly {
        text: String,
    }
    let mut move_only = vec![MoveOnly {
        text: String::from("payload"),
    }];
    let moved_payload = move_only.pop().expect("payload exists");
    findings.push(finding(
        "payload-move-remains-explicit",
        Class::AppendOnlyCandidate,
        moved_payload.text == "payload" && move_only.is_empty(),
        "removing a move-only payload transfers it explicitly; reads do not imply clone",
    ));

    #[derive(Debug, PartialEq, Eq)]
    struct Inner {
        value: i64,
    }
    #[derive(Debug, PartialEq, Eq)]
    struct Outer {
        inner: Inner,
    }
    let nested = Vec::from([Outer {
        inner: Inner { value: 42 },
    }]);
    findings.push(finding(
        "nested-nominal-payload",
        Class::AppendOnlyCandidate,
        nested[0].inner.value == 42,
        "owned sequence payloads compose with ordinary nominal ownership",
    ));

    let shared = Rc::new(String::from("shared"));
    let shared_payloads = Vec::from([Rc::clone(&shared)]);
    findings.push(finding(
        "explicit-shared-owner-payload-composition",
        Class::CompositionControl,
        Rc::strong_count(&shared) == 2 && shared_payloads[0].as_str() == "shared",
        "shared-owner payload duplication remains explicit and separate from container growth",
    ));

    findings.push(finding(
        "no-hidden-ownership-runtime-required",
        Class::AppendOnlyCandidate,
        true,
        "accepted append-only storage needs no GC, global registry, per-edge refcount, lock, RefCell, or unsafe pointer table",
    ));

    let brackets_currently_invalid = lex("[").is_err() && lex("]").is_err();
    findings.push(finding(
        "bracket-surface-is-currently-unallocated",
        Class::SurfaceConstraint,
        brackets_currently_invalid,
        "square brackets are currently invalid lexer characters and would require explicit new punctuation tokens",
    ));

    let seq_identifier_available = is_identifier("seq", "seq");
    findings.push(finding(
        "seq-is-currently-an-identifier",
        Class::SurfaceConstraint,
        seq_identifier_available,
        "a sequence prefix could be contextual like shared, but seq is currently valid identifier space and cannot be silently reserved globally",
    ));

    let parser_probe = r#"
record Item
value int
end
fn inspect(owner shared Item, item &Item) int
return item.value
end
"#;
    let parsed = parse(&lex(parser_probe).expect("surface parser probe must lex"))
        .expect("surface parser probe must parse");
    let function = parsed.functions.first().expect("probe function");
    let bounded_type_constructor_precedent = matches!(
        &function.parameters[0].type_name,
        TypeName::SharedOwner(name) if name == "Item"
    ) && matches!(
        &function.parameters[1].type_name,
        TypeName::SharedRef(inner) if matches!(inner.as_ref(), TypeName::Named(name) if name == "Item")
    );
    findings.push(finding(
        "bounded-type-constructor-precedent-exists",
        Class::SurfaceConstraint,
        bounded_type_constructor_precedent,
        "the current type algebra already represents bounded ownership/reference wrappers without general generic syntax",
    ));

    let compile_root = env::temp_dir().join(format!(
        "evo-collection-surface-research-{}",
        std::process::id()
    ));
    if compile_root.exists() {
        fs::remove_dir_all(&compile_root).expect("reset compile root");
    }
    fs::create_dir_all(&compile_root).expect("create compile root");
    let compile_findings = COMPILE_CASES
        .iter()
        .map(|spec| run_compile_case(spec, &compile_root))
        .collect::<Vec<_>>();
    let _ = fs::remove_dir_all(&compile_root);

    assert!(findings.iter().all(|item| item.matched));
    assert!(compile_findings.iter().all(|item| item.matched));

    let count = |class| {
        findings
            .iter()
            .filter(|finding| finding.class == class)
            .count()
    };
    assert!(count(Class::AppendOnlyCandidate) >= 7);
    assert!(count(Class::CheckedGrowthCandidate) >= 3);
    assert!(count(Class::RemovalIdentityRisk) >= 1);
    assert!(count(Class::HoleRemovalCandidate) >= 1);
    assert!(count(Class::SurfaceConstraint) >= 3);
    assert!(count(Class::CompositionControl) >= 1);

    let verdict = "APPEND-ONLY-FIRST";
    let recommended_surface = "CONTEXTUAL-SEQUENCE-TYPE-CANDIDATE";
    let git_sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "unknown".to_owned());
    let out = env::var_os("EVO_COLLECTION_SURFACE_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/evo-collection-surface-research")
        });

    write_reports(
        &findings,
        &compile_findings,
        &rustc_vv,
        &git_sha,
        &out,
        verdict,
        recommended_surface,
        brackets_currently_invalid,
        seq_identifier_available,
        bounded_type_constructor_precedent,
    );
}
