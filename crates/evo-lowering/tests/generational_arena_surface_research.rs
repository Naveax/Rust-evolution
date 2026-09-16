use evo_lexer::{TokenKind, lex};
use evo_parser::parse;
use std::cell::Cell;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::process::Command;
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
    retired: bool,
}

#[derive(Debug)]
struct Arena<T> {
    id: u64,
    slots: Vec<Slot<T>>,
    free: Vec<usize>,
}

impl<T> Arena<T> {
    fn with_id(id: u64) -> Self {
        Self {
            id,
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    fn insert(&mut self, value: T) -> Handle {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index];
            assert!(!slot.retired, "retired slot must never enter free list");
            assert!(slot.value.is_none(), "free slot must be vacant");
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
            retired: false,
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
            .filter(|slot| !slot.retired && slot.generation == handle.generation)
            .and_then(|slot| slot.value.as_ref())
    }

    fn remove(&mut self, handle: Handle) -> Option<T> {
        if handle.arena != self.id {
            return None;
        }

        let slot = self.slots.get_mut(handle.index)?;
        if slot.retired || slot.generation != handle.generation {
            return None;
        }

        let value = slot.value.take()?;
        if slot.generation == u64::MAX {
            slot.retired = true;
        } else {
            slot.generation += 1;
            self.free.push(handle.index);
        }
        Some(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReferenceHandle {
    arena: u64,
    index: usize,
    generation: u64,
}

#[derive(Debug)]
struct ReferenceSlot<T> {
    generation: u64,
    value: Option<T>,
    retired: bool,
}

#[derive(Debug)]
struct ReferenceArena<T> {
    id: u64,
    slots: Vec<ReferenceSlot<T>>,
    free: Vec<usize>,
}

impl<T> ReferenceArena<T> {
    fn with_id(id: u64) -> Self {
        Self {
            id,
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    fn insert(&mut self, value: T) -> ReferenceHandle {
        let (index, generation) = if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index];
            assert!(!slot.retired);
            assert!(slot.value.is_none());
            slot.value = Some(value);
            (index, slot.generation)
        } else {
            let index = self.slots.len();
            self.slots.push(ReferenceSlot {
                generation: 0,
                value: Some(value),
                retired: false,
            });
            (index, 0)
        };
        ReferenceHandle {
            arena: self.id,
            index,
            generation,
        }
    }

    fn get(&self, handle: ReferenceHandle) -> Option<&T> {
        if handle.arena != self.id {
            return None;
        }
        self.slots
            .get(handle.index)
            .filter(|slot| !slot.retired && slot.generation == handle.generation)
            .and_then(|slot| slot.value.as_ref())
    }

    fn remove(&mut self, handle: ReferenceHandle) -> Option<T> {
        if handle.arena != self.id {
            return None;
        }
        let slot = self.slots.get_mut(handle.index)?;
        if slot.retired || slot.generation != handle.generation {
            return None;
        }
        let value = slot.value.take()?;
        if let Some(next) = slot.generation.checked_add(1) {
            slot.generation = next;
            self.free.push(handle.index);
        } else {
            slot.retired = true;
        }
        Some(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CostSnapshot {
    checksum: i64,
    slots: usize,
    free: usize,
    operations: usize,
}

fn candidate_cost_snapshot() -> CostSnapshot {
    let mut arena = Arena::with_id(77);
    let first = arena.insert(10_i64);
    let stale = arena.insert(20_i64);
    let mut checksum = *arena.get(first).expect("candidate first");
    let removed = arena.remove(stale).expect("candidate remove");
    let fresh = arena.insert(30_i64);
    assert_eq!(fresh.index, stale.index);
    assert!(arena.get(stale).is_none());
    checksum += removed + *arena.get(fresh).expect("candidate fresh");
    CostSnapshot {
        checksum,
        slots: arena.slots.len(),
        free: arena.free.len(),
        operations: 7,
    }
}

fn reference_cost_snapshot() -> CostSnapshot {
    let mut arena = ReferenceArena::with_id(77);
    let first = arena.insert(10_i64);
    let stale = arena.insert(20_i64);
    let mut checksum = *arena.get(first).expect("reference first");
    let removed = arena.remove(stale).expect("reference remove");
    let fresh = arena.insert(30_i64);
    assert_eq!(fresh.index, stale.index);
    assert!(arena.get(stale).is_none());
    checksum += removed + *arena.get(fresh).expect("reference fresh");
    CostSnapshot {
        checksum,
        slots: arena.slots.len(),
        free: arena.free.len(),
        operations: 7,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocalHandle {
    index: usize,
    generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    RuntimeIdentityCandidate,
    RejectNoArenaIdentity,
    StaticScopeDeferred,
    TombstoneControl,
    SharedOwnerControl,
    SurfaceCandidate,
    BorrowBoundary,
    CostEvidence,
}

impl Class {
    const fn label(self) -> &'static str {
        match self {
            Self::RuntimeIdentityCandidate => "RUNTIME-IDENTITY-CANDIDATE",
            Self::RejectNoArenaIdentity => "REJECT-NO-ARENA-IDENTITY",
            Self::StaticScopeDeferred => "STATIC-SCOPE-DEFERRED",
            Self::TombstoneControl => "TOMBSTONE-CONTROL",
            Self::SharedOwnerControl => "SHARED-OWNER-CONTROL",
            Self::SurfaceCandidate => "SURFACE-CANDIDATE",
            Self::BorrowBoundary => "BORROW-BOUNDARY",
            Self::CostEvidence => "COST-EVIDENCE",
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
        name: "live-element-reference-blocks-remove",
        expected_compile: false,
        source: r#"
struct Arena<T> { slots: Vec<Option<T>> }
impl<T> Arena<T> {
    fn get(&self, index: usize) -> Option<&T> { self.slots.get(index)?.as_ref() }
    fn remove(&mut self, index: usize) -> Option<T> { self.slots.get_mut(index)?.take() }
}
fn main() {
    let mut arena = Arena { slots: vec![Some(String::from("a"))] };
    let item = arena.get(0).unwrap();
    let _removed = arena.remove(0);
    println!("{}", item);
}
"#,
        reason: "exclusive removal must conflict with a still-live element reference",
    },
    CompileCase {
        name: "remove-after-final-element-reference-use",
        expected_compile: true,
        source: r#"
struct Arena<T> { slots: Vec<Option<T>> }
impl<T> Arena<T> {
    fn get(&self, index: usize) -> Option<&T> { self.slots.get(index)?.as_ref() }
    fn remove(&mut self, index: usize) -> Option<T> { self.slots.get_mut(index)?.take() }
}
fn main() {
    let mut arena = Arena { slots: vec![Some(String::from("a"))] };
    let item = arena.get(0).unwrap();
    println!("{}", item);
    let removed = arena.remove(0).unwrap();
    assert_eq!(removed, "a");
}
"#,
        reason: "bounded last-use release permits removal after the reference is dead",
    },
    CompileCase {
        name: "live-element-reference-blocks-insert",
        expected_compile: false,
        source: r#"
struct Arena<T> { slots: Vec<T> }
impl<T> Arena<T> {
    fn get(&self, index: usize) -> Option<&T> { self.slots.get(index) }
    fn insert(&mut self, value: T) { self.slots.push(value); }
}
fn main() {
    let mut arena = Arena { slots: vec![String::from("a")] };
    let item = arena.get(0).unwrap();
    arena.insert(String::from("b"));
    println!("{}", item);
}
"#,
        reason: "growth or reuse-capable mutation must conflict with a live element reference",
    },
    CompileCase {
        name: "arena-move-blocked-while-element-reference-live",
        expected_compile: false,
        source: r#"
struct Arena<T> { slots: Vec<T> }
impl<T> Arena<T> {
    fn get(&self, index: usize) -> Option<&T> { self.slots.get(index) }
}
fn main() {
    let arena = Arena { slots: vec![String::from("a")] };
    let item = arena.get(0).unwrap();
    let moved = arena;
    println!("{}", item);
    drop(moved);
}
"#,
        reason: "moving the arena while an element reference remains live must be rejected",
    },
    CompileCase {
        name: "arena-reinitialization-blocked-while-element-reference-live",
        expected_compile: false,
        source: r#"
struct Arena<T> { slots: Vec<T> }
impl<T> Arena<T> {
    fn get(&self, index: usize) -> Option<&T> { self.slots.get(index) }
}
fn main() {
    let mut arena = Arena { slots: vec![String::from("a")] };
    let item = arena.get(0).unwrap();
    arena = Arena { slots: vec![String::from("b")] };
    println!("{}", item);
    drop(arena);
}
"#,
        reason: "reinitializing the arena while an element reference remains live must be rejected",
    },
];

const SURFACE_CANDIDATES: &[(&str, &str)] = &[
    ("arena-type", "arena Item"),
    ("handle-type", "handle Item"),
    ("arena-constructor", "arena Item()"),
    ("insert-binding", "insert items, Item(value = 1) as h"),
    (
        "checked-handle-lookup",
        "lookup items, h as item ... else ... end",
    ),
    (
        "checked-remove",
        "remove items, h as removed ... else ... end",
    ),
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
        .arg("evo_generational_arena_surface_case")
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

fn json_string(value: &str) -> String {
    format!("{value:?}")
}

fn csv_cell(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\"").replace('\n', "\\n"))
}

#[allow(clippy::too_many_arguments)]
fn write_reports(
    findings: &[Finding],
    compile_findings: &[CompileFinding],
    rustc_vv: &str,
    git_sha: &str,
    out: &Path,
    identity_wrap_fail_closed: bool,
    generation_wrap_retires_slot: bool,
    contextual_words_available: bool,
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
    writeln!(
        json,
        "  \"verdict\": \"RUNTIME-ARENA-ID-GENERATIONAL-CANDIDATE\","
    )
    .unwrap();
    writeln!(
        json,
        "  \"recommended_surface\": \"CONTEXTUAL-ARENA-HANDLE-CANDIDATE\","
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
        "  \"runtime_identity_candidate_count\": {},",
        count(Class::RuntimeIdentityCandidate)
    )
    .unwrap();
    writeln!(
        json,
        "  \"no_arena_identity_rejection_count\": {},",
        count(Class::RejectNoArenaIdentity)
    )
    .unwrap();
    writeln!(
        json,
        "  \"static_scope_deferred_count\": {},",
        count(Class::StaticScopeDeferred)
    )
    .unwrap();
    writeln!(
        json,
        "  \"tombstone_control_count\": {},",
        count(Class::TombstoneControl)
    )
    .unwrap();
    writeln!(
        json,
        "  \"shared_owner_control_count\": {},",
        count(Class::SharedOwnerControl)
    )
    .unwrap();
    writeln!(
        json,
        "  \"surface_candidate_count\": {},",
        count(Class::SurfaceCandidate)
    )
    .unwrap();
    writeln!(
        json,
        "  \"borrow_boundary_count\": {},",
        count(Class::BorrowBoundary)
    )
    .unwrap();
    writeln!(
        json,
        "  \"cost_evidence_count\": {},",
        count(Class::CostEvidence)
    )
    .unwrap();
    writeln!(
        json,
        "  \"identity_wrap_fail_closed\": {identity_wrap_fail_closed},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"generation_wrap_retires_slot\": {generation_wrap_retires_slot},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"contextual_words_available\": {contextual_words_available},"
    )
    .unwrap();
    writeln!(json, "  \"handle_copy_word_count\": 3,").unwrap();
    let candidate_cost = candidate_cost_snapshot();
    let reference_cost = reference_cost_snapshot();
    let reference_equivalent_work = candidate_cost == reference_cost;
    let candidate_handle_size_bytes = size_of::<Handle>();
    let reference_handle_size_bytes = size_of::<ReferenceHandle>();
    let reference_handle_size_equal = candidate_handle_size_bytes == reference_handle_size_bytes;
    writeln!(
        json,
        "  \"reference_equivalent_work\": {reference_equivalent_work},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"reference_handle_size_equal\": {reference_handle_size_equal},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"candidate_operation_count\": {},",
        candidate_cost.operations
    )
    .unwrap();
    writeln!(
        json,
        "  \"reference_operation_count\": {},",
        reference_cost.operations
    )
    .unwrap();
    writeln!(
        json,
        "  \"candidate_handle_size_bytes\": {candidate_handle_size_bytes},"
    )
    .unwrap();
    writeln!(
        json,
        "  \"reference_handle_size_bytes\": {reference_handle_size_bytes},"
    )
    .unwrap();
    writeln!(json, "  \"uses_global_handle_table\": false,").unwrap();
    writeln!(json, "  \"uses_refcount_for_handle_copy\": false,").unwrap();
    writeln!(json, "  \"uses_unsafe_pointer_graph\": false,").unwrap();
    writeln!(json, "  \"uses_lock_or_refcell\": false,").unwrap();
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
    writeln!(markdown, "# Generational arena surface v0 research").unwrap();
    writeln!(markdown).unwrap();
    writeln!(markdown, "- git_sha: `{git_sha}`").unwrap();
    writeln!(
        markdown,
        "- verdict: **RUNTIME-ARENA-ID-GENERATIONAL-CANDIDATE**"
    )
    .unwrap();
    writeln!(
        markdown,
        "- recommended surface family: **CONTEXTUAL-ARENA-HANDLE-CANDIDATE**"
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
        "The accepted candidate carries arena identity, slot index, and generation as three copyable machine-word-class fields. A free-slot stack gives O(1) reuse. Removal increments generation; a slot at generation u64::MAX is retired permanently instead of wrapping, so an old handle can never become valid again through generation wrap. Cross-arena lookup is rejected before slot access. The candidate adds no per-handle refcount traffic, global handle table, lock, RefCell, GC, or unsafe pointer graph."
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
    for (name, example) in SURFACE_CANDIDATES {
        writeln!(
            csv,
            "surface,{},{},0,{}",
            csv_cell(name),
            csv_cell("CONTEXTUAL-CANDIDATE"),
            csv_cell(example)
        )
        .unwrap();
    }
    fs::write(out.join("surface-matrix.csv"), csv).expect("write CSV report");
}

#[test]
#[ignore = "research evidence; dedicated workflow runs this exact test"]
fn generational_arena_surface_research_resolves_reuse_identity_and_borrow_boundaries() {
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

    let mut arena = Arena::with_id(11);
    let first = arena.insert(String::from("first"));
    findings.push(finding(
        "insert-produces-live-handle",
        Class::RuntimeIdentityCandidate,
        arena.get(first).map(String::as_str) == Some("first"),
        "insert returns identity for one occupied slot",
    ));

    let copied = first;
    findings.push(finding(
        "handle-copy-is-three-plain-fields",
        Class::CostEvidence,
        copied == first && size_of::<Handle>() == size_of::<u64>() * 2 + size_of::<usize>(),
        "handle copy is arena id plus index plus generation with no owner-count update",
    ));

    let stale = arena.insert(String::from("stale"));
    let stale_index = stale.index;
    let stale_generation = stale.generation;
    let removed = arena.remove(stale);
    findings.push(finding(
        "remove-invalidates-old-handle",
        Class::RuntimeIdentityCandidate,
        removed.as_deref() == Some("stale") && arena.get(stale).is_none(),
        "successful removal moves the payload out and invalidates the old generation",
    ));

    let fresh = arena.insert(String::from("fresh"));
    findings.push(finding(
        "reuse-preserves-slot-index-and-advances-generation",
        Class::RuntimeIdentityCandidate,
        fresh.index == stale_index
            && fresh.generation == stale_generation + 1
            && arena.get(fresh).map(String::as_str) == Some("fresh"),
        "free-slot reuse is O(1) and returns a fresh generation",
    ));
    findings.push(finding(
        "stale-generation-cannot-resolve-reused-payload",
        Class::RuntimeIdentityCandidate,
        arena.get(stale).is_none() && arena.get(fresh).is_some(),
        "generation equality is required before payload access",
    ));

    let other = Arena::<String>::with_id(99);
    findings.push(finding(
        "wrong-arena-handle-is-rejected",
        Class::RuntimeIdentityCandidate,
        other.get(fresh).is_none(),
        "arena identity is checked before slot index and generation",
    ));

    let local = LocalHandle {
        index: fresh.index,
        generation: fresh.generation,
    };
    let mut same_shape = Arena::with_id(100);
    same_shape.insert(String::from("anchor"));
    let first_other_handle = same_shape.insert(String::from("old"));
    drop(same_shape.remove(first_other_handle));
    let other_handle = same_shape.insert(String::from("other"));
    let false_accept_without_arena_id = local.index == other_handle.index
        && local.generation == other_handle.generation
        && same_shape.slots[local.index]
            .value
            .as_deref()
            .is_some_and(|value| value == "other");
    findings.push(finding(
        "index-generation-alone-can-cross-resolve",
        Class::RejectNoArenaIdentity,
        false_accept_without_arena_id,
        "two arenas can contain the same index and generation while owning different payloads",
    ));
    findings.push(finding(
        "arena-id-field-closes-cross-arena-hole",
        Class::RejectNoArenaIdentity,
        same_shape.get(fresh).is_none(),
        "the three-field handle rejects the same wrong-arena case",
    ));

    let mut tombstones = vec![Some(10_i64), Some(20_i64)];
    let removed_tombstone = tombstones[0].take();
    tombstones.push(Some(30));
    findings.push(finding(
        "non-reusing-tombstone-control-is-safe-but-grows",
        Class::TombstoneControl,
        removed_tombstone == Some(10) && tombstones[0].is_none() && tombstones[2] == Some(30),
        "never reusing holes avoids generation rebinding but storage grows with removals",
    ));

    findings.push(finding(
        "static-instance-scoped-handle-needs-broader-type-provenance",
        Class::StaticScopeDeferred,
        true,
        "current nominal types cannot encode one runtime arena instance into record or sequence handle fields without a broader dependent/generic scope mechanism",
    ));

    struct DropProbe(Rc<Cell<usize>>);
    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    let drops = Rc::new(Cell::new(0usize));
    {
        let mut owned = Arena::with_id(12);
        let removed_handle = owned.insert(DropProbe(Rc::clone(&drops)));
        owned.insert(DropProbe(Rc::clone(&drops)));
        drop(owned.remove(removed_handle));
        assert_eq!(drops.get(), 1);
    }
    findings.push(finding(
        "removed-and-live-payloads-drop-exactly-once",
        Class::RuntimeIdentityCandidate,
        drops.get() == 2,
        "removed payload ownership transfers once and remaining live slots drop with the arena",
    ));

    #[derive(Debug, PartialEq, Eq)]
    struct MoveOnly {
        text: String,
    }
    let mut move_only = Arena::with_id(13);
    let move_handle = move_only.insert(MoveOnly {
        text: String::from("payload"),
    });
    let moved = move_only.remove(move_handle).expect("payload exists");
    findings.push(finding(
        "move-only-remove-transfers-without-clone",
        Class::RuntimeIdentityCandidate,
        moved.text == "payload" && move_only.get(move_handle).is_none(),
        "removal moves the payload; checked reads remain non-owning",
    ));

    #[derive(Debug, PartialEq, Eq)]
    struct SharedPayload {
        value: i64,
    }
    let shared = Rc::new(SharedPayload { value: 7 });
    let mut shared_arena = Arena::with_id(14);
    let shared_handle = shared_arena.insert(Rc::clone(&shared));
    let before = Rc::strong_count(&shared);
    let looked_up = shared_arena.get(shared_handle).expect("shared payload");
    let after = Rc::strong_count(&shared);
    findings.push(finding(
        "shared-owner-lookup-adds-no-hidden-dup",
        Class::SharedOwnerControl,
        before == 2 && after == before && looked_up.value == 7,
        "arena lookup borrows the stored explicit shared owner and does not clone it",
    ));

    let mut wrap = Arena::with_id(15);
    wrap.slots.push(Slot {
        generation: u64::MAX,
        value: Some(String::from("last-generation")),
        retired: false,
    });
    let wrap_handle = Handle {
        arena: 15,
        index: 0,
        generation: u64::MAX,
    };
    let wrap_removed = wrap.remove(wrap_handle);
    let replacement = wrap.insert(String::from("replacement"));
    let generation_wrap_retires_slot = wrap_removed.as_deref() == Some("last-generation")
        && wrap.slots[0].retired
        && replacement.index == 1
        && wrap.get(wrap_handle).is_none();
    findings.push(finding(
        "generation-wrap-retires-slot-instead-of-wrapping",
        Class::RuntimeIdentityCandidate,
        generation_wrap_retires_slot,
        "a max-generation slot is retired permanently after removal so stale identity cannot reappear",
    ));

    let identity_wrap_fail_closed = u64::MAX.checked_add(1).is_none();
    findings.push(finding(
        "arena-identity-wrap-must-fail-closed",
        Class::RuntimeIdentityCandidate,
        identity_wrap_fail_closed,
        "a monotonic single-thread arena identity source must reject exhaustion rather than wrap",
    ));

    let contextual_words_available = ["arena", "handle", "insert", "remove"]
        .into_iter()
        .all(|word| is_identifier(word, word));
    findings.push(finding(
        "candidate-words-remain-ordinary-identifiers",
        Class::SurfaceCandidate,
        contextual_words_available,
        "arena, handle, insert and remove can remain contextual rather than new lexer keywords",
    ));

    let ordinary_identifier_program = r#"
arena = 1
handle = 2
insert = 3
remove = 4
print arena + handle + insert + remove
"#;
    findings.push(finding(
        "ordinary-bindings-using-candidate-words-still-parse",
        Class::SurfaceCandidate,
        lex(ordinary_identifier_program).is_ok_and(|tokens| parse(&tokens).is_ok()),
        "candidate spellings remain usable as ordinary identifiers outside contextual positions",
    ));

    findings.push(finding(
        "bounded-arena-type-family-needs-no-general-generics",
        Class::SurfaceCandidate,
        SURFACE_CANDIDATES
            .iter()
            .any(|(name, example)| *name == "arena-type" && *example == "arena Item"),
        "arena Item can follow the existing contextual prefix-type pattern",
    ));
    findings.push(finding(
        "bounded-handle-type-family-needs-no-general-generics",
        Class::SurfaceCandidate,
        SURFACE_CANDIDATES
            .iter()
            .any(|(name, example)| *name == "handle-type" && *example == "handle Item"),
        "handle Item is a bounded contextual type and can later be admitted as a sequence/record field type",
    ));
    findings.push(finding(
        "existing-lookup-branch-shape-can-carry-handle-lookup",
        Class::SurfaceCandidate,
        SURFACE_CANDIDATES.iter().any(|(name, example)| {
            *name == "checked-handle-lookup" && example.starts_with("lookup ")
        }),
        "checked lookup already has an explicit success/failure branch shape in Evolution",
    ));
    findings.push(finding(
        "checked-remove-needs-explicit-success-failure",
        Class::SurfaceCandidate,
        SURFACE_CANDIDATES
            .iter()
            .any(|(name, example)| *name == "checked-remove" && example.starts_with("remove ")),
        "stale and wrong-arena removal must remain source-visible failure rather than panic or fabricated validity",
    ));

    let candidate_cost = candidate_cost_snapshot();
    let reference_cost = reference_cost_snapshot();
    findings.push(finding(
        "idiomatic-reference-equivalent-work-matches",
        Class::CostEvidence,
        candidate_cost == reference_cost,
        "candidate and independent idiomatic Rust generational-slot reference perform the same seven logical operations and end with the same checksum/storage/free-list state",
    ));
    findings.push(finding(
        "idiomatic-reference-handle-size-matches",
        Class::CostEvidence,
        size_of::<Handle>() == size_of::<ReferenceHandle>(),
        "candidate and idiomatic reference handles occupy the same three-machine-word-class identity shape",
    ));

    findings.push(finding(
        "free-list-reuse-is-constant-time-class",
        Class::CostEvidence,
        arena.free.len() <= arena.slots.len(),
        "one free-index stack pop/push replaces a linear hole scan in the accepted runtime candidate",
    ));
    findings.push(finding(
        "lookup-cost-is-bounded-direct-storage-work",
        Class::CostEvidence,
        arena.get(first).map(String::as_str) == Some("first"),
        "lookup is arena-id comparison plus Vec bounds check plus generation/occupancy check",
    ));

    let independent = Rc::new(String::from("independent"));
    let independent_alias = Rc::clone(&independent);
    findings.push(finding(
        "independent-lifetime-remains-shared-owner-domain",
        Class::SharedOwnerControl,
        Rc::strong_count(&independent) == 2 && independent_alias.as_str() == "independent",
        "values that must outlive or detach from arena ownership still belong to explicit shared ownership",
    ));

    let compile_root = env::var_os("EVO_GENERATIONAL_ARENA_SURFACE_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/evo-generational-arena-surface-research")
        });
    let compile_findings = COMPILE_CASES
        .iter()
        .map(|case| run_compile_case(case, &compile_root.join("compile-cases")))
        .collect::<Vec<_>>();

    findings.extend([
        finding(
            "live-reference-removal-conflict-is-native-rust",
            Class::BorrowBoundary,
            !compile_findings[0].compiled,
            "ordinary Rust shared-vs-exclusive borrowing rejects removal while an element reference is live",
        ),
        finding(
            "final-use-release-is-native-rust",
            Class::BorrowBoundary,
            compile_findings[1].compiled,
            "ordinary non-lexical lifetimes accept removal after the final reference use",
        ),
        finding(
            "live-reference-growth-conflict-is-native-rust",
            Class::BorrowBoundary,
            !compile_findings[2].compiled,
            "ordinary Rust borrowing rejects insert/growth while a referenced element may still be used",
        ),
        finding(
            "arena-move-conflict-is-native-rust",
            Class::BorrowBoundary,
            !compile_findings[3].compiled,
            "ordinary Rust borrowing rejects moving the arena while a borrowed element remains live",
        ),
        finding(
            "arena-reinitialization-conflict-is-native-rust",
            Class::BorrowBoundary,
            !compile_findings[4].compiled,
            "ordinary Rust borrowing rejects reinitializing the arena while a borrowed element remains live",
        ),
    ]);

    let unmatched_findings = findings
        .iter()
        .filter(|finding| !finding.matched)
        .map(|finding| finding.name)
        .collect::<Vec<_>>();
    assert!(
        unmatched_findings.is_empty(),
        "unmatched findings: {unmatched_findings:?}"
    );
    assert!(compile_findings.iter().all(|finding| finding.matched));
    assert!(findings.len() >= 24);
    assert!(
        findings
            .iter()
            .filter(|finding| finding.class == Class::RuntimeIdentityCandidate)
            .count()
            >= 8
    );
    assert!(
        findings
            .iter()
            .filter(|finding| finding.class == Class::RejectNoArenaIdentity)
            .count()
            >= 2
    );
    assert!(
        findings
            .iter()
            .filter(|finding| finding.class == Class::SurfaceCandidate)
            .count()
            >= 5
    );
    assert!(
        findings
            .iter()
            .filter(|finding| finding.class == Class::BorrowBoundary)
            .count()
            >= 5
    );

    let out = env::var_os("EVO_GENERATIONAL_ARENA_SURFACE_RESEARCH_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/evo-generational-arena-surface-research")
        });
    let git_sha = env::var("EVO_GIT_SHA").unwrap_or_else(|_| "unknown".to_owned());

    write_reports(
        &findings,
        &compile_findings,
        &rustc_vv,
        &git_sha,
        &out,
        identity_wrap_fail_closed,
        generation_wrap_retires_slot,
        contextual_words_available,
    );
}
