# Rust Evolution — Project State

Last verified update: **2026-09-11**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Current verified stable main predecessor for active #102: `3f501b51c79c39241107df1cbe098acb6ec058ad`
- Rust toolchain: **1.98.0**
- production flags: edition 2024, opt-level 3, codegen-units 1
- measured GNU/Linux linker path: `rustc -> cc -> lld`

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

Durable reports remain under `docs/` for each accepted/rejected/deferred research step.

## #95 / #97 — bounded shared-borrow work completed

#95 / PR #96 established a useful local read-only nominal-parameter subset and returned **IMPLEMENT-CANDIDATE**. #97 / PR #99 implemented that bounded rule.

PR #99 squash-merged to main as:

`355847a23faa29360e1da10bdfb2739eec6f8b6a`

Natural post-merge CI #406 / run `34577783740`: **SUCCESS** on Ubuntu, Windows and macOS.

Issue #97 is closed/completed.

### Production shared-borrow contract

The lowering layer has explicit parameter passing modes:

- `Owned`: ordinary by-value behavior;
- `SharedBorrow`: call-duration immutable borrow for a proven read-only nominal parameter.

A parameter can become `SharedBorrow` only when the accepted local/non-transitive classifier proves the nominal parameter is inspected, never consumed and never reinitialized under the current body-use rules. Classification does not recursively depend on callee modes.

Rust codegen emits ordinary `&T` parameters and `&expr` arguments. This feature creates no stored first-class reference value and no returned/escaping reference. No implicit clone/copy, boxing, RC/GC, runtime ownership map, invented lifetime or mutable borrow was added.

## #100 — lifetime elision feasibility completed

Issue #100 / PR #101 established the boundary required before escaping borrowed results.

PR #101 squash-merged to main as:

`3f501b51c79c39241107df1cbe098acb6ec058ad`

Exact-SHA post-merge validation:

- CI #410 / run `34581869543`: **SUCCESS** on Ubuntu, Windows and macOS;
- Lifetime elision research #4 / run `34581869494`: **SUCCESS**;
- artifact id `10192045985`;
- digest `sha256:2a21e3a591a33bcf07fc36bb2b2c29c2bfed15234a4a3e30fdc010e02c8e976e`;
- aggregate result remains **REFERENCE-SURFACE-FIRST** with zero compile-expectation mismatches.

Issue #100 is complete.

### Meaning of REFERENCE-SURFACE-FIRST

Rust 1.98 can elide named lifetime parameters for useful deterministic single-source returned-reference signatures. That does not make a returned reference an owned value.

Current `T -> T` APIs remain owned. Escaping borrowed results require an explicit caller-visible reference distinction, and ambiguous multi-owner relationships remain outside the bounded subset.

Durable report: `docs/LIFETIME_ELISION_RESEARCH.md`.

## #102 / PR #103 — immutable reference surface research completed

The immutable-reference surface research is complete and merged to `main` as:

`cc7e7002bd1dd9726e0fd6bcf3d73687fddc0e15`

Accepted result: **SURFACE-CANDIDATE / PUNCTUATION-AMPERSAND**. The locked research matrix contains 19 semantic cases with zero compile-expectation mismatches. Durable report: `docs/IMMUTABLE_REFERENCE_SURFACE_RESEARCH.md`.

## Active production — #104 / PR #105 immutable references v0

Issue #104 / draft PR #105 implements the bounded production successor on `feature/immutable-reference-surface-v0`.

### Implemented surface and representation

Production code now has explicit immutable-reference representation across the frontend and lowering pipeline:

```text
syntax type:       SharedRef(TypeName)
syntax expression: SharedBorrow(Expr)
semantic type:     SharedRef(SemanticType)
lowered value:     SharedRef(ValueType)
provenance:        Parameter(name) | LocalOwner(name)
```

The accepted source forms are `&T` in nominal-record function contracts and `&expr` for first-class immutable borrows. Existing owned `T` remains owned and inferred `SharedBorrow` remains a separate call-duration passing mode.

### Provenance and liveness contract

- A returned reference has one deterministic source reference parameter in v0.
- Multiple possible reference inputs for a borrowed result fail closed rather than inventing a lifetime relation.
- Local reference bindings retain provenance from their owner/reference source.
- Forwarding calls, direct recursion and same-owner branches preserve that source.
- A reference derived from a function-local owner cannot escape via return.
- Owner move or reinitialization is rejected while a possibly-live reference points at it.
- Bounded last-use analysis releases a local reference after its final proven use in the current statement block.
- An outer reference used inside `if`/`repeat` remains live through the complete control-flow statement and may end afterward when there is no later use.
- Uncertain/over-approximated cases remain live conservatively; safety is preferred over convenience.
- Scalar fields may be inspected through a shared reference; moving a nominal record field through a reference is rejected with no implicit clone.

### Code generation

The feature lowers directly to safe ordinary Rust references. There is no hidden allocation, wrapper/reference object, RC/GC, runtime borrow map, unsafe lifetime extension, or invented `'static`.

### Permanent validation

Permanent tests cover lexer/recovery compatibility, formatter idempotence, parser fail-fast/recovery parity, semantic reference types, direct and local borrowing, live-owner move/reinit rejection, bounded final use, dead-local escape rejection, ambiguous-source rejection, forwarding, nested scalar field reads, recursion, same-owner branches, inferred `SharedBorrow` interoperability, nominal-field move rejection, and direct zero-cost Rust codegen.

The historical immutable-reference research workflow is retired. Temporary development bootstrap workflows are removed after each verified source slice and are not part of the production gate.

### Explicit non-goals

No mutable references, nested references, primitive reference types, reference fields in records/enums, generalized/user-written lifetime syntax, multi-owner lifetime solving, self-referential reference layouts, unsafe widening, hidden clone/copy, allocation, RC/GC, or runtime ownership machinery are approved by #104.

## Completion gate still open

PR #105 remains draft until documentation is synchronized and a clean user-authored exact head receives natural normal CI success across Ubuntu, Windows and macOS, including formatter, Clippy, workspace tests, release build and existing performance gates.

After merge, the natural exact-SHA `main` CI must succeed before #104 is closed completed.

## Implemented language / tooling state

`docs/LANGUAGE_SPEC_V0.md` is the implemented-language source of truth. The production core now includes integer/bool/static strings, input/repeat/control flow, functions, lexical block locals, Records v0, Enums v0, by-value ownership/reinitialization, inferred call-duration shared-borrow parameters, first-class bounded immutable references, source-native move/reference diagnostics, source maps, formatter, native check/emit/build/run and verified run/build caches.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed SHAs remain evidence and are not rerun merely for a better color.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
