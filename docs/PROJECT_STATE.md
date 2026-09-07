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

Completed and merged:

- parser/formatter child #51;
- semantic umbrella #54;
- ownership child #60 / PR #63;
- static executable codegen child #61 / PR #64:
  - final PR head `8d45ea094db44e777a5c6f8c0876ecde45320b4f`;
  - final PR CI #273 / run `34106104357`: **SUCCESS**;
  - squash merge `a7b8c08a71283cffa15216c7359078f1c8a34873`;
  - post-merge main CI #274 / run `34106421537`: **SUCCESS**.

Merged Enums v0 now has:

- nominal declarations and payload schemas;
- qualified variant constructors;
- exhaustive statement-only matches with typed lexical payload bindings;
- explicit by-value ownership and source-native move diagnostics;
- structured executable enum/variant identity and explicit record-vs-enum nominal kinds;
- ordinary static Rust enum definitions, direct constructors and direct exhaustive matches;
- executable source maps for enum/variant/constructor/match/arm lines;
- native unit/scalar/record payload, enum return, nested control-flow and invalid-match process coverage;
- no hidden clone, allocation, boxing, GC/RC, runtime map, reflection metadata or dynamic dispatch.

## Active final child — #62 differential performance + final spec sync

- Issue: **#62 — P0 enums performance: differential parity gate and language spec sync**
- Feature branch: `feature/enums-performance-v0`
- Staging branch: `work/enums-performance-v0`
- Exact branch base: `a7b8c08a71283cffa15216c7359078f1c8a34873`

### First staging slice

The first #62 slice adds:

1. **Runtime-dependent Enums differential case**
   - case path: `benchmarks/cases/enums-v0`;
   - stdin drives `n = 20,000,000` and initial `x = 9`;
   - every hot-loop iteration constructs one of two scalar-payload variants through `classify`, returns the enum by value, exhaustively matches it and consumes the payload binding;
   - Evolution and reference Rust use equivalent static enum layout, payload types, algorithm, stdin/stdout behavior and benchmark compiler path;
   - expected output: `15099959897`;
   - warmup=3, samples=13, max relative MAD=0.15, matching the Records v0 gate policy.
2. **Native same-type reinitialization process case**
   - consume an enum local through a by-value function call;
   - explicitly assign a fresh value of the same enum type;
   - build and run natively, proving reinitialization restores availability at the process level.
3. **CI integration**
   - Ubuntu-only Enums v0 performance gate;
   - benchmark report uploaded as a workflow artifact;
   - all previous gates remain in place.

This first slice is staged but not yet accepted until its own feature-head CI runs. No performance result is claimed from source inspection alone.

## #62 evidence requirements

Before final language-spec sync, retain actual harness evidence for:

- differential stdout/stderr/exit correctness;
- generated Rust direct static lowering inspection;
- no hidden allocation/boxing/clone/dispatch/runtime metadata;
- normalized LLVM comparison where meaningful;
- exact executable equality where achievable;
- binary sizes;
- raw timing samples, median, p95, MAD and normalized ratio;
- PASS/FAIL/INCONCLUSIVE exactly according to #4/#5.

A repeatable ratio above 1.00 is FAIL. A noisy measurement is INCONCLUSIVE, not PASS. Do not rerun an old failed SHA just to fish for a friendlier sample.

## Parent #50 remaining items

All syntax/type/match/ownership/codegen/source-map acceptance is merged on `main` and post-merge verified.

Still open:

- dedicated native reinitialization process success until the new #62 test passes CI;
- dedicated Enums differential performance gate;
- final `docs/LANGUAGE_SPEC_V0.md` synchronization from accepted behavior/performance evidence;
- final #50 closure after #62 merge + post-merge main CI.

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
