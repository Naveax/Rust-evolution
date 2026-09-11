use evo_formatter::format_source;
use evo_lexer::{TokenKind, lex};
use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceClassification {
    DeferGenericInfrastructure,
    DeferRustCoupledGeneric,
    PreferredCandidate,
    DeferNewPunctuation,
}

impl SurfaceClassification {
    const fn as_str(self) -> &'static str {
        match self {
            Self::DeferGenericInfrastructure => "DEFER-GENERIC-INFRASTRUCTURE",
            Self::DeferRustCoupledGeneric => "DEFER-RUST-COUPLED-GENERIC",
            Self::PreferredCandidate => "PREFERRED-CANDIDATE",
            Self::DeferNewPunctuation => "DEFER-NEW-PUNCTUATION",
        }
    }
}

#[derive(Debug)]
struct SurfaceSpec {
    name: &'static str,
    classification: SurfaceClassification,
    type_spelling: &'static str,
    create_spelling: &'static str,
    duplicate_spelling: &'static str,
    parser_cost: &'static str,
    lexer_cost: &'static str,
    compatibility_cost: &'static str,
    formatter_cost: &'static str,
    codegen_mapping: &'static str,
    identifier_words: &'static [&'static str],
    reason: &'static str,
}

const SURFACES: &[SurfaceSpec] = &[
    SurfaceSpec {
        name: "GENERIC-SHARED",
        classification: SurfaceClassification::DeferGenericInfrastructure,
        type_spelling: "Shared<Item>\n",
        create_spelling: "shared(owner)\n",
        duplicate_spelling: "shared_clone(owner)\n",
        parser_cost: "requires generic-looking type parsing absent from TypeName v0",
        lexer_cost: "reuses existing Less/Greater comparison tokens",
        compatibility_cost: "Shared/shared/shared_clone are currently identifiers",
        formatter_cost: "current formatter spaces Less/Greater as comparison operators",
        codegen_mapping: "Shared<T> -> Rc<T>; shared -> Rc::new; shared_clone -> Rc::clone",
        identifier_words: &["Shared", "shared", "shared_clone"],
        reason: "language-neutral naming is clear, but v0 has no general generic type surface and angle brackets already carry comparison formatting semantics",
    },
    SurfaceSpec {
        name: "RUST-TRANSPARENT-RC",
        classification: SurfaceClassification::DeferRustCoupledGeneric,
        type_spelling: "Rc<Item>\n",
        create_spelling: "rc(owner)\n",
        duplicate_spelling: "rc_clone(owner)\n",
        parser_cost: "requires generic-looking type parsing absent from TypeName v0",
        lexer_cost: "reuses existing Less/Greater comparison tokens",
        compatibility_cost: "Rc/rc/rc_clone are currently identifiers",
        formatter_cost: "current formatter spaces Less/Greater as comparison operators",
        codegen_mapping: "source names mirror Rc<T>, Rc::new and Rc::clone",
        identifier_words: &["Rc", "rc", "rc_clone"],
        reason: "backend transparency is excellent, but it couples Evolution spelling to Rust and pays the same missing-generic/frontend cost",
    },
    SurfaceSpec {
        name: "CONTEXTUAL-WORDS",
        classification: SurfaceClassification::PreferredCandidate,
        type_spelling: "shared Item\n",
        create_spelling: "share owner\n",
        duplicate_spelling: "dup owner\n",
        parser_cost: "dedicated contextual type/prefix forms; no general generic AST required",
        lexer_cost: "no new token kinds; words remain Identifier tokens",
        compatibility_cost: "can remain contextual in token sequences that are invalid in current grammar",
        formatter_cost: "current identifier spacing already preserves the isolated forms",
        codegen_mapping: "shared Item -> Rc<Item>; share -> Rc::new; dup -> Rc::clone",
        identifier_words: &["shared", "share", "dup"],
        reason: "allocation and owner duplication use distinct visible verbs while avoiding generic syntax, new punctuation and mandatory hard-keyword reservation",
    },
    SurfaceSpec {
        name: "PUNCTUATION-AT",
        classification: SurfaceClassification::DeferNewPunctuation,
        type_spelling: "@Item\n",
        create_spelling: "@owner\n",
        duplicate_spelling: "@@owner\n",
        parser_cost: "dedicated built-in type and unary forms",
        lexer_cost: "requires a new punctuation token; @ is currently rejected",
        compatibility_cost: "does not reinterpret currently valid @ source because @ is invalid today",
        formatter_cost: "requires new punctuation spacing/idempotence rules",
        codegen_mapping: "@Item -> Rc<Item>; @expr -> Rc::new; @@expr -> Rc::clone",
        identifier_words: &[],
        reason: "source compatibility is strong and allocation/duplication differ visibly, but punctuation has weak semantic discoverability and higher lexer/formatter cost",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeStatus {
    Pass,
    Fail,
    NotApplicable,
}

impl ProbeStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::NotApplicable => "N/A",
        }
    }
}

