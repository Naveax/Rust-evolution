use evo_lexer::{TokenKind, lex};
use evo_lowering::lower;
use evo_parser::parse;
use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    ScopedUpgrade,
    MaybeSharedControl,
    OwnershipBoundary,
    ConcurrencyBoundary,
}

impl Family {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ScopedUpgrade => "SCOPED-UPGRADE",
            Self::MaybeSharedControl => "MAYBE-SHARED-CONTROL",
            Self::OwnershipBoundary => "OWNERSHIP-BOUNDARY",
            Self::ConcurrencyBoundary => "CONCURRENCY-BOUNDARY",
        }
    }
}

struct Case {
    name: &'static str,
    family: Family,
    compile: bool,
    stdout: Option<&'static str>,
    source: &'static str,
}

struct Finding {
    compile_matched: bool,
    rust_ran: bool,
    runtime_matched: bool,
}

const CASES: &[Case] = &[
    Case {
        name: "live-upgrade-single-strong-owner",
        family: Family::ScopedUpgrade,
        compile: true,
        stdout: Some("success 7"),
        source: r#"use std::rc::Rc; fn main(){let x=Rc::new(7_i64);let w=Rc::downgrade(&x);match w.upgrade(){Some(owner)=>println!("success {}",*owner),None=>println!("failure")}}"#,
    },
    Case {
        name: "live-upgrade-multiple-strong-owners",
        family: Family::ScopedUpgrade,
        compile: true,
        stdout: Some("success 3"),
        source: r#"use std::rc::Rc; fn main(){let x=Rc::new(7_i64);let keep=Rc::clone(&x);let w=Rc::downgrade(&x);match w.upgrade(){Some(owner)=>println!("success {}",Rc::strong_count(&owner)),None=>println!("failure")}drop(keep);}"#,
    },
    Case {
        name: "dead-upgrade-enters-failure-branch",
        family: Family::ScopedUpgrade,
        compile: true,
        stdout: Some("failure"),
        source: r#"use std::rc::Rc; fn main(){let w={let x=Rc::new(7_i64);Rc::downgrade(&x)};match w.upgrade(){Some(_)=>println!("success"),None=>println!("failure")}}"#,
    },
    Case {
        name: "upgraded-owner-survives-weak-and-original-owner",
        family: Family::ScopedUpgrade,
        compile: true,
        stdout: Some("7"),
        source: r#"use std::rc::Rc; fn main(){let x=Rc::new(7_i64);let w=Rc::downgrade(&x);let owner=w.upgrade().unwrap();drop(w);drop(x);println!("{}",*owner);}"#,
    },
    Case {
        name: "dropping-upgraded-owner-restores-strong-count",
        family: Family::ScopedUpgrade,
        compile: true,
        stdout: Some("2 1"),
        source: r#"use std::rc::Rc; fn main(){let x=Rc::new(7_i64);let w=Rc::downgrade(&x);let owner=w.upgrade().unwrap();print!("{} ",Rc::strong_count(&x));drop(owner);println!("{}",Rc::strong_count(&x));}"#,
    },
    Case {
        name: "upgraded-owner-follows-existing-move-rules",
        family: Family::ScopedUpgrade,
        compile: false,
        stdout: None,
        source: r#"use std::rc::Rc; fn take<T>(_:T){} fn main(){let x=Rc::new(7_i64);let w=Rc::downgrade(&x);if let Some(owner)=w.upgrade(){take(owner);println!("{}",*owner);}}"#,
    },
    Case {
        name: "caller-retention-requires-explicit-strong-duplication",
        family: Family::ScopedUpgrade,
        compile: true,
        stdout: Some("2 7"),
        source: r#"use std::rc::Rc; fn take<T>(_:T){} fn main(){let x=Rc::new(7_i64);let w=Rc::downgrade(&x);let owner=w.upgrade().unwrap();let retained=Rc::clone(&owner);take(owner);println!("{} {}",Rc::strong_count(&retained),*retained);}"#,
    },
    Case {
        name: "failed-upgrade-never-fabricates-payload-access",
        family: Family::ScopedUpgrade,
        compile: true,
        stdout: Some("none"),
        source: r#"use std::rc::Rc; fn main(){let w={let x=Rc::new(String::from("x"));Rc::downgrade(&x)};if let Some(owner)=w.upgrade(){println!("{}",owner)}else{println!("none")}}"#,
    },
    Case {
        name: "weak-handle-remains-usable-after-non-consuming-check",
        family: Family::ScopedUpgrade,
        compile: true,
        stdout: Some("true true false"),
        source: r#"use std::rc::Rc; fn main(){let x=Rc::new(7_i64);let w=Rc::downgrade(&x);let a=w.upgrade().is_some();let b=w.upgrade().is_some();drop(x);let c=w.upgrade().is_some();println!("{a} {b} {c}");}"#,
    },
    Case {
        name: "checked-upgrade-inside-function",
        family: Family::ScopedUpgrade,
        compile: true,
        stdout: Some("7 -1"),
        source: r#"use std::rc::{Rc,Weak}; fn read(w:&Weak<i64>)->i64{match w.upgrade(){Some(owner)=>*owner,None=>-1}} fn main(){let x=Rc::new(7_i64);let w=Rc::downgrade(&x);print!("{} ",read(&w));drop(x);println!("{}",read(&w));}"#,
    },
    Case {
        name: "first-class-option-control-crosses-statements",
        family: Family::MaybeSharedControl,
        compile: true,
        stdout: Some("true 7"),
        source: r#"use std::rc::Rc; fn main(){let x=Rc::new(7_i64);let w=Rc::downgrade(&x);let result=w.upgrade();let present=result.is_some();let value=result.as_ref().map(|owner|**owner).unwrap_or(-1);println!("{present} {value}");}"#,
    },
    Case {
        name: "nested-control-flow-with-scoped-upgrade",
        family: Family::ScopedUpgrade,
        compile: true,
        stdout: Some("7"),
        source: r#"use std::rc::Rc; fn main(){let x=Rc::new(7_i64);let w=Rc::downgrade(&x);if true{match w.upgrade(){Some(owner)=>{if *owner>0{println!("{}",*owner)}else{println!("zero")}},None=>println!("none")}}else{println!("skip")}}"#,
    },
    Case {
        name: "payload-deep-clone-remains-distinct",
        family: Family::OwnershipBoundary,
        compile: true,
        stdout: Some("true false"),
        source: r#"use std::rc::Rc; #[derive(Clone)] struct N(i64); fn main(){let x=Rc::new(N(7));let same=Rc::clone(&x);let copied=Rc::new((*x).clone());let _=copied.0;println!("{} {}",Rc::ptr_eq(&x,&same),Rc::ptr_eq(&x,&copied));}"#,
    },
    Case {
        name: "lexical-reference-remains-distinct",
        family: Family::OwnershipBoundary,
        compile: true,
        stdout: Some("7"),
        source: r#"fn main(){let x=7_i64;let r=&x;println!("{}",r);}"#,
    },
    Case {
        name: "arc-weak-remains-cross-thread-sibling",
        family: Family::ConcurrencyBoundary,
        compile: true,
        stdout: Some("true false"),
        source: r#"use std::sync::Arc; fn main(){let x=Arc::new(7_i64);let w=Arc::downgrade(&x);print!("{} ",w.upgrade().is_some());drop(x);println!("{}",w.upgrade().is_some());}"#,
    },
];

