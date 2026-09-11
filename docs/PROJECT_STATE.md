# Rust Evolution — Project State

Last verified update: **2026-09-11**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Current verified stable main before #108 research merge: `3be6b212a549831c15ef5a30df2b35da6362683e`
- Rust toolchain: **1.98.0**
- Production flags: edition 2024, opt-level 3, codegen-units 1
- Measured GNU/Linux linker path: `rustc -> cc -> lld`
- Natural exact-SHA validation for current main: CI #459 / run `34612808532` **SUCCESS** on Ubuntu, Windows and macOS, including permanent Ubuntu build/cache/runtime gates and release build.

## Accepted build / compile sequence

- #76: build latency baseline established; single-file native builds are rustc-dominated.
- #79 / PR #81: `build-cache-v0` accepted for verified unchanged-build artifact reuse.
- #82 / PR #84: changed-source rustc incremental research **REJECT / DEFER**.
- #85 / PR #86: link-time attribution established current `cc -> lld` path and link cost.
- #87 / PR #88: GNU ld / mold candidate experiment **REJECT / DEFER**.
- #89 / PR #90: opt-level 3 -> 2 release optimization candidate **REJECT / DEFER**.
- #91 / PR #92: compile-memory baseline **DEFER / NO ACTION**.
- #93 / PR #94: binary-size baseline **DEFER / NO ACTION**; controlled reference/Evolution binaries were byte-identical across the seven-case corpus.

Dependency-build, proc-macro cost and workspace scaling remain deferred until Evolution has a real user-program package/dependency graph.

## Ownership-ergonomics chain

### #95 / PR #96 — borrow inference feasibility

Research verdict: **IMPLEMENT-CANDIDATE**. A local, deliberately non-transitive classifier can infer call-duration shared borrows for read-only nominal parameters without generalized lifetime solving.

### #97 / PR #99 — inferred shared-borrow nominal parameters v0

Implemented explicit `Owned` / `SharedBorrow` parameter modes. `SharedBorrow` is call-duration and non-owning; it does not create a stored reference or escaping lifetime relation.

### #100 / PR #101 — lifetime-elision feasibility

Verdict: **REFERENCE-SURFACE-FIRST**. Rust 1.98 can elide named lifetimes for deterministic single-source returned-reference signatures, but escaping results remain caller-visible references and ambiguous multi-owner relations fail closed. Durable report: `docs/LIFETIME_ELISION_RESEARCH.md`.

### #102 / PR #103 — immutable reference surface research

Verdict: **SURFACE-CANDIDATE / PUNCTUATION-AMPERSAND**. The selected caller-visible surface is `&T` / `&expr`. The research retained 19 semantic cases with zero compile-expectation mismatches. Durable report: `docs/IMMUTABLE_REFERENCE_SURFACE_RESEARCH.md`.

### #104 / PR #105 — immutable references v0 production implementation

Issue #104 is closed/completed. PR #105 squash-merged as `5e3c111a903dbe717f374fcbe6fb2e93ab6864c1`. Exact post-merge CI #457 / run `34609435365` succeeded on Ubuntu, Windows and macOS.

Production behavior includes:

- lexer/parser/formatter `&T` and `&expr`;
- explicit immutable-reference semantic/lowered value types;
- first-class stored reference locals;
- deterministic single-source provenance through forwarding/calls/recursion/same-source branches;
- fail-closed multi-owner/dead-local return rejection;
- owner move/reinitialization conflicts while dependent references remain live;
- bounded final-use liveness allowing owner move after the final proven reference use;
- scalar reads but no move-only nominal field extraction through a shared reference;
- interoperability with inferred call-duration `SharedBorrow` without accidental `&&T`;
- direct ordinary safe Rust reference lowering;
- no hidden allocation, clone, RC/GC, runtime ownership map, invented `'static`, or unsafe lifetime widening.

### #107 — post-implementation handoff

Docs PR #107 synchronized the durable handoff and merged as current verified main `3be6b212a549831c15ef5a30df2b35da6362683e`. Exact post-merge CI #459 succeeded.