#[derive(Debug)]
struct TextProbe {
    status: ProbeStatus,
    formatted: String,
}

#[derive(Debug)]
struct SurfaceFinding {
    spec: &'static SurfaceSpec,
    type_probe: TextProbe,
    create_probe: TextProbe,
    duplicate_probe: TextProbe,
    identifiers_remain_identifiers: ProbeStatus,
}

#[derive(Debug)]
struct SemanticCase {
    name: &'static str,
    contract: &'static str,
    expected_rust_compile: bool,
    expected_stdout: Option<&'static str>,
    rust_source: &'static str,
}

#[derive(Debug)]
struct SemanticFinding {
    spec: &'static SemanticCase,
    rust_compiled: bool,
    compile_expectation_matched: bool,
    rust_ran: bool,
    runtime_expectation_matched: bool,
    stdout: String,
    stderr_summary: String,
}

const SEMANTIC_CASES: &[SemanticCase] = &[
    SemanticCase {
        name: "create-and-inspect",
        contract: "initial shared allocation is explicit and readable through the handle",
        expected_rust_compile: true,
        expected_stdout: Some("1 7"),
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    println!("{} {}", Rc::strong_count(&owner), owner.value);
}
"#,
    },
    SemanticCase {
        name: "explicit-handle-duplication",
        contract: "duplicating an owner handle performs one visible strong-count increment",
        expected_rust_compile: true,
        expected_stdout: Some("2 1"),
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let alias = Rc::clone(&owner);
    let with_alias = Rc::strong_count(&owner);
    drop(alias);
    println!("{} {}", with_alias, Rc::strong_count(&owner));
}
"#,
    },
    SemanticCase {
        name: "ordinary-assignment-moves-handle",
        contract: "ordinary assignment moves a handle and does not increment the strong count",
        expected_rust_compile: true,
        expected_stdout: Some("2 2 14"),
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let alias = Rc::clone(&owner);
    let before = Rc::strong_count(&owner);
    let moved = owner;
    let after = Rc::strong_count(&moved);
    println!("{} {} {}", before, after, moved.value + alias.value);
}
"#,
    },
    SemanticCase {
        name: "drop-one-retain-other",
        contract: "dropping one owner keeps the payload alive while another owner remains",
        expected_rust_compile: true,
        expected_stdout: Some("1 7"),
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let alias = Rc::clone(&owner);
    drop(owner);
    println!("{} {}", Rc::strong_count(&alias), alias.value);
}
"#,
    },
    SemanticCase {
        name: "function-by-value-move",
        contract: "by-value function forwarding moves the handle without an implicit count increment",
        expected_rust_compile: true,
        expected_stdout: Some("1 7"),
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn forward(item: Rc<Item>) -> Rc<Item> { item }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let moved = forward(owner);
    println!("{} {}", Rc::strong_count(&moved), moved.value);
}
"#,
    },
    SemanticCase {
        name: "reuse-after-function-move-rejected",
        contract: "a handle moved into a by-value call is unavailable afterward",
        expected_rust_compile: false,
        expected_stdout: None,
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn forward(item: Rc<Item>) -> Rc<Item> { item }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let moved = forward(owner);
    println!("{}", owner.value + moved.value);
}
"#,
    },
    SemanticCase {
        name: "duplicate-then-forward-return",
        contract: "explicit duplication preserves one owner while another handle is forwarded and returned",
        expected_rust_compile: true,
        expected_stdout: Some("2 7 1"),
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn forward(item: Rc<Item>) -> Rc<Item> { item }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let forwarded = forward(Rc::clone(&owner));
    let with_forwarded = Rc::strong_count(&owner);
    let value = forwarded.value;
    drop(forwarded);
    println!("{} {} {}", with_forwarded, value, Rc::strong_count(&owner));
}
"#,
    },
    SemanticCase {
        name: "repeat-duplicate-drop",
        contract: "control-flow duplication makes repeated refcount work explicit",
        expected_rust_compile: true,
        expected_stdout: Some("21 1"),
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
    SemanticCase {
        name: "payload-borrow",
        contract: "borrowing through a shared handle is still an ordinary non-owning reference",
        expected_rust_compile: true,
        expected_stdout: Some("7 1"),
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let borrowed: &Item = &owner;
    println!("{} {}", borrowed.value, Rc::strong_count(&owner));
}
"#,
    },
    SemanticCase {
        name: "source-handle-move-with-live-borrow-rejected",
        contract: "another owner does not permit moving the particular handle that sources a live payload borrow",
        expected_rust_compile: false,
        expected_stdout: None,
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let _alias = Rc::clone(&owner);
    let borrowed: &Item = &owner;
    let moved = owner;
    println!("{}", borrowed.value);
    drop(moved);
}
"#,
    },
    SemanticCase {
        name: "different-handle-move-with-live-borrow-allowed",
        contract: "moving a different duplicated handle does not invalidate a borrow sourced from the original handle",
        expected_rust_compile: true,
        expected_stdout: Some("7 2"),
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let alias = Rc::clone(&owner);
    let borrowed: &Item = &owner;
    let moved = alias;
    println!("{} {}", borrowed.value, Rc::strong_count(&moved));
    drop(moved);
}
"#,
    },
    SemanticCase {
        name: "borrow-final-use-then-source-move",
        contract: "bounded final-use liveness permits moving the source handle after the final payload-reference use",
        expected_rust_compile: true,
        expected_stdout: Some("7\n7"),
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let borrowed: &Item = &owner;
    println!("{}", borrowed.value);
    let moved = owner;
    println!("{}", moved.value);
}
"#,
    },
    SemanticCase {
        name: "handle-duplicate-vs-deep-clone",
        contract: "owner duplication is observably distinct from payload deep clone and a second allocation",
        expected_rust_compile: true,
        expected_stdout: Some("2 1 false"),
        rust_source: r#"
use std::rc::Rc;
#[derive(Clone)]
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    let handle_duplicate = Rc::clone(&owner);
    let deep_clone = Rc::new((*owner).clone());
    let _ = handle_duplicate.value + deep_clone.value;
    println!(
        "{} {} {}",
        Rc::strong_count(&owner),
        Rc::strong_count(&deep_clone),
        Rc::ptr_eq(&owner, &deep_clone)
    );
}
"#,
    },
    SemanticCase {
        name: "immutable-shared-mutation-rejected",
        contract: "immutable shared ownership does not silently introduce interior mutability",
        expected_rust_compile: false,
        expected_stdout: None,
        rust_source: r#"
use std::rc::Rc;
struct Item { value: i64 }
fn main() {
    let owner = Rc::new(Item { value: 7 });
    owner.value += 1;
}
"#,
    },
    SemanticCase {
        name: "cross-thread-transfer-rejected",
        contract: "the one-thread handle is not silently upgraded to Arc-like thread-safe ownership",
        expected_rust_compile: false,
        expected_stdout: None,
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
];

