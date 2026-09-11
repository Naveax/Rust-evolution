use evo_lexer::{TokenKind, lex};
use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SemanticClass {
    OwnedControl,
    NoEscapingRelation,
    SingleSourceReference,
    BoundedLastUse,
    RequiresLifetimeRelation,
    InvalidOwner,
}

impl SemanticClass {
    const fn as_str(self) -> &'static str {
        match self {
            Self::OwnedControl => "OWNED-CONTROL",
            Self::NoEscapingRelation => "NO-ESCAPING-RELATION",
            Self::SingleSourceReference => "SINGLE-SOURCE-REFERENCE",
            Self::BoundedLastUse => "BOUNDED-LAST-USE",
            Self::RequiresLifetimeRelation => "REQUIRES-LIFETIME-RELATION",
            Self::InvalidOwner => "INVALID-OWNER",
        }
    }
}

#[derive(Debug)]
struct SemanticCase {
    name: &'static str,
    class: SemanticClass,
    owner_sources: usize,
    result_stored: bool,
    named_lifetime_required: bool,
    reference_distinction_required: bool,
    expected_rust_compile: bool,
    reason: &'static str,
    rust_source: &'static str,
}

#[derive(Debug)]
struct SemanticFinding {
    spec: &'static SemanticCase,
    rust_compiled: bool,
    expectation_matched: bool,
    stderr_summary: String,
}

#[derive(Debug)]
struct SurfaceCandidate {
    name: &'static str,
    signature_example: &'static str,
    borrow_example: &'static str,
    added_lexer_tokens: usize,
    reserved_identifiers: usize,
    compatibility: &'static str,
    direct_rust_lowering: bool,
    caller_visible_reference: bool,
    reason: &'static str,
}

const SURFACE_CANDIDATES: &[SurfaceCandidate] = &[
    SurfaceCandidate {
        name: "PUNCTUATION-AMPERSAND",
        signature_example: "fn borrow_item(item &Item) &Item",
        borrow_example: "r = &item",
        added_lexer_tokens: 1,
        reserved_identifiers: 0,
        compatibility: "occupies-currently-invalid-character",
        direct_rust_lowering: true,
        caller_visible_reference: true,
        reason: "adds one previously-invalid token, reserves no current identifier, and maps directly to Rust immutable-reference syntax",
    },
    SurfaceCandidate {
        name: "KEYWORD-REF-BORROW",
        signature_example: "fn borrow_item(item ref Item) ref Item",
        borrow_example: "r = borrow item",
        added_lexer_tokens: 2,
        reserved_identifiers: 2,
        compatibility: "would-reserve-current-identifiers",
        direct_rust_lowering: true,
        caller_visible_reference: true,
        reason: "is readable but would reserve ref/borrow names that are currently ordinary identifiers",
    },
    SurfaceCandidate {
        name: "GENERIC-LIKE-REF",
        signature_example: "fn borrow_item(item Ref(Item)) Ref(Item)",
        borrow_example: "r = borrow(item)",
        added_lexer_tokens: 0,
        reserved_identifiers: 1,
        compatibility: "collides-with-current-nominal-call-space",
        direct_rust_lowering: false,
        caller_visible_reference: true,
        reason: "looks type-like but special-cases ordinary identifier/call-shaped syntax and can collide with a user nominal named Ref",
    },
];

