# Rust Evolution — Project State

Last verified update: **2026-09-11**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Current verified stable main: `5e3c111a903dbe717f374fcbe6fb2e93ab6864c1`
- Rust toolchain: **1.98.0**
- Production flags: edition 2024, opt-level 3, codegen-units 1
- Measured GNU/Linux linker path: `rustc -> cc -> lld`
- Natural post-merge validation for current main: CI #457 / run `34609435365` **SUCCESS** on Ubuntu, Windows and macOS, including permanent Ubuntu build/cache/runtime gates and release build.

## Accepted build / compile sequence

- #76: build latency baseline established; single-file native builds are rustc-dominated.
- #79 / PR #81: `build-cache-v0` accepted for verified unchanged-build artifact reuse.
- #82 / PR #84: changed-source rustc incremental research **REJECT / DEFER**.
- #85 / PR #86: link-time attribution established current `cc -> lld` path and link cost.
- #87 / PR #88: GNU ld / mold candidate experiment **REJECT / DEFER**.
- #89 / PR #90: opt-level 3 -> 2 release optimization candidate **REJECT / DEFER**.
- #91 / PR #92: compile-memory baseline **DEFER / NO ACTION**.
- #93 / PR #94: binary-size baseline **DEFER / NO ACTION**; controlled reference/Evolution binaries were byte-identical across the seven-case corpus.

Dependency-build, proc-macro cost and workspace scaling remain deferred until Evolution has a real user-program package/dependency graph. Durable reports remain under `docs/` for each accepted/rejected/deferred research step.

## Ownership-ergonomics chain completed through immutable references v0

### #95 / PR #96 — borrow inference feasibility

Research verdict: **IMPLEMENT-CANDIDATE**. A local, deliberately non-transitive classifier can infer call-duration shared borrows for read-only nominal parameters without generalized lifetime solving.

PR #96 merged as `cfc19e3308a505a45c970e883dffc72a8bf0b81b`.

### #97 / PR #99 — inferred shared-borrow nominal parameters v0

Implemented explicit `Owned` / `SharedBorrow` parameter modes. `SharedBorrow` is a call-duration non-owning mode only; it does not create a stored reference or escaping lifetime relation.

PR #99 merged as `355847a23faa29360e1da10bdfb2739eec6f8b6a`; #97 is closed/completed.

### #100 / PR #101 — lifetime-elision feasibility

Verdict: **REFERENCE-SURFACE-FIRST**. Rust 1.98 can elide named lifetimes for deterministic single-source returned-reference signatures, but escaping results remain caller-visible references and ambiguous multi-owner relations fail closed.

PR #101 merged as `3f501b51c79c39241107df1cbe098acb6ec058ad`. Durable report: `docs/LIFETIME_ELISION_RESEARCH.md`.

### #102 / PR #103 — immutable reference surface research

Verdict: **SURFACE-CANDIDATE / PUNCTUATION-AMPERSAND**. The selected source surface is:

```text
&T
&expr
```

The research matrix retained 19 semantic cases with zero compile-expectation mismatches. Durable report: `docs/IMMUTABLE_REFERENCE_SURFACE_RESEARCH.md`.

PR #103 merged as `cc7e7002bd1dd9726e0fd6bcf3d73687fddc0e15`.

### #104 / PR #105 — immutable references v0 production implementation

Issue #104 is closed/completed. PR #105 squash-merged to current `main` as:

`5e3c111a903dbe717f374fcbe6fb2e93ab6864c1`

Exact post-merge CI #457 / run `34609435365`: **SUCCESS** on Ubuntu, Windows and macOS.

Production behavior now includes:

- lexer, parser and formatter support for `&T` and `&expr`;
- explicit immutable-reference syntax, semantic and lowered value types;
- nominal-record reference parameters and returns;
- first-class local reference bindings;
- deterministic single-source provenance across direct/local forwarding, calls, recursion and same-source branches;
- fail-closed rejection of ambiguous returned-reference sources and dead-local escapes;
- owner move/reinitialization rejection while a dependent reference may still be live;
- bounded final-use liveness allowing a later owner move/reinitialization after the final proven reference use;
- conservative control-flow liveness through branch/repeat statements;
- scalar field inspection through immutable references;
- source-native rejection of moving nominal move-only fields through a shared reference;
- interoperability with inferred call-duration `SharedBorrow` without accidental `&&T`;
- direct ordinary safe Rust `&T` / `&expr` lowering;
- no hidden allocation, wrapper reference object, clone, RC/GC, runtime ownership map, unsafe lifetime widening or invented `'static`.

Permanent lexer/parser/formatter/lowering/diagnostic/codegen/compatibility tests carry the production proof. Historical immutable-reference research evidence is preserved, while its obsolete main-push research workflow was retired after adoption.

## Active successor — #106 shared ownership ergonomics research

Issue #106 is open:

`P0 research shared ownership ergonomics v0: explicit aliasing without hidden ownership cost`

Start gate is current verified `main` `5e3c111a903dbe717f374fcbe6fb2e93ab6864c1` after #104 completion.

The research must determine whether the current language is ready for an explicit shared-owner contract and must distinguish:

- owned `T`;
- borrowed `&T`;
- one-thread multi-owner `Rc`-like semantics;
- cross-thread `Arc`-like semantics;
- interior mutability and synchronization as separate semantic/cost boundaries;
- `Weak`/cycle behavior and arena/index alternatives.

No production syntax is approved yet. In particular, `SharedBorrow` and `SharedRef` must not be overloaded into shared ownership: both remain non-owning concepts.

The central transparency rule is that ordinary assignment or parameter passing must never silently introduce allocation, reference-count increments, atomic operations, locking, deep clone, GC or runtime ownership tables. Any future shared-ownership mechanism must make its ownership cost caller-visible and lower directly to the equivalent idiomatic Rust mechanism with no extra Evolution runtime layer.

Required final research verdict: `IMPLEMENT-CANDIDATE`, `SPLIT-RESEARCH`, `DEFER`, or `REJECT`.

## Implemented language / tooling state

`docs/LANGUAGE_SPEC_V0.md` is the implemented-language source of truth. The production core currently includes integer/bool/static strings, input/repeat/control flow, functions, lexical block locals, Records v0, Enums v0, by-value ownership/reinitialization, inferred call-duration shared-borrow parameters, first-class bounded immutable references, source-native move/reference diagnostics, source maps, formatter, native check/emit/build/run and verified run/build caches.

## Explicit current non-goals

No mutable references, generalized/user-written lifetime solver, implicit shared ownership, hidden clone/allocation, interior-mutability implementation, concurrency runtime design, cyclic/self-referential production structures, GC or silent changes to existing `T` / `&T` contracts are approved by the current ownership work.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed SHAs remain evidence and are not rerun merely for a better color.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
