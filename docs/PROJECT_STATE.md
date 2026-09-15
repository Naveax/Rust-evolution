# Rust Evolution — Project State

Last verified update: **2026-09-15**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Exact verified stable main before active #132 / PR #139: `f4af6aa89d83dcdee79cabcb4ed166aaf4db6612`
- Rust toolchain: **1.98.0**
- Production flags: edition 2024, opt-level 3, codegen-units 1
- Natural exact-main CI #547 / run `34857717355`: **SUCCESS** on Ubuntu 24.04, Windows and macOS
- Natural exact-main Arena generational handles research / run `34857717387`: **SUCCESS**
- Natural exact-main Explicit shared owner performance / run `34857717373`: **SUCCESS**

`f4af6aa...` is PR #131 squash merge and completes #126 arena/generational graph-handle research. #126 is closed/completed.

## Build / compile sequence

- #76: build latency baseline established; single-file native builds are rustc-dominated.
- #79 / PR #81: verified unchanged-build cache accepted.
- #82 / PR #84: changed-source rustc incremental research **REJECT / DEFER**.
- #85 / PR #86: link-time attribution established the current `rustc -> cc -> lld` path.
- #87 / PR #88: alternative linker experiment **REJECT / DEFER**.
- #89 / PR #90: opt-level 3 -> 2 candidate **REJECT / DEFER**.
- #91 / PR #92: compile-memory baseline **DEFER / NO ACTION**.
- #93 / PR #94: binary-size baseline **DEFER / NO ACTION**; controlled Evolution/reference binaries were byte-identical across the accepted corpus.

Dependency-build, proc-macro cost and workspace scaling remain deferred until Evolution has a real package/dependency graph.

## Ownership ergonomics implemented

### Inferred shared-borrow parameters

`Owned` and call-duration `SharedBorrow` parameter modes are distinct. SharedBorrow is non-owning and does not create a stored or escaping reference.

### First-class immutable references

`&T` / `&expr` are implemented for the bounded nominal slice with deterministic provenance, stored reference locals, source-owner move/reinitialization conflicts, bounded final-use liveness and direct safe Rust reference lowering.

### Explicit one-thread shared owners

Production source surface:

```text
shared Item
share expr
dup owner
```

Direct generated Rust mapping remains ordinary safe `Rc<T>`:

```text
shared Item -> std::rc::Rc<Item>
share expr  -> Rc::new(expr)
dup expr    -> Rc::clone(&expr)
```

Locked invariants:

- shared owners are distinct from owned values, first-class immutable references, and inferred SharedBorrow parameters;
- allocation and owner duplication are explicit only;
- ordinary assignment, by-value calls and returns move handles without hidden count increments;
- payload references remain tied to the specific source handle;
- moving/reinitializing that source handle rejects while its dependent reference may still be live;
- bounded final-use analysis releases the source after the final proven reference use;
- move-only payload extraction through shared ownership rejects instead of cloning;
- no hidden `Arc`, `RefCell`, synchronization, GC, global ownership registry or unsafe ownership emulation.

The permanent Explicit shared owner performance workflow remains a regression gate for changes that touch the relevant language/compiler surface.

## Cyclic / graph ownership research sequence

### Split boundary

Cyclic/graph research established that one ownership mechanism should not pretend to solve every graph problem. Explicit weak edges and container-owned arena identity remain separate cost/safety models.

### Arena/generational handles — #126 / PR #131 completed

Durable report: `docs/ARENA_GENERATIONAL_HANDLES_RESEARCH.md`.

Accepted result:

- aggregate gate: **REQUIRES-COLLECTION-SURFACE**;
- graph-identity recommendation: **GENERATIONAL-HANDLE-CANDIDATE**;
- plain numeric indices are acceptable only while removal/reuse cannot silently rebind identity;
- reusable/removable slots require generation checking;
- independent node lifetime remains a shared-owner problem rather than an arena problem.

The hard successor is #132, because Evolution must expose bounded explicit indexed storage before a production arena model can honestly state allocation, bounds, mutation, removal, borrowing and destruction semantics.