fn probe_text(source: &str) -> TextProbe {
    match lex(source) {
        Ok(tokens) => TextProbe {
            status: ProbeStatus::Pass,
            formatted: format_source(source, &tokens),
        },
        Err(_) => TextProbe {
            status: ProbeStatus::Fail,
            formatted: String::new(),
        },
    }
}

fn identifiers_remain_identifiers(words: &[&str]) -> ProbeStatus {
    if words.is_empty() {
        return ProbeStatus::NotApplicable;
    }

    let all_identifiers = words.iter().all(|word| {
        let Ok(tokens) = lex(word) else {
            return false;
        };
        matches!(tokens.first().map(|token| &token.kind), Some(TokenKind::Identifier(name)) if name == word)
    });
    if all_identifiers {
        ProbeStatus::Pass
    } else {
        ProbeStatus::Fail
    }
}

fn run_surface_probe(spec: &'static SurfaceSpec) -> SurfaceFinding {
    SurfaceFinding {
        spec,
        type_probe: probe_text(spec.type_spelling),
        create_probe: probe_text(spec.create_spelling),
        duplicate_probe: probe_text(spec.duplicate_spelling),
        identifiers_remain_identifiers: identifiers_remain_identifiers(spec.identifier_words),
    }
}

fn parser_has_general_generic_type_ast() -> bool {
    let parser_source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../evo-parser/src/lib.rs");
    let source = fs::read_to_string(&parser_source)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", parser_source.display()));
    source.contains("TypeName::Generic") || source.contains("Generic(Box<TypeName>)")
}

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

