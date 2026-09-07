# Rust Evolution — Project State

Last verified update: **2026-09-07**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Current authoritative `main`: `a7b8c08a71283cffa15216c7359078f1c8a34873`
- Post-merge main CI #274 / run `34106421537`: **SUCCESS**

Records v0 remains the accepted ZERO-cost nominal product-type baseline with its differential performance gate preserved.

## Enums v0 milestone — #50

Completed and merged before #62:

- parser/formatter child #51;
- semantic umbrella #54;
- ownership child #60 / PR #63;
- static executable codegen child #61 / PR #64:
  - final PR head `8d45ea094db44e777a5c6f8c0876ecde45320b4f`;
  - final PR CI #273 / run `34106104357`: **SUCCESS**;
  - squash merge `a7b8c08a71283cffa15216c7359078f1c8a34873`;
  - post-merge main CI #274 / run `34106421537`: **SUCCESS**.

Merged Enums v0 now has nominal declarations, qualified constructors, exhaustive statement-only matches, typed lexical payload bindings, explicit by-value ownership, static Rust enum/match codegen, source maps and native correctness coverage without hidden clone/allocation/boxing/GC/RC/runtime maps/reflection/dynamic dispatch.

## Active final child — #62 differential performance + final spec sync

- Issue: **#62 — P0 enums performance: differential parity gate and language spec sync**
- PR: **#65 — `perf: add Enums v0 differential parity gate`**
- Feature branch: `feature/enums-performance-v0`
- Staging branch: `work/enums-performance-v0`
- Exact branch base: `a7b8c08a71283cffa15216c7359078f1c8a34873`

## First benchmark run — retained failure

First feature head:

- `2c898fa6b845f12f78de99356441cf98afe0e23b`
- CI #275 / run `34107195001`: **FAILURE**

The failure is retained and must not be rerun.

All existing Ubuntu runtime/performance gates before the new Enums gate remained green. Ubuntu/Windows/macOS fmt, Clippy and workspace tests were green, and the new native same-type enum reinitialization process test passed.

The first Enums differential result was:

- correctness: **PASS**
- normalized LLVM IR equal: `false`
- exact binary equal: `false`
- reference median: `16,535,571 ns`
- Evolution median: `16,547,263 ns`
- observed ratio: `1.000707082`
- stable: `true`
- timing verdict: **FAIL**
- final verdict: **FAIL**
- verdict basis: `timing-median-ratio`

This run exposed a benchmark reference-equivalence defect. The handwritten generated-style Rust reference did not mirror the actual emitter ordering/shape: codegen emits `enum -> input helper -> function -> main`, while the initial reference used `enum -> function -> input helper -> main`; exact condition/arm formatting also differed. Therefore LLVM and binary identity were not a valid deterministic parity check on #275.

The timing failure remains recorded; it is not overwritten by another run of the same SHA.

## Current staging correction

Current staging contains three targeted corrections beyond the #275 feature head:

1. `Upload Enums v0 report` uses `always()` so unfavorable or inconclusive benchmark artifacts are retained.
2. `benchmarks/cases/enums-v0/reference.rs` mirrors actual generated Rust ordering/shape.
3. `crates/evo-bench/tests/enums_reference.rs` asserts that the Enums benchmark reference and generated Rust are exactly equal after newline normalization, analogous to the existing function-call benchmark lock.

No compiler or codegen semantics were changed merely to improve timing.

## Benchmark design

`benchmarks/cases/enums-v0` remains runtime-dependent and exercises the feature in the hot path:

- stdin: `n = 20,000,000`, initial `x = 9`;
- expected stdout: `15099959897`;
- two scalar-payload `Step` variants;
- by-value enum return from `classify`;
- constructor + exhaustive match + payload binding on every loop iteration;
- recurrence visits both variants;
- warmup=3, samples=13, timeout=5000 ms, max relative MAD=0.15.

Evolution and reference Rust must use the same static enum layout, payload types, algorithm, input/output and pinned rustc release path.

## #62 acceptance discipline

Retain actual harness evidence for:

- differential stdout/stderr/exit correctness;
- generated Rust direct static lowering;
- no hidden allocation/boxing/clone/dispatch/runtime metadata;
- normalized LLVM comparison where meaningful;
- exact executable equality where achievable;
- binary sizes;
- raw timing samples, median, p95, MAD and normalized ratio;
- PASS/FAIL/INCONCLUSIVE exactly according to #4/#5.

Correctness must pass before timing. Byte-identical binary parity is stronger deterministic parity evidence when achieved. Otherwise stable `T_evolution / T_reference <= 1.00` is required. Noisy measurement is INCONCLUSIVE, never PASS.

## Parent #50 remaining items

Merged-main evidence already covers syntax/type/match/ownership/codegen/source-map/native correctness. The new process-level explicit reinitialization case passed #275 on all supported platforms, but #50 should be finalized only after #62 itself merges and post-merge main CI is green.

Still open:

- accepted dedicated Enums differential performance evidence;
- final `docs/LANGUAGE_SPEC_V0.md` synchronization from accepted behavior/performance evidence;
- #62 merge + post-merge main CI;
- final #50 closure.

## ZERO-cost boundary

Enums v0 remains a ZERO-cost-class target: ordinary static Rust enum values and matches, with no hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.

Do not add generics, guards, wildcard/or/arbitrary nested patterns, references/borrow inference, methods, derives or runtime reflection as collateral work.

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
