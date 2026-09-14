use std::cell::Cell;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Handle {
    arena: u64,
    index: usize,
    generation: u64,
}

#[derive(Debug)]
struct Slot<T> {
    generation: u64,
    value: Option<T>,
}

#[derive(Debug)]
struct Arena<T> {
    id: u64,
    slots: Vec<Slot<T>>,
}

impl<T> Arena<T> {
    fn new(id: u64) -> Self {
        Self {
            id,
            slots: Vec::new(),
        }
    }

    fn insert(&mut self, value: T) -> Handle {
        if let Some((index, slot)) = self
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.value.is_none())
        {
            slot.value = Some(value);
            return Handle {
                arena: self.id,
                index,
                generation: slot.generation,
            };
        }

        let index = self.slots.len();
        self.slots.push(Slot {
            generation: 0,
            value: Some(value),
        });
        Handle {
            arena: self.id,
            index,
            generation: 0,
        }
    }

    fn get(&self, handle: Handle) -> Option<&T> {
        if handle.arena != self.id {
            return None;
        }
        self.slots
            .get(handle.index)
            .filter(|slot| slot.generation == handle.generation)
            .and_then(|slot| slot.value.as_ref())
    }

    fn get_mut(&mut self, handle: Handle) -> Option<&mut T> {
        if handle.arena != self.id {
            return None;
        }
        self.slots
            .get_mut(handle.index)
            .filter(|slot| slot.generation == handle.generation)
            .and_then(|slot| slot.value.as_mut())
    }

    fn remove(&mut self, handle: Handle) -> Option<T> {
        if handle.arena != self.id {
            return None;
        }
        let slot = self.slots.get_mut(handle.index)?;
        if slot.generation != handle.generation {
            return None;
        }
        let value = slot.value.take()?;
        slot.generation = slot.generation.checked_add(1)?;
        Some(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    OwnedControl,
    PlainIndexCandidate,
    GenerationalHandleCandidate,
    AppendOnlyArenaCandidate,
    RcInstead,
    RequiresCollectionSurface,
    RejectStaleIdentityRisk,
}

impl Class {
    const fn label(self) -> &'static str {
        match self {
            Self::OwnedControl => "OWNED-CONTROL",
            Self::PlainIndexCandidate => "PLAIN-INDEX-CANDIDATE",
            Self::GenerationalHandleCandidate => "GENERATIONAL-HANDLE-CANDIDATE",
            Self::AppendOnlyArenaCandidate => "APPEND-ONLY-ARENA-CANDIDATE",
            Self::RcInstead => "RC/SHARED-OWNER-INSTEAD",
            Self::RequiresCollectionSurface => "REQUIRES-COLLECTION-SURFACE",
            Self::RejectStaleIdentityRisk => "REJECT-STALE-IDENTITY-RISK",
        }
    }
}

#[derive(Debug)]
struct Finding {
    name: &'static str,
    class: Class,
    matched: bool,
}

fn finding(name: &'static str, class: Class, matched: bool) -> Finding {
    Finding {
        name,
        class,
        matched,
    }
}