fn normalize_stdout(stdout: &[u8]) -> String {
    String::from_utf8_lossy(stdout).trim().to_owned()
}

fn stderr_summary(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(5)
        .collect::<Vec<_>>()
        .join(" | ")
}

fn run_semantic_case(spec: &'static SemanticCase, root: &Path) -> SemanticFinding {
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
        .arg("evo_explicit_shared_handle_case")
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

    SemanticFinding {
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

fn write_surface_csv(findings: &[SurfaceFinding], out: &Path) {
    let mut csv = String::from(
        "surface,classification,type_spelling,create_spelling,duplicate_spelling,type_lex,create_lex,duplicate_lex,identifiers_remain_identifiers,formatted_type,formatted_create,formatted_duplicate,parser_cost,lexer_cost,compatibility_cost,formatter_cost,codegen_mapping,reason\n",
    );
    for item in findings {
        writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            csv_cell(item.spec.name),
            csv_cell(item.spec.classification.as_str()),
            csv_cell(item.spec.type_spelling.trim()),
            csv_cell(item.spec.create_spelling.trim()),
            csv_cell(item.spec.duplicate_spelling.trim()),
            csv_cell(item.type_probe.status.as_str()),
            csv_cell(item.create_probe.status.as_str()),
            csv_cell(item.duplicate_probe.status.as_str()),
            csv_cell(item.identifiers_remain_identifiers.as_str()),
            csv_cell(item.type_probe.formatted.trim()),
            csv_cell(item.create_probe.formatted.trim()),
            csv_cell(item.duplicate_probe.formatted.trim()),
            csv_cell(item.spec.parser_cost),
            csv_cell(item.spec.lexer_cost),
            csv_cell(item.spec.compatibility_cost),
            csv_cell(item.spec.formatter_cost),
            csv_cell(item.spec.codegen_mapping),
            csv_cell(item.spec.reason),
        )
        .expect("writing surface CSV to String cannot fail");
    }
    fs::write(out.join("surface-matrix.csv"), csv)
        .unwrap_or_else(|error| panic!("failed to write surface matrix: {error}"));
}

fn write_semantic_csv(findings: &[SemanticFinding], out: &Path) {
    let mut csv = String::from(
        "case,contract,expected_rust_compile,rust_compiled,compile_expectation_matched,rust_ran,runtime_expectation_matched,stdout,stderr_summary\n",
    );
    for item in findings {
        writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{}",
            csv_cell(item.spec.name),
            csv_cell(item.spec.contract),
            item.spec.expected_rust_compile,
            item.rust_compiled,
            item.compile_expectation_matched,
            item.rust_ran,
            item.runtime_expectation_matched,
            csv_cell(&item.stdout),
            csv_cell(&item.stderr_summary),
        )
        .expect("writing semantic CSV to String cannot fail");
    }
    fs::write(out.join("semantic-matrix.csv"), csv)
        .unwrap_or_else(|error| panic!("failed to write semantic matrix: {error}"));
}

