use evo_formatter::format_source;
use evo_lexer::{TokenKind, lex};
use std::env;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    WeakSurfaceCandidate,
    BorrowInstead,
    InteriorMutabilityComposition,
    ConcurrencyBoundary,
    RejectHiddenCost,
}

struct Case {
    name: &'static str,
    class: Class,
    compile: bool,
    stdout: Option<&'static str>,
    source: &'static str,
}

const CASES: &[Case] = &[
    Case {
        name: "downgrade-does-not-add-strong-owner",
        class: Class::WeakSurfaceCandidate,
        compile: true,
        stdout: Some("1 1"),
        source: r#"use std::rc::Rc; fn main(){ let x=Rc::new(7); let w=Rc::downgrade(&x); println!("{} {}",Rc::strong_count(&x),Rc::weak_count(&x)); drop(w); }"#,
    },
    Case {
        name: "weak-count-lifecycle",
        class: Class::WeakSurfaceCandidate,
        compile: true,
        stdout: Some("2 1 0"),
        source: r#"use std::rc::Rc; fn main(){ let x=Rc::new(7); let a=Rc::downgrade(&x); let b=a.clone(); println!("{} {} {}",Rc::weak_count(&x),{drop(a);Rc::weak_count(&x)},{drop(b);Rc::weak_count(&x)}); }"#,
    },
    Case {
        name: "upgrade-live-succeeds",
        class: Class::WeakSurfaceCandidate,
        compile: true,
        stdout: Some("true 7"),
        source: r#"use std::rc::Rc; fn main(){ let x=Rc::new(7); let w=Rc::downgrade(&x); let y=w.upgrade(); println!("{} {}",y.is_some(),*y.unwrap()); }"#,
    },
    Case {
        name: "upgrade-after-final-strong-drop-fails",
        class: Class::WeakSurfaceCandidate,
        compile: true,
        stdout: Some("false"),
        source: r#"use std::rc::Rc; fn main(){ let x=Rc::new(7); let w=Rc::downgrade(&x); drop(x); println!("{}",w.upgrade().is_some()); }"#,
    },
    Case {
        name: "weak-back-edge-breaks-cycle",
        class: Class::WeakSurfaceCandidate,
        compile: true,
        stdout: Some("1 1 true"),
        source: r#"use std::cell::RefCell; use std::rc::{Rc,Weak}; struct N{parent:RefCell<Weak<N>>} fn main(){ let p=Rc::new(N{parent:RefCell::new(Weak::new())}); let c=Rc::new(N{parent:RefCell::new(Weak::new())}); *c.parent.borrow_mut()=Rc::downgrade(&p); println!("{} {} {}",Rc::strong_count(&p),Rc::weak_count(&p),c.parent.borrow().upgrade().is_some()); }"#,
    },
    Case {
        name: "all-strong-cycle-retains",
        class: Class::RejectHiddenCost,
        compile: true,
        stdout: Some("2 2"),
        source: r#"use std::cell::RefCell; use std::rc::Rc; struct N{next:RefCell<Option<Rc<N>>>} fn main(){ let a=Rc::new(N{next:RefCell::new(None)}); let b=Rc::new(N{next:RefCell::new(None)}); *a.next.borrow_mut()=Some(Rc::clone(&b)); *b.next.borrow_mut()=Some(Rc::clone(&a)); println!("{} {}",Rc::strong_count(&a),Rc::strong_count(&b)); }"#,
    },
    Case {
        name: "drop-weak-does-not-drop-live-payload",
        class: Class::WeakSurfaceCandidate,
        compile: true,
        stdout: Some("0 1"),
        source: r#"use std::cell::Cell; use std::rc::Rc; struct D<'a>(&'a Cell<u32>); impl Drop for D<'_>{fn drop(&mut self){self.0.set(self.0.get()+1)}} fn main(){let n=Cell::new(0); let x=Rc::new(D(&n)); let w=Rc::downgrade(&x); drop(w); print!("{} ",n.get()); drop(x); println!("{}",n.get());}"#,
    },
    Case {
        name: "weak-handle-moves",
        class: Class::WeakSurfaceCandidate,
        compile: false,
        stdout: None,
        source: r#"use std::rc::Rc; fn take<T>(x:T){drop(x)} fn main(){let x=Rc::new(7); let w=Rc::downgrade(&x); take(w); let _=w.upgrade();}"#,
    },
    Case {
        name: "weak-duplication-is-explicit",
        class: Class::WeakSurfaceCandidate,
        compile: true,
        stdout: Some("2 true"),
        source: r#"use std::rc::Rc; fn main(){let x=Rc::new(7); let w=Rc::downgrade(&x); let w2=w.clone(); println!("{} {}",Rc::weak_count(&x),w2.upgrade().is_some());}"#,
    },
    Case {
        name: "weak-vs-reference-lifetime",
        class: Class::WeakSurfaceCandidate,
        compile: true,
        stdout: Some("false"),
        source: r#"use std::rc::Rc; fn main(){let w={let x=Rc::new(7); Rc::downgrade(&x)}; println!("{}",w.upgrade().is_some());}"#,
    },
    Case {
        name: "payload-clone-distinct-from-edge-ops",
        class: Class::RejectHiddenCost,
        compile: true,
        stdout: Some("true false"),
        source: r#"use std::rc::Rc; #[derive(Clone)] struct N(i32); fn main(){let x=Rc::new(N(7)); let y=Rc::clone(&x); let z=Rc::new((*x).clone()); let _=z.0; println!("{} {}",Rc::ptr_eq(&x,&y),Rc::ptr_eq(&x,&z));}"#,
    },
    Case {
        name: "weak-plus-refcell-is-composition",
        class: Class::InteriorMutabilityComposition,
        compile: true,
        stdout: Some("8 true"),
        source: r#"use std::cell::RefCell; use std::rc::Rc; fn main(){let x=Rc::new(RefCell::new(7)); let w=Rc::downgrade(&x); *x.borrow_mut()+=1; println!("{} {}",x.borrow(),w.upgrade().is_some());}"#,
    },
    Case {
        name: "arc-weak-is-concurrency-sibling",
        class: Class::ConcurrencyBoundary,
        compile: true,
        stdout: Some("true false"),
        source: r#"use std::sync::Arc; fn main(){let x=Arc::new(7); let w=Arc::downgrade(&x); print!("{} ",w.upgrade().is_some()); drop(x); println!("{}",w.upgrade().is_some());}"#,
    },
    Case {
        name: "lexical-borrow-control",
        class: Class::BorrowInstead,
        compile: true,
        stdout: Some("7"),
        source: r#"fn main(){let x=7; let r=&x; println!("{}",r);}"#,
    },
    Case {
        name: "weak-never-fabricates-dead-target",
        class: Class::WeakSurfaceCandidate,
        compile: true,
        stdout: Some("none"),
        source: r#"use std::rc::Rc; fn main(){let w={let x=Rc::new(String::from("x")); Rc::downgrade(&x)}; println!("{}",if w.upgrade().is_some(){"some"}else{"none"});}"#,
    },
];

