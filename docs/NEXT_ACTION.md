# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-08-27**

## Active P0

Parent milestone: **#50 — Enums v0: nominal sum types + exhaustive static matching**

Semantic umbrella: **#54 — Enums semantics: nominal typing, constructors and exhaustive match**

Completed first semantic slice:

- **PR #55 — validate Enums v0 nominal declarations**
- squash merge: `ca9641d6c7c57ab603cda8b6a4a091f50cfd625d`
- final PR CI #220 / run `33097848047`: **SUCCESS**
- post-merge main CI #221 / run `33098011627`: **SUCCESS**

Active child / PR:

- **#56 — resolved variants and constructor typing**
- **PR #58 — type-check Enums v0 variant constructors**
- branch `feature/enums-constructor-typing-v0`
- staging branch `work/enums-constructor-typing-v0`

Following child:

- **#57 — exhaustive match typing and arm scopes**

## Verified stable baseline

Stable `main` baseline for #56:

`ca9641d6c7c57ab603cda8b6a4a091f50cfd625d`

Post-merge main CI #221 is authoritative for the #55 nominal-declaration slice. Ubuntu passed every existing runtime/performance gate including Records v0; Windows/macOS passed the quality/test/release matrix.

## Active #56 implementation

Resolved constructor semantics under PR #58 include:

- resolved enum schemas retaining enum name, variant name, optional payload type and source spans;
- payload types distinguished nominally as scalar / record / enum for semantic validation;
- exact `Enum.Variant(...)` enum + variant lookup;
- unit variant requires zero arguments;
- payload variant requires exactly one argument;
- source-native unknown enum / unknown variant / wrong arity diagnostics;
- literal, operator and nested-enum payload type checks;
- ownership-free constructor type propagation through local bindings, function parameters/returns, named record constructors and record field access;
- real CLI regressions for unknown variant and wrong payload type before rustc;
- match payload bindings deliberately remain outside #56 and belong to #57;
- valid enum execution remains fail-closed before ownership/Rust codegen.

## CI evidence for PR #58

- `1791baac...` resolved-schema head: CI #222 / run `33098431733` — **SUCCESS**
- `9335fdc0...` identity/arity head: CI #223 / run `33098652712` — **SUCCESS**
- `702fa089...` first payload-typing head: CI #224 / run `33099006049` — failed only because one new test expected line 12 while the correct Evolution payload span was line 13; fmt/Clippy passed
- `6c0342ec...` corrected payload-span head: CI #225 / run `33099288196` — **SUCCESS**
- `4c4f1d2f...` first full payload-propagation head: CI #226 / run `33099643034` — Clippy exposed an unused validation wrapper and `&mut Vec`/slice API issue; both were fixed on a new SHA, never rerun
- `e8f84fb2...` corrected full payload-propagation head: CI #227 / run `33099872021` — format, Clippy, CLI regressions and 89/90 lowering tests passed; failure was one incorrect test expectation (`record_field_type_flows_into_enum_payload_check` expected line 8 while the correct payload span is line 9); the failed SHA was not rerun
- corrected docs-synchronized head `2a6e7dbbe47e52c0c02499da5de5d1a7a40610cb`: CI #228 / run `33103658784` — **SUCCESS**
- Ubuntu #228 passed format, Clippy, workspace tests, benchmark smoke, runtime repeat, control-flow, logical operators, Functions v0, Block Locals v0, Records v0 and release build
- Windows/macOS #228 passed format, Clippy, workspace tests, benchmark smoke and release build

The current staging head is newer only because it records #228 in durable docs. That docs-only head requires one final CI after a single fast-forward to PR #58.

## Resume here

1. Inspect `work/enums-constructor-typing-v0` and PR #58. The staging head should be ahead only by docs recording CI #228.
2. Confirm no active Action exists for the staging target SHA, then fast-forward `feature/enums-constructor-typing-v0` exactly once.
3. Follow the single final docs-synchronized CI. Do not rerun #227/#228 and do not create duplicate Actions.
4. Require green format, Clippy, workspace tests, CLI constructor regressions, existing runtime/performance gates and release build.
5. Mark PR #58 ready and merge only with expected-head protection after that final CI succeeds.
6. Verify the post-merge `main` CI before closing/updating #56.
7. Update #56 checklist only from merged-main evidence, then close it completed if all acceptance items remain satisfied.
8. Start #57 from the actual #58 squash-merge commit, not from pre-merge staging ancestry.
9. First #57 slice should reuse resolved enum schemas to validate structured match arm enum/variant membership and deterministic exhaustiveness while keeping execution fail-closed.
10. Then add enum-typed scrutinee propagation and arm-local typed payload bindings with sibling-scope isolation.
11. Keep enum ownership/move policy, payload extraction partial moves, Rust enum/match codegen and Enums performance work outside #57 unless a minimal dependency is proven.
12. Preserve every Records v0 and earlier quality/runtime/performance gate.

## Engineering constraints

Enum and variant identity must remain structured. Do not encode `Enum.Variant` as a concatenated magic name.

Do not infer arbitrary method-call semantics from `object.member(...)`.

Do not modify `SemanticType` / `MoveTracker` merely to make constructor typing convenient. #56 deliberately uses an ownership-free semantic view so ownership remains a later explicit decision.

Match pattern bindings are not constructor-typing locals. Their semantic type/scope belongs to #57.

Enums v0 remains cost class **ZERO**: no implicit cloning, hidden boxing, GC/RC, runtime variant maps, reflection metadata, dynamic dispatch or managed-runtime machinery.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent work, but never create multiple active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Do not rerun an old failed SHA merely to obtain a different result or timing sample.
