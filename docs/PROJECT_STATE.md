# Rust Evolution — Project State

Last verified update: **2026-09-07**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Current merged `main` candidate: `6c7bc8a4376966775728765444002f0b6774cd31`
- Post-merge main CI #278 / run `34114143728`: **ACTIVE**

Do not treat the Enums milestone as closed until that exact push run succeeds.

## Records v0 baseline

Records v0 remains the accepted ZERO-cost nominal product-type baseline with direct static Rust lowering, explicit by-value ownership, source-native diagnostics and its differential performance gate preserved.

## Enums v0 milestone — #50

Delivery slices are merged:

- #51 parser/formatter surface;
- #54 semantic umbrella;
- #60 ownership / PR #63;
- #61 static executable codegen/source maps/native correctness / PR #64;
- #62 differential performance + final language-spec sync / PR #65.

Final #65 evidence:

- final PR head: `1c946af9d15946163b77a92f5d29c89f73469be2`
- final PR CI #277 / run `34112029258`: **SUCCESS** on Ubuntu/macOS/Windows
- squash merge: `6c7bc8a4376966775728765444002f0b6774cd31`
- post-merge main CI #278 / run `34114143728`: **ACTIVE**

Merged Enums v0 includes:

- nominal enum declarations;
- unit and single typed-payload variants;
- qualified constructors;
- exhaustive statement-only matching;
- arm-local typed payload bindings;
- explicit by-value move semantics and exact-type reinitialization;
- direct static Rust enum/constructor/match lowering;
- source maps and source-native diagnostics;
- native correctness coverage;
- dedicated differential performance evidence;
- no hidden clone/allocation/boxing/GC/RC/runtime maps/reflection/dynamic dispatch.

## Performance evidence

### Retained first failure

Initial #62 head `2c898fa6b845f12f78de99356441cf98afe0e23b`, CI #275 / `34107195001`: **FAILURE**, never rerun.

The handwritten generated-style Rust benchmark reference did not exactly mirror actual emitter ordering/shape. Its Enums result was:

- correctness PASS;
- normalized LLVM equal `false`;
- exact executable equal `false`;
- ratio `1.000707082`;
- final verdict FAIL;
- basis `timing-median-ratio`.

The failure remains preserved as evidence.

### Accepted corrected parity

Corrected head `69bc2d1b15db1bd841b85e8a508c156dc689550d`, CI #276 / `34108814832`: **SUCCESS**.

Accepted Enums artifact evidence:

- exact reference/generated-Rust integration lock: PASS;
- differential correctness: PASS;
- normalized LLVM IR equal: `true`;
- exact executable equal: `true`;
- binary size: `2,267,072 B` on both sides;
- reference median: `16,506,786 ns`;
- Evolution median: `16,520,050 ns`;
- ratio: `1.000803548`;
- stable: `true`;
- timing-only verdict: FAIL retained visibly;
- final verdict: PASS;
- basis: `byte-identical-binary-parity`.

Correctness PASS plus byte-identical executables is accepted stronger deterministic runtime parity evidence under #4/#5. No compiler/lowering/codegen semantics were changed merely to obtain a friendlier benchmark result.

All previous Ubuntu runtime/performance gates remained green on #276 and final PR head #277.

## Language specification

`docs/LANGUAGE_SPEC_V0.md` now contains the implemented Enums v0 behavior and accepted performance evidence. Vision-only enum ergonomics remain outside the current spec.

Explicit non-goals for this slice include generic enums, `Option`/`Result` sugar, guards, wildcard/or/arbitrary nested patterns, methods/impl/derive/traits, references/borrow inference and runtime reflection.

## Current closure gate

#62 and parent #50 remain open solely because post-merge `main` CI #278 is still active.

If #278 succeeds:

1. mark merged-main verification complete on #62 and #50;
2. close #62;
3. close #50;
4. synchronize this file, `docs/NEXT_ACTION.md`, and continuity issue #40 with the final merged-main result;
5. then atomize one new P0 weakness from #6 under parent #2.

Do not begin a new language feature before #50 closes.

## ZERO-cost boundary

Current accepted Core-language slices must not silently introduce hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.

Future syntax/ergonomics must keep failing closed until semantics, safety, codegen and #4/#5 performance evidence are explicitly established.

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
