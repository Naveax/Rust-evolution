# Rust Evolution — Project State

Last verified update: **2026-08-27**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Records v0 final feature baseline: `ce3018d158d2ce4084a9e569b8eebac6eeb51f8f`
- Records post-merge CI #197 / run `33074128274`: **SUCCESS**

Records v0 remains the accepted ZERO-cost nominal product-type baseline: static Rust structs/field access, by-value move tracking, no hidden allocation/boxing/GC/RC/clone/dynamic dispatch/runtime metadata, with its differential performance gate preserved.

## Enums v0 milestone — #50

Parent: **#50 — Enums v0: nominal sum types + exhaustive static matching**

Completed parser/semantic work:

- parser/formatter child #51; post-merge main CI #216: **SUCCESS**
- PR #55 nominal declarations; post-merge main CI #221: **SUCCESS**
- #56 / PR #58 constructor typing; squash merge `875b4d8fc255d6699b4f89b5e5c769d8dd34383b`; post-merge main CI #230: **SUCCESS**
- #57 / PR #59 exhaustive match typing; squash merge `e698b7f094863f017b3a29ad0210b638b6bd6a3f`; post-merge main CI #236 / run `33113950293`: **SUCCESS**

Semantic umbrella #54 is closed completed.

## Stable semantic baseline on `main`

Current `main` baseline for ownership work:

`e698b7f094863f017b3a29ad0210b638b6bd6a3f`

It provides nominal enum schemas, structured variant identity, constructor typing, exhaustive match typing, lexical payload bindings, resolved match sidecar metadata and a fail-closed boundary before executable enum lowering.

## Active ownership child — #60 / PR #63

**#60 — Enums v0 ownership + match payload extraction**

PR: **#63 — validate Enums v0 ownership before codegen**

Branches:

- PR: `feature/enums-ownership-v0`
- staging: `work/enums-ownership-v0`

### Proven shared move-state infrastructure

A diagnostics-free generic `MoveState<T>` contains availability mechanics shared by Records and Enums ownership analysis:

- define / forget / inspect / consume;
- exact-type reinitialization;
- N-way continuing-exit merge;
- repeat later-iteration safety;
- availability introspection for wrapper diagnostics.

Records `MoveTracker` remains a compatibility wrapper and preserves accepted Records diagnostics, partial-move policy and runtime behavior.

Evidence:

- `c918aa50241e3fb740c4f4408eeb94ee6b07540d`: CI #237 / run `33114685122` failed only `cargo fmt --check`; failed SHA was not rerun;
- corrected `1ad5336ce03134eae618518bacf74ff98d45e23f`: CI #238 / run `33114937178` — **SUCCESS**.

### Proven enum pre-codegen ownership

Final behavior/regression head:

`d3e9ac042e71cca21b2aa84995fd9d9ba10f6d4d`

CI #241 / run `33117447546`: **SUCCESS**.

Accepted behavior:

- nominal enum values are move-only regardless of scalar/static payload;
- enum/record payload bindings are move-only while int/bool/string remain reusable;
- by-value local reads, function arguments/returns and constructor payloads consume move-only values;
- exact same-type reinitialization restores availability;
- owned exhaustive match consumes the whole scrutinee;
- every match arm begins from the same post-scrutinee-consumption state;
- continuing `if` / `match` states merge conservatively and terminal branches/arms do not poison continuation;
- repeat rejects later-iteration-invalidating moves;
- terminal repeat bodies preserve the conservative zero-iteration continuation state;
- non-reusable nominal record-field move-out is explicitly rejected without implicit clone or guessed partial-move semantics;
- ownership stays pre-codegen; enum execution remains fail-closed before #61.

The ownership traversal computes per-arm continuation while walking arm bodies, so the semantic sidecar did not require additional per-arm terminal flags. Existing `all_arms_return` remains an invariant cross-check.

### Ownership CI evidence

