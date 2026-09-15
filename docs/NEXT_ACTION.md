# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-15**

## Stable gate

Current exact verified `main` before the active #132 research PR:

`f4af6aa89d83dcdee79cabcb4ed166aaf4db6612`

This is PR #131 squash merge, completing #126 arena/generational graph-handle research.

Natural validation on that exact SHA:

- CI #547 / run `34857717355`: **SUCCESS** on Ubuntu 24.04, Windows and macOS;
- Arena generational handles research / run `34857717387`: **SUCCESS**;
- Explicit shared owner performance / run `34857717373`: **SUCCESS**.

Issue #126 is closed/completed.

## Active P0 research — #132 / PR #139

`#132 P0 research collection surface v0: bounded indexed storage for arena foundations`

Branch:

`research/collection-surface-v0`

Accepted executable research head before documentation synchronization:

`2e620c246580bd446b3b42b3495325a599ffcff9`

Dedicated evidence:

- Collection surface research #4 / run `34950557176`: **SUCCESS**;
- artifact `evo-collection-surface-research-ubuntu-24.04`;
- artifact id `10388584777`;
- digest `sha256:dfae195602260d8bb5b900ea821abd3950d013314938012e153d19f42182d690`;
- Rust **1.98.0**;
- runtime/surface cases: **17**;
- compile-boundary cases: **4**;
- expectation mismatches: **0**;
- verdict: **APPEND-ONLY-FIRST**;
- recommended surface family: **CONTEXTUAL-SEQUENCE-TYPE-CANDIDATE**.

Evidence supports an explicit append-only owned sequence first. General removal is not part of this slice because shifting removal can silently rebind numeric indices; hole-preserving removal is a separate explicit storage policy.

Production successor boundaries already fixed by the evidence:

- contextual bounded sequence type, not general generic syntax merely for convenience;
- explicit allocation/construction and append;
- checked indexed lookup;
- ordinary container move/drop ownership;
- immutable element references use the existing reference model;
- live element references block conflicting container growth/move;
- bounded final-use release permits later growth after the reference is dead;
- no element removal/reuse in v0;
- no hidden clone, `Rc`/`Arc` duplication, `RefCell`, lock, GC, global registry, unsafe pointer table, or fabricated stable identity.

Durable decision report: `docs/COLLECTION_SURFACE_RESEARCH.md`.

## Immediate execution order

1. Synchronize `COLLECTION_SURFACE_RESEARCH`, `PROJECT_STATE`, and this file on PR #139.
2. Require one exact documentation-synchronized PR head to pass:
   - normal CI on Ubuntu, Windows and macOS;
   - Collection surface research;
   - Explicit shared owner performance regression gate.
3. Review the final PR diff and merge PR #139 only with expected-head protection.
4. Track natural exact-main postmerge CI and Collection surface research; track any naturally triggered ownership/performance gate without dispatching duplicates.
5. Close #132 completed only after the required postmerge exact-main gates succeed.
6. Open the bounded production successor for append-only indexed sequence v0 from that verified main. Do not reopen removal/generation semantics in that first implementation issue.
7. After the collection implementation is independently verified, return to the generation-checked arena-handle candidate from #126.

## Separate ownership/research lanes

Keep distinct rather than folding them into the collection model:

- explicit Weak/cycle edges;
- interior mutability;
- cross-thread shared ownership / synchronization;
- generation-checked removable arena slots;
- mutable references;
- generalized lifetime solving;
- general generic type syntax.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed/cancelled historical SHAs remain evidence and are not rerun merely for cosmetic green. CI running does not block independent source/docs work.
