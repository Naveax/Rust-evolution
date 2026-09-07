# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-07**

## Active P0

Parent milestone: **#50 — Enums v0: nominal sum types + exhaustive static matching**

Stable `main` baseline:

- #61 / PR #64 squash merge: `a7b8c08a71283cffa15216c7359078f1c8a34873`
- post-merge main CI #274 / run `34106421537`: **SUCCESS**
- Rust toolchain: **1.98.0**

Active child:

- **#62 — Enums v0 differential performance parity + final language spec sync**
- PR: **#65 — `perf: add Enums v0 differential parity gate`**
- feature branch: `feature/enums-performance-v0`
- staging branch: `work/enums-performance-v0`

## #62 first benchmark evidence

First feature head:

- `2c898fa6b845f12f78de99356441cf98afe0e23b`
- CI #275 / run `34107195001`: **FAILURE**

#275 must never be rerun. Its Ubuntu result is retained as real evidence:

- all existing runtime/performance gates before Enums v0: **PASS**
- new native enum reinitialization process test: **PASS** on Ubuntu, Windows and macOS
- Enums differential correctness: **PASS**
- normalized LLVM IR equality: `false`
- exact binary equality: `false`
- reference median: `16,535,571 ns`
- Evolution median: `16,547,263 ns`
- observed ratio: `1.000707082`
- stable measurement: `true`
- timing verdict: **FAIL**
- final verdict: **FAIL**
- verdict basis: `timing-median-ratio`

The failure exposed a benchmark-reference defect rather than accepted parity: the handwritten generated-style Rust reference did not mirror actual emitter ordering/shape. Actual codegen emits `enum -> input helper -> function -> main`; the first reference used `enum -> function -> input helper -> main` and also differed in exact condition/arm formatting. This made LLVM/binary comparison non-identical before timing.

## Current staging correction

Code/evidence correction head:

- `9da2a360948edf9f474428260bc9bedfed45566c`

Changes since failed #275 head are intentionally limited to:

1. Enums artifact upload uses `always()` so FAIL/INCONCLUSIVE reports are retained.
2. `benchmarks/cases/enums-v0/reference.rs` now mirrors actual generated Rust ordering/shape.
3. `crates/evo-bench/tests/enums_reference.rs` requires the Enums benchmark reference to match generated Rust exactly, analogous to the existing function-call reference lock.

Staging may be ahead of `9da2a360...` only by durable documentation updates recording #275 and this correction. No compiler/codegen semantics were changed to obtain a more favorable benchmark result.

## Resume sequence

1. Confirm #275 is completed and do not rerun it.
2. Fast-forward `feature/enums-performance-v0` to the current staging head with `force=false`.
3. Let that new SHA receive exactly one CI run.
4. If the exact-reference integration test fails, fix that mismatch on a new SHA; do not reinterpret timing evidence.
5. If the Enums gate runs, always retain and inspect `evo-bench-enums-ubuntu-latest` artifact:
   - `report.json`
   - `report.md`
   - `raw-samples.csv`
   - generated Rust / LLVM / binaries where present.
6. Accept performance only from the reported #4/#5 contract:
   - correctness must PASS first;
   - byte-identical binary parity is stronger deterministic parity evidence when achieved;
   - otherwise stable `T_evolution / T_reference <= 1.00` is required;
   - noisy measurement is INCONCLUSIVE, never PASS.
7. Only after performance evidence is accepted, update `docs/LANGUAGE_SPEC_V0.md` for Enums v0.
8. Final docs-synchronized head must receive its own CI before PR #65 is made ready/merged.
9. Verify post-merge `main` CI before closing #62 or parent #50.

## #62 scope still pending

- accepted Enums differential performance evidence;
- generated Rust / LLVM / binary evidence recorded from the accepted run;
- final `LANGUAGE_SPEC_V0.md` synchronization;
- final #50 checklist/documentation sync;
- PR #65 merge + post-merge main verification.

## Engineering constraints

- No hidden clone, allocation, boxing, GC/RC, runtime map, reflection metadata or dynamic dispatch.
- Evolution and reference Rust benchmark sides must perform the same work with the same enum layout, payload types, input/output and release compiler path.
- Do not weaken or remove previous performance gates to make Enums pass.
- Preserve unfavorable/noisy benchmark evidence rather than rerunning old SHAs.
- `LANGUAGE_SPEC_V0.md` changes only after behavior and performance are proven.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent staging/docs work, but never create duplicate active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Never rerun an old failed SHA merely to obtain another result.
