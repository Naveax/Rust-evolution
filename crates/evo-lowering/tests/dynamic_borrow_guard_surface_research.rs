use evo_lexer::{TokenKind, lex};
use evo_parser::parse;
use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Classification {
    OwnedMutInstead,
    LexicalGuardsFirst,
    RequiresGuardValueType,
    ExplicitRcRefCellComposition,
    RequiresConcurrencyDesign,
    RejectHiddenCost,
}

impl Classification {
    const fn as_str(self) -> &'static str {
        match self {
            Self::OwnedMutInstead => "OWNED-MUT-INSTEAD",
            Self::LexicalGuardsFirst => "LEXICAL-GUARDS-FIRST",
            Self::RequiresGuardValueType => "REQUIRES-GUARD-VALUE-TYPE",
            Self::ExplicitRcRefCellComposition => "EXPLICIT-RC-REFCELL-COMPOSITION",
            Self::RequiresConcurrencyDesign => "REQUIRES-CONCURRENCY-DESIGN",
            Self::RejectHiddenCost => "REJECT-HIDDEN-COST",
        }
    }
}

struct Case {
    name: &'static str,
    classification: Classification,
    compile: bool,
    run_success: bool,
    stdout: Option<&'static str>,
    stderr_contains: Option<&'static str>,
    source: &'static str,
}

struct Finding {
    compile_matched: bool,
    run_matched: bool,
    stdout: String,
    stderr_summary: String,
}