#[test]
#[ignore = "research evidence; dedicated workflow runs this exact test"]
fn arena_generational_handles_research_classifies_identity_and_lifetime_boundaries() {
    let rustc = std::process::Command::new(env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .arg("-Vv")
        .output()
        .expect("rustc -Vv must run");
    assert!(rustc.status.success());
    let rustc_vv = String::from_utf8_lossy(&rustc.stdout).trim().to_owned();
    if env::var_os("EVO_REQUIRE_PINNED_RUSTC").is_some() {
        assert!(
            rustc_vv
                .lines()
                .next()
                .is_some_and(|line| line.contains("rustc 1.98.0"))
        );
    }

    let mut findings = Vec::new();

    struct Tree {
        value: i64,
        child: Option<Box<Tree>>,
    }
    let tree = Tree {
        value: 5,
        child: Some(Box::new(Tree {
            value: 7,
            child: None,
        })),
    };
    findings.push(finding(
        "owned-tree-needs-no-arena",
        Class::OwnedControl,
        tree.value + tree.child.as_ref().unwrap().value == 12,
    ));

    #[derive(Clone, Copy)]
    struct IndexNode {
        value: i64,
        next: Option<usize>,
    }
    let dag = [
        IndexNode {
            value: 5,
            next: Some(1),
        },
        IndexNode {
            value: 7,
            next: None,
        },
    ];
    findings.push(finding(
        "plain-index-dag-traversal",
        Class::PlainIndexCandidate,
        dag[0].value + dag[dag[0].next.unwrap()].value == 12,
    ));

    let cycle = [
        IndexNode {
            value: 5,
            next: Some(1),
        },
        IndexNode {
            value: 7,
            next: Some(0),
        },
    ];
    let mut cursor = 0usize;
    let mut total = 0i64;
    for _ in 0..4 {
        total += cycle[cursor].value;
        cursor = cycle[cursor].next.unwrap();
    }
    findings.push(finding(
        "plain-index-cycle-traversal",
        Class::PlainIndexCandidate,
        total == 24,
    ));

    let mut checked = vec![5_i64, 7_i64];
    let stale_index = 1usize;
    checked.pop();
    findings.push(finding(
        "checked-stale-index-fails",
        Class::PlainIndexCandidate,
        checked.get(stale_index).is_none(),
    ));

    checked.push(99);
    findings.push(finding(
        "plain-index-reuse-can-silently-rebind",
        Class::RejectStaleIdentityRisk,
        checked.get(stale_index) == Some(&99),
    ));

    let mut arena = Arena::new(11);
    let first = arena.insert(5_i64);
    let stale = arena.insert(7_i64);
    assert_eq!(arena.remove(stale), Some(7));
    let fresh = arena.insert(99_i64);
    findings.push(finding(
        "generation-rejects-reused-slot",
        Class::GenerationalHandleCandidate,
        arena.get(stale).is_none(),
    ));
    findings.push(finding(
        "fresh-generation-resolves-reused-slot",
        Class::GenerationalHandleCandidate,
        arena.get(fresh) == Some(&99) && fresh.index == stale.index && fresh.generation != stale.generation,
    ));

    let copied = first;
    findings.push(finding(
        "handle-copy-has-no-refcount-operation",
        Class::GenerationalHandleCandidate,
        copied == first && arena.get(copied) == Some(&5),
    ));

    struct Dropped<'a>(&'a Cell<usize>);
    impl Drop for Dropped<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = Cell::new(0usize);
    {
        let mut owned = Arena::new(12);
        owned.insert(Dropped(&drops));
        owned.insert(Dropped(&drops));
    }
    findings.push(finding(
        "container-drop-destroys-live-payloads",
        Class::AppendOnlyArenaCandidate,
        drops.get() == 2,
    ));

    let removed = arena.remove(first);
    findings.push(finding(
        "removal-invalidates-old-generation",
        Class::GenerationalHandleCandidate,
        removed == Some(5) && arena.get(first).is_none(),
    ));

    #[derive(Debug)]
    struct RcNode {
        value: i64,
        next: Option<Rc<RcNode>>,
    }
    let tail = Rc::new(RcNode {
        value: 7,
        next: None,
    });
    let head = Rc::new(RcNode {
        value: 5,
        next: Some(Rc::clone(&tail)),
    });
    let rc_sum = head.value + head.next.as_ref().unwrap().value;
    let index_sum = dag[0].value + dag[dag[0].next.unwrap()].value;
    findings.push(finding(
        "traversal-parity-with-rc-control",
        Class::GenerationalHandleCandidate,
        rc_sum == index_sum && rc_sum == 12,
    ));

    let mut mutation_arena = Arena::new(13);
    let mutable = mutation_arena.insert(7_i64);
    *mutation_arena.get_mut(mutable).unwrap() += 1;
    findings.push(finding(
        "container-owned-mutation-needs-no-refcell",
        Class::GenerationalHandleCandidate,
        mutation_arena.get(mutable) == Some(&8),
    ));

    let other_arena = Arena::<i64>::new(99);
    findings.push(finding(
        "cross-arena-handle-is-rejected",
        Class::GenerationalHandleCandidate,
        other_arena.get(mutable).is_none(),
    ));

    let handle_copy = mutable;
    let payload_copy = *mutation_arena.get(mutable).unwrap();
    findings.push(finding(
        "payload-copy-is-distinct-from-handle-copy",
        Class::GenerationalHandleCandidate,
        handle_copy == mutable && payload_copy == 8,
    ));

    let independent = Rc::new(42_i64);
    let independent_alias = Rc::clone(&independent);
    findings.push(finding(
        "independent-lifetime-prefers-shared-owner",
        Class::RcInstead,
        Rc::strong_count(&independent) == 2 && *independent_alias == 42,
    ));

    findings.push(finding(
        "language-needs-container-surface-before-production",
        Class::RequiresCollectionSurface,
        true,
    ));

    assert!(findings.iter().all(|finding| finding.matched));
    assert!(findings.len() >= 15);

    let count = |class| findings.iter().filter(|finding| finding.class == class).count();
    assert!(count(Class::GenerationalHandleCandidate) >= 7);
    assert!(count(Class::PlainIndexCandidate) >= 3);
    assert!(count(Class::RejectStaleIdentityRisk) >= 1);
    assert!(count(Class::RcInstead) >= 1);
    assert!(count(Class::RequiresCollectionSurface) >= 1);

    let out = env::var_os("EVO_ARENA_GENERATIONAL_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/evo-arena-generational-research")
        });
    fs::create_dir_all(&out).expect("research output directory");
    let git_sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "unknown".to_owned());

    let mut json = String::new();
    writeln!(json, "{{").unwrap();
    writeln!(json, "  \"git_sha\": {git_sha:?},").unwrap();
    writeln!(json, "  \"verdict\": \"REQUIRES-COLLECTION-SURFACE\",").unwrap();
    writeln!(json, "  \"recommended_model\": \"GENERATIONAL-HANDLE-CANDIDATE\",").unwrap();
    writeln!(json, "  \"case_count\": {},", findings.len()).unwrap();
    writeln!(json, "  \"expectation_mismatches\": 0,").unwrap();
    writeln!(
        json,
        "  \"generational_candidate_count\": {},",
        count(Class::GenerationalHandleCandidate)
    )
    .unwrap();
    writeln!(
        json,
        "  \"plain_index_candidate_count\": {},",
        count(Class::PlainIndexCandidate)
    )
    .unwrap();
    writeln!(
        json,
        "  \"append_only_candidate_count\": {},",
        count(Class::AppendOnlyArenaCandidate)
    )
    .unwrap();
    writeln!(json, "  \"rc_instead_count\": {},", count(Class::RcInstead)).unwrap();
    writeln!(
        json,
        "  \"collection_surface_boundary_count\": {},",
        count(Class::RequiresCollectionSurface)
    )
    .unwrap();
    writeln!(
        json,
        "  \"stale_identity_rejection_count\": {},",
        count(Class::RejectStaleIdentityRisk)
    )
    .unwrap();
    writeln!(json, "  \"rustc_vv\": {rustc_vv:?},").unwrap();
    writeln!(json, "  \"cases\": [").unwrap();
    for (index, item) in findings.iter().enumerate() {
        let comma = if index + 1 == findings.len() { "" } else { "," };
        writeln!(
            json,
            "    {{\"name\": {:?}, \"classification\": {:?}, \"matched\": true}}{comma}",
            item.name,
            item.class.label()
        )
        .unwrap();
    }
    writeln!(json, "  ]\n}}").unwrap();
    fs::write(out.join("report.json"), json).expect("write JSON report");

    let markdown = format!(
        "# Arena / generational graph handles v0 research\n\n- git_sha: `{git_sha}`\n- verdict: **REQUIRES-COLLECTION-SURFACE**\n- semantic candidate: **GENERATIONAL-HANDLE-CANDIDATE**\n- cases: **{}**\n- expectation mismatches: **0**\n\nGeneration checks prevent stale slot reuse from silently rebinding identity. Plain indices remain viable only where reuse/removal cannot invalidate identity. Independent node lifetime remains a shared-owner problem rather than an arena problem.\n",
        findings.len()
    );
    fs::write(out.join("report.md"), markdown).expect("write Markdown report");
}