fn rustc() -> OsString {
    env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"))
}
fn rustc_vv() -> String {
    let o = Command::new(rustc()).arg("-Vv").output().expect("rustc");
    assert!(o.status.success());
    String::from_utf8_lossy(&o.stdout).trim().to_owned()
}

fn surface_evidence() -> (bool, bool, bool) {
    let contextual = "weak Item\ndowngrade owner\nupgrade edge\n";
    let tokens = lex(contextual).expect("contextual words lex");
    let words_are_identifiers = tokens
        .iter()
        .filter(|t| matches!(t.kind, TokenKind::Identifier(_)))
        .count()
        >= 6;
    let contextual_stable = format_source(contextual, &tokens) == contextual;
    let generic = "Weak<Item>\n";
    let generic_tokens = lex(generic).expect("generic spelling lexes as comparison tokens");
    let generic_rewritten = format_source(generic, &generic_tokens) != generic;
    (words_are_identifiers, contextual_stable, generic_rewritten)
}

fn run_case(case: &Case, root: &Path) -> bool {
    let dir = root.join(case.name);
    fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("case.rs");
    let bin = dir.join(format!("case{}", env::consts::EXE_SUFFIX));
    fs::write(&src, case.source).expect("write");
    let c = Command::new(rustc())
        .arg("--edition=2024")
        .arg("-C")
        .arg("opt-level=3")
        .arg(&src)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("compile");
    if c.status.success() != case.compile {
        return false;
    }
    if !case.compile {
        return true;
    }
    let r = Command::new(bin).output().expect("run");
    let out = String::from_utf8_lossy(&r.stdout).trim().to_owned();
    r.status.success() && case.stdout.is_none_or(|x| x == out)
}

