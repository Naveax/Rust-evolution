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

Completed parser surface:

- parser/formatter child #51 completed
- PR #53 squash-merged as `c454fcfe5811a9122b8dfeefad7f4eb4c22c8afa`
- post-merge main CI #216 / run `33091396504`: **SUCCESS**

Completed semantic umbrella #54:

1. PR #55 nominal declaration validation — post-merge main CI #221: **SUCCESS**
2. #56 / PR #58 resolved variants and constructor typing — squash merge `875b4d8fc255d6699b4f89b5e5c769d8dd34383b`; post-merge main CI #230: **SUCCESS**
3. #57 / PR #59 exhaustive match typing and arm scopes — squash merge `e698b7f094863f017b3a29ad0210b638b6bd6a3f`; post-merge main CI #236 / run `33113950293`: **SUCCESS**

## Stable semantic baseline on `main`

Current `main` baseline for ownership work:

`e698b7f094863f017b3a29ad0210b638b6bd6a3f`

It provides:

- source-spanned nominal enum schemas and structured variant identity;
- namespace and recursive by-value layout validation;
- static constructor arity/payload typing;
- statically known enum match scrutinees;
- duplicate-free deterministic exhaustive matching;
- typed lexical payload bindings with sibling isolation;
- a non-executable resolved match sidecar with structured identity, spans, typed bindings and exhaustive-only return summary;
- source-native failures before rustc;
- an explicit fail-closed runtime boundary before executable enum/match lowering.

## Active ownership child — #60 / PR #63

**#60 — Enums v0 ownership + match payload extraction**

PR: **#63 — validate Enums v0 ownership before codegen**

Branches:

- PR: `feature/enums-ownership-v0`
- staging: `work/enums-ownership-v0`

### Proven Slice 1 — shared move-state infrastructure

A diagnostics-free generic `MoveState<T>` now contains availability mechanics used by Records and Enums ownership analysis:

- define / forget / inspect / consume;
- exact-type reinitialization;
- N-way continuing-exit merge;
- repeat later-iteration safety;
- availability introspection used by wrapper diagnostics.

Records `MoveTracker` remains a compatibility wrapper and preserves accepted Records diagnostics, partial-move policy and runtime behavior.

Evidence:

- first infrastructure head `c918aa50241e3fb740c4f4408eeb94ee6b07540d`: CI #237 / run `33114685122` failed only `cargo fmt --check`; failed SHA was not rerun;
- corrected infrastructure head `1ad5336ce03134eae618518bacf74ff98d45e23f`: CI #238 / run `33114937178` — **SUCCESS**;
- Ubuntu #238 preserved every existing runtime/performance gate including Records v0; Windows/macOS preserved quality/test/release.

### Proven Slice 2 — enum pre-codegen ownership

The enum ownership validator runs after declaration, constructor, match typing and match-sidecar validation, but before the existing fail-closed executable enum boundary.

Accepted behavior on verified PR head `7e10e901dc567c093fd5f83c5552d6838ee3ace2`:

- nominal enum values are move-only regardless of payload;
- enum/record payload bindings are move-only while int/bool/string are reusable;
- by-value local reads, function arguments/returns and constructor payloads consume move-only values;
- exact same-type reinitialization restores availability;
- owned exhaustive match consumes the whole scrutinee;
- each match arm starts from the same post-scrutinee-consumption ownership state;
- only continuing `if` / `match` exits participate in joins; terminal branches/arms do not poison continuing state;
- repeat rejects moves that break later iterations;
- terminal repeat bodies preserve the conservative zero-iteration continuation state;
- moving a non-reusable nominal field out through field access is rejected instead of inventing partial-move or implicit-clone semantics;
- ownership remains pre-codegen; no executable enum IR or Rust enum/match emission is introduced.

The ownership traversal derives each arm's continuing/terminal result while walking the body. The match sidecar therefore did not require a new per-arm terminal flag; existing `all_arms_return` is retained as a cross-check for all-terminal matches.

### Ownership CI evidence

- implementation head `888c30bf94edf4f7e0bc73f2ea0ecbcef537ffdf`: CI #239 / run `33116782824` — format and Clippy green across all three platforms; workspace reached 133/135 lowering tests. The only failures were two incorrect expected diagnostic line numbers. Production diagnostics correctly pointed to lines 14 and 11; failed SHA was not rerun.
- corrected ownership + first CLI regression head `7e10e901dc567c093fd5f83c5552d6838ee3ace2`: CI #240 / run `33117131359` — **SUCCESS**.
- Ubuntu #240 passed format, Clippy, workspace tests, benchmark smoke, runtime repeat, control-flow, logical operators, Functions v0, Block Locals v0, Records v0 and release build.
- Windows/macOS #240 passed format, Clippy, workspace tests, benchmark smoke and release build.

CLI coverage on the verified #240 head proves source-native failure before rustc for enum reuse-after-move, whole-enum reuse after owned match and record payload-binding reuse.

### Final staging validation before merge

Staging is intentionally ahead of verified PR head #240 only by:

- real CLI regression for enum local consumption on `return`;
- real CLI regression rejecting move-out of an enum-valued record field with explicit no-implicit-clone wording;
- durable handoff documentation synchronized with #240.

This final staging head must receive its own CI after one fast-forward to PR #63. Do not merge from #240 alone because the additional regressions/docs are not yet verified on the PR head.

## Remaining Enums v0 queue

After #60 lands and post-merge `main` CI is green:

1. **#61 — static Rust enum/match codegen + source maps**
   - promote accepted semantic/ownership data into executable enum/match IR;
   - direct static Rust enum definitions, constructors and `match`;
   - generated-source mapping and native correctness corpus;
   - no hidden allocation/boxing/clone/dispatch.
2. **#62 — differential performance parity + final spec sync**
   - runtime-dependent Enums v0 differential benchmark against equivalent Rust;
   - #4/#5 parity evidence;
   - final `LANGUAGE_SPEC_V0.md` synchronization only after correctness/ownership/codegen/performance are accepted.

## Parent #50 state

Completed and merged on `main`:

- syntax/parser/formatter surface;
- nominal/type semantics;
- constructor semantics;
- exhaustive static match semantics.

Implemented and PR-verified, but not yet merged:

- enum by-value ownership and match payload extraction under #60 / PR #63.

Still open after #60:

- executable static Rust enum/match lowering and source maps (#61);
- native correctness corpus (#61);
- dedicated Enums performance parity evidence (#62);
- final stable language spec synchronization (#62).

## Deliberate current boundary

Do not add in #60:

- executable Rust enum/match codegen;
- generated enum source-map snapshots;
- dedicated Enums performance gate;
- final language spec sync;
- generics, guards, wildcard/or/nested arbitrary patterns, Option/Result sugar.

Enums v0 remains a **ZERO** cost-class target: eventually ordinary static Rust enums/matches, never hidden allocation, boxing, clone, dispatch or runtime metadata.

## Durable continuation infrastructure

Read order:

1. `AGENTS.md`
2. `docs/PROJECT_STATE.md`
3. `docs/NEXT_ACTION.md`
4. `docs/LANGUAGE_SPEC_V0.md`
5. active issue/PR/Actions referenced there

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.

`LANGUAGE_SPEC_V0.md` intentionally remains behind Enums experiments until #62 because executable codegen/performance acceptance is not complete.

## Handoff invariant

Every significant merge or incomplete stopping point must keep `PROJECT_STATE.md`, `NEXT_ACTION.md`, issue/PR evidence and durable decisions synchronized with GitHub reality.

The repository is the project memory. The chat transcript is not.