## Active research — #132 / PR #139 collection surface v0

Research branch:

`research/collection-surface-v0`

Accepted executable evidence head before documentation synchronization:

`2e620c246580bd446b3b42b3495325a599ffcff9`

Collection surface research #4 / run `34950557176`: **SUCCESS**.

Artifact:

- name: `evo-collection-surface-research-ubuntu-24.04`;
- id: `10388584777`;
- digest: `sha256:dfae195602260d8bb5b900ea821abd3950d013314938012e153d19f42182d690`;
- exact report SHA: `2e620c246580bd446b3b42b3495325a599ffcff9`;
- Rust: **1.98.0**;
- runtime/surface cases: **17**;
- compile-boundary cases: **4**;
- expectation mismatches: **0**;
- verdict: **APPEND-ONLY-FIRST**;
- recommended surface: **CONTEXTUAL-SEQUENCE-TYPE-CANDIDATE**.

Evidence summary:

- empty/non-empty owned indexed storage is straightforward;
- append/growth plus checked lookup maps directly to ordinary safe Rust storage;
- out-of-bounds checked lookup is explicit;
- exclusive container mutation needs no hidden interior mutability;
- container move/drop follows ordinary ownership;
- append-only numeric indices remain logically stable across physical `Vec` reallocation;
- shifting removal can silently rebind an old numeric index to a different payload;
- hole-preserving removal avoids shifting but introduces an explicit slot-occupancy model;
- move-only payload transfer remains explicit;
- explicit shared-owner payload composition remains separate from container growth;
- live element references block conflicting container growth/move;
- growth after the final element-reference use is valid under bounded last-use analysis;
- moving move-only payload through a shared element reference rejects;
- no hidden runtime mechanism is required for the accepted append-only slice.

Surface evidence:

- `[` / `]` are currently unallocated lexer punctuation;
- `seq` is currently a valid ordinary identifier;
- current type algebra already has bounded contextual wrapper precedent through `shared Item` and `&Item` without a general generic type system;
- the research therefore advances a contextual bounded sequence type family rather than `Vec(T)`-style general-generic syntax.

Durable report: `docs/COLLECTION_SURFACE_RESEARCH.md`.

## Collection production boundary authorized by research

Only after PR #139 final-head and natural postmerge gates succeed may a production successor start.

That first production slice is bounded to:

- contextual sequence type;
- explicit creation/allocation;
- explicit append/growth;
- checked indexed lookup;
- ordinary container move/drop;
- immutable element references integrated with the existing reference/liveness model;
- source-native rejection of conflicting growth/move while an element reference is live;
- no element removal or slot reuse;
- no general generics merely for this feature;
- direct safe Rust `Vec<T>`-class lowering;
- no hidden clone/refcount/GC/lock/registry/unsafe machinery.

Removal, holes, generations and arena slot reuse remain later explicit layers. Generation-checked arena handles from #126 can only resume after this collection prerequisite is production-verified.

## Separate / deferred research tracks

Do not silently fold these into collection semantics:

- explicit Weak/cycle-edge surface;
- interior mutability;
- cross-thread shared ownership / `Arc` / synchronization;
- mutable references;
- generalized/user-written lifetime solving;
- general generic type syntax;
- shared-owner record fields / enum payloads;
- hidden allocation, owner duplication, deep clone, GC or runtime ownership maps.

## Current operational sequence

1. Finish PR #139 documentation synchronization.
2. Require one exact final PR head to pass normal CI, Collection surface research, and Explicit shared owner performance.
3. Merge PR #139 with expected-head protection only after those gates pass.
4. Require natural exact-main postmerge CI and Collection surface research before closing #132 completed.
5. Open the bounded append-only indexed-sequence production successor from that verified main.
6. Implement and verify that slice before returning to generation-checked reusable arena slots.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed/cancelled historical SHAs remain evidence and are not rerun merely for color. CI running does not block independent source/docs work.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