#[test]
#[ignore = "research evidence; dedicated workflow runs this exact test"]
fn weak_edge_surface_research_classifies_explicit_weak_semantics() {
    let vv = rustc_vv();
    if env::var_os("EVO_REQUIRE_PINNED_RUSTC").is_some() {
        assert!(
            vv.lines()
                .next()
                .is_some_and(|x| x.contains("rustc 1.98.0"))
        );
    }
    let (ids, stable, generic_rewritten) = surface_evidence();
    assert!(ids && stable && generic_rewritten);
    let scratch = env::temp_dir().join(format!("evo-weak-edge-research-{}", std::process::id()));
    if scratch.exists() {
        fs::remove_dir_all(&scratch).expect("cleanup");
    }
    fs::create_dir_all(&scratch).expect("mkdir");
    let matches: Vec<_> = CASES.iter().map(|c| run_case(c, &scratch)).collect();
    let mismatches: Vec<_> = CASES
        .iter()
        .zip(matches.iter())
        .filter(|(_, matched)| !**matched)
        .map(|(case, _)| case.name)
        .collect();
    assert!(
        mismatches.is_empty(),
        "Weak research mismatches: {mismatches:?}"
    );
    let count = |class| CASES.iter().filter(|c| c.class == class).count();
    let out = env::var_os("EVO_WEAK_EDGE_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/evo-weak-edge-research")
        });
    fs::create_dir_all(&out).expect("out");
    let sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "unknown".into());
    let mut json = String::new();
    writeln!(json, "{{").unwrap();
    writeln!(json, "  \"git_sha\": {sha:?},").unwrap();
    writeln!(json, "  \"verdict\": \"WEAK-SURFACE-CANDIDATE\",").unwrap();
    writeln!(json, "  \"recommended_surface\": \"CONTEXTUAL-WORDS\",").unwrap();
    writeln!(json, "  \"case_count\": {},", CASES.len()).unwrap();
    writeln!(json, "  \"expectation_mismatches\": 0,").unwrap();
    writeln!(
        json,
        "  \"weak_candidate_count\": {},",
        count(Class::WeakSurfaceCandidate)
    )
    .unwrap();
    writeln!(
        json,
        "  \"borrow_instead_count\": {},",
        count(Class::BorrowInstead)
    )
    .unwrap();
    writeln!(
        json,
        "  \"interior_mutability_composition_count\": {},",
        count(Class::InteriorMutabilityComposition)
    )
    .unwrap();
    writeln!(
        json,
        "  \"concurrency_boundary_count\": {},",
        count(Class::ConcurrencyBoundary)
    )
    .unwrap();
    writeln!(
        json,
        "  \"hidden_cost_rejection_count\": {},",
        count(Class::RejectHiddenCost)
    )
    .unwrap();
    writeln!(json, "  \"contextual_words_are_identifiers\": {ids},").unwrap();
    writeln!(json, "  \"contextual_formatter_stable\": {stable},").unwrap();
    writeln!(
        json,
        "  \"generic_angle_formatter_rewrite_observed\": {generic_rewritten},"
    )
    .unwrap();
    writeln!(json, "  \"rustc_vv\": {vv:?}\n}}").unwrap();
    fs::write(out.join("report.json"), json).unwrap();
    fs::write(out.join("report.md"),format!("# Weak edge surface v0 research\n\n- git_sha: `{sha}`\n- verdict: **WEAK-SURFACE-CANDIDATE / CONTEXTUAL-WORDS**\n- semantic cases: **{}**\n- expectation mismatches: **0**\n- contextual words remain identifiers and format stably; `Weak<Item>` currently collides with comparison-style angle formatting.\n",CASES.len())).unwrap();
    fs::remove_dir_all(scratch).expect("cleanup");
}
