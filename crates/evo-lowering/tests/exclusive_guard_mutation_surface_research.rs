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
    WholePayloadReplace,
    MutablePlaceBoundary,
    ExplicitUpdateComparison,
    GuardValueBoundary,
    ExplicitRcRefCellComposition,
    ConcurrencyBoundary,
    RejectHiddenCost,
}

impl Classification {
    const fn as_str(self) -> &'static str {
        match self {
            Self::WholePayloadReplace => "WHOLE-PAYLOAD-REPLACE",
            Self::MutablePlaceBoundary => "MUTABLE-PLACE-BOUNDARY",
            Self::ExplicitUpdateComparison => "EXPLICIT-UPDATE-COMPARISON",
            Self::GuardValueBoundary => "GUARD-VALUE-BOUNDARY",
            Self::ExplicitRcRefCellComposition => "EXPLICIT-RC-REFCELL-COMPOSITION",
            Self::ConcurrencyBoundary => "CONCURRENCY-BOUNDARY",
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
        name: "scalar-whole-payload-replacement",
        classification: Classification::WholePayloadReplace,
        compile: true,
        run_success: true,
        stdout: Some("9"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);{let mut edit=cell.borrow_mut();*edit=9;}println!("{}",*cell.borrow());}"#,
    },
    Case {
        name: "record-whole-payload-replacement",
        classification: Classification::WholePayloadReplace,
        compile: true,
        run_success: true,
        stdout: Some("9"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;struct Item{value:i64}fn main(){let cell=RefCell::new(Item{value:7});{let mut edit=cell.borrow_mut();*edit=Item{value:9};}println!("{}",cell.borrow().value);}"#,
    },
    Case {
        name: "sequential-whole-payload-replacements",
        classification: Classification::WholePayloadReplace,
        compile: true,
        run_success: true,
        stdout: Some("11"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);{let mut edit=cell.borrow_mut();*edit=9;}{let mut edit=cell.borrow_mut();*edit=11;}println!("{}",*cell.borrow());}"#,
    },
    Case {
        name: "replacement-is-visible-to-later-shared-guard",
        classification: Classification::WholePayloadReplace,
        compile: true,
        run_success: true,
        stdout: Some("12"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);{let mut edit=cell.borrow_mut();*edit=12;}let view=cell.borrow();println!("{}",*view);}"#,
    },
    Case {
        name: "replacement-drops-old-move-only-payload-once",
        classification: Classification::WholePayloadReplace,
        compile: true,
        run_success: true,
        stdout: Some("1 2"),
        stderr_contains: None,
        source: r#"use std::cell::{Cell,RefCell};use std::rc::Rc;struct Tracked(Rc<Cell<u32>>);impl Drop for Tracked{fn drop(&mut self){self.0.set(self.0.get()+1)}}fn main(){let drops=Rc::new(Cell::new(0));{let cell=RefCell::new(Tracked(Rc::clone(&drops)));{let mut edit=cell.borrow_mut();*edit=Tracked(Rc::clone(&drops));}print!("{} ",drops.get());}println!("{}",drops.get());}"#,
    },
    Case {
        name: "replacement-does-not-call-clone",
        classification: Classification::RejectHiddenCost,
        compile: true,
        run_success: true,
        stdout: Some("0"),
        stderr_contains: None,
        source: r#"use std::cell::{Cell,RefCell};use std::rc::Rc;struct Item{value:i64,clones:Rc<Cell<u32>>}impl Clone for Item{fn clone(&self)->Self{self.clones.set(self.clones.get()+1);Self{value:self.value,clones:Rc::clone(&self.clones)}}}fn main(){let clones=Rc::new(Cell::new(0));let cell=RefCell::new(Item{value:7,clones:Rc::clone(&clones)});{let mut edit=cell.borrow_mut();*edit=Item{value:9,clones:Rc::clone(&clones)};}println!("{}",clones.get());}"#,
    },
    Case {
        name: "replacement-of-rc-payload-does-not-duplicate-owner",
        classification: Classification::RejectHiddenCost,
        compile: true,
        run_success: true,
        stdout: Some("1"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;use std::rc::Rc;fn main(){let cell=RefCell::new(Rc::new(7_i64));{let mut edit=cell.borrow_mut();*edit=Rc::new(9_i64);}println!("{}",Rc::strong_count(&cell.borrow()));}"#,
    },
    Case {
        name: "shared-guard-cannot-replace-payload",
        classification: Classification::WholePayloadReplace,
        compile: false,
        run_success: false,
        stdout: None,
        stderr_contains: Some("error[E0594]"),
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let view=cell.borrow();*view=9;}"#,
    },
    Case {
        name: "overlapping-shared-guard-still-blocks-exclusive-replacement",
        classification: Classification::WholePayloadReplace,
        compile: true,
        run_success: false,
        stdout: None,
        stderr_contains: Some("already borrowed"),
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let view=cell.borrow();let mut edit=cell.borrow_mut();*edit=9;println!("{}",*view);}"#,
    },
    Case {
        name: "fallible-exclusive-conflict-still-enters-failure",
        classification: Classification::WholePayloadReplace,
        compile: true,
        run_success: true,
        stdout: Some("failure"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let view=cell.borrow();match cell.try_borrow_mut(){Ok(mut edit)=>{*edit=9;println!("success")},Err(_)=>println!("failure")}drop(view);}"#,
    },
    Case {
        name: "direct-field-mutation-uses-a-mutable-place",
        classification: Classification::MutablePlaceBoundary,
        compile: true,
        run_success: true,
        stdout: Some("8"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;struct Item{value:i64}fn main(){let cell=RefCell::new(Item{value:7});{let mut edit=cell.borrow_mut();edit.value+=1;}println!("{}",cell.borrow().value);}"#,
    },
    Case {
        name: "nested-field-mutation-expands-place-grammar",
        classification: Classification::MutablePlaceBoundary,
        compile: true,
        run_success: true,
        stdout: Some("8"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;struct Inner{value:i64}struct Outer{inner:Inner}fn main(){let cell=RefCell::new(Outer{inner:Inner{value:7}});{let mut edit=cell.borrow_mut();edit.inner.value+=1;}println!("{}",cell.borrow().inner.value);}"#,
    },
    Case {
        name: "index-mutation-expands-place-grammar",
        classification: Classification::MutablePlaceBoundary,
        compile: true,
        run_success: true,
        stdout: Some("8"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(vec![7_i64]);{let mut edit=cell.borrow_mut();edit[0]+=1;}println!("{}",cell.borrow()[0]);}"#,
    },
    Case {
        name: "field-mutation-preserves-unrelated-move-only-field",
        classification: Classification::MutablePlaceBoundary,
        compile: true,
        run_success: true,
        stdout: Some("8 kept"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;struct Item{value:i64,label:String}fn main(){let cell=RefCell::new(Item{value:7,label:String::from("kept")});{let mut edit=cell.borrow_mut();edit.value+=1;}let view=cell.borrow();println!("{} {}",view.value,view.label);}"#,
    },
    Case {
        name: "explicit-update-helper-wraps-replacement",
        classification: Classification::ExplicitUpdateComparison,
        compile: true,
        run_success: true,
        stdout: Some("9"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn update<T>(slot:&mut T,value:T){*slot=value}fn main(){let cell=RefCell::new(7_i64);{let mut edit=cell.borrow_mut();update(&mut *edit,9);}println!("{}",*cell.borrow());}"#,
    },
    Case {
        name: "guard-move-allows-mutation-through-new-binding",
        classification: Classification::GuardValueBoundary,
        compile: true,
        run_success: true,
        stdout: Some("9"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let edit=cell.borrow_mut();let mut moved=edit;*moved=9;drop(moved);println!("{}",*cell.borrow());}"#,
    },
    Case {
        name: "guard-move-invalidates-old-binding",
        classification: Classification::GuardValueBoundary,
        compile: false,
        run_success: false,
        stdout: None,
        stderr_contains: Some("error[E0382]"),
        source: r#"use std::cell::RefCell;fn main(){let cell=RefCell::new(7_i64);let edit=cell.borrow_mut();let mut moved=edit;*moved=9;*edit=11;}"#,
    },
    Case {
        name: "returned-exclusive-guard-still-requires-lifetime-type",
        classification: Classification::GuardValueBoundary,
        compile: true,
        run_success: true,
        stdout: Some("9"),
        stderr_contains: None,
        source: r#"use std::cell::{RefCell,RefMut};fn edit(cell:&RefCell<i64>)->RefMut<'_,i64>{cell.borrow_mut()}fn main(){let cell=RefCell::new(7_i64);{let mut guard=edit(&cell);*guard=9;}println!("{}",*cell.borrow());}"#,
    },
    Case {
        name: "rc-refcell-composition-keeps-owner-duplication-explicit",
        classification: Classification::ExplicitRcRefCellComposition,
        compile: true,
        run_success: true,
        stdout: Some("9 2"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;use std::rc::Rc;fn main(){let owner=Rc::new(RefCell::new(7_i64));let alias=Rc::clone(&owner);{let mut edit=alias.borrow_mut();*edit=9;}println!("{} {}",*owner.borrow(),Rc::strong_count(&owner));}"#,
    },
    Case {
        name: "dropping-one-owner-keeps-mutation-model",
        classification: Classification::ExplicitRcRefCellComposition,
        compile: true,
        run_success: true,
        stdout: Some("9 1"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;use std::rc::Rc;fn main(){let owner=Rc::new(RefCell::new(7_i64));let alias=Rc::clone(&owner);drop(owner);{let mut edit=alias.borrow_mut();*edit=9;}println!("{} {}",*alias.borrow(),Rc::strong_count(&alias));}"#,
    },
    Case {
        name: "arc-mutex-remains-separate-concurrency-model",
        classification: Classification::ConcurrencyBoundary,
        compile: true,
        run_success: true,
        stdout: Some("9"),
        stderr_contains: None,
        source: r#"use std::sync::{Arc,Mutex};use std::thread;fn main(){let owner=Arc::new(Mutex::new(7_i64));let alias=Arc::clone(&owner);thread::spawn(move||*alias.lock().unwrap()=9).join().unwrap();println!("{}",*owner.lock().unwrap());}"#,
    },
    Case {
        name: "exclusive-owner-get-mut-needs-no-dynamic-guard",
        classification: Classification::RejectHiddenCost,
        compile: true,
        run_success: true,
        stdout: Some("9"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;fn main(){let mut cell=RefCell::new(7_i64);*cell.get_mut()=9;println!("{}",cell.into_inner());}"#,
    },
];

#[derive(Debug)]
struct SurfaceEvidence {
    contextual_words_are_identifiers: bool,
    ordinary_identifier_compatibility: bool,
    whole_replace_candidate_current_parser_rejects: bool,
    direct_field_assignment_current_parser_rejects: bool,
    explicit_update_candidate_current_parser_rejects: bool,
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

    if !compile_matched {
        eprintln!(
            "compile mismatch for {}: expected_success={}, actual_success={}, stderr={}",
            case.name,
            case.compile,
            compiled,
            compile_stderr.trim()
        );
    }

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
    let contextual =
        lex("replace update set edit value\n").expect("candidate contextual words should lex");
    let identifiers: Vec<_> = contextual
        .iter()
        .filter_map(|token| match &token.kind {
            TokenKind::Identifier(name) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let contextual_words_are_identifiers =
        identifiers.as_slice() == ["replace", "update", "set", "edit", "value"];

    let ordinary = concat!(
        "fn replace(value int) int\n",
        "return value\n",
        "end\n",
        "fn update(set int) int\n",
        "return set\n",
        "end\n",
        "value = replace(update(7))\n",
        "print value\n",
    );
    let ordinary_identifier_compatibility =
        lex(ordinary).is_ok_and(|tokens| parse(&tokens).is_ok());

    let whole_replace = concat!("replace edit, Item(value = 2)\n", "print 0\n",);
    let whole_replace_candidate_current_parser_rejects =
        lex(whole_replace).is_ok_and(|tokens| parse(&tokens).is_err());

    let direct_field_assignment = concat!("edit.value = 2\n", "print 0\n",);
    let direct_field_assignment_current_parser_rejects =
        lex(direct_field_assignment).is_ok_and(|tokens| parse(&tokens).is_err());

    let explicit_update = concat!("update edit, Item(value = 2)\n", "print 0\n",);
    let explicit_update_candidate_current_parser_rejects =
        lex(explicit_update).is_ok_and(|tokens| parse(&tokens).is_err());

    SurfaceEvidence {
        contextual_words_are_identifiers,
        ordinary_identifier_compatibility,
        whole_replace_candidate_current_parser_rejects,
        direct_field_assignment_current_parser_rejects,
        explicit_update_candidate_current_parser_rejects,
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

    let whole_replace_count = count(findings, Classification::WholePayloadReplace);
    let mutable_place_count = count(findings, Classification::MutablePlaceBoundary);
    let explicit_update_count = count(findings, Classification::ExplicitUpdateComparison);
    let guard_value_count = count(findings, Classification::GuardValueBoundary);
    let composition_count = count(findings, Classification::ExplicitRcRefCellComposition);
    let concurrency_count = count(findings, Classification::ConcurrencyBoundary);
    let reject_hidden_cost_count = count(findings, Classification::RejectHiddenCost);

    let whole_replace_sufficient = mismatches == 0
        && whole_replace_count >= 8
        && mutable_place_count >= 4
        && explicit_update_count >= 1
        && guard_value_count >= 3
        && composition_count >= 2
        && concurrency_count >= 1
        && reject_hidden_cost_count >= 3
        && surface.contextual_words_are_identifiers
        && surface.ordinary_identifier_compatibility
        && surface.whole_replace_candidate_current_parser_rejects
        && surface.direct_field_assignment_current_parser_rejects
        && surface.explicit_update_candidate_current_parser_rejects;

    let verdict = if whole_replace_sufficient {
        "WHOLE-PAYLOAD-REPLACE-CANDIDATE"
    } else {
        "DEFER"
    };
    let recommended_surface = if whole_replace_sufficient {
        "replace guard, expr"
    } else {
        "NONE"
    };

    assert_eq!(verdict, "WHOLE-PAYLOAD-REPLACE-CANDIDATE");
    assert_eq!(recommended_surface, "replace guard, expr");

    let mut json = String::new();
    writeln!(json, "{{").unwrap();
    writeln!(json, "  \"git_sha\": {sha:?},").unwrap();
    writeln!(json, "  \"rustc_vv\": {rustc:?},").unwrap();
    writeln!(json, "  \"verdict\": {verdict:?},").unwrap();
    writeln!(json, "  \"recommended_surface\": {recommended_surface:?},").unwrap();
    writeln!(json, "  \"case_count\": {},", CASES.len()).unwrap();
    writeln!(json, "  \"expectation_mismatches\": {mismatches},").unwrap();
    writeln!(json, "  \"whole_replace_case_count\": {whole_replace_count},").unwrap();
    writeln!(json, "  \"mutable_place_boundary_count\": {mutable_place_count},").unwrap();
    writeln!(json, "  \"explicit_update_comparison_count\": {explicit_update_count},").unwrap();
    writeln!(json, "  \"guard_value_boundary_count\": {guard_value_count},").unwrap();
    writeln!(json, "  \"explicit_rc_refcell_case_count\": {composition_count},").unwrap();
    writeln!(json, "  \"concurrency_boundary_count\": {concurrency_count},").unwrap();
    writeln!(json, "  \"reject_hidden_cost_count\": {reject_hidden_cost_count},").unwrap();
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
        "  \"whole_replace_candidate_current_parser_rejects\": {},",
        surface.whole_replace_candidate_current_parser_rejects
    )
    .unwrap();
    writeln!(
        json,
        "  \"direct_field_assignment_current_parser_rejects\": {},",
        surface.direct_field_assignment_current_parser_rejects
    )
    .unwrap();
    writeln!(
        json,
        "  \"explicit_update_candidate_current_parser_rejects\": {},",
        surface.explicit_update_candidate_current_parser_rejects
    )
    .unwrap();
    writeln!(json, "  \"field_mutation_authorized\": false,").unwrap();
    writeln!(json, "  \"index_mutation_authorized\": false,").unwrap();
    writeln!(json, "  \"escaping_guard_authorized\": false").unwrap();
    writeln!(json, "}}").unwrap();
    fs::write(out.join("report.json"), json).expect("json report should be writable");

    let mut csv = String::from(
        "git_sha,case,classification,compile_matched,run_matched,stdout,stderr_summary\n",
    );
    for (case, finding) in CASES.iter().zip(findings) {
        writeln!(
            csv,
            "{},{},{},{},{},{:?},{:?}",
            sha,
            case.name,
            case.classification.as_str(),
            finding.compile_matched,
            finding.run_matched,
            finding.stdout,
            finding.stderr_summary
        )
        .unwrap();
    }
    fs::write(out.join("report.csv"), csv).expect("csv report should be writable");

    let mut markdown = String::new();
    writeln!(markdown, "# Exclusive guard mutation surface research").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "- git sha: {sha}").unwrap();
    writeln!(markdown, "- verdict: **{verdict}**").unwrap();
    writeln!(markdown, "- recommended surface: **{recommended_surface}**").unwrap();
    writeln!(markdown, "- cases: **{}**", CASES.len()).unwrap();
    writeln!(markdown, "- expectation mismatches: **{mismatches}**").unwrap();
    writeln!(markdown, "- field mutation authorized: **no**").unwrap();
    writeln!(markdown, "- index mutation authorized: **no**").unwrap();
    writeln!(markdown, "- escaping guard authorized: **no**").unwrap();
    writeln!(markdown, "\n## Candidate source contract\n").unwrap();
    writeln!(markdown, "~~~text").unwrap();
    writeln!(markdown, "borrow_mut state as edit").unwrap();
    writeln!(markdown, "    replace edit, Item(value = 2)").unwrap();
    writeln!(markdown, "end").unwrap();
    writeln!(markdown, "~~~\n").unwrap();
    writeln!(
        markdown,
        "The replacement consumes the new payload expression, drops the old payload exactly once, and remains valid only through the lexical exclusive guard. Direct field/index mutation is deliberately not selected because it expands the language's mutable-place grammar. A separate explicit update statement adds syntax without changing the underlying whole-payload operation."
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
fn exclusive_guard_mutation_surface_research_selects_whole_payload_replacement() {
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
    assert!(surface.whole_replace_candidate_current_parser_rejects);
    assert!(surface.direct_field_assignment_current_parser_rejects);
    assert!(surface.explicit_update_candidate_current_parser_rejects);

    let scratch = env::temp_dir().join(format!(
        "evo-exclusive-guard-mutation-surface-research-{}",
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
        "exclusive guard mutation research mismatches: {mismatches:?}"
    );

    assert!(count(&findings, Classification::WholePayloadReplace) >= 8);
    assert!(count(&findings, Classification::MutablePlaceBoundary) >= 4);
    assert!(count(&findings, Classification::ExplicitUpdateComparison) >= 1);
    assert!(count(&findings, Classification::GuardValueBoundary) >= 3);
    assert!(count(&findings, Classification::ExplicitRcRefCellComposition) >= 2);
    assert!(count(&findings, Classification::ConcurrencyBoundary) >= 1);
    assert!(count(&findings, Classification::RejectHiddenCost) >= 3);

    let out = env::var_os("EVO_EXCLUSIVE_GUARD_MUTATION_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/evo-exclusive-guard-mutation-surface-research")
        });
    write_reports(&findings, &surface, &out, &rustc);

    fs::remove_dir_all(scratch).expect("scratch should be removable");
}
