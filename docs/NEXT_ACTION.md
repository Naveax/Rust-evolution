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

## Retained first benchmark failure

Initial feature head:

- `2c898fa6b845f12f78de99356441cf98afe0e23b`
- CI #275 / run `34107195001`: **FAILURE**

#275 must never be rerun. Its Ubuntu result remains real evidence:

- all existing runtime/performance gates before Enums v0: **PASS**
- native enum reinitialization process test: **PASS** on Ubuntu, Windows and macOS
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

Root cause: the handwritten generated-style Rust benchmark reference did not mirror actual emitter ordering/shape. Actual codegen emits `enum -> input helper -> function -> main`; the first reference used `enum -> function -> input helper -> main` and also differed in exact condition/arm formatting. The timing failure remains recorded rather than being rerun away.

## Corrected feature head / active CI

Feature branch currently points at:

- `69bc2d1b15db1bd841b85e8a508c156dc689550d`
- CI #276 / run `34108814832`: **ACTIVE**

The correction is intentionally benchmark/evidence-only:

1. Enums artifact upload uses `always()` so FAIL/INCONCLUSIVE reports are retained.
2. `benchmarks/cases/enums-v0/reference.rs` mirrors actual generated Rust ordering/shape.
3. `crates/evo-bench/tests/enums_reference.rs` requires reference Rust to equal generated Rust exactly after newline normalization.
4. Durable docs record #275 rather than hiding it.

No compiler/lowering/codegen semantics changed to improve timing.

Current #276 evidence:

- Windows job: **SUCCESS**
- macOS job: **SUCCESS**
- Ubuntu job: queued at the last verified check
- therefore the exact-reference lock has already passed workspace tests on Windows/macOS, but Enums performance is not yet accepted.

Staging may be ahead of the feature branch by documentation-only commits recording this active state. Do not fast-forward the feature branch while #276 is active.

## Resume sequence

1. Check CI #276 / run `34108814832` for exact SHA `69bc2d1b15db1bd841b85e8a508c156dc689550d`.
2. Do not rerun #275 or #276 manually.
3. When Ubuntu finishes, fetch artifact `evo-bench-enums-ubuntu-latest` even if the Enums gate fails or is inconclusive.
4. Inspect and retain at minimum:
   - `report.json`
   - `report.md`
   - `raw-samples.csv`
   - generated Rust / LLVM / binaries where present.
5. Record exact correctness, normalized LLVM equality, binary equality, binary sizes, raw samples, median/p95/MAD, normalized ratio, timing verdict, final verdict and verdict basis.
6. Accept performance only under #4/#5:
   - correctness must PASS first;
   - byte-identical binary parity is stronger deterministic runtime parity evidence when achieved against the exact-reference lock;
   - otherwise stable `T_evolution / T_reference <= 1.00` is required;
   - noisy measurement is INCONCLUSIVE, never PASS.
7. If #276 performance evidence is accepted, update `docs/LANGUAGE_SPEC_V0.md` on staging from proven implementation/tests only.
8. Update #62 / #50 / PR #65 / handoff docs with exact accepted evidence.
9. Fast-forward the final spec/docs-synchronized staging head to the feature branch only after #276 completes, then let it receive exactly one fresh CI run.
10. On final green CI, mark PR #65 ready, re-read exact head/mergeability, squash-merge with expected head SHA, and verify post-merge `main` push CI before closing #62 or #50.

## #62 scope still pending

- accepted corrected Enums differential performance evidence;
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
