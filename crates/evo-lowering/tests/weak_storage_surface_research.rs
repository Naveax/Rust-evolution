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
    RecordFieldCandidate,
    SequenceStorageBoundary,
    ExplicitWeakDuplicationBoundary,
    ArenaIdentityBoundary,
    EnumIntegrationBoundary,
    RecursiveLayoutControl,
    RejectHiddenCost,
}

impl Classification {
    const fn as_str(self) -> &'static str {
        match self {
            Self::RecordFieldCandidate => "WEAK-RECORD-FIELD-CANDIDATE",
            Self::SequenceStorageBoundary => "SEQUENCE-STORAGE-BOUNDARY",
            Self::ExplicitWeakDuplicationBoundary => "EXPLICIT-WEAK-DUPLICATION-BOUNDARY",
            Self::ArenaIdentityBoundary => "ARENA-IDENTITY-BOUNDARY",
            Self::EnumIntegrationBoundary => "ENUM-INTEGRATION-BOUNDARY",
            Self::RecursiveLayoutControl => "RECURSIVE-LAYOUT-CONTROL",
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
        name: "record-weak-field-upgrades-while-owner-live",
        classification: Classification::RecordFieldCandidate,
        compile: true,
        run_success: true,
        stdout: Some("7"),
        stderr_contains: None,
        source: r#"use std::rc::{Rc,Weak};struct Node{edge:Weak<Node>,value:i64}fn main(){let target=Rc::new(Node{edge:Weak::new(),value:7});let holder=Node{edge:Rc::downgrade(&target),value:1};println!("{}",holder.edge.upgrade().unwrap().value);}"#,
    },
    Case {
        name: "record-weak-field-fails-after-final-owner-dies",
        classification: Classification::RecordFieldCandidate,
        compile: true,
        run_success: true,
        stdout: Some("dead"),
        stderr_contains: None,
        source: r#"use std::rc::{Rc,Weak};struct Node{edge:Weak<Node>}fn main(){let edge={let target=Rc::new(Node{edge:Weak::new()});Rc::downgrade(&target)};let holder=Node{edge};println!("{}",if holder.edge.upgrade().is_some(){"live"}else{"dead"});}"#,
    },
    Case {
        name: "weak-back-edge-breaks-strong-cycle",
        classification: Classification::RecordFieldCandidate,
        compile: true,
        run_success: true,
        stdout: Some("1 0"),
        stderr_contains: None,
        source: r#"use std::cell::RefCell;use std::rc::{Rc,Weak};struct Node{parent:RefCell<Weak<Node>>}fn main(){let root=Rc::new(Node{parent:RefCell::new(Weak::new())});let child=Rc::new(Node{parent:RefCell::new(Rc::downgrade(&root))});println!("{} {}",Rc::strong_count(&root),Rc::strong_count(&child)-1);}"#,
    },
    Case {
        name: "dropping-weak-field-does-not-drop-live-payload",
        classification: Classification::RecordFieldCandidate,
        compile: true,
        run_success: true,
        stdout: Some("1"),
        stderr_contains: None,
        source: r#"use std::rc::{Rc,Weak};struct Holder<T>{edge:Weak<T>}fn main(){let owner=Rc::new(7_i64);let holder=Holder{edge:Rc::downgrade(&owner)};drop(holder);println!("{}",Rc::strong_count(&owner));}"#,
    },
    Case {
        name: "moving-record-containing-weak-field-moves-handle",
        classification: Classification::RecordFieldCandidate,
        compile: true,
        run_success: true,
        stdout: Some("7"),
        stderr_contains: None,
        source: r#"use std::rc::{Rc,Weak};struct Holder<T>{edge:Weak<T>}fn main(){let owner=Rc::new(7_i64);let holder=Holder{edge:Rc::downgrade(&owner)};let moved=holder;println!("{}",*moved.edge.upgrade().unwrap());}"#,
    },
    Case {
        name: "record-weak-field-does-not-increment-strong-count",
        classification: Classification::RejectHiddenCost,
        compile: true,
        run_success: true,
        stdout: Some("1 1"),
        stderr_contains: None,
        source: r#"use std::rc::{Rc,Weak};struct Holder<T>{edge:Weak<T>}fn main(){let owner=Rc::new(7_i64);let holder=Holder{edge:Rc::downgrade(&owner)};println!("{} {}",Rc::strong_count(&owner),holder.edge.strong_count());}"#,
    },
    Case {
        name: "moving-weak-field-does-not-call-clone",
        classification: Classification::RejectHiddenCost,
        compile: true,
        run_success: true,
        stdout: Some("1"),
        stderr_contains: None,
        source: r#"use std::rc::{Rc,Weak};struct Holder<T>{edge:Weak<T>}fn main(){let owner=Rc::new(7_i64);let holder=Holder{edge:Rc::downgrade(&owner)};let moved=holder.edge;println!("{}",moved.strong_count());}"#,
    },
    Case {
        name: "weak-clone-is-explicit-separate-bookkeeping",
        classification: Classification::ExplicitWeakDuplicationBoundary,
        compile: true,
        run_success: true,
        stdout: Some("2 1"),
        stderr_contains: None,
        source: r#"use std::rc::Rc;fn main(){let owner=Rc::new(7_i64);let edge=Rc::downgrade(&owner);let alias=edge.clone();println!("{} {}",Rc::weak_count(&owner),alias.strong_count());}"#,
    },
    Case {
        name: "vector-stores-moved-weak-handle-without-strong-duplication",
        classification: Classification::SequenceStorageBoundary,
        compile: true,
        run_success: true,
        stdout: Some("1 7"),
        stderr_contains: None,
        source: r#"use std::rc::Rc;fn main(){let owner=Rc::new(7_i64);let edge=Rc::downgrade(&owner);let mut edges=Vec::new();edges.push(edge);println!("{} {}",Rc::strong_count(&owner),*edges[0].upgrade().unwrap());}"#,
    },
    Case {
        name: "vector-get-yields-borrowed-weak-handle",
        classification: Classification::SequenceStorageBoundary,
        compile: true,
        run_success: true,
        stdout: Some("7"),
        stderr_contains: None,
        source: r#"use std::rc::Rc;fn main(){let owner=Rc::new(7_i64);let edges=vec![Rc::downgrade(&owner)];let edge=edges.get(0).unwrap();println!("{}",*edge.upgrade().unwrap());}"#,
    },
    Case {
        name: "moving-out-of-indexed-vector-requires-removal-operation",
        classification: Classification::SequenceStorageBoundary,
        compile: false,
        run_success: false,
        stdout: None,
        stderr_contains: Some("error[E0507]"),
        source: r#"use std::rc::{Rc,Weak};fn main(){let owner=Rc::new(7_i64);let edges:Vec<Weak<i64>>=vec![Rc::downgrade(&owner)];let _moved=edges[0];}"#,
    },
    Case {
        name: "vector-weak-reference-can-upgrade-without-moving-element",
        classification: Classification::SequenceStorageBoundary,
        compile: true,
        run_success: true,
        stdout: Some("7"),
        stderr_contains: None,
        source: r#"use std::rc::Rc;fn main(){let owner=Rc::new(7_i64);let edges=vec![Rc::downgrade(&owner)];let borrowed=&edges[0];println!("{}",*borrowed.upgrade().unwrap());}"#,
    },
    Case {
        name: "arena-handle-identity-remains-distinct-from-weak-owner",
        classification: Classification::ArenaIdentityBoundary,
        compile: true,
        run_success: true,
        stdout: Some("1 0"),
        stderr_contains: None,
        source: r#"use std::rc::Rc;#[derive(Clone,Copy)]struct Handle{arena:u64,index:usize,generation:u64}fn main(){let owner=Rc::new(7_i64);let edge=Rc::downgrade(&owner);let handle=Handle{arena:1,index:0,generation:0};println!("{} {}",edge.strong_count(),handle.index);}"#,
    },
    Case {
        name: "weak-storage-does-not-convert-to-arena-handle",
        classification: Classification::ArenaIdentityBoundary,
        compile: false,
        run_success: false,
        stdout: None,
        stderr_contains: Some("error[E0308]"),
        source: r#"use std::rc::{Rc,Weak};#[derive(Clone,Copy)]struct Handle{arena:u64,index:usize,generation:u64}fn takes(_:Handle){}fn main(){let owner=Rc::new(7_i64);let edge:Weak<i64>=Rc::downgrade(&owner);takes(edge);}"#,
    },
    Case {
        name: "enum-can-technically-carry-weak-but-is-separate-surface",
        classification: Classification::EnumIntegrationBoundary,
        compile: true,
        run_success: true,
        stdout: Some("7"),
        stderr_contains: None,
        source: r#"use std::rc::{Rc,Weak};enum Edge<T>{None,Weak(Weak<T>)}fn main(){let owner=Rc::new(7_i64);let edge=Edge::Weak(Rc::downgrade(&owner));if let Edge::Weak(edge)=edge{println!("{}",*edge.upgrade().unwrap())}}"#,
    },
    Case {
        name: "recursive-by-value-record-remains-invalid",
        classification: Classification::RecursiveLayoutControl,
        compile: false,
        run_success: false,
        stdout: None,
        stderr_contains: Some("error[E0072]"),
        source: r#"struct Node{next:Node}fn main(){}"#,
    },
    Case {
        name: "recursive-record-through-weak-handle-is-fixed-size",
        classification: Classification::RecursiveLayoutControl,
        compile: true,
        run_success: true,
        stdout: Some("dead"),
        stderr_contains: None,
        source: r#"use std::rc::Weak;struct Node{next:Weak<Node>}fn main(){let node=Node{next:Weak::new()};println!("{}",if node.next.upgrade().is_some(){"live"}else{"dead"});}"#,
    },
    Case {
        name: "weak-field-does-not-deep-clone-payload",
        classification: Classification::RejectHiddenCost,
        compile: true,
        run_success: true,
        stdout: Some("0"),
        stderr_contains: None,
        source: r#"use std::cell::Cell;use std::rc::{Rc,Weak};struct Item{clones:Rc<Cell<u32>>}impl Clone for Item{fn clone(&self)->Self{self.clones.set(self.clones.get()+1);Self{clones:Rc::clone(&self.clones)}}}struct Holder{edge:Weak<Item>}fn main(){let clones=Rc::new(Cell::new(0));let owner=Rc::new(Item{clones:Rc::clone(&clones)});let _holder=Holder{edge:Rc::downgrade(&owner)};println!("{}",clones.get());}"#,
    },
    Case {
        name: "arc-weak-remains-separate-cross-thread-type",
        classification: Classification::RejectHiddenCost,
        compile: true,
        run_success: true,
        stdout: Some("7"),
        stderr_contains: None,
        source: r#"use std::sync::Arc;fn main(){let owner=Arc::new(7_i64);let edge=Arc::downgrade(&owner);println!("{}",*edge.upgrade().unwrap());}"#,
    },
    Case {
        name: "rc-weak-and-arc-weak-do-not-convert",
        classification: Classification::RejectHiddenCost,
        compile: false,
        run_success: false,
        stdout: None,
        stderr_contains: Some("error[E0308]"),
        source: r#"use std::rc::{Rc,Weak as RcWeak};use std::sync::Weak as ArcWeak;fn takes(_:ArcWeak<i64>){}fn main(){let owner=Rc::new(7_i64);let edge:RcWeak<i64>=Rc::downgrade(&owner);takes(edge);}"#,
    },
];