const SEMANTIC_CASES: &[SemanticCase] = &[
    SemanticCase {
        name: "owned-identity-remains-owned",
        class: SemanticClass::OwnedControl,
        owner_sources: 1,
        result_stored: true,
        named_lifetime_required: false,
        reference_distinction_required: false,
        expected_rust_compile: true,
        reason: "existing T -> T ownership transfer must stay owned",
        rust_source: r#"
struct Item { value: i64 }
fn identity(item: Item) -> Item { item }
fn main() { let item = identity(Item { value: 7 }); assert_eq!(item.value, 7); }
"#,
    },
    SemanticCase {
        name: "borrowed-input-scalar-result",
        class: SemanticClass::NoEscapingRelation,
        owner_sources: 1,
        result_stored: false,
        named_lifetime_required: false,
        reference_distinction_required: false,
        expected_rust_compile: true,
        reason: "scalar data copied from a shared reference carries no escaping borrow",
        rust_source: r#"
struct Item { value: i64 }
fn read(item: &Item) -> i64 { item.value }
fn main() { let item = Item { value: 7 }; assert_eq!(read(&item), 7); }
"#,
    },
    SemanticCase {
        name: "single-source-whole-reference",
        class: SemanticClass::SingleSourceReference,
        owner_sources: 1,
        result_stored: true,
        named_lifetime_required: false,
        reference_distinction_required: true,
        expected_rust_compile: true,
        reason: "one borrowed input uniquely determines the returned reference lifetime",
        rust_source: r#"
struct Item { value: i64 }
fn borrow_item(item: &Item) -> &Item { item }
fn main() { let item = Item { value: 7 }; let r = borrow_item(&item); assert_eq!(r.value, 7); }
"#,
    },
    SemanticCase {
        name: "single-source-nested-reference",
        class: SemanticClass::SingleSourceReference,
        owner_sources: 1,
        result_stored: true,
        named_lifetime_required: false,
        reference_distinction_required: true,
        expected_rust_compile: true,
        reason: "a nested nominal field reference remains tied to the sole input owner",
        rust_source: r#"
struct Inner { value: i64 }
struct Outer { inner: Inner }
fn inner(outer: &Outer) -> &Inner { &outer.inner }
fn main() { let outer = Outer { inner: Inner { value: 7 } }; assert_eq!(inner(&outer).value, 7); }
"#,
    },
    SemanticCase {
        name: "reference-immediate-inspection",
        class: SemanticClass::SingleSourceReference,
        owner_sources: 1,
        result_stored: false,
        named_lifetime_required: false,
        reference_distinction_required: true,
        expected_rust_compile: true,
        reason: "the return contract is still a reference even when immediately inspected",
        rust_source: r#"
struct Item { value: i64 }
fn borrow_item(item: &Item) -> &Item { item }
fn main() { let item = Item { value: 7 }; assert_eq!(borrow_item(&item).value, 7); }
"#,
    },
    SemanticCase {
        name: "reference-stored-local",
        class: SemanticClass::SingleSourceReference,
        owner_sources: 1,
        result_stored: true,
        named_lifetime_required: false,
        reference_distinction_required: true,
        expected_rust_compile: true,
        reason: "a first-class immutable reference local can remain live across statements",
        rust_source: r#"
struct Item { value: i64 }
fn borrow_item(item: &Item) -> &Item { item }
fn main() { let item = Item { value: 7 }; let r = borrow_item(&item); let x = r.value; assert_eq!(x, 7); }
"#,
    },
    SemanticCase {
        name: "owner-move-while-reference-live",
        class: SemanticClass::SingleSourceReference,
        owner_sources: 1,
        result_stored: true,
        named_lifetime_required: false,
        reference_distinction_required: true,
        expected_rust_compile: false,
        reason: "moving the owner while a later reference use remains must fail",
        rust_source: r#"
struct Item { value: i64 }
fn main() { let item = Item { value: 7 }; let r = &item; let moved = item; assert_eq!(r.value, 7); let _ = moved; }
"#,
    },
    SemanticCase {
        name: "owner-reinit-while-reference-live",
        class: SemanticClass::SingleSourceReference,
        owner_sources: 1,
        result_stored: true,
        named_lifetime_required: false,
        reference_distinction_required: true,
        expected_rust_compile: false,
        reason: "reinitializing the owner while a later reference use remains must fail",
        rust_source: r#"
struct Item { value: i64 }
fn main() { let mut item = Item { value: 7 }; let r = &item; item = Item { value: 8 }; assert_eq!(r.value, 7); }
"#,
    },
    SemanticCase {
        name: "owner-move-after-final-reference-use",
        class: SemanticClass::BoundedLastUse,
        owner_sources: 1,
        result_stored: true,
        named_lifetime_required: false,
        reference_distinction_required: true,
        expected_rust_compile: true,
        reason: "a bounded last-use analysis can end the borrow before a later owner move",
        rust_source: r#"
struct Item { value: i64 }
fn main() { let item = Item { value: 7 }; let r = &item; assert_eq!(r.value, 7); let moved = item; assert_eq!(moved.value, 7); }
"#,
    },
    SemanticCase {
        name: "move-nominal-field-through-shared-reference",
        class: SemanticClass::SingleSourceReference,
        owner_sources: 1,
        result_stored: false,
        named_lifetime_required: false,
        reference_distinction_required: true,
        expected_rust_compile: false,
        reason: "a move-only nominal value cannot be moved out through an immutable reference",
        rust_source: r#"
struct Inner { value: i64 }
struct Outer { inner: Inner }
fn take(outer: &Outer) -> Inner { outer.inner }
fn main() {}
"#,
    },
    SemanticCase {
        name: "read-scalar-field-through-shared-reference",
        class: SemanticClass::NoEscapingRelation,
        owner_sources: 1,
        result_stored: false,
        named_lifetime_required: false,
        reference_distinction_required: false,
        expected_rust_compile: true,
        reason: "copying a scalar field through an immutable reference is sound",
        rust_source: r#"
struct Item { value: i64 }
fn read(item: &Item) -> i64 { item.value }
fn main() { let item = Item { value: 7 }; assert_eq!(read(&item), 7); }
"#,
    },
    SemanticCase {
        name: "single-source-forwarding",
        class: SemanticClass::SingleSourceReference,
        owner_sources: 1,
        result_stored: true,
        named_lifetime_required: false,
        reference_distinction_required: true,
        expected_rust_compile: true,
        reason: "forwarding preserves the sole owner relation without named lifetime syntax",
        rust_source: r#"
struct Item { value: i64 }
fn borrow_item(item: &Item) -> &Item { item }
fn forward(item: &Item) -> &Item { borrow_item(item) }
fn main() { let item = Item { value: 7 }; assert_eq!(forward(&item).value, 7); }
"#,
    },
    SemanticCase {
        name: "single-source-recursion",
        class: SemanticClass::SingleSourceReference,
        owner_sources: 1,
        result_stored: true,
        named_lifetime_required: false,
        reference_distinction_required: true,
        expected_rust_compile: true,
        reason: "recursive forwarding can retain one deterministic owner relation",
        rust_source: r#"
struct Item { value: i64 }
fn recurse(item: &Item, n: u32) -> &Item { if n == 0 { item } else { recurse(item, n - 1) } }
fn main() { let item = Item { value: 7 }; assert_eq!(recurse(&item, 2).value, 7); }
"#,
    },
    SemanticCase {
        name: "same-owner-branch",
        class: SemanticClass::SingleSourceReference,
        owner_sources: 1,
        result_stored: true,
        named_lifetime_required: false,
        reference_distinction_required: true,
        expected_rust_compile: true,
        reason: "different control-flow paths are sound when every path returns the same owner",
        rust_source: r#"
struct Item { value: i64 }
fn choose(item: &Item, flag: bool) -> &Item { if flag { item } else { item } }
fn main() { let item = Item { value: 7 }; assert_eq!(choose(&item, true).value, 7); }
"#,
    },
    SemanticCase {
        name: "two-owner-ambiguous-return",
        class: SemanticClass::RequiresLifetimeRelation,
        owner_sources: 2,
        result_stored: true,
        named_lifetime_required: true,
        reference_distinction_required: true,
        expected_rust_compile: false,
        reason: "ordinary elision cannot choose one of two input owner lifetimes",
        rust_source: r#"
struct Item { value: i64 }
fn first(a: &Item, _b: &Item) -> &Item { a }
fn main() {}
"#,
    },
    SemanticCase {
        name: "different-owner-branch",
        class: SemanticClass::RequiresLifetimeRelation,
        owner_sources: 2,
        result_stored: true,
        named_lifetime_required: true,
        reference_distinction_required: true,
        expected_rust_compile: false,
        reason: "a branch that may select different owners needs a relation outside v0",
        rust_source: r#"
struct Item { value: i64 }
fn choose(a: &Item, b: &Item, flag: bool) -> &Item { if flag { a } else { b } }
fn main() {}
"#,
    },
    SemanticCase {
        name: "reference-to-local-owner",
        class: SemanticClass::InvalidOwner,
        owner_sources: 0,
        result_stored: true,
        named_lifetime_required: true,
        reference_distinction_required: true,
        expected_rust_compile: false,
        reason: "a returned reference cannot outlive a fresh local owner",
        rust_source: r#"
struct Item { value: i64 }
fn bad() -> &'static Item { let item = Item { value: 7 }; &item }
fn main() {}
"#,
    },
    SemanticCase {
        name: "reference-to-shared-borrow-parameter",
        class: SemanticClass::SingleSourceReference,
        owner_sources: 1,
        result_stored: false,
        named_lifetime_required: false,
        reference_distinction_required: true,
        expected_rust_compile: true,
        reason: "an existing reference can pass directly to an ordinary shared-borrow parameter",
        rust_source: r#"
struct Item { value: i64 }
fn inspect(item: &Item) -> i64 { item.value }
fn pass(item: &Item) -> i64 { inspect(item) }
fn main() { let item = Item { value: 7 }; assert_eq!(pass(&item), 7); }
"#,
    },
    SemanticCase {
        name: "owned-local-call-duration-borrow",
        class: SemanticClass::NoEscapingRelation,
        owner_sources: 1,
        result_stored: false,
        named_lifetime_required: false,
        reference_distinction_required: false,
        expected_rust_compile: true,
        reason: "borrowing an owned local only for one call does not create a stored first-class reference",
        rust_source: r#"
struct Item { value: i64 }
fn inspect(item: &Item) -> i64 { item.value }
fn main() { let item = Item { value: 7 }; assert_eq!(inspect(&item), 7); let moved = item; assert_eq!(moved.value, 7); }
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

fn run_case(spec: &'static SemanticCase, root: &Path) -> SemanticFinding {
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
        .arg("evo_immutable_reference_case")
        .arg("--crate-type")
        .arg("bin")
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap_or_else(|error| panic!("failed to execute rustc for {}: {error}", spec.name));
    let rust_compiled = output.status.success();

    if rust_compiled {
        let status = Command::new(&binary)
            .status()
            .unwrap_or_else(|error| panic!("failed to run {}: {error}", spec.name));
        assert!(status.success(), "compiled fixture {} failed", spec.name);
    }

    SemanticFinding {
        spec,
        rust_compiled,
        expectation_matched: rust_compiled == spec.expected_rust_compile,
        stderr_summary: stderr_summary(&output.stderr),
    }
}

fn is_identifier(text: &str, expected: &str) -> bool {
    let Ok(tokens) = lex(text) else {
        return false;
    };
    matches!(
        tokens.first().map(|token| &token.kind),
        Some(TokenKind::Identifier(name)) if name == expected
    )
}

fn json_string(value: &str) -> String {
    format!("{value:?}")
}

fn csv_cell(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\"").replace('\n', "\\n"))
}