- `888c30bf94edf4f7e0bc73f2ea0ecbcef537ffdf`: CI #239 / run `33116782824` — fmt/Clippy green; 133/135 lowering tests passed. The two failures were incorrect expected span lines only. Production diagnostics correctly returned lines 14 and 11; failed SHA was not rerun.
- corrected `7e10e901dc567c093fd5f83c5552d6838ee3ace2`: CI #240 / run `33117131359` — **SUCCESS**.
- final behavior/regression head `d3e9ac042e71cca21b2aa84995fd9d9ba10f6d4d`: CI #241 / run `33117447546` — **SUCCESS**.
- Ubuntu #241 passed format, Clippy, workspace tests, benchmark smoke, runtime repeat, control-flow, logical operators, Functions v0, Block Locals v0, Records v0 and release build.
- Windows/macOS #241 passed format, Clippy, workspace tests, benchmark smoke and release build.

Source-native CLI evidence on #241 covers enum reuse-after-move, whole-enum reuse after match, record payload-binding reuse, by-value return consumption and explicit nominal-field partial-move rejection before rustc.

### Final docs synchronization

The staging branch is newer than green behavior head `d3e9ac04...` only because `PROJECT_STATE.md` and `NEXT_ACTION.md` now record CI #241. No production or test code changes occur after #241.

This docs-synchronized head must receive one final PR CI before merge. After that exact head is green, PR #63 may be marked ready and squash-merged with expected-head protection. #60 stays open until post-merge `main` CI succeeds.

## Next child — #61 codegen/source maps

Start #61 only from the actual PR #63 squash-merge SHA after post-merge main validation.

First #61 architectural slice should promote the accepted semantic + ownership representation into executable structured IR **before** Rust emission:

- add enum schemas with spans to lowered `Program`;
- retain structured enum/variant identity for constructors and matches;
- represent payload-binding ownership decisions explicitly rather than reconstructing them in codegen;
- distinguish record nominal types from enum nominal types in IR. Existing bare `RecordType::Named(String)` must not cause codegen to guess a generated type prefix from only a name;
- preserve the existing scalar/function/Records executable path while enum IR is introduced atomically.

Only after executable IR is proven should #61 add direct static Rust enum/constructor/match emission and source mappings.

## Remaining Enums v0 queue

1. **#61 — static Rust enum/match codegen + source maps**
2. **#62 — differential performance parity + final spec sync**

#62 owns the runtime-dependent Enums differential benchmark, #4/#5 performance evidence and final `LANGUAGE_SPEC_V0.md` synchronization.

## Parent #50 state

Completed and merged on `main`:

- syntax/parser/formatter surface;
- nominal/type semantics;
- constructor semantics;
- exhaustive static match semantics.

Implemented and fully PR-validated but not yet merged:

- enum by-value ownership and match payload extraction under #60 / PR #63.

Still open after #60:

- executable static Rust enum/match lowering and source maps (#61);
- native correctness corpus (#61);
- dedicated Enums performance parity evidence (#62);
- final stable language spec synchronization (#62).

## Deliberate boundary

Enums v0 remains a **ZERO** cost-class target: ordinary static Rust enums/matches, never hidden allocation, boxing, clone, dispatch or runtime metadata.

`LANGUAGE_SPEC_V0.md` intentionally remains behind Enums experiments until #62 because executable codegen/performance acceptance is not complete.

## Durable continuation infrastructure

Read order:

1. `AGENTS.md`
2. `docs/PROJECT_STATE.md`
3. `docs/NEXT_ACTION.md`
4. `docs/LANGUAGE_SPEC_V0.md`
5. active issue/PR/Actions referenced there

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.

## Handoff invariant

Every significant merge or incomplete stopping point must keep `PROJECT_STATE.md`, `NEXT_ACTION.md`, issue/PR evidence and durable decisions synchronized with GitHub reality.

The repository is the project memory. The chat transcript is not.