fn rustc() -> OsString {
    env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"))
}

fn rustc_vv() -> String {
    let output = Command::new(rustc()).arg("-Vv").output().expect("rustc -Vv");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn run_case(case: &Case, root: &Path) -> Finding {
    let dir = root.join(case.name);
    fs::create_dir_all(&dir).expect("create case directory");
    let source = dir.join("case.rs");
    let binary = dir.join(format!("case{}", env::consts::EXE_SUFFIX));
    fs::write(&source, case.source).expect("write case source");
    let compile = Command::new(rustc())
        .arg("--edition=2024")
        .arg("-C")
        .arg("opt-level=3")
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("compile case");
    let compiled = compile.status.success();
    let compile_matched = compiled == case.compile;
    if !compiled {
        return Finding {
            compile_matched,
            rust_ran: false,
            runtime_matched: !case.compile,
        };
    }

    let run = Command::new(&binary).output().expect("run case");
    let stdout = String::from_utf8_lossy(&run.stdout).trim().to_owned();
    Finding {
        compile_matched,
        rust_ran: true,
        runtime_matched: run.status.success() && case.stdout.is_none_or(|expected| expected == stdout),
    }
}

#[derive(Debug)]
struct SurfaceEvidence {
    contextual_words_are_identifiers: bool,
    ordinary_upgrade_identifier_parse: bool,
    scoped_upgrade_current_parser_rejects: bool,
    maybe_shared_type_current_parser_rejects: bool,
    generic_option_current_parser_rejects: bool,
    enum_shared_owner_currently_fail_closed: bool,
}

fn surface_evidence() -> SurfaceEvidence {
    let contextual = lex("upgrade edge as owner\n").expect("contextual source should lex");
    let identifiers: Vec<_> = contextual
        .iter()
        .filter_map(|token| match &token.kind {
            TokenKind::Identifier(name) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let contextual_words_are_identifiers =
        identifiers.as_slice() == ["upgrade", "edge", "as", "owner"];

    let ordinary = concat!(
        "fn upgrade(as int) int\n",
        "return as\n",
        "end\n",
        "value = upgrade(1)\n",
        "print value\n",
    );
    let ordinary_upgrade_identifier_parse =
        lex(ordinary).is_ok_and(|tokens| parse(&tokens).is_ok());

    let scoped = concat!(
        "upgrade edge as owner\n",
        "print owner.value\n",
        "else\n",
        "print 0\n",
        "end\n",
    );
    let scoped_upgrade_current_parser_rejects =
        lex(scoped).is_ok_and(|tokens| parse(&tokens).is_err());

    let maybe_shared = concat!(
        "record Item\n",
        "value int\n",
        "end\n",
        "fn f(value maybe shared Item) int\n",
        "return 0\n",
        "end\n",
    );
    let maybe_shared_type_current_parser_rejects =
        lex(maybe_shared).is_ok_and(|tokens| parse(&tokens).is_err());

    let generic_option = concat!(
        "record Item\n",
        "value int\n",
        "end\n",
        "fn f(value Option<shared Item>) int\n",
        "return 0\n",
        "end\n",
    );
    let generic_option_current_parser_rejects =
        lex(generic_option).is_ok_and(|tokens| parse(&tokens).is_err());

    let enum_shared = concat!(
        "enum MaybeOwner\n",
        "None\n",
        "Some shared Item\n",
        "end\n",
        "value = MaybeOwner.None()\n",
        "match value\n",
        "case MaybeOwner.None\n",
        "print 0\n",
        "case MaybeOwner.Some(owner)\n",
        "print 1\n",
        "end\n",
    );
    let enum_shared_owner_currently_fail_closed = lex(enum_shared).is_ok_and(|tokens| {
        parse(&tokens)
            .map(|program| lower(&program).is_err())
            .unwrap_or(true)
    });

    SurfaceEvidence {
        contextual_words_are_identifiers,
        ordinary_upgrade_identifier_parse,
        scoped_upgrade_current_parser_rejects,
        maybe_shared_type_current_parser_rejects,
        generic_option_current_parser_rejects,
        enum_shared_owner_currently_fail_closed,
    }
}

#[test]
#[ignore = "research evidence; dedicated workflow runs this exact test"]
fn weak_upgrade_result_surface_research_selects_scoped_upgrade_candidate() {
    let rustc_vv = rustc_vv();
    if env::var_os("EVO_REQUIRE_PINNED_RUSTC").is_some() {
        assert!(
            rustc_vv
                .lines()
                .next()
                .is_some_and(|line| line.contains("rustc 1.98.0"))
        );
    }

    let surface = surface_evidence();
    assert!(surface.contextual_words_are_identifiers);
    assert!(surface.ordinary_upgrade_identifier_parse);
    assert!(surface.scoped_upgrade_current_parser_rejects);
    assert!(surface.maybe_shared_type_current_parser_rejects);
    assert!(surface.generic_option_current_parser_rejects);
    assert!(surface.enum_shared_owner_currently_fail_closed);

    let scratch = env::temp_dir().join(format!(
        "evo-weak-upgrade-result-research-{}",
        std::process::id()
    ));
    if scratch.exists() {
        fs::remove_dir_all(&scratch).expect("remove stale scratch");
    }
    fs::create_dir_all(&scratch).expect("create scratch");

    let findings: Vec<_> = CASES
        .iter()
        .map(|case| run_case(case, &scratch))
        .collect();
    let mismatches: Vec<_> = CASES
        .iter()
        .zip(&findings)
        .filter(|(_, finding)| !(finding.compile_matched && finding.runtime_matched))
        .map(|(case, _)| case.name)
        .collect();
    assert!(
        mismatches.is_empty(),
        "checked Weak upgrade research mismatches: {mismatches:?}"
    );

    let scoped_case_count = CASES
        .iter()
        .filter(|case| case.family == Family::ScopedUpgrade)
        .count();
    assert!(scoped_case_count >= 10);

    let weak_upgrade_non_consuming = CASES
        .iter()
        .zip(&findings)
        .find(|(case, _)| case.name == "weak-handle-remains-usable-after-non-consuming-check")
        .is_some_and(|(_, finding)| finding.compile_matched && finding.runtime_matched);
    assert!(weak_upgrade_non_consuming);

    let first_class_result_required = false;
    let out = env::var_os("EVO_WEAK_UPGRADE_RESULT_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/evo-weak-upgrade-result-research")
        });
    fs::create_dir_all(&out).expect("create report directory");
    let sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "unknown".to_owned());

    let mut json = String::new();
    writeln!(json, "{{").unwrap();
    writeln!(json, "  \"git_sha\": {sha:?},").unwrap();
    writeln!(json, "  \"verdict\": \"SCOPED-UPGRADE-CANDIDATE\",").unwrap();
    writeln!(
        json,
        "  \"recommended_surface\": \"SCOPED-UPGRADE-BRANCH\","
    )
    .unwrap();
    writeln!(json, "  \"case_count\": {},", CASES.len()).unwrap();
    writeln!(json, "  \"scoped_case_count\": {scoped_case_count},").unwrap();
    writeln!(json, "  \"expectation_mismatches\": 0,").unwrap();
    writeln!(
        json,
        "  \"contextual_words_are_identifiers\": {},",
        surface.contextual_words_are_identifiers
    )
    .unwrap();
    writeln!(
        json,
        "  \"ordinary_upgrade_identifier_parse\": {},",
        surface.ordinary_upgrade_identifier_parse
    )
    .unwrap();
    writeln!(
        json,
        "  \"scoped_upgrade_current_parser_rejects\": {},",
        surface.scoped_upgrade_current_parser_rejects
    )
    .unwrap();
    writeln!(
        json,
        "  \"maybe_shared_type_current_parser_rejects\": {},",
        surface.maybe_shared_type_current_parser_rejects
    )
    .unwrap();
    writeln!(
        json,
        "  \"generic_option_current_parser_rejects\": {},",
        surface.generic_option_current_parser_rejects
    )
    .unwrap();
    writeln!(
        json,
        "  \"enum_shared_owner_currently_fail_closed\": {},",
        surface.enum_shared_owner_currently_fail_closed
    )
    .unwrap();
    writeln!(
        json,
        "  \"weak_upgrade_non_consuming\": {weak_upgrade_non_consuming},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"first_class_result_required\": {first_class_result_required},"
    )
    .unwrap();
    writeln!(json, "  \"rustc_vv\": {rustc_vv:?},").unwrap();
    writeln!(json, "  \"cases\": [").unwrap();
    for (index, (case, finding)) in CASES.iter().zip(&findings).enumerate() {
        let comma = if index + 1 == CASES.len() { "" } else { "," };
        writeln!(
            json,
            "    {{\"name\": {:?}, \"family\": {:?}, \"compile_expected\": {}, \"compile_matched\": {}, \"rust_ran\": {}, \"runtime_matched\": {}}}{comma}",
            case.name,
            case.family.as_str(),
            case.compile,
            finding.compile_matched,
            finding.rust_ran,
            finding.runtime_matched
        )
        .unwrap();
    }
    writeln!(json, "  ]").unwrap();
    writeln!(json, "}}").unwrap();
    fs::write(out.join("report.json"), json).expect("write json report");

    let mut csv = String::from(
        "git_sha,name,family,compile_expected,compile_matched,rust_ran,runtime_matched\n",
    );
    for (case, finding) in CASES.iter().zip(&findings) {
        writeln!(
            csv,
            "{},{},{},{},{},{},{}",
            sha,
            case.name,
            case.family.as_str(),
            case.compile,
            finding.compile_matched,
            finding.rust_ran,
            finding.runtime_matched
        )
        .unwrap();
    }
    fs::write(out.join("report.csv"), csv).expect("write csv report");

    let mut md = String::new();
    writeln!(md, "# Checked Weak upgrade result surface v0 research\n").unwrap();
    writeln!(md, "- git_sha: `{sha}`").unwrap();
    writeln!(md, "- verdict: **SCOPED-UPGRADE-CANDIDATE**").unwrap();
    writeln!(md, "- recommended surface: **SCOPED-UPGRADE-BRANCH**").unwrap();
    writeln!(md, "- semantic cases: **{}**", CASES.len()).unwrap();
    writeln!(md, "- expectation mismatches: **0**").unwrap();
    writeln!(
        md,
        "- first-class optional shared-owner result required for v0: **no**"
    )
    .unwrap();
    writeln!(
        md,
        "- current `maybe shared T`, generic `Option<shared T>`, and shared-owner enum result paths remain fail-closed"
    )
    .unwrap();
    writeln!(md, "\n## Candidate contract\n").unwrap();
    writeln!(md, "```text").unwrap();
    writeln!(md, "upgrade edge as owner").unwrap();
    writeln!(md, "    ...").unwrap();
    writeln!(md, "else").unwrap();
    writeln!(md, "    ...").unwrap();
    writeln!(md, "end").unwrap();
    writeln!(md, "```\n").unwrap();
    writeln!(
        md,
        "The success binding is a branch-local `shared T` owner created by checked `Weak::upgrade`; failure is explicit and no first-class optional owner escapes the construct."
    )
    .unwrap();
    writeln!(md, "\n## Matrix\n").unwrap();
    writeln!(
        md,
        "| case | family | compile expectation matched | runtime matched |"
    )
    .unwrap();
    writeln!(md, "|---|---|---:|---:|").unwrap();
    for (case, finding) in CASES.iter().zip(&findings) {
        writeln!(
            md,
            "| {} | {} | {} | {} |",
            case.name,
            case.family.as_str(),
            finding.compile_matched,
            finding.runtime_matched
        )
        .unwrap();
    }
    fs::write(out.join("report.md"), md).expect("write markdown report");

    fs::remove_dir_all(scratch).expect("remove scratch");
}
