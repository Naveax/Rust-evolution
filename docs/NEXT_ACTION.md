# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-07**

## Active P0

Parent milestone: **#50 — Enums v0: nominal sum types + exhaustive static matching**

Stable `main` baseline:

- #61 / PR #64 squash merge: `a7b8c08a71283cffa15216c7359078f1c8a34873`
- post-merge main CI #274 / run `34106421537`: **SUCCESS**
- Rust toolchain: **1.98.0**

Active final child:

- **#62 — Enums v0 differential performance parity + final language spec sync**
- PR: **#65 — `perf: add Enums v0 differential parity gate`**
- feature branch: `feature/enums-performance-v0`
- staging branch: `work/enums-performance-v0`

## Retained first benchmark failure

Initial feature head `2c898fa6b845f12f78de99356441cf98afe0e23b` ran as CI #275 / run `34107195001` and **FAILED** at the first Enums timing gate. Never rerun it.

Retained #275 Enums evidence:

- correctness: **PASS**
- normalized LLVM IR equal: `false`
- exact binary equal: `false`
- reference median: `16,535,571 ns`
- Evolution median: `16,547,263 ns`
- ratio: `1.000707082`
- stable: `true`
- final verdict: **FAIL**
- verdict basis: `timing-median-ratio`

Root cause was a benchmark-reference equivalence defect: the handwritten generated-style Rust reference did not exactly mirror emitter ordering/shape. No compiler/codegen semantics were changed to obtain a better result.

## Accepted corrected benchmark evidence

Corrected feature head:

- `69bc2d1b15db1bd841b85e8a508c156dc689550d`
- CI #276 / run `34108814832`: **SUCCESS**
- Ubuntu, macOS, Windows: **SUCCESS**
- Enums artifact: `evo-bench-enums-ubuntu-latest`, artifact id `10014284630`

The corrected head adds an exact reference/generated-Rust integration lock and retains Enums artifacts even on unfavorable verdicts.

Accepted #276 Enums evidence:

- exact benchmark reference == generated Rust after newline normalization: **PASS**
- differential stdout/stderr/exit correctness: **PASS**
- normalized LLVM IR equal: `true`
- exact executable equal: `true`
- reference binary: `2,267,072 B`
- Evolution binary: `2,267,072 B`
- reference median: `16,506,786 ns`
- Evolution median: `16,520,050 ns`
- reference p95: `16,596,414 ns`
- Evolution p95: `16,619,046 ns`
- relative MAD: `0.001764426` reference / `0.002294908` Evolution
- stable: `true`
- observed ratio: `1.000803548`
- timing-only verdict: **FAIL**
- final verdict: **PASS**
- verdict basis: `byte-identical-binary-parity`

Under #4/#5, correctness PASS plus byte-identical executable parity is stronger deterministic runtime parity evidence. Raw timing remains retained instead of hidden.

All previous Ubuntu runtime gates and the release build also passed on #276.

## Spec sync

Accepted performance evidence unlocked the final language-spec synchronization.

`docs/LANGUAGE_SPEC_V0.md` now documents proven Enums v0 behavior only:

- `enum` declaration grammar and declaration placement;
- unit/single-payload variants;
- qualified `Enum.Variant(...)` construction;
- nominal payload typing and by-value layout-cycle rejection;
- exhaustive statement-only `match` / `case`;
- arm-local payload binding scope and typing;
- enum move semantics and exact-type reinitialization;
- direct static Rust enum/constructor/match lowering;
- source-map/diagnostic policy;
- accepted #276 differential parity evidence;
- explicit non-goals such as generics, guards, wildcard/or/nested patterns, methods, references and runtime reflection.

## Resume sequence

1. Synchronize #62 / #50 / PR #65 / handoff docs with #276 evidence and the spec update.
2. Compare final staging against feature and confirm only intended #62 code/benchmark/spec/docs changes.
3. Because #276 is completed, fast-forward `feature/enums-performance-v0` to the final staging head with `force=false`.
4. Let that final docs-synchronized SHA receive exactly one new CI run. Do not manually rerun #275 or #276.
5. If final CI fails, fix the actual failure on a new SHA and let that SHA receive its own CI.
6. When final CI is green, update PR #65 validation evidence with exact final SHA/run.
7. Mark PR #65 ready for review.
8. Re-read PR head and mergeability, then squash-merge with `expected_head_sha` equal to the verified final head.
9. Fetch post-merge `main` push CI for the returned squash SHA and verify it to completion.
10. Only after post-merge main SUCCESS: close #62, complete/close parent #50, and update durable docs from merged-main evidence.

## Engineering constraints

- No hidden clone, allocation, boxing, GC/RC, runtime map, reflection metadata or dynamic dispatch.
- Evolution and reference Rust benchmark sides must perform the same work under the same compiler path.
- Do not weaken previous performance gates.
- Preserve unfavorable/noisy benchmark evidence rather than rerunning old SHAs.
- No generics, guards, wildcard/or/arbitrary nested patterns, borrow inference, methods or reflection as collateral work.

## CI rule

A running CI is work in progress, not a reason to create duplicate Actions. A failed SHA is evidence, not a slot machine lever.
