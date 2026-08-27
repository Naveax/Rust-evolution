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

## #60 Slice 1 — generic move-state infrastructure — PROVEN

Generic diagnostics-free `MoveState<T>` owns availability mechanics:

- define / forget / inspect / consume;
- exact-type reinitialization;
- N-way continuing-exit merge;
- repeat later-iteration safety;
- availability introspection for wrapper diagnostics.

Records `MoveTracker` is a compatibility wrapper over the shared state core and preserves accepted Records v0 diagnostics and behavior.

CI evidence:

- `c918aa50241e3fb740c4f4408eeb94ee6b07540d`: CI #237 / run `33114685122` failed only `cargo fmt --check`; failed SHA was not rerun;
- corrected `1ad5336ce03134eae618518bacf74ff98d45e23f`: CI #238 / run `33114937178` — **SUCCESS**;
- Ubuntu #238 preserved every existing runtime/performance gate including Records v0.

## #60 Slice 2 — enum pre-codegen ownership — PROVEN ON PR HEAD

Verified PR head:

`7e10e901dc567c093fd5f83c5552d6838ee3ace2`

CI #240 / run `33117131359`: **SUCCESS**.

Ubuntu #240 passed format, Clippy, workspace tests, benchmark smoke, runtime repeat, control-flow, logical operators, Functions v0, Block Locals v0, Records v0 and release build. Windows/macOS passed format, Clippy, workspace tests, benchmark smoke and release build.

Implemented conservative ownership rules:

- nominal enum values are move-only regardless of payload;
- enum/record payload bindings are move-only; int/bool/string bindings remain reusable;
- by-value local reads, function arguments/returns and constructor payload uses consume move-only values;
- exact same-type reinitialization restores availability;
- owned exhaustive match consumes the whole scrutinee;
- every match arm starts from the same post-scrutinee-consumption state;
- only continuing `if` / `match` exits participate in ownership joins; terminal branches/arms are ignored;
- repeat rejects moves that would break a later iteration;
- terminal repeat bodies preserve the conservative zero-iteration continuation state;
- moving a non-reusable nominal field out through field access is rejected rather than inventing partial-move or implicit-clone behavior;
- ownership validation remains before the existing fail-closed executable enum/codegen boundary.

The ownership traversal derives each arm's continuing/terminal result directly while walking the body. No extra per-arm terminal flag was needed in the semantic sidecar; existing `all_arms_return` remains an invariant cross-check.

## Ownership CI history

- implementation head `888c30bf94edf4f7e0bc73f2ea0ecbcef537ffdf`: CI #239 / run `33116782824` — fmt and Clippy green on all three platforms; workspace reached 133/135 lowering tests. The two failures were incorrect expected span lines only: production diagnostics correctly returned line 14 instead of expected 13 and line 11 instead of expected 10. Failed SHA was not rerun.
- corrected head `7e10e901dc567c093fd5f83c5552d6838ee3ace2`: CI #240 / run `33117131359` — **SUCCESS**.

CLI regressions on the #240 head prove before rustc:

- enum local reuse after move;
- whole-enum reuse after owned exhaustive match;
- record payload-binding reuse after move.

## Final staging delta

`work/enums-ownership-v0` is intentionally newer than the green #240 PR head only by:

- CLI regression proving an enum local already moved cannot later be returned by value;
- CLI regression proving move-out of an enum-valued record field is explicitly rejected with no implicit clone;
- durable `NEXT_ACTION.md` / `PROJECT_STATE.md` synchronization recording #240.

There is no production ownership behavior change after the green #240 head.

## Resume here

1. Compare the current staging head against green PR head `7e10e901...`; expected differences are only the two additional CLI ownership regressions and durable docs.
2. Confirm the exact staging target SHA has no active Action.
3. Fast-forward `feature/enums-ownership-v0` exactly once to the staging head.
4. Follow the single final docs/regression CI. Do not rerun #239/#240 and do not create duplicate Actions.
5. Require the exact final head to pass format, Clippy, workspace tests, all enum ownership CLI regressions, every existing Ubuntu runtime/performance gate and release build.
6. Once final CI is green, update #60 checkboxes from evidence and synchronize PR #63 body with that final run.
7. Mark PR #63 ready only after the exact final head is green.
8. Squash merge PR #63 with expected-head protection.
9. Verify the post-merge `main` CI before closing #60 or marking parent #50 ownership acceptance complete.
10. Start #61 only from the actual #63 squash-merge SHA, not from pre-merge staging ancestry.
11. Keep executable static Rust enum/match IR, source maps and native correctness in #61.
12. Keep dedicated Enums performance evidence and final `LANGUAGE_SPEC_V0.md` sync in #62.

## Engineering constraints

Enum and variant identity must remain structured. Do not encode `Enum.Variant` as a concatenated magic name.

Do not modify executable Records `ValueType` / `SemanticType` merely to make enum ownership convenient. #60 remains pre-codegen until #61 deliberately promotes accepted semantics into executable IR.

Do not add hidden `.clone()`, boxing, GC/RC, runtime maps, reflection metadata, dynamic dispatch or inferred borrowing.

Do not change accepted Records v0 ownership diagnostics or move behavior as collateral damage from shared state machinery.

`LANGUAGE_SPEC_V0.md` remains intentionally unsynchronized with executable Enums behavior until #62 because codegen/performance acceptance is not complete.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent work, but never create multiple active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Do not rerun an old failed SHA merely to obtain a different result or timing sample.