fn write_report(
    surface_findings: &[SurfaceFinding],
    semantic_findings: &[SemanticFinding],
    out: &Path,
    git_sha: &str,
    rustc: &str,
) {
    fs::create_dir_all(out)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", out.display()));
    write_surface_csv(surface_findings, out);
    write_semantic_csv(semantic_findings, out);

    let compile_mismatches = semantic_findings
        .iter()
        .filter(|item| !item.compile_expectation_matched)
        .count();
    let runtime_mismatches = semantic_findings
        .iter()
        .filter(|item| !item.runtime_expectation_matched)
        .count();
    let preferred = surface_findings
        .iter()
        .find(|item| item.spec.classification == SurfaceClassification::PreferredCandidate)
        .expect("preferred surface must exist");
    let generic = surface_findings
        .iter()
        .find(|item| item.spec.name == "GENERIC-SHARED")
        .expect("generic surface must exist");
    let punctuation = surface_findings
        .iter()
        .find(|item| item.spec.name == "PUNCTUATION-AT")
        .expect("punctuation surface must exist");
    let parser_has_generics = parser_has_general_generic_type_ast();
    let generic_angle_formatter_rewrite_observed =
        generic.type_probe.formatted != generic.spec.type_spelling;
    let punctuation_requires_lexer_token = punctuation.type_probe.status == ProbeStatus::Fail;

    let mut json = String::new();
    writeln!(json, "{{").expect("writing JSON cannot fail");
    writeln!(json, "  \"git_sha\": {},", json_string(git_sha)).expect("writing JSON cannot fail");
    writeln!(json, "  \"rustc_vv\": {},", json_string(rustc)).expect("writing JSON cannot fail");
    writeln!(json, "  \"verdict\": \"IMPLEMENT-CANDIDATE\",").expect("writing JSON cannot fail");
    writeln!(json, "  \"recommended_surface\": \"CONTEXTUAL-WORDS\",")
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"surface_count\": {},", surface_findings.len())
        .expect("writing JSON cannot fail");
    writeln!(json, "  \"semantic_case_count\": {},", semantic_findings.len())
        .expect("writing JSON cannot fail");
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
        "  \"parser_has_general_generic_type_ast\": {parser_has_generics},"
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"generic_angle_formatter_rewrite_observed\": {generic_angle_formatter_rewrite_observed},"
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"punctuation_requires_lexer_token\": {punctuation_requires_lexer_token},"
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"preferred_type_probe\": {},",
        json_string(preferred.type_probe.status.as_str())
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"preferred_create_probe\": {},",
        json_string(preferred.create_probe.status.as_str())
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"preferred_duplicate_probe\": {},",
        json_string(preferred.duplicate_probe.status.as_str())
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"preferred_identifier_probe\": {},",
        json_string(preferred.identifiers_remain_identifiers.as_str())
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"preferred_formatted_type\": {},",
        json_string(preferred.type_probe.formatted.trim())
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"preferred_formatted_create\": {},",
        json_string(preferred.create_probe.formatted.trim())
    )
    .expect("writing JSON cannot fail");
    writeln!(
        json,
        "  \"preferred_formatted_duplicate\": {}",
        json_string(preferred.duplicate_probe.formatted.trim())
    )
    .expect("writing JSON cannot fail");
    writeln!(json, "}}").expect("writing JSON cannot fail");
    fs::write(out.join("report.json"), json)
        .unwrap_or_else(|error| panic!("failed to write report JSON: {error}"));

    let mut markdown = String::new();
    writeln!(markdown, "# Explicit shared handle surface v0 research")
        .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "- git_sha: `{git_sha}`").expect("writing Markdown cannot fail");
    writeln!(markdown, "- verdict: **IMPLEMENT-CANDIDATE**")
        .expect("writing Markdown cannot fail");
    writeln!(markdown, "- recommended surface: **CONTEXTUAL-WORDS**")
        .expect("writing Markdown cannot fail");
    writeln!(markdown, "- surface candidates: **{}**", surface_findings.len())
        .expect("writing Markdown cannot fail");
    writeln!(markdown, "- Rust semantic cases: **{}**", semantic_findings.len())
        .expect("writing Markdown cannot fail");
    writeln!(markdown, "- compile mismatches: **{compile_mismatches}**")
        .expect("writing Markdown cannot fail");
    writeln!(markdown, "- runtime mismatches: **{runtime_mismatches}**")
        .expect("writing Markdown cannot fail");
    writeln!(markdown, "- parser has general generic type AST: **{parser_has_generics}**")
        .expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "```text\n{rustc}\n```").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "## Surface findings").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    for item in surface_findings {
        writeln!(
            markdown,
            "- **{}** — {} — type `{}` / create `{}` / duplicate `{}` — {}",
            item.spec.name,
            item.spec.classification.as_str(),
            item.spec.type_spelling.trim(),
            item.spec.create_spelling.trim(),
            item.spec.duplicate_spelling.trim(),
            item.spec.reason,
        )
        .expect("writing Markdown cannot fail");
    }
    writeln!(markdown).expect("writing Markdown cannot fail");
    writeln!(markdown, "## Semantic findings").expect("writing Markdown cannot fail");
    writeln!(markdown).expect("writing Markdown cannot fail");
    for item in semantic_findings {
        writeln!(
            markdown,
            "- **{}** — compile expected/matched: `{}/{}`; runtime matched: `{}` — {}",
            item.spec.name,
            item.spec.expected_rust_compile,
            item.compile_expectation_matched,
            item.runtime_expectation_matched,
            item.spec.contract,
        )
        .expect("writing Markdown cannot fail");
    }
    fs::write(out.join("report.md"), markdown)
        .unwrap_or_else(|error| panic!("failed to write report Markdown: {error}"));
}

