# Rust Evolution — Project State

Last verified update: **2026-09-11**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Current verified stable main predecessor for active #100: `355847a23faa29360e1da10bdfb2739eec6f8b6a`
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

Rust codegen emits ordinary `&T` parameters and `&expr` arguments. The feature creates no stored first-class reference value and no returned/escaping reference. No implicit clone/copy, boxing, RC/GC, runtime ownership map, invented lifetime or mutable borrow was added.

The permanent inferred-shared-borrow benchmark remains part of Ubuntu CI and is guarded by generated/reference Rust equality plus the established #4 performance policy.

## Active research — #100 / PR #101 lifetime elision feasibility v0

Issue #100 investigates the next Phase 3.3 ownership-ergonomics question: whether useful escaping borrowed results can avoid **named lifetime annotations** without silently changing current owned API contracts.

PR #101: `research: classify lifetime elision feasibility v0`  
Branch: `research/lifetime-elision-feasibility-v0`

Accepted research code/evidence head before documentation synchronization:

`f65db4a68a93240abe51d26444e70fc568a7ba02`

The PR changes research evidence only:

- `crates/evo-lowering/tests/lifetime_elision_research.rs`;
- `.github/workflows/lifetime-elision-research.yml`.

No production parser, lowering, ownership, codegen, language syntax, runtime or accepted-program semantics change.

### Dedicated research evidence

Lifetime elision research #2 / run `34580003076`: **SUCCESS** on Ubuntu 24.04 with Rust 1.98.0.

Artifact:

- `evo-lifetime-elision-research-ubuntu-24.04`;
- id `10191268492`;
- digest `sha256:40b6d338e7ed74aa7d158f32ac07a956cc2694892d3f945015ef99cf8dfad62f`.

Observed matrix:

- compile-expectation mismatches: **0**;
- `ELISION-CANDIDATE-REQUIRES-REFERENCE-TYPE`: **8** cases;
- `REQUIRES-EXPLICIT-LIFETIME-RELATION`: **2** cases;
- `UNSAFE-OR-UNREPRESENTABLE`: **1** case;
- aggregate verdict: **REFERENCE-SURFACE-FIRST**.

### Meaning of REFERENCE-SURFACE-FIRST

Rust can elide lifetime names for useful single-source borrowed-return signatures, including whole-value, nested-field, forwarding and recursive single-source shapes.

That does not make a returned reference an owned value:

- a stored borrowed result keeps an ownership relation to its source owner;
- owner move and owner reinitialization while that result is live are rejected;
- two possible borrowed input owners do not receive a unique returned lifetime from ordinary Rust elision;
- a returned reference derived from a temporary/local owner is invalid;
- current `T -> T` owned return semantics cannot silently become `&T`.

Therefore production escaping borrows require a **caller-visible immutable reference / borrowed-result type distinction first**. Named lifetime syntax can remain absent for the proven single-source subset.

Durable report: `docs/LIFETIME_ELISION_RESEARCH.md`.

### Failed-SHA evidence retained

Initial research head:

`6203569451abc1ea448a36eaa47e21fddbe0eed7`

- Lifetime elision research #1 / run `34579682344`: **SUCCESS** with the same decision and zero expectation mismatches;
- artifact id `10191069517`, digest `sha256:d97af7e415cfe0d27ce3515531a2aa54dae4feba1f5c685796ed4689d2be0c56`;
- normal CI #407 / run `34579682411` failed only rustfmt on the research test;
- the SHA was not rerun.

Formatting-only head `f65db4a68a93240abe51d26444e70fc568a7ba02` preserves the matrix and decision. Dedicated research #2 is green. Normal CI #408 / run `34580003002` is the exact-head three-OS gate that must be green before documentation synchronization is attached.

## Required successor direction after #100 completion

Do not implement escaping borrowed returns directly from this research result.

First atomize an immutable reference surface v0 research/design slice covering:

- explicit caller-visible owned `T` versus borrowed `&T`-equivalent distinction;
- immutable borrowed values stored in locals;
- source-native owner move/reinitialization conflict diagnostics;
- single-source lifetime-name elision;
- compatibility with current owned return contracts;
- fail-closed behavior for multiple-source lifetime relationships.

Mutable references, generalized lifetime syntax/inference, invented `'static`, hidden clone/copy, boxing, RC/GC, runtime ownership maps and unsafe lifetime widening remain outside that first slice.

## Implemented language / tooling state

`docs/LANGUAGE_SPEC_V0.md` is the implemented-language source of truth. Current accepted core includes integer/bool/static strings, input/repeat/control flow, functions, lexical block locals, Records v0, Enums v0, by-value ownership/reinitialization, bounded inferred shared-borrow nominal parameters, source-native move diagnostics, source maps, formatter, native check/emit/build/run and verified run/build caches.

There is still **no production user-facing first-class reference syntax/value or returned/escaping reference feature**.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed SHAs remain evidence and are not rerun merely for a better color.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