#[derive(Debug)]
struct SurfaceEvidence {
    contextual_weak_identifier: bool,
    ordinary_weak_named_type_compatibility: bool,
    record_field_current_parser_rejects: bool,
    sequence_element_current_parser_rejects: bool,
    enum_payload_current_parser_rejects: bool,
    arena_payload_current_parser_rejects: bool,
    production_upgrade_requires_local_weak: bool,
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
        lex("weak downgrade upgrade edge owner\n").expect("Weak contextual words should lex");
    let identifiers: Vec<_> = contextual
        .iter()
        .filter_map(|token| match &token.kind {
            TokenKind::Identifier(name) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let contextual_weak_identifier =
        identifiers.as_slice() == ["weak", "downgrade", "upgrade", "edge", "owner"];

    let ordinary = concat!(
        "record weak\n",
        "value int\n",
        "end\n",
        "fn keep(x weak) weak\n",
        "return x\n",
        "end\n",
        "weak = weak(value = 1)\n",
        "print weak.value\n",
    );
    let ordinary_weak_named_type_compatibility =
        lex(ordinary).is_ok_and(|tokens| parse(&tokens).is_ok());

    let record_field = concat!("record Node\n", "value int\n", "next weak Node\n", "end\n",);
    let record_field_current_parser_rejects = lex(record_field).is_ok_and(|tokens| {
        parse(&tokens).is_err_and(|error| error.message.contains("weak-owner record fields"))
    });

    let sequence_element = concat!(
        "record Node\n",
        "value int\n",
        "end\n",
        "fn take(edges seq weak Node) int\n",
        "return 0\n",
        "end\n",
    );
    let sequence_element_current_parser_rejects = lex(sequence_element).is_ok_and(|tokens| {
        parse(&tokens).is_err_and(|error| error.message.contains("weak-owner sequence elements"))
    });

    let enum_payload = concat!(
        "record Node\n",
        "value int\n",
        "end\n",
        "enum Edge\n",
        "Next weak Node\n",
        "end\n",
    );
    let enum_payload_current_parser_rejects = lex(enum_payload).is_ok_and(|tokens| {
        parse(&tokens).is_err_and(|error| error.message.contains("weak-owner enum payloads"))
    });

    let arena_payload = concat!(
        "record Node\n",
        "value int\n",
        "end\n",
        "fn take(items arena weak Node) int\n",
        "return 0\n",
        "end\n",
    );
    let arena_payload_current_parser_rejects = lex(arena_payload).is_ok_and(|tokens| {
        parse(&tokens).is_err_and(|error| error.message.contains("weak-owner arena payloads"))
    });

    let upgrade_field = concat!(
        "record Node\n",
        "value int\n",
        "end\n",
        "record Holder\n",
        "value int\n",
        "end\n",
        "owner = share Node(value = 1)\n",
        "edge = downgrade owner\n",
        "upgrade edge as live\n",
        "print live.value\n",
        "else\n",
        "print 0\n",
        "end\n",
    );
    let production_upgrade_requires_local_weak =
        lex(upgrade_field).is_ok_and(|tokens| parse(&tokens).is_ok());

    SurfaceEvidence {
        contextual_weak_identifier,
        ordinary_weak_named_type_compatibility,
        record_field_current_parser_rejects,
        sequence_element_current_parser_rejects,
        enum_payload_current_parser_rejects,
        arena_payload_current_parser_rejects,
        production_upgrade_requires_local_weak,
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

    let record_count = count(findings, Classification::RecordFieldCandidate);
    let sequence_count = count(findings, Classification::SequenceStorageBoundary);
    let weak_dup_count = count(findings, Classification::ExplicitWeakDuplicationBoundary);
    let arena_count = count(findings, Classification::ArenaIdentityBoundary);
    let enum_count = count(findings, Classification::EnumIntegrationBoundary);
    let recursive_count = count(findings, Classification::RecursiveLayoutControl);
    let reject_hidden_cost_count = count(findings, Classification::RejectHiddenCost);

    let record_field_sufficient = mismatches == 0
        && record_count >= 5
        && sequence_count >= 4
        && weak_dup_count >= 1
        && arena_count >= 2
        && enum_count >= 1
        && recursive_count >= 2
        && reject_hidden_cost_count >= 4
        && surface.contextual_weak_identifier
        && surface.ordinary_weak_named_type_compatibility
        && surface.record_field_current_parser_rejects
        && surface.sequence_element_current_parser_rejects
        && surface.enum_payload_current_parser_rejects
        && surface.arena_payload_current_parser_rejects
        && surface.production_upgrade_requires_local_weak;

    let sequence_first_slice_blocked = sequence_count >= 4
        && surface.sequence_element_current_parser_rejects
        && weak_dup_count >= 1;

    let verdict = if record_field_sufficient && sequence_first_slice_blocked {
        "WEAK-RECORD-FIELD-CANDIDATE"
    } else {
        "DEFER"
    };

    assert_eq!(verdict, "WEAK-RECORD-FIELD-CANDIDATE");

    let mut json = String::new();
    writeln!(json, "{{").unwrap();
    writeln!(json, "  \"git_sha\": {sha:?},").unwrap();
    writeln!(json, "  \"rustc_vv\": {rustc:?},").unwrap();
    writeln!(json, "  \"verdict\": {verdict:?},").unwrap();
    writeln!(json, "  \"case_count\": {},", CASES.len()).unwrap();
    writeln!(json, "  \"expectation_mismatches\": {mismatches},").unwrap();
    writeln!(json, "  \"record_field_case_count\": {record_count},").unwrap();
    writeln!(
        json,
        "  \"sequence_boundary_case_count\": {sequence_count},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"weak_duplication_boundary_count\": {weak_dup_count},"
    )
    .unwrap();
    writeln!(json, "  \"arena_identity_boundary_count\": {arena_count},").unwrap();
    writeln!(json, "  \"enum_integration_boundary_count\": {enum_count},").unwrap();
    writeln!(
        json,
        "  \"recursive_layout_control_count\": {recursive_count},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"reject_hidden_cost_count\": {reject_hidden_cost_count},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"contextual_weak_identifier\": {},",
        surface.contextual_weak_identifier
    )
    .unwrap();
    writeln!(
        json,
        "  \"ordinary_weak_named_type_compatibility\": {},",
        surface.ordinary_weak_named_type_compatibility
    )
    .unwrap();
    writeln!(
        json,
        "  \"record_field_current_parser_rejects\": {},",
        surface.record_field_current_parser_rejects
    )
    .unwrap();
    writeln!(
        json,
        "  \"sequence_element_current_parser_rejects\": {},",
        surface.sequence_element_current_parser_rejects
    )
    .unwrap();
    writeln!(
        json,
        "  \"enum_payload_current_parser_rejects\": {},",
        surface.enum_payload_current_parser_rejects
    )
    .unwrap();
    writeln!(
        json,
        "  \"arena_payload_current_parser_rejects\": {},",
        surface.arena_payload_current_parser_rejects
    )
    .unwrap();
    writeln!(
        json,
        "  \"sequence_first_slice_blocked\": {sequence_first_slice_blocked},"
    )
    .unwrap();
    writeln!(json, "  \"enum_integration_authorized\": false,").unwrap();
    writeln!(json, "  \"arena_weak_identity_unified\": false,").unwrap();
    writeln!(json, "  \"implicit_weak_duplication_authorized\": false").unwrap();
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
    writeln!(markdown, "# Weak storage surface research").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "- git sha: {sha}").unwrap();
    writeln!(markdown, "- verdict: **{verdict}**").unwrap();
    writeln!(markdown, "- cases: **{}**", CASES.len()).unwrap();
    writeln!(markdown, "- expectation mismatches: **{mismatches}**").unwrap();
    writeln!(
        markdown,
        "- sequence first slice blocked: **{sequence_first_slice_blocked}**"
    )
    .unwrap();
    writeln!(markdown, "- enum integration authorized: **no**").unwrap();
    writeln!(markdown, "- arena/Weak identity unified: **no**").unwrap();
    writeln!(markdown, "- implicit Weak duplication authorized: **no**").unwrap();
    writeln!(markdown, "\n## Candidate source contract\n").unwrap();
    writeln!(markdown, "~~~text").unwrap();
    writeln!(markdown, "record Node").unwrap();
    writeln!(markdown, "    next weak Node").unwrap();
    writeln!(markdown, "end").unwrap();
    writeln!(markdown, "~~~\n").unwrap();
    writeln!(
        markdown,
        "The candidate stores the fixed-size one-thread Weak handle directly in a nominal record field. It does not make recursive by-value records legal, does not unify Weak with arena handles, and does not authorize enum Weak payloads."
    )
    .unwrap();
    writeln!(
        markdown,
        "Sequence storage remains a separate boundary because existing checked sequence lookup binds move-only elements by immutable reference, while production v0 upgrade accepts a Weak local. Supporting `seq weak T` honestly therefore needs either a borrowed-Weak upgrade rule or a separately accepted explicit Weak duplication operation."
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
fn weak_storage_surface_research_selects_record_field_first() {
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
    assert!(surface.contextual_weak_identifier);
    assert!(surface.ordinary_weak_named_type_compatibility);
    assert!(surface.record_field_current_parser_rejects);
    assert!(surface.sequence_element_current_parser_rejects);
    assert!(surface.enum_payload_current_parser_rejects);
    assert!(surface.arena_payload_current_parser_rejects);
    assert!(surface.production_upgrade_requires_local_weak);

    let scratch = env::temp_dir().join(format!(
        "evo-weak-storage-surface-research-{}",
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
        "Weak storage research mismatches: {mismatches:?}"
    );

    assert!(count(&findings, Classification::RecordFieldCandidate) >= 5);
    assert!(count(&findings, Classification::SequenceStorageBoundary) >= 4);
    assert!(count(&findings, Classification::ExplicitWeakDuplicationBoundary) >= 1);
    assert!(count(&findings, Classification::ArenaIdentityBoundary) >= 2);
    assert!(count(&findings, Classification::EnumIntegrationBoundary) >= 1);
    assert!(count(&findings, Classification::RecursiveLayoutControl) >= 2);
    assert!(count(&findings, Classification::RejectHiddenCost) >= 4);

    let out = env::var_os("EVO_WEAK_STORAGE_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/evo-weak-storage-surface-research")
        });
    write_reports(&findings, &surface, &out, &rustc);

    fs::remove_dir_all(scratch).expect("scratch should be removable");
}