const CASES: &[Case] = &[
    Case {
        name: "ordinary-owned-mutation-remains-cheapest",
        classification: Classification::OwnedMutInstead,
        compile: true,
        run_success: true,
        stdout: Some("8"),
        stderr_contains: None,
        source: r#"fn main(){let mut value=7_i64;value+=1;println!("{value}");}"#,
    },
    Case {
        name: "owned-cell-shared-guard",
        classification: Classification::LexicalGuardsFirst,
        compile: true,
        run_success: true,
        stdout: Some("7"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let guard=cell.borrow();println!("{}",*guard);}"#,
    },
    Case {
        name: "owned-cell-exclusive-guard",
        classification: Classification::LexicalGuardsFirst,
        compile: true,
        run_success: true,
        stdout: Some("8"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);{let mut guard=cell.borrow_mut();*guard+=1;}println!("{}",*cell.borrow());}"#,
    },
    Case {
        name: "sequential-exclusive-guards-after-release",
        classification: Classification::LexicalGuardsFirst,
        compile: true,
        run_success: true,
        stdout: Some("9"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);{let mut a=cell.borrow_mut();*a+=1;}{let mut b=cell.borrow_mut();*b+=1;}println!("{}",*cell.borrow());}"#,
    },
    Case {
        name: "multiple-simultaneous-shared-guards",
        classification: Classification::LexicalGuardsFirst,
        compile: true,
        run_success: true,
        stdout: Some("14"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let a=cell.borrow();let b=cell.borrow();println!("{}",*a+*b);}"#,
    },
    Case {
        name: "shared-then-exclusive-panics",
        classification: Classification::LexicalGuardsFirst,
        compile: true,
        run_success: false,
        stdout: None,
        stderr_contains: Some("already borrowed"),
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let shared=cell.borrow();let _exclusive=cell.borrow_mut();println!("{}",*shared);}"#,
    },
    Case {
        name: "exclusive-then-shared-panics",
        classification: Classification::LexicalGuardsFirst,
        compile: true,
        run_success: false,
        stdout: None,
        stderr_contains: Some("already mutably borrowed"),
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let exclusive=cell.borrow_mut();let _shared=cell.borrow();println!("{}",*exclusive);}"#,
    },
    Case {
        name: "fallible-shared-acquisition-conflict",
        classification: Classification::LexicalGuardsFirst,
        compile: true,
        run_success: true,
        stdout: Some("failure"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let exclusive=cell.borrow_mut();match cell.try_borrow(){Ok(_)=>println!("success"),Err(_)=>println!("failure")}drop(exclusive);}"#,
    },
    Case {
        name: "fallible-exclusive-acquisition-conflict",
        classification: Classification::LexicalGuardsFirst,
        compile: true,
        run_success: true,
        stdout: Some("failure"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let shared=cell.borrow();match cell.try_borrow_mut(){Ok(_)=>println!("success"),Err(_)=>println!("failure")}drop(shared);}"#,
    },
    Case {
        name: "explicit-guard-drop-restores-availability",
        classification: Classification::LexicalGuardsFirst,
        compile: true,
        run_success: true,
        stdout: Some("8"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let shared=cell.borrow();assert_eq!(*shared,7);drop(shared);*cell.borrow_mut()+=1;println!("{}",*cell.borrow());}"#,
    },
    Case {
        name: "lexical-scope-end-restores-availability",
        classification: Classification::LexicalGuardsFirst,
        compile: true,
        run_success: true,
        stdout: Some("8"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);{let shared=cell.borrow();assert_eq!(*shared,7);}{let mut exclusive=cell.borrow_mut();*exclusive+=1;}println!("{}",*cell.borrow());}"#,
    },
    Case {
        name: "guard-remains-live-across-statements-in-scope",
        classification: Classification::LexicalGuardsFirst,
        compile: true,
        run_success: true,
        stdout: Some("7 14"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let guard=cell.borrow();let first=*guard;let second=*guard*2;println!("{first} {second}");}"#,
    },
    Case {
        name: "shared-guard-cannot-mutate-payload",
        classification: Classification::LexicalGuardsFirst,
        compile: false,
        run_success: false,
        stdout: None,
        stderr_contains: Some("cannot assign to data in dereference of"),
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let guard=cell.borrow();*guard+=1;}"#,
    },
    Case {
        name: "guard-move-invalidates-old-binding",
        classification: Classification::RequiresGuardValueType,
        compile: false,
        run_success: false,
        stdout: None,
        stderr_contains: Some("borrow of moved value"),
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let guard=cell.borrow();let moved=guard;println!("{} {}",*moved,*guard);}"#,
    },
    Case {
        name: "returned-guard-requires-explicit-guard-type",
        classification: Classification::RequiresGuardValueType,
        compile: true,
        run_success: true,
        stdout: Some("7"),
        stderr_contains: None,
        source: r#"use std::cell::{Ref,RefCell};fn view(cell:&RefCell<i64>)->Ref<'_,i64>{cell.borrow()}fn main(){let cell=RefCell::new(7_i64);let guard=view(&cell);println!("{}",*guard);}"#,
    },
    Case {
        name: "explicit-rc-refcell-owner-duplication",
        classification: Classification::ExplicitRcRefCellComposition,
        compile: true,
        run_success: true,
        stdout: Some("8 2"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;use std::rc::Rc;fn main(){let owner=Rc::new(RefCell::new(7_i64));let alias=Rc::clone(&owner);*alias.borrow_mut()+=1;println!("{} {}",*owner.borrow(),Rc::strong_count(&owner));}"#,
    },
    Case {
        name: "drop-one-shared-owner-retains-cell",
        classification: Classification::ExplicitRcRefCellComposition,
        compile: true,
        run_success: true,
        stdout: Some("7 1"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;use std::rc::Rc;fn main(){let owner=Rc::new(RefCell::new(7_i64));let alias=Rc::clone(&owner);drop(owner);println!("{} {}",*alias.borrow(),Rc::strong_count(&alias));}"#,
    },
    Case {
        name: "shared-cell-alias-is-not-payload-deep-clone",
        classification: Classification::RejectHiddenCost,
        compile: true,
        run_success: true,
        stdout: Some("true false"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;use std::rc::Rc;fn main(){let owner=Rc::new(RefCell::new(7_i64));let alias=Rc::clone(&owner);let deep=Rc::new(RefCell::new(*owner.borrow()));println!("{} {}",Rc::ptr_eq(&owner,&alias),Rc::ptr_eq(&owner,&deep));}"#,
    },
    Case {
        name: "ordinary-immutable-reference-remains-distinct",
        classification: Classification::RejectHiddenCost,
        compile: true,
        run_success: true,
        stdout: Some("7"),
        stderr_contains: None,
        source: r#"fn main(){let value=7_i64;let reference=&value;println!("{}",*reference);}"#,
    },
    Case {
        name: "arc-mutex-is-separate-concurrency-model",
        classification: Classification::RequiresConcurrencyDesign,
        compile: true,
        run_success: true,
        stdout: Some("8"),
        stderr_contains: None,
        source: r#"use std::sync::{Arc,Mutex};use std::thread;fn main(){let owner=Arc::new(Mutex::new(7_i64));let alias=Arc::clone(&owner);thread::spawn(move||*alias.lock().unwrap()+=1).join().unwrap();println!("{}",*owner.lock().unwrap());}"#,
    },
    Case {
        name: "exclusive-owner-get-mut-needs-no-dynamic-borrow",
        classification: Classification::OwnedMutInstead,
        compile: true,
        run_success: true,
        stdout: Some("8"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let mut cell=RefCell::new(7_i64);*cell.get_mut()+=1;println!("{}",cell.into_inner());}"#,
    },
];

#[derive(Debug)]
struct SurfaceEvidence {
    contextual_words_are_identifiers: bool,
    ordinary_identifier_compatibility: bool,
    ordinary_cell_call_parse: bool,
    cell_type_candidate_current_parser_rejects: bool,
    cell_constructor_candidate_current_parser_rejects: bool,
    lexical_borrow_candidate_current_parser_rejects: bool,
    fallible_borrow_candidate_current_parser_rejects: bool,
}

fn rustc() -> OsString {
    env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"))
}

fn rustc_vv() -> String {
    let output = Command::new(rustc())
        .arg("-Vv")
        .output()
        .expect("rustc -Vv should execute");
    assert!(output.status.success(), "rustc -Vv should succeed");
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

fn run_case(case: &Case, root: &Path) -> Finding {
    let dir = root.join(case.name);
    fs::create_dir_all(&dir).expect("case directory should be creatable");
    let source = dir.join("case.rs");
    let binary = dir.join(format!("case{}", env::consts::EXE_SUFFIX));
    fs::write(&source, case.source).expect("case source should be writable");

    let compile = Command::new(rustc())
        .arg("--edition=2024")
        .arg("-C")
        .arg("opt-level=3")
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("case rustc should execute");

    let compiled = compile.status.success();
    let compile_stderr = String::from_utf8_lossy(&compile.stderr);
    let compile_matched = compiled == case.compile
        && (compiled
            || case
                .stderr_contains
                .is_none_or(|expected| compile_stderr.contains(expected)));

    if !compiled {
        return Finding {
            compile_matched,
            run_matched: !case.compile,
            stdout: String::new(),
            stderr_summary: summarize(&compile.stderr),
        };
    }

    let run = Command::new(&binary)
        .output()
        .expect("compiled research case should execute");
    let stdout = String::from_utf8_lossy(&run.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&run.stderr);
    let run_matched = run.status.success() == case.run_success
        && case.stdout.is_none_or(|expected| stdout == expected)
        && case
            .stderr_contains
            .is_none_or(|expected| stderr.contains(expected));

    Finding {
        compile_matched,
        run_matched,
        stdout,
        stderr_summary: summarize(&run.stderr),
    }
}

fn surface_evidence() -> SurfaceEvidence {
    let contextual = lex("cell borrow borrow_mut try_borrow try_borrow_mut as\n")
        .expect("candidate contextual words should lex");
    let identifiers: Vec<_> = contextual
        .iter()
        .filter_map(|token| match &token.kind {
            TokenKind::Identifier(name) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let contextual_words_are_identifiers = identifiers.as_slice()
        == [
            "cell",
            "borrow",
            "borrow_mut",
            "try_borrow",
            "try_borrow_mut",
            "as",
        ];

    let ordinary = concat!(
        "fn borrow(value int) int\n",
        "return value\n",
        "end\n",
        "cell = borrow(7)\n",
        "print cell\n",
    );
    let ordinary_identifier_compatibility =
        lex(ordinary).is_ok_and(|tokens| parse(&tokens).is_ok());

    let ordinary_cell_call = concat!(
        "fn cell(value int) int\n",
        "return value\n",
        "end\n",
        "value = cell(7)\n",
        "print value\n",
    );
    let ordinary_cell_call_parse =
        lex(ordinary_cell_call).is_ok_and(|tokens| parse(&tokens).is_ok());

    let cell_type = concat!(
        "record Item\n",
        "value int\n",
        "end\n",
        "fn inspect(state cell Item) int\n",
        "return 0\n",
        "end\n",
    );
    let cell_type_candidate_current_parser_rejects = lex(cell_type).is_ok_and(|tokens| {
        parse(&tokens).is_err_and(|error| {
            error
                .message
                .contains("expected ')' after function parameters")
        })
    });

    let cell_constructor = concat!(
        "record Item\n",
        "value int\n",
        "end\n",
        "state = cell Item(value = 1)\n",
        "print 0\n",
    );
    let cell_constructor_candidate_current_parser_rejects =
        lex(cell_constructor).is_ok_and(|tokens| {
            parse(&tokens).is_err_and(|error| {
                error
                    .message
                    .contains("expected end of line after statement")
            })
        });

    let lexical = concat!("borrow state as view\n", "print view\n", "end\n",);
    let lexical_borrow_candidate_current_parser_rejects = lex(lexical).is_ok_and(|tokens| {
        parse(&tokens).is_err_and(|error| error.message.contains("expected '=' after binding name"))
    });

    let fallible = concat!(
        "try_borrow state as view\n",
        "print view\n",
        "else\n",
        "print 0\n",
        "end\n",
    );
    let fallible_borrow_candidate_current_parser_rejects = lex(fallible).is_ok_and(|tokens| {
        parse(&tokens).is_err_and(|error| error.message.contains("expected '=' after binding name"))
    });

    SurfaceEvidence {
        contextual_words_are_identifiers,
        ordinary_identifier_compatibility,
        ordinary_cell_call_parse,
        cell_type_candidate_current_parser_rejects,
        cell_constructor_candidate_current_parser_rejects,
        lexical_borrow_candidate_current_parser_rejects,
        fallible_borrow_candidate_current_parser_rejects,
    }
}

fn count(findings: &[Finding], classification: Classification) -> usize {
    CASES
        .iter()
        .zip(findings)
        .filter(|(case, finding)| {
            case.classification == classification && finding.compile_matched && finding.run_matched
        })
        .count()
}

fn write_reports(findings: &[Finding], surface: &SurfaceEvidence, out: &Path, rustc: &str) {
    fs::create_dir_all(out).expect("report directory should be creatable");
    let sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "unknown".to_owned());
    let mismatches = findings
        .iter()
        .filter(|finding| !(finding.compile_matched && finding.run_matched))
        .count();

    let lexical_count = count(findings, Classification::LexicalGuardsFirst);
    let guard_value_count = count(findings, Classification::RequiresGuardValueType);
    let composition_count = count(findings, Classification::ExplicitRcRefCellComposition);
    let owned_mut_count = count(findings, Classification::OwnedMutInstead);
    let concurrency_count = count(findings, Classification::RequiresConcurrencyDesign);
    let reject_hidden_cost_count = count(findings, Classification::RejectHiddenCost);
    let lexical_guards_sufficient = mismatches == 0
        && lexical_count >= 10
        && guard_value_count >= 2
        && composition_count >= 2
        && owned_mut_count >= 2
        && concurrency_count >= 1
        && reject_hidden_cost_count >= 2
        && surface.contextual_words_are_identifiers
        && surface.ordinary_identifier_compatibility
        && surface.ordinary_cell_call_parse
        && surface.cell_type_candidate_current_parser_rejects
        && surface.cell_constructor_candidate_current_parser_rejects
        && surface.lexical_borrow_candidate_current_parser_rejects
        && surface.fallible_borrow_candidate_current_parser_rejects;
    let verdict = if lexical_guards_sufficient {
        "LEXICAL-GUARDS-FIRST"
    } else {
        "DEFER"
    };
    let recommended_cell_surface = if lexical_guards_sufficient {
        "EXPLICIT-OWNED-CELL"
    } else {
        "NONE"
    };
    assert_eq!(verdict, "LEXICAL-GUARDS-FIRST");
    assert_eq!(recommended_cell_surface, "EXPLICIT-OWNED-CELL");

    let mut json = String::new();
    writeln!(json, "{{").unwrap();
    writeln!(json, "  \"git_sha\": {sha:?},").unwrap();
    writeln!(json, "  \"rustc_vv\": {rustc:?},").unwrap();
    writeln!(json, "  \"verdict\": {verdict:?},").unwrap();
    writeln!(
        json,
        "  \"recommended_cell_surface\": {recommended_cell_surface:?},"
    )
    .unwrap();
    writeln!(json, "  \"cell_type_surface\": \"cell T\",").unwrap();
    writeln!(json, "  \"cell_constructor_surface\": \"cell expr\",").unwrap();
    writeln!(json, "  \"case_count\": {},", CASES.len()).unwrap();
    writeln!(json, "  \"expectation_mismatches\": {mismatches},").unwrap();
    writeln!(json, "  \"lexical_guard_case_count\": {lexical_count},").unwrap();
    writeln!(
        json,
        "  \"guard_value_type_case_count\": {guard_value_count},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"explicit_rc_refcell_case_count\": {composition_count},"
    )
    .unwrap();
    writeln!(json, "  \"owned_mut_instead_count\": {owned_mut_count},").unwrap();
    writeln!(
        json,
        "  \"concurrency_boundary_count\": {concurrency_count},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"reject_hidden_cost_count\": {reject_hidden_cost_count},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"contextual_words_are_identifiers\": {},",
        surface.contextual_words_are_identifiers
    )
    .unwrap();
    writeln!(
        json,
        "  \"ordinary_identifier_compatibility\": {},",
        surface.ordinary_identifier_compatibility
    )
    .unwrap();
    writeln!(
        json,
        "  \"ordinary_cell_call_parse\": {},",
        surface.ordinary_cell_call_parse
    )
    .unwrap();
    writeln!(
        json,
        "  \"cell_type_candidate_current_parser_rejects\": {},",
        surface.cell_type_candidate_current_parser_rejects
    )
    .unwrap();
    writeln!(
        json,
        "  \"cell_constructor_candidate_current_parser_rejects\": {},",
        surface.cell_constructor_candidate_current_parser_rejects
    )
    .unwrap();
    writeln!(
        json,
        "  \"lexical_borrow_candidate_current_parser_rejects\": {},",
        surface.lexical_borrow_candidate_current_parser_rejects
    )
    .unwrap();
    writeln!(
        json,
        "  \"fallible_borrow_candidate_current_parser_rejects\": {},",
        surface.fallible_borrow_candidate_current_parser_rejects
    )
    .unwrap();
    writeln!(json, "  \"escaping_guard_requires_value_type\": true,").unwrap();
    writeln!(json, "  \"production_guard_escape_authorized\": false,").unwrap();
    writeln!(json, "  \"cases\": [").unwrap();
    for (index, (case, finding)) in CASES.iter().zip(findings).enumerate() {
        let comma = if index + 1 == CASES.len() { "" } else { "," };
        writeln!(
            json,
            "    {{\"name\": {:?}, \"classification\": {:?}, \"compile_matched\": {}, \"run_matched\": {}, \"stdout\": {:?}, \"stderr_summary\": {:?}}}{comma}",
            case.name,
            case.classification.as_str(),
            finding.compile_matched,
            finding.run_matched,
            finding.stdout,
            finding.stderr_summary,
        )
        .unwrap();
    }
    writeln!(json, "  ]").unwrap();
    writeln!(json, "}}").unwrap();
    fs::write(out.join("report.json"), json).expect("json report should be writable");

    let mut csv = String::from("git_sha,name,classification,compile_matched,run_matched\n");
    for (case, finding) in CASES.iter().zip(findings) {
        writeln!(
            csv,
            "{sha},{},{},{},{}",
            case.name,
            case.classification.as_str(),
            finding.compile_matched,
            finding.run_matched
        )
        .unwrap();
    }
    fs::write(out.join("report.csv"), csv).expect("csv report should be writable");

    let mut markdown = String::new();
    writeln!(markdown, "# Dynamic borrow guard surface v0 research\n").unwrap();
    writeln!(markdown, "- git_sha: `{sha}`").unwrap();
    writeln!(markdown, "- verdict: **{verdict}**").unwrap();
    writeln!(
        markdown,
        "- recommended cell surface: **{recommended_cell_surface}** (`cell T` + `cell expr`)"
    )
    .unwrap();
    writeln!(markdown, "- cases: **{}**", CASES.len()).unwrap();
    writeln!(markdown, "- expectation mismatches: **{mismatches}**").unwrap();
    writeln!(
        markdown,
        "- lexical/local guards are sufficient for the bounded first slice: **yes**"
    )
    .unwrap();
    writeln!(
        markdown,
        "- returned/escaping guards require a first-class guard value/lifetime contract: **yes**"
    )
    .unwrap();
    writeln!(
        markdown,
        "- production guard escape authorized by this research: **no**"
    )
    .unwrap();
    writeln!(markdown, "\n## Candidate source contract\n").unwrap();
    writeln!(markdown, "```text").unwrap();
    writeln!(markdown, "state = cell Item(value = 1)").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "fn inspect(state cell Item) int").unwrap();
    writeln!(markdown, "    ...").unwrap();
    writeln!(markdown, "end").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "borrow state as view").unwrap();
    writeln!(markdown, "    ...").unwrap();
    writeln!(markdown, "end").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "borrow_mut state as edit").unwrap();
    writeln!(markdown, "    ...").unwrap();
    writeln!(markdown, "end").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "try_borrow state as view").unwrap();
    writeln!(markdown, "    ...").unwrap();
    writeln!(markdown, "else").unwrap();
    writeln!(markdown, "    ...").unwrap();
    writeln!(markdown, "end").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "try_borrow_mut state as edit").unwrap();
    writeln!(markdown, "    ...").unwrap();
    writeln!(markdown, "else").unwrap();
    writeln!(markdown, "    ...").unwrap();
    writeln!(markdown, "end").unwrap();
    writeln!(markdown, "```\n").unwrap();
    writeln!(
        markdown,
        "The binding is lexical to the success/body scope. Scope exit releases the Rust guard. \
         Panicking and fallible acquisition remain distinct. Returned or otherwise escaping guards \
         are outside the bounded candidate."
    )
    .unwrap();
    writeln!(markdown, "\n## Matrix\n").unwrap();
    writeln!(
        markdown,
        "| case | classification | compile matched | run matched |"
    )
    .unwrap();
    writeln!(markdown, "|---|---|---:|---:|").unwrap();
    for (case, finding) in CASES.iter().zip(findings) {
        writeln!(
            markdown,
            "| {} | {} | {} | {} |",
            case.name,
            case.classification.as_str(),
            finding.compile_matched,
            finding.run_matched
        )
        .unwrap();
    }
    fs::write(out.join("report.md"), markdown).expect("markdown report should be writable");
}

