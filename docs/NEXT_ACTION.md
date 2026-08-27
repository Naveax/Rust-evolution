# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-08-27**

## Active P0

Parent milestone: **#50 — Enums v0: nominal sum types + exhaustive static matching**

Completed semantic umbrella: **#54 — Enums semantics: nominal typing, constructors and exhaustive match**

Active child / PR:

- **#60 — Enums v0 ownership + match payload extraction**
- **PR #63 — validate Enums v0 ownership before codegen**
- PR branch: `feature/enums-ownership-v0`
- staging branch: `work/enums-ownership-v0`
- stable semantic base: #59 squash merge `e698b7f094863f017b3a29ad0210b638b6bd6a3f`
- post-merge main CI #236 / run `33113950293`: **SUCCESS**

Following children:

1. **#61 — static Rust enum/match codegen + source maps**
2. **#62 — differential performance parity + final language spec sync**

## Completed semantic baseline

The semantic layer on `main` already proves:

- nominal enum declarations and structured variant identity;
- static constructor arity/payload typing;
- statically typed enum match scrutinees;
- duplicate-free deterministic exhaustive coverage;
- typed lexical payload bindings with sibling isolation;
- structured resolved match sidecar with spans and exhaustive-only return-path summary;
- source-native semantic failures before rustc;
- fail-closed execution before enum ownership/codegen.

#57 and #54 are closed completed from post-merge CI #236.

## #60 Slice 1 — generic move-state infrastructure — PROVEN

Generic diagnostics-free `MoveState<T>` now owns availability mechanics:

- define / forget / inspect / consume;
- exact-type reinitialization;
- N-way continuing-exit merge;
- repeat later-iteration safety;
- read-only availability introspection for wrapper diagnostics.

Records `MoveTracker` is now a compatibility wrapper over this core while retaining Records v0 diagnostics, field access policy and partial-move rejection.

CI evidence:

- first infrastructure head `c918aa50241e3fb740c4f4408eeb94ee6b07540d`: CI #237 / run `33114685122` failed only `cargo fmt --check`; failed SHA was not rerun;
- corrected infrastructure head `1ad5336ce03134eae618518bacf74ff98d45e23f`: CI #238 / run `33114937178` — **SUCCESS**;
- Ubuntu #238 passed every existing runtime/performance gate including Records v0;
- Windows/macOS #238 passed format, Clippy, workspace tests, benchmark smoke and release build.

## #60 Slice 2 — enum pre-codegen ownership — IMPLEMENTED

Ownership validation runs after enum declaration/constructor/match typing and sidecar validation but before the existing fail-closed executable enum boundary.

Implemented conservative v0 rules:

- nominal enum values are move-only regardless of scalar/static payload;
- enum/record payload bindings are move-only; int/bool/string bindings remain reusable;
- by-value local reads, function arguments/returns and constructor payload uses consume move-only values;
- exact same-type reinitialization restores availability;
- owned exhaustive match consumes the whole scrutinee;
- every match arm starts from the same post-scrutinee-consumption state;
- only continuing `if` / `match` exits participate in ownership joins; terminal branches/arms are ignored;
- repeat rejects a move that would make a later iteration invalid;
- terminal repeat bodies preserve the conservative zero-iteration continuation state;
- moving a non-reusable nominal field out through field access is rejected instead of inserting an implicit clone or inventing partial-move semantics;
- no Rust enum/match codegen is enabled.

The match sidecar did **not** need per-arm terminal flags. The ownership traversal derives whether each arm continues while walking its body and cross-checks the all-terminal result against existing `all_arms_return` metadata.

## Current CI sequence

- ownership implementation head `888c30bf94edf4f7e0bc73f2ea0ecbcef537ffdf`: CI #239 / run `33116782824` — format and Clippy green on all three platforms; workspace reached **133/135** new lowering tests; the only failures were two incorrect expected diagnostic line numbers:
  - record payload reuse actual line 14, test expected 13;
  - `if` join reuse actual line 11, test expected 10.
  Production ownership diagnostics were correct. #239 was not rerun.
- corrected ownership + CLI regression head `7e10e901dc567c093fd5f83c5552d6838ee3ace2`: CI #240 / run `33117131359` — **single active run** at this handoff point.

The corrected head also adds real CLI `evo check` regressions proving before rustc:

- enum local reuse after move;
- whole-enum reuse after owned exhaustive match;
- record payload-binding reuse after move.

## Staging state

`work/enums-ownership-v0` may be ahead of PR #63 while #240 runs only by independent documentation / follow-up validation work. Do not move `feature/enums-ownership-v0` again until #240 completes.

## Resume here

1. Follow CI #240 / run `33117131359`. Do not rerun #239 and do not create a second Action for `7e10e901...`.
2. If #240 fails, inspect the actual failed job/log and fix only the real failure on a new staging SHA. Never rerun the stale failed SHA for a different outcome.
3. If #240 succeeds, record cross-platform quality and Ubuntu runtime/performance evidence for `7e10e901...`.
4. Review #60 acceptance against the proven corpus. Before finalizing, ensure the deliberately unsupported non-reusable nominal field/partial-move surface has explicit regression coverage, not only implementation code.
5. Update #60 checkboxes only from green evidence.
6. Synchronize `PROJECT_STATE.md`, this file and PR #63 body with the final ownership evidence.
7. Create one final docs-synchronized staging head if needed, confirm no Action exists for that SHA, then fast-forward PR #63 exactly once.
8. Require the exact final PR head to pass format, Clippy, workspace tests, CLI ownership regressions, all existing Ubuntu runtime/performance gates and release build.
9. Mark PR #63 ready only after that exact head is green.
10. Squash merge with expected-head protection.
11. Verify post-merge `main` CI before closing #60 or marking parent #50 ownership acceptance complete.
12. Start #61 only from the actual #63 squash-merge SHA, never from pre-merge staging ancestry.
13. Keep the dedicated Enums benchmark and final `LANGUAGE_SPEC_V0.md` sync in #62.

## Engineering constraints

Enum and variant identity must remain structured. Do not encode `Enum.Variant` as a concatenated magic name.

Do not modify executable Records `ValueType` / `SemanticType` merely to make enum ownership convenient. #60 remains a pre-codegen ownership pass until #61 deliberately promotes accepted semantics into executable IR.

Do not add hidden `.clone()`, boxing, GC/RC, runtime maps, reflection metadata, dynamic dispatch or inferred borrowing.

Do not change accepted Records v0 ownership diagnostics or move behavior as collateral damage from shared state machinery.

`LANGUAGE_SPEC_V0.md` remains intentionally unsynchronized with executable Enums behavior until #62 because ownership/codegen/performance acceptance is not complete.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent work, but never create multiple active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Do not rerun an old failed SHA merely to obtain a different result or timing sample.
