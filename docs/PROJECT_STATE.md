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

Initial feature head `2c898fa6b845f12f78de99356441cf98afe0e23b` ran as CI #275 / `34107195001` and **FAILED** at the new Enums performance gate. It must not be rerun.

Everything before that new gate remained green, including the new native same-type enum reinitialization process test on Ubuntu, Windows and macOS.

#275 Enums evidence:

- correctness: **PASS**
- normalized LLVM IR equal: `false`
- exact binary equal: `false`
- reference median: `16,535,571 ns`
- Evolution median: `16,547,263 ns`
- ratio: `1.000707082`
- stable: `true`
- timing verdict/final verdict: **FAIL**
- verdict basis: `timing-median-ratio`

The benchmark reference was not actually identical to the emitter output: declaration/helper/function ordering and exact condition/arm formatting differed. That invalidated deterministic LLVM/binary parity for #275. The unfavorable timing result remains preserved rather than rerun away.

## Corrected benchmark contract

Feature head is now:

- `69bc2d1b15db1bd841b85e8a508c156dc689550d`
- authoritative run: CI #276 / `34108814832`

The corrected head changes benchmark/evidence infrastructure only:

1. Enums artifact upload runs with `always()` so unfavorable/inconclusive reports survive.
2. `benchmarks/cases/enums-v0/reference.rs` mirrors actual generated Rust ordering/shape.
3. `crates/evo-bench/tests/enums_reference.rs` asserts exact reference/generated Rust equality after newline normalization.
4. Durable docs retain the #275 failure/root cause.

No compiler/lowering/codegen semantics changed merely to improve timing.

At the last verified #276 state:

- Windows: **SUCCESS**
- macOS: **SUCCESS**
- Ubuntu: queued

Therefore the exact-reference integration lock has already passed on two supported platforms. Performance is still pending Ubuntu evidence.

Staging is intentionally allowed to move ahead of the feature branch only with documentation while #276 is active. Do not fast-forward feature again until #276 completes.

## Benchmark design

`benchmarks/cases/enums-v0`:

- runtime stdin: `20,000,000` iterations, initial `x = 9`;
- expected stdout: `15099959897`;
- two scalar-payload `Step` variants;
- by-value enum return from `classify`;
- constructor + exhaustive match + payload binding on every iteration;
- recurrence visits both variants;
- warmup=3, samples=13, timeout=5000 ms, max relative MAD=0.15.

Evolution and reference Rust must use the same static enum layout, payload types, algorithm, input/output and pinned rustc release path.

## #62 acceptance discipline

Retain actual harness evidence for correctness, generated Rust, normalized LLVM, binary equality/size, raw samples, median/p95/MAD, normalized ratio and final PASS/FAIL/INCONCLUSIVE verdict.

Correctness must pass first. With the exact-reference lock, byte-identical binary parity is stronger deterministic runtime parity evidence when achieved. Otherwise stable `T_evolution / T_reference <= 1.00` is required. Noisy measurement is INCONCLUSIVE, never PASS.

`docs/LANGUAGE_SPEC_V0.md` remains unchanged until corrected performance evidence is accepted.

## Parent #50 remaining items

The explicit same-type reinitialization process case is now proven by #275 on all supported platforms. Parent closure still waits for:

- accepted dedicated Enums differential performance evidence;
- final `docs/LANGUAGE_SPEC_V0.md` synchronization;
- #62 merge + post-merge main CI;
- final #50 closure from merged-main evidence.

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