fn punctuation_count(text: &str) -> usize {
    text.chars()
        .filter(|ch| !ch.is_ascii_alphanumeric() && !ch.is_whitespace() && *ch != '_')
        .count()
}

#[allow(
    clippy::too_many_arguments,
    reason = "research report writer records independent evidence dimensions verbatim"
)]
fn write_reports(
    findings: &[SemanticFinding],
    verdict: &str,
    recommended_surface: &str,
    out: &Path,
    git_sha: &str,
    rustc: &str,
    ampersand_currently_invalid: bool,
    keyword_identifiers_available: bool,
    generic_ref_identifier_available: bool,
) {
    fs::create_dir_all(out)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", out.display()));

    let mismatches = findings
        .iter()
        .filter(|finding| !finding.expectation_matched)
        .count();

    let mut csv = String::from(
        "row_kind,name,class,owner_sources,result_stored,named_lifetime_required,reference_distinction_required,expected_rust_compile,rust_compiled,compile_match,added_lexer_tokens,reserved_identifiers,character_count,punctuation_count,compatibility,reason,stderr_summary\n",
    );

    for candidate in SURFACE_CANDIDATES {
        let combined = format!(
            "{} {}",
            candidate.signature_example, candidate.borrow_example
        );
        writeln!(
            csv,
            "surface,{},{},,,,,,,,,{},{},{},{},{},{},",
            csv_cell(candidate.name),
            csv_cell("SURFACE-CANDIDATE"),
            candidate.added_lexer_tokens,
            candidate.reserved_identifiers,
            combined.chars().count(),
            punctuation_count(&combined),
            csv_cell(candidate.compatibility),
            csv_cell(candidate.reason),
        )
        .expect("writing CSV cannot fail");
    }

    for finding in findings {
        writeln!(
            csv,
            "semantic,{},{},{},{},{},{},{},{},{},,,,,,{},{},",
            csv_cell(finding.spec.name),
            csv_cell(finding.spec.class.as_str()),
            finding.spec.owner_sources,
            finding.spec.result_stored,
            finding.spec.named_lifetime_required,
            finding.spec.reference_distinction_required,
            finding.spec.expected_rust_compile,
            finding.rust_compiled,
            finding.expectation_matched,
            csv_cell(finding.spec.reason),
            csv_cell(&finding.stderr_summary),
        )
        .expect("writing CSV cannot fail");
    }
    fs::write(out.join("surface-matrix.csv"), csv)
        .unwrap_or_else(|error| panic!("failed to write surface matrix: {error}"));

    let mut json = String::new();
    writeln!(json, "{{").expect("writing JSON cannot fail");
    writeln!(json, "  \"git_sha\": {},", json_string(git_sha)).expect("writing JSON cannot fail");
    writeln!(json, "  \"rustc_vv\": {},", json_string(rustc)).expect("writing JSON cannot fail");
    writeln!(json, "  \"verdict\": {},", json_string(verdict)).expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"recommended_surface\": {},",
        json_string(recommended_surface)
    )
    .expect("writing JSON cannot fail");
    writeln!(json, "  \"semantic_case_count\": {},", findings.len())
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"compile_expectation_mismatches\": {mismatches},")
        .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"ampersand_currently_invalid\": {ampersand_currently_invalid},"
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"keyword_identifiers_available\": {keyword_identifiers_available},"
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"generic_ref_identifier_available\": {generic_ref_identifier_available},"
    )
    .expect("writing JSON cannot fail");
    writeln!(json, "  \"surface_candidates\": [").expect("writing JSON cannot fail");
    for (index, candidate) in SURFACE_CANDIDATES.iter().enumerate() {
        let comma = if index + 1 == SURFACE_CANDIDATES.len() {
            ""
        } else {
            ","
        };
        let combined = format!(
            "{} {}",
            candidate.signature_example, candidate.borrow_example
        );
        writeln!(
            json,
            "    {{\"name\": {}, \"signature_example\": {}, \"borrow_example\": {}, \"added_lexer_tokens\": {}, \"reserved_identifiers\": {}, \"character_count\": {}, \"punctuation_count\": {}, \"compatibility\": {}, \"direct_rust_lowering\": {}, \"caller_visible_reference\": {}, \"reason\": {}}}{comma}",
            json_string(candidate.name),
            json_string(candidate.signature_example),
            json_string(candidate.borrow_example),
            candidate.added_lexer_tokens,
            candidate.reserved_identifiers,
            combined.chars().count(),
            punctuation_count(&combined),
            json_string(candidate.compatibility),
            candidate.direct_rust_lowering,
            candidate.caller_visible_reference,
            json_string(candidate.reason),
        )
        .expect("writing JSON cannot fail");
    }
    writeln!(json, "  ],").expect("writing JSON cannot fail");
    writeln!(json, "  \"semantic_cases\": [").expect("writing JSON cannot fail");
    for (index, finding) in findings.iter().enumerate() {
        let comma = if index + 1 == findings.len() { "" } else { "," };
        writeln!(
            json,
            "    {{\"name\": {}, \"class\": {}, \"owner_sources\": {}, \"result_stored\": {}, \"named_lifetime_required\": {}, \"reference_distinction_required\": {}, \"expected_rust_compile\": {}, \"rust_compiled\": {}, \"compile_match\": {}, \"reason\": {}, \"stderr_summary\": {}}}{comma}",
            json_string(finding.spec.name),
            json_string(finding.spec.class.as_str()),
            finding.spec.owner_sources,
            finding.spec.result_stored,
            finding.spec.named_lifetime_required,
            finding.spec.reference_distinction_required,
            finding.spec.expected_rust_compile,
            finding.rust_compiled,
            finding.expectation_matched,
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
    writeln!(markdown, "# Immutable reference surface v0 research")
        .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "- git_sha: `{git_sha}`").expect("writing Markdown cannot fail");
    writeln!(markdown, "- verdict: **{verdict}**").expect("writing Markdown cannot fail");
    writeln!(markdown, "- recommended surface: **{recommended_surface}**")
        .expect("writing Markdown cannot fail");
    writeln!(markdown, "- semantic cases: **{}**", findings.len())
        .expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "- compile expectation mismatches: **{mismatches}**"
    )
    .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "```text\n{rustc}\n```").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "## Current lexer compatibility probes")
        .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "- `&` currently rejected by lexer: **{ampersand_currently_invalid}**"
    )
    .expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "- `ref` and `borrow` currently ordinary identifiers: **{keyword_identifiers_available}**"
    )
    .expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "- `Ref` currently ordinary identifier space: **{generic_ref_identifier_available}**"
    )
    .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "## Surface candidates").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "| Candidate | Added lexer tokens | Reserved identifiers | Chars | Punctuation | Compatibility | Direct Rust |"
    )
    .expect("writing Markdown cannot fail");
    writeln!(markdown, "| --- | ---: | ---: | ---: | ---: | --- | --- |")
        .expect("writing Markdown cannot fail");
    for candidate in SURFACE_CANDIDATES {
        let combined = format!(
            "{} {}",
            candidate.signature_example, candidate.borrow_example
        );
        writeln!(
            markdown,
            "| {} | {} | {} | {} | {} | {} | {} |",
            candidate.name,
            candidate.added_lexer_tokens,
            candidate.reserved_identifiers,
            combined.chars().count(),
            punctuation_count(&combined),
            candidate.compatibility,
            candidate.direct_rust_lowering,
        )
        .expect("writing Markdown cannot fail");
    }
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "## Semantic matrix").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "| Case | Class | Owners | Stored | Named lifetime | Ref distinction | Rust | Expected | Match |"
    )
    .expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "| --- | --- | ---: | --- | --- | --- | --- | --- | --- |"
    )
    .expect("writing Markdown cannot fail");
    for finding in findings {
        writeln!(
            markdown,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} |",
            finding.spec.name,
            finding.spec.class.as_str(),
            finding.spec.owner_sources,
            finding.spec.result_stored,
            finding.spec.named_lifetime_required,
            finding.spec.reference_distinction_required,
            finding.rust_compiled,
            finding.spec.expected_rust_compile,
            finding.expectation_matched,
        )
        .expect("writing Markdown cannot fail");
    }
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "## Decision basis").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "The punctuation family is the bounded candidate only when the semantic matrix matches completely. It occupies a character the current lexer rejects, reserves no currently-valid identifier, keeps owned and referenced types visibly distinct, and maps directly to safe Rust `&T` / `&expr`. The keyword family would reserve currently-valid identifiers, while the generic-like `Ref(...)` family collides with current nominal/call-shaped identifier space."
    )
    .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "The semantic evidence also shows a useful bounded last-use case: an owner move after the final reference use compiles, while move/reinitialization before a later reference use fails. A production successor should therefore attempt local last-use liveness before falling back to a documented conservative restriction."
    )
    .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(
        markdown,
        "This is research evidence only. Production Evolution syntax, parser behavior, ownership semantics, generated Rust, and runtime behavior are unchanged."
    )
    .expect("writing Markdown cannot fail");
    fs::write(out.join("report.md"), &markdown)
        .unwrap_or_else(|error| panic!("failed to write report Markdown: {error}"));
    print!("{markdown}");
}

