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

## Active research — #102 / PR #103 immutable reference surface v0

Issue #102 is the next Phase 3.3 ownership-ergonomics slice. It researches the first explicit immutable reference / borrowed-result surface and its bounded ownership model before production implementation.

PR #103: `research: classify immutable reference surface v0`  
Branch: `research/immutable-reference-surface-v0`

Accepted research code/evidence head before documentation synchronization:

`7664d28dc0865ef4442063ed702875302271087c`

The PR is research-only. It adds a dedicated workflow and research harness, not production syntax or semantics.

### Dedicated research evidence

Immutable reference surface research #3 / run `34598646494`: **SUCCESS** on Ubuntu 24.04 / Rust 1.98.0.

Artifact:

- `evo-immutable-reference-surface-research-ubuntu-24.04`;
- id `10263292508`;
- digest `sha256:59535a36276e7c5903d37c1b271128bf54de9e0af894d61ecb7052f52428d1f8`.

Normal exact-head CI #413 / run `34598646488`: **SUCCESS** on Ubuntu, Windows and macOS.

Observed result:

- aggregate verdict: **SURFACE-CANDIDATE**;
- recommended surface: **PUNCTUATION-AMPERSAND**;
- semantic cases: **19**;
- compile-expectation mismatches: **0**.

### Surface decision

The bounded candidate is explicit punctuation syntax:

```text
&T
&expr
```

Reasons supported by the locked comparison:

- `&` currently occupies invalid Evolution source space, so one new token does not reserve an existing identifier;
- keyword `ref` / `borrow` syntax would reserve two current ordinary identifiers;
- generic-like `Ref(...)` collides with current nominal/call-shaped identifier space;
- `&T` / `&expr` maps directly to safe Rust shared-reference syntax.

### Ownership/liveness result

The matrix establishes the required semantic boundary:

- existing owned `T -> T` remains owned;
- one deterministic owner source can back a first-class immutable reference result without user-written lifetime names;
- a reference can be stored in a local;
- owner move/reinitialization while a later reference use remains is rejected;
- owner move after the final reference use is valid;
- move-only nominal data cannot be moved through an immutable reference, while scalar reads are allowed;
- forwarding, recursion and branches are representable when every returned reference retains the same owner source;
- multiple possible owner sources fail closed;
- references derived from a dead local owner fail closed;
- the new first-class reference model can interoperate with the existing non-escaping `SharedBorrow` call boundary.

Durable report: `docs/IMMUTABLE_REFERENCE_SURFACE_RESEARCH.md`.

### Recommended production-successor representation

This is a design recommendation, not current implemented behavior.

Current parser `TypeName` and semantic `SemanticType` encode owned values only. The bounded implementation should add explicit immutable-reference variants instead of changing the meaning of existing owned nodes.

Recommended shape:

```text
syntax type:       SharedRef(TypeName)
syntax expression: SharedBorrow(Expr)
semantic type:     SharedRef(SemanticType)
provenance:        one deterministic owner/source identity for v0
```

Function signatures should expose `&T` explicitly. Local reference bindings may infer the produced reference type, but semantic IR/ownership analysis must retain reference-ness and owner provenance.

The existing inferred `SharedBorrow` parameter mode remains a separate call-duration mechanism. It must not silently become a stored/escaping reference.

For liveness, the production successor should attempt bounded local final-use analysis and conservatively keep a reference live where control flow cannot prove an earlier end. Safety wins over convenience when the bounded analysis is uncertain.

### Explicit non-goals

The #102 result does not approve:

- mutable references;
- generalized/user-written lifetime syntax;
- multi-owner lifetime relation solving;
- reference fields in records/enums;
- self-referential/cyclic reference structures;
- unsafe lifetime widening or invented `'static`;
- hidden clone/copy;
- boxing, RC/GC or runtime ownership maps.

### Failed-SHA evidence retained

Initial head `6f9f9f10c61f2b01ca44ac02a46106c8af5c604c` produced successful dedicated evidence, but CI #411 / run `34583343742` failed only rustfmt. It was not rerun.

Format-only head `733f7efe154422ac0e5c84ce500204b62cf84f51` produced immutable-reference research #2 / run `34583596906`: **SUCCESS**, but normal CI #412 / run `34583596924` failed Clippy on the research-only `write_reports` helper because it had 9 parameters under `-D warnings`.

Head `7664d28dc0865ef4442063ed702875302271087c` scopes a justified `clippy::too_many_arguments` allowance to that report writer only. It does not relax workspace lint policy or alter research semantics. Dedicated research #3 and normal CI #413 are both green.

## Completion gate still open

The research code/evidence head is accepted. PR #103 must not merge until this documentation-synchronized head receives natural **SUCCESS** from both normal CI and Immutable reference surface research, and its final artifact still reports the same surface/verdict/mismatch counts.

After merge, natural exact-SHA `main` CI and research push workflow must both succeed before #102 is closed completed.

## Required successor direction after #102 completion

Only after the post-merge gate should a production implementation issue be atomized from the accepted bounded design:

- lexer/parser support for `&` type and borrow expression;
- explicit syntax and semantic reference types;
- first-class immutable-reference local bindings;
- deterministic single-source provenance;
- source-native move/reinit/dead-owner/multi-owner diagnostics;
- bounded final-use liveness;
- direct safe Rust `&T` / `&expr` codegen;
- compatibility tests proving existing owned programs are unchanged;
- differential/runtime proof under the existing performance contract.

Generalized/mutable/multi-owner lifetime machinery remains a later research problem.

## Implemented language / tooling state

`docs/LANGUAGE_SPEC_V0.md` remains the implemented-language source of truth. Current accepted production core includes integer/bool/static strings, input/repeat/control flow, functions, lexical block locals, Records v0, Enums v0, by-value ownership/reinitialization, bounded inferred shared-borrow nominal parameters, source-native move diagnostics, source maps, formatter, native check/emit/build/run and verified run/build caches.

There is still **no production user-facing first-class reference syntax/value or returned/escaping reference feature** on the active research branch.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed SHAs remain evidence and are not rerun merely for a better color.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
