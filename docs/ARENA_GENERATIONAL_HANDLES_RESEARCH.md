# Arena / generational graph handles v0 research

Status: **REQUIRES-COLLECTION-SURFACE / GENERATIONAL-HANDLE-CANDIDATE**

Parent: #126

This track isolates container-owned graph identity from reference-counted ownership. It adds no Evolution production collection syntax, graph runtime machinery, GC, per-edge refcounting, or unsafe pointer semantics.

## Decision

The evidence supports **generation-checked arena handles** as the safer identity model once removable/reusable slots exist, but Evolution is not yet ready to expose that model honestly because it lacks a bounded explicit collection/container surface.

Final classification:

- aggregate gate: `REQUIRES-COLLECTION-SURFACE`;
- recommended graph-identity model: `GENERATIONAL-HANDLE-CANDIDATE`;
- plain indices remain acceptable only where removal/reuse cannot silently rebind identity;
- independent node lifetime remains an explicit shared-owner problem rather than an arena problem.

The gated successor is #132, which researches explicit indexed storage first. #132 must not start until this PR merges and natural exact-main CI plus dedicated arena research succeed.

## Exact executable evidence

Validated research source head:

`aebcf4ab7b6f7c427dac41fb694a121bde53fa20`

Arena generational handles research run `34842974855`: **SUCCESS** on pinned Rust 1.98.0.

Artifact:

- name: `evo-arena-generational-handles-research-ubuntu-24.04`;
- id: `10346249280`;
- digest: `sha256:e8f681525e82ade36b585d94674a158d1d490f27eeaeea44803652f993d9503b`;
- report `git_sha`: exact match to the validated source head;
- cases: **16**;
- expectation mismatches: **0**.

Observed classifications:

- generational-handle candidate: **8**;
- plain-index candidate: **3**;
- append-only arena candidate: **1**;
- shared-owner/Rc instead: **1**;
- collection-surface prerequisite: **1**;
- stale-identity rejection: **1**.

The final PR docs head still requires its own exact-head normal CI, dedicated research, and existing explicit-shared-owner regression gate before merge.

## What the matrix proves

The pinned Rust controls demonstrate:

- direct owned trees need no arena machinery;
- plain copied indices can represent DAG/cyclic traversal cheaply;
- checked lookup can reject an out-of-bounds stale index;
- slot reuse can silently make the same plain index denote a different payload;
- `(arena, index, generation)` identity rejects an old handle after slot reuse;
- a fresh generation resolves the reused slot while the stale generation remains invalid;
- handle copying requires no strong/weak refcount update;
- container drop deterministically destroys live payloads;
- explicit removal invalidates the old generation;
- generational traversal can match an equivalent `Rc` traversal result;
- exclusively owned arena mutation needs neither `RefCell` nor locking;
- handles from one arena are rejected by another arena;
- payload copying is distinct from handle copying;
- values requiring lifetime independent of the container still belong to the explicit shared-owner model.

## Safety / cost invariants

Any later production candidate must keep explicit:

- what owns node storage;
- whether storage is append-only or removable;
- generation increment and wraparound policy;
- cross-arena identity policy;
- checked lookup failure;
- handle copy versus payload clone;
- index stability under insertion/removal;
- container mutation versus interior mutability;
- independent node lifetime versus arena-tied lifetime.

Rejected shortcuts:

- hidden global handle tables;
- hidden per-edge reference counting;
- tracing GC;
- locks or `RefCell` inserted by the compiler;
- unsafe pointer graphs;
- silent cross-arena acceptance;
- fabricated stale-target validity;
- implicit payload clone.

## Successor boundary

No arena production implementation is authorized yet. The next bounded question is whether Evolution can expose explicit indexed storage with visible allocation, bounds, mutation, removal, borrowing, and payload-destruction rules without first becoming a general generic/container language. That question is isolated in #132.