#[test]
#[ignore = "research evidence; dedicated workflow runs this exact test"]
fn immutable_reference_surface_research_selects_bounded_candidate() {
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

    let ampersand_currently_invalid = lex("&").is_err();
    let keyword_identifiers_available =
        is_identifier("ref", "ref") && is_identifier("borrow", "borrow");
    let generic_ref_identifier_available = is_identifier("Ref", "Ref");

    let scratch = env::temp_dir().join(format!(
        "evo-immutable-reference-surface-research-{}",
        std::process::id()
    ));
    if scratch.exists() {
        fs::remove_dir_all(&scratch)
            .unwrap_or_else(|error| panic!("failed to reset {}: {error}", scratch.display()));
    }
    fs::create_dir_all(&scratch)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", scratch.display()));

    let findings: Vec<SemanticFinding> = SEMANTIC_CASES
        .iter()
        .map(|spec| run_case(spec, &scratch))
        .collect();
    let mismatch_count = findings
        .iter()
        .filter(|finding| !finding.expectation_matched)
        .count();
    let bounded_last_use_works = findings.iter().any(|finding| {
        finding.spec.class == SemanticClass::BoundedLastUse
            && finding.rust_compiled
            && finding.expectation_matched
    });
    let live_conflicts_rejected = findings
        .iter()
        .filter(|finding| {
            finding.spec.class == SemanticClass::SingleSourceReference
                && !finding.spec.expected_rust_compile
        })
        .all(|finding| !finding.rust_compiled && finding.expectation_matched);
    let multi_owner_fails_closed = findings
        .iter()
        .filter(|finding| finding.spec.class == SemanticClass::RequiresLifetimeRelation)
        .all(|finding| !finding.rust_compiled && finding.expectation_matched);
    let invalid_owner_fails_closed = findings
        .iter()
        .filter(|finding| finding.spec.class == SemanticClass::InvalidOwner)
        .all(|finding| !finding.rust_compiled && finding.expectation_matched);

    let punctuation_surface_compatible = ampersand_currently_invalid
        && keyword_identifiers_available
        && generic_ref_identifier_available;

    let verdict = if mismatch_count == 0
        && bounded_last_use_works
        && live_conflicts_rejected
        && multi_owner_fails_closed
        && invalid_owner_fails_closed
        && punctuation_surface_compatible
    {
        "SURFACE-CANDIDATE"
    } else if mismatch_count == 0
        && live_conflicts_rejected
        && multi_owner_fails_closed
        && invalid_owner_fails_closed
    {
        "SURFACE-CANDIDATE-WITH-CONSERVATIVE-LIVENESS"
    } else if mismatch_count == 0 && !multi_owner_fails_closed {
        "NEEDS-LIFETIME-RELATION-MODEL"
    } else {
        "DEFER"
    };

    let recommended_surface =
        if verdict.starts_with("SURFACE-CANDIDATE") && punctuation_surface_compatible {
            "PUNCTUATION-AMPERSAND"
        } else {
            "NONE"
        };

    let git_sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "local".to_owned());
    let out = env::var_os("EVO_IMMUTABLE_REFERENCE_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("evo-immutable-reference-surface-research")
        });

    write_reports(
        &findings,
        verdict,
        recommended_surface,
        &out,
        &git_sha,
        &rustc,
        ampersand_currently_invalid,
        keyword_identifiers_available,
        generic_ref_identifier_available,
    );

    assert_eq!(
        mismatch_count, 0,
        "all pre-registered compile expectations must match"
    );
    assert!(
        bounded_last_use_works,
        "bounded last-use control must compile"
    );
    assert!(
        live_conflicts_rejected,
        "owner move/reinit and move-through-reference conflicts must fail closed"
    );
    assert!(
        multi_owner_fails_closed,
        "multi-owner returned-reference relationships must fail closed in v0"
    );
    assert!(
        invalid_owner_fails_closed,
        "references to dead local owners must fail closed"
    );
    assert!(
        punctuation_surface_compatible,
        "surface selection relies on the current lexer compatibility probe"
    );
    assert_eq!(verdict, "SURFACE-CANDIDATE");
    assert_eq!(recommended_surface, "PUNCTUATION-AMPERSAND");
}