#[test]
#[ignore = "research evidence; dedicated workflow runs this exact test"]
fn dynamic_borrow_guard_surface_research_selects_lexical_guards_first() {
    let rustc = rustc_vv();
    if env::var_os("EVO_REQUIRE_PINNED_RUSTC").is_some() {
        assert!(
            rustc
                .lines()
                .next()
                .is_some_and(|line| line.contains("rustc 1.98.0")),
            "research workflow requires Rust 1.98.0, got: {rustc}"
        );
    }

    let surface = surface_evidence();
    assert!(surface.contextual_words_are_identifiers);
    assert!(surface.ordinary_identifier_compatibility);
    assert!(surface.ordinary_cell_call_parse);
    assert!(surface.cell_type_candidate_current_parser_rejects);
    assert!(surface.cell_constructor_candidate_current_parser_rejects);
    assert!(surface.lexical_borrow_candidate_current_parser_rejects);
    assert!(surface.fallible_borrow_candidate_current_parser_rejects);

    let scratch = env::temp_dir().join(format!(
        "evo-dynamic-borrow-guard-surface-research-{}",
        std::process::id()
    ));
    if scratch.exists() {
        fs::remove_dir_all(&scratch).expect("stale scratch should be removable");
    }
    fs::create_dir_all(&scratch).expect("scratch should be creatable");

    let findings: Vec<_> = CASES.iter().map(|case| run_case(case, &scratch)).collect();
    assert!(CASES.len() >= 20);

    let mismatches: Vec<_> = CASES
        .iter()
        .zip(&findings)
        .filter(|(_, finding)| !(finding.compile_matched && finding.run_matched))
        .map(|(case, finding)| {
            (
                case.name,
                finding.compile_matched,
                finding.run_matched,
                finding.stdout.as_str(),
                finding.stderr_summary.as_str(),
            )
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "dynamic borrow guard research mismatches: {mismatches:?}"
    );

    assert!(count(&findings, Classification::LexicalGuardsFirst) >= 10);
    assert!(count(&findings, Classification::RequiresGuardValueType) >= 2);
    assert!(count(&findings, Classification::ExplicitRcRefCellComposition) >= 2);
    assert!(count(&findings, Classification::OwnedMutInstead) >= 2);
    assert!(count(&findings, Classification::RequiresConcurrencyDesign) >= 1);
    assert!(count(&findings, Classification::RejectHiddenCost) >= 2);

    let out = env::var_os("EVO_DYNAMIC_BORROW_GUARD_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/evo-dynamic-borrow-guard-surface-research")
        });
    write_reports(&findings, &surface, &out, &rustc);

    fs::remove_dir_all(scratch).expect("scratch should be removable");
}
