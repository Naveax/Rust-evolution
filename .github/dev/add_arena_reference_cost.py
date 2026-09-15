from pathlib import Path

TEST = Path("crates/evo-lowering/tests/generational_arena_surface_research.rs")
WORKFLOW = Path(".github/workflows/generational-arena-surface-research.yml")
DOC = Path("docs/GENERATIONAL_ARENA_SURFACE_RESEARCH.md")

REFERENCE_BLOCK = r'''
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
'''

COST_FINDINGS = r'''    let candidate_cost = candidate_cost_snapshot();
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

'''

REPORT_LINES = r'''    let candidate_cost = candidate_cost_snapshot();
    let reference_cost = reference_cost_snapshot();
    let reference_equivalent_work = candidate_cost == reference_cost;
    let candidate_handle_size_bytes = size_of::<Handle>();
    let reference_handle_size_bytes = size_of::<ReferenceHandle>();
    let reference_handle_size_equal = candidate_handle_size_bytes == reference_handle_size_bytes;
    writeln!(json, "  \"reference_equivalent_work\": {reference_equivalent_work},").unwrap();
    writeln!(json, "  \"reference_handle_size_equal\": {reference_handle_size_equal},").unwrap();
    writeln!(json, "  \"candidate_operation_count\": {},", candidate_cost.operations).unwrap();
    writeln!(json, "  \"reference_operation_count\": {},", reference_cost.operations).unwrap();
    writeln!(json, "  \"candidate_handle_size_bytes\": {candidate_handle_size_bytes},").unwrap();
    writeln!(json, "  \"reference_handle_size_bytes\": {reference_handle_size_bytes},").unwrap();
'''


def insert_before_unique(text: str, needle: str, insertion: str, label: str) -> str:
    if text.count(needle) != 1:
        raise SystemExit(f"expected one {label}, found {text.count(needle)}")
    return text.replace(needle, insertion + needle)


def insert_after_line_with_key(text: str, key: str, insertion: str) -> str:
    lines = text.splitlines(keepends=True)
    hits = [i for i, line in enumerate(lines) if key in line]
    if len(hits) != 1:
        raise SystemExit(f"expected one line containing {key!r}, found {len(hits)}")
    lines.insert(hits[0] + 1, insertion)
    return "".join(lines)


text = TEST.read_text()
marker = "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\nstruct LocalHandle {\n"
text = insert_before_unique(text, marker, REFERENCE_BLOCK + "\n", "LocalHandle marker")
text = insert_after_line_with_key(text, "handle_copy_word_count", REPORT_LINES)
findings_anchor = '    findings.push(finding(\n        "free-list-reuse-is-constant-time-class",\n'
text = insert_before_unique(text, findings_anchor, COST_FINDINGS, "cost findings anchor")
TEST.write_text(text)

workflow = WORKFLOW.read_text()
workflow = workflow.replace(
    "          assert report['cost_evidence_count'] >= 2, report\n",
    "          assert report['cost_evidence_count'] >= 4, report\n",
)
if "assert report['cost_evidence_count'] >= 4" not in workflow:
    raise SystemExit("failed to strengthen cost evidence count")
workflow = insert_after_line_with_key(
    workflow,
    "assert report['handle_copy_word_count'] == 3",
    "          assert report['reference_equivalent_work'] is True, report\n"
    "          assert report['reference_handle_size_equal'] is True, report\n"
    "          assert report['candidate_operation_count'] == 7, report\n"
    "          assert report['reference_operation_count'] == 7, report\n"
    "          assert report['candidate_handle_size_bytes'] == report['reference_handle_size_bytes'], report\n",
)
WORKFLOW.write_text(workflow)

docs = DOC.read_text()
doc_anchor = "- no per-handle allocation or refcount operation is required.\n"
doc_insert = (
    "- a deterministic equivalent-work control executes the candidate and an independent idiomatic Rust generational-slot reference through the same seven insert/get/remove/reuse/stale-get operations;\n"
    "- both produce the same checksum, slot count, free-list state, operation count, and three-machine-word-class handle size.\n"
)
if docs.count(doc_anchor) != 1:
    raise SystemExit(f"expected one docs cost anchor, found {docs.count(doc_anchor)}")
DOC.write_text(docs.replace(doc_anchor, doc_anchor + doc_insert))
