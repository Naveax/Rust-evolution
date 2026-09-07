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
  - final PR CI #273 / run `34106104357`: **SUCCESS**;
  - squash merge `a7b8c08a71283cffa15216c7359078f1c8a34873`;
  - post-merge main CI #274 / run `34106421537`: **SUCCESS**.

Merged Enums behavior includes nominal declarations, qualified constructors, exhaustive statement-only matches, typed lexical payload bindings, explicit by-value ownership, static Rust enum/match codegen, source maps and native correctness coverage without hidden clone/allocation/boxing/GC/RC/runtime maps/reflection/dynamic dispatch.

## Active final child — #62

- Issue: **#62 — P0 enums performance: differential parity gate and language spec sync**
- PR: **#65 — `perf: add Enums v0 differential parity gate`**
- Feature branch: `feature/enums-performance-v0`
- Staging branch: `work/enums-performance-v0`
- Exact branch base: `a7b8c08a71283cffa15216c7359078f1c8a34873`

## Retained first benchmark failure

Initial feature head `2c898fa6b845f12f78de99356441cf98afe0e23b` ran as CI #275 / `34107195001` and **FAILED** the first Enums timing gate. It must not be rerun.

#275 Enums evidence:

- correctness: **PASS**
- normalized LLVM IR equal: `false`
- exact binary equal: `false`
- reference median: `16,535,571 ns`
- Evolution median: `16,547,263 ns`
- ratio: `1.000707082`
- stable: `true`
- final verdict: **FAIL**
- verdict basis: `timing-median-ratio`

The benchmark reference was not exactly equivalent to emitter output, so deterministic LLVM/binary parity was invalid on #275. The unfavorable result remains retained rather than rerun away.

## Accepted corrected benchmark evidence

Corrected feature head:

- `69bc2d1b15db1bd841b85e8a508c156dc689550d`
- CI #276 / run `34108814832`: **SUCCESS**
- Ubuntu/macOS/Windows matrix: **SUCCESS**
- Enums artifact id: `10014284630`

The corrected benchmark locks `reference.rs` to generated Rust exactly after newline normalization and uploads Enums evidence with `always()`.

Accepted #276 Enums result:

- correctness: **PASS**
- normalized LLVM IR equal: `true`
- exact executable equal: `true`
- binary size: `2,267,072 B` on both sides
- reference median: `16,506,786 ns`
- Evolution median: `16,520,050 ns`
- reference p95: `16,596,414 ns`
- Evolution p95: `16,619,046 ns`
- relative MAD: `0.001764426` / `0.002294908`
- stable: `true`
- ratio: `1.000803548`
- timing-only verdict: **FAIL**
- final verdict: **PASS**
- verdict basis: `byte-identical-binary-parity`

Correctness PASS plus byte-identical executables is accepted deterministic runtime parity under #4/#5. Raw timing remains preserved as evidence.

All previous Ubuntu runtime gates and release build passed on #276.

## Native correctness completion

The dedicated explicit same-type enum reinitialization process test passed on Ubuntu, Windows and macOS in #275 and remained green in #276. This satisfies the remaining process-level reinitialization evidence for parent #50.

## Language spec sync

Accepted #276 evidence unlocked the final stable-sketch update on staging.

`docs/LANGUAGE_SPEC_V0.md` now includes Enums v0:

- declaration grammar/placement;
- unit and single-payload variants;
- exact nominal payload typing;
- qualified construction;
- exhaustive statement-only matching;
- arm-local payload binding scope/type;
- by-value ownership and exact-type reinitialization;
- direct static Rust lowering;
- source-map/diagnostic rules;
- accepted #276 parity evidence;
- explicit Enums non-goals.

## Remaining #62 completion path

Only release-process work remains:

1. synchronize issue/PR/handoff evidence;
2. fast-forward final staging to feature only after completed #276;
3. run exactly one CI on that final docs-synchronized head;
4. mark PR #65 ready only after final CI SUCCESS;
5. squash-merge with exact expected head SHA;
6. verify post-merge `main` push CI;
7. close #62 and parent #50 only from merged-main evidence.

## ZERO-cost boundary

Enums v0 is a ZERO-cost-class target: ordinary static Rust enums and matches, with no hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.

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
