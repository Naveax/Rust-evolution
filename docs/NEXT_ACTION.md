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

Stable semantic behavior from #55 includes:

- deterministic record/enum/function namespace policy;
- duplicate enum and per-enum duplicate variant diagnostics;
- builtin, record and enum payload-reference validation;
- mixed record/enum nominal-reference validation;
- cross-record/enum by-value layout-cycle rejection without hidden boxing;
- valid enum execution still fails closed before unsupported ownership/codegen.

## Active #56 implementation

Resolved constructor semantics now under PR #58 include:

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
- current PR head before this docs-only staging work: `e8f84fb2c2c193f4d30c8b513653edf53bdee604`
- CI #227 / run `33099872021` is the active validation for that code head. Do not duplicate it.

## Resume here

1. Inspect CI #227 / run `33099872021`. Never rerun it while active and never create another Action for `e8f84fb2...` / CI.
2. If #227 fails, fix the actual root cause on `work/enums-constructor-typing-v0`, then fast-forward PR #58 to the new SHA. Do not rerun the failed SHA.
3. If #227 succeeds, review the current staging head because it may contain handoff/docs commits newer than the PR branch. Fast-forward only after the previous run completes.
4. Require a green CI for the final docs-synchronized PR #58 head before merge.
5. Before merging #58, verify #56 acceptance: exact constructor identity, arity, payload typing through locals/functions/records, source-native CLI failures, no rustc reach-through, and no ownership/codegen behavior added.
6. Merge #58 only with expected-head protection, then verify post-merge `main` CI.
7. Close/update #56 only after merged-main evidence exists.
8. Start #57 from the actual #58 squash-merge commit, not from pre-merge staging ancestry.
9. #57 must add enum-typed match scrutinees, arm enum/variant membership, duplicate-arm rejection, deterministic exhaustiveness, typed arm-local payload bindings and sibling-scope isolation.
10. Keep enum ownership/move policy, payload extraction partial moves, Rust enum/match codegen and Enums performance work outside #57 unless a minimal dependency is proven.
11. Preserve every Records v0 and earlier quality/runtime/performance gate.

## Engineering constraints

Enum and variant identity must remain structured. Do not encode `Enum.Variant` as a concatenated magic name.

Do not infer arbitrary method-call semantics from `object.member(...)`.

Do not modify `SemanticType` / `MoveTracker` merely to make constructor typing convenient. #56 deliberately uses an ownership-free semantic view so ownership remains a later explicit decision.

Match pattern bindings are not constructor-typing locals. Their semantic type/scope belongs to #57.

Enums v0 remains cost class **ZERO**: no implicit cloning, hidden boxing, GC/RC, runtime variant maps, reflection metadata, dynamic dispatch or managed-runtime machinery.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent work, but never create multiple active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Do not rerun an old failed SHA merely to obtain a different result or timing sample.
