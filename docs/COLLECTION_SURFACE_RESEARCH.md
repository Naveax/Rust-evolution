# Collection surface v0 research

Status: **APPEND-ONLY-FIRST / CONTEXTUAL-SEQUENCE-TYPE-CANDIDATE**

Parent: #132
Predecessor: #126 / PR #131 arena/generational graph-handle research
Research PR: #139

This track isolates the smallest explicit indexed-storage surface needed before arena/generational graph handles can become production behavior. It adds no production Evolution collection syntax, parser semantics, storage runtime, general generic type system, arena implementation, GC, hidden ownership map, or unsafe pointer machinery.

## Decision

The evidence supports an **append-only owned sequence first**.

The accepted first-slice direction is deliberately narrower than a general collection library:

- explicit owned container creation;
- explicit append/growth;
- checked indexed lookup;
- ordinary container move/drop ownership;
- ordinary immutable element borrows;
- exclusive container mutation only when no conflicting element borrow is live;
- bounded final-use release so a later growth operation can proceed after the last element-reference use;
- no removal in the first production slice;
- no fabricated stable identity beyond append-only numeric indices.

General shifting removal is rejected for the first slice because it can silently make an old numeric index denote a different payload. Hole-preserving removal avoids shifting but introduces a distinct occupied/empty slot policy, so it remains a later explicit storage-model decision rather than being smuggled into a generic sequence operation.

The preferred surface family is a **contextual bounded sequence type**, not general generics:

```text
seq Item
```

The exact production constructor/operations remain implementation-successor work. `seq` is currently ordinary identifier space, so any production parser must treat it contextually rather than reserve it globally unless a separate compatibility decision says otherwise.

## Exact accepted executable evidence

Validated research source head:

`2e620c246580bd446b3b42b3495325a599ffcff9`

Collection surface research #4 / run `34950557176`: **SUCCESS** on Ubuntu 24.04 with pinned Rust 1.98.0.

Artifact:

- name: `evo-collection-surface-research-ubuntu-24.04`;
- id: `10388584777`;
- digest: `sha256:dfae195602260d8bb5b900ea821abd3950d013314938012e153d19f42182d690`;
- report `git_sha`: exact match to the validated source head;
- runtime/surface cases: **17**;
- compile-boundary cases: **4**;
- expectation mismatches: **0**.

Observed classifications:

- append-only candidate: **8**;
- checked-growth candidate: **3**;
- removal identity risk: **1**;
- hole-removal candidate: **1**;
- surface constraint: **3**;
- explicit shared-owner composition control: **1**.

The report also records:

- square brackets are currently unallocated lexer punctuation;
- `seq` is currently a valid ordinary identifier;
- the current type algebra already has bounded contextual wrapper precedent through `shared Item` and `&Item` without requiring general generic syntax.

## Borrow / mutation boundary

The pinned Rust compile controls establish the intended zero-cost static boundary:

- an element reference live across container growth rejects;
- growth after the final element-reference use compiles;
- moving the owning container while an element reference remains live rejects;
- moving a move-only payload through a shared element reference rejects.

A production successor must preserve these rules in Evolution-source terms before Rust codegen. Generated Rust may enforce the same invariant as a backstop, but user-facing semantics and diagnostics must not depend on generated-Rust borrow-checker wording.

## Ownership and cost model

Accepted append-only storage maps to ordinary safe Rust `Vec<T>`-class storage behavior. The research requires no:

- hidden payload cloning;
- hidden `Rc`/`Arc` duplication;
- per-edge reference counting;
- `RefCell` or lock insertion;
- tracing GC;
- global handle/ownership registry;
- unsafe pointer table;
- fabricated index stability after removal.

Explicit shared-owner payloads remain a separate model. Storing a shared owner in a later collection slice must not make container growth silently duplicate owner handles.

## Surface alternatives retained as evidence

Three families were compared:

1. contextual prefix: `seq Item`;
2. bracket form: `[Item]`;
3. generic-like form: `Vec(Item)`.

The contextual prefix advances because it matches the existing bounded contextual type-constructor pattern without introducing general generic infrastructure. Brackets would consume currently-invalid punctuation but risk array/slice semantic overclaim. `Vec(Item)` collides with current nominal call-shaped identifier space and implies broader generic machinery that the bounded problem does not require.

## Deferred boundaries

Not authorized by this decision:

- element removal/reuse;
- hole management;
- generational slot reuse;
- stale-handle generation checking;
- general generic types;
- arbitrary collection algorithms;
- mutable references;
- hidden interior mutability;
- cross-thread synchronization;
- graph/arena production syntax;
- independent node lifetime beyond the container.

Generation-checked arena handles from #126 remain a later layer. They may advance only after the bounded collection surface is implemented and verified, because removable/reusable arena slots require explicit storage semantics that append-only sequences intentionally do not provide.

## Production-successor gate

A production successor may implement only the bounded append-only slice supported above. It must retain direct safe Rust lowering, source-native ownership diagnostics, checked lookup semantics, explicit growth, no removal, and the existing zero-hidden-work invariant.

PR #139 still requires one exact documentation-synchronized head to pass normal CI, the dedicated Collection surface research workflow, and the existing explicit-shared-owner regression/performance gate before merge. After merge, natural exact-main validation must succeed before #132 closes completed or the production successor starts.