### #106 / PR #108 — shared ownership ergonomics v0 research

Current research verdict: **SPLIT-RESEARCH**.

Validated exact research evidence head:

`651ef4f3e9fc70e493eb59e3b1b2e7dcc7526d6a`

Evidence:

- normal CI #463 / run `34614250716`: **SUCCESS** on Ubuntu, Windows and macOS;
- dedicated Shared ownership ergonomics research #4 / run `34614250773`: **SUCCESS**;
- artifact `evo-shared-ownership-research-ubuntu-24.04`;
- artifact id `10269252562`;
- digest `sha256:f33fd905c3feb550550a8fc99da75575dcce1d8a54c9ef1b1a531a68256149b5`;
- 14 Rust 1.98 cases;
- zero compile-expectation mismatches;
- zero runtime-expectation mismatches.

Durable report: `docs/SHARED_OWNERSHIP_RESEARCH.md`.

Key classification result:

- one owned control;
- one `BORROW-INSTEAD` control;
- five `EXPLICIT-SHARED-CANDIDATE` cases for one-thread `Rc`-like ownership / borrowing from such a handle;
- one `REQUIRES-INTERIOR-MUTABILITY-DESIGN` case;
- three `REQUIRES-CONCURRENCY-DESIGN` cases;
- one `REQUIRES-WEAK/CYCLE-MODEL` case;
- one `ARENA/INDEX-PREFERRED` case;
- one `REJECT-HIDDEN-COST/AMBIGUOUS` handle-clone/deep-clone case.

The research proves that shared ownership cannot be one magic aliasing mode. `SharedBorrow`, `SharedRef` and `ReferenceTracker` remain non-owning concepts and must not be overloaded into shared ownership.

The useful bounded split is a caller-visible, one-thread, reference-counted shared-owner handle whose allocation and owner duplication are explicit and whose future codegen, if accepted, maps directly to idiomatic `Rc<T>` operations. `Arc`, thread transfer, interior mutability, synchronization, `Weak`/cycles and arena/index ownership remain separate research/design tracks.

PR #108 remains research-only until its final exact head passes normal CI plus the dedicated research workflow and is merged. #106 must not close until natural post-merge exact-SHA main validation succeeds.

## Gated successor — #109

Issue #109 is open:

`P0 research explicit shared handle surface v0: one-thread Rc-like nominal ownership`

It is **gated** until #108 merges and natural exact-SHA `main` normal CI plus dedicated shared-ownership research validation succeed.

#109 will compare explicit surface families for an `Rc`-equivalent ownership contract. Required invariants include:

- initial shared allocation is caller-visible;
- handle duplication is caller-visible and distinct from deep-cloning the payload;
- ordinary assignment and by-value parameter/return remain moves of the handle rather than hidden refcount increments;
- payload references remain ordinary non-owning references with existing provenance/liveness rules;
- moving the specific handle that produced a live borrow remains invalid even when another shared owner exists;
- no automatic `Arc`, locking, interior mutability, `Weak`, GC or runtime ownership map is smuggled into the abstraction.

No production shared-owner syntax or semantic type is currently accepted.

## Implemented language / tooling state

`docs/LANGUAGE_SPEC_V0.md` is the implemented-language source of truth. The production core currently includes integer/bool/static strings, input/repeat/control flow, functions, lexical block locals, Records v0, Enums v0, by-value ownership/reinitialization, inferred call-duration shared-borrow parameters, first-class bounded immutable references, source-native move/reference diagnostics, source maps, formatter, native check/emit/build/run and verified run/build caches.

## Explicit current non-goals

No mutable references, generalized/user-written lifetime solver, implicit shared ownership, hidden handle duplication/allocation/deep clone, `Arc`-style concurrency, interior-mutability implementation, synchronization runtime design, cyclic/self-referential production structures, GC or silent changes to existing `T` / `&T` contracts are approved by the current ownership work.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed SHAs remain evidence and are not rerun merely for a better color.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