#[test]
#[ignore = "dedicated explicit shared handle surface research"]
fn explicit_shared_handle_surface_research_classifies_candidate() {
    let rustc = rustc_version();
    if env::var("EVO_REQUIRE_PINNED_RUSTC").ok().as_deref() == Some("1") {
        assert!(
            rustc.contains("release: 1.98.0"),
            "research must run with Rust 1.98.0: {rustc}"
        );
    }

    let git_sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "local".to_owned());
    let out = env::var_os("EVO_EXPLICIT_SHARED_HANDLE_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/evo-explicit-shared-handle-surface-research")
        });
    let semantic_root = out.join("semantic-cases");

    let surface_findings = SURFACES.iter().map(run_surface_probe).collect::<Vec<_>>();
    let semantic_findings = SEMANTIC_CASES
        .iter()
        .map(|spec| run_semantic_case(spec, &semantic_root))
        .collect::<Vec<_>>();

    assert!(
        !parser_has_general_generic_type_ast(),
        "generic surface assumptions must be revisited if TypeName gains general generics"
    );

    let preferred = surface_findings
        .iter()
        .find(|item| item.spec.name == "CONTEXTUAL-WORDS")
        .expect("contextual surface must exist");
    assert_eq!(preferred.type_probe.status, ProbeStatus::Pass);
    assert_eq!(preferred.create_probe.status, ProbeStatus::Pass);
    assert_eq!(preferred.duplicate_probe.status, ProbeStatus::Pass);
    assert_eq!(preferred.identifiers_remain_identifiers, ProbeStatus::Pass);
    assert_eq!(preferred.type_probe.formatted, preferred.spec.type_spelling);
    assert_eq!(preferred.create_probe.formatted, preferred.spec.create_spelling);
    assert_eq!(
        preferred.duplicate_probe.formatted,
        preferred.spec.duplicate_spelling
    );

    let generic = surface_findings
        .iter()
        .find(|item| item.spec.name == "GENERIC-SHARED")
        .expect("generic surface must exist");
    assert_eq!(generic.type_probe.status, ProbeStatus::Pass);
    assert_ne!(generic.type_probe.formatted, generic.spec.type_spelling);

    let punctuation = surface_findings
        .iter()
        .find(|item| item.spec.name == "PUNCTUATION-AT")
        .expect("punctuation surface must exist");
    assert_eq!(punctuation.type_probe.status, ProbeStatus::Fail);
    assert_eq!(punctuation.create_probe.status, ProbeStatus::Fail);
    assert_eq!(punctuation.duplicate_probe.status, ProbeStatus::Fail);

    assert!(
        semantic_findings
            .iter()
            .all(|item| item.compile_expectation_matched),
        "Rust compile expectations must match every semantic case"
    );
    assert!(
        semantic_findings
            .iter()
            .all(|item| item.runtime_expectation_matched),
        "Rust runtime expectations must match every semantic case"
    );

    write_report(&surface_findings, &semantic_findings, &out, &git_sha, &rustc);
}
