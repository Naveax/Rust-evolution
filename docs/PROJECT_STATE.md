# Rust Evolution — Project State

Last verified update: **2026-09-07**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Authoritative `main`: `6c7bc8a4376966775728765444002f0b6774cd31`
- Post-merge main CI #278 / run `34114143728`: **SUCCESS**

## Completed core data-model milestones

### Records v0

Records v0 remains the accepted ZERO-cost nominal product-type baseline with direct static Rust lowering, explicit by-value ownership, source-native diagnostics and its differential performance gate preserved.

### Enums v0 — #50

**COMPLETED** from merged-main evidence.

Delivered slices:

- #51 parser/formatter surface;
- #54 semantic umbrella;
- #60 ownership / PR #63;
- #61 executable static Rust codegen/source maps/native correctness / PR #64;
- #62 differential performance + final language-spec sync / PR #65.

Final evidence:

- final PR #65 head `1c946af9d15946163b77a92f5d29c89f73469be2`;
- final PR CI #277 / run `34112029258`: **SUCCESS** on Ubuntu/macOS/Windows;
- squash merge `6c7bc8a4376966775728765444002f0b6774cd31`;
- post-merge main CI #278 / run `34114143728`: **SUCCESS**.

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

## Enums v0 performance evidence

### Retained first failure

Initial #62 head `2c898fa6b845f12f78de99356441cf98afe0e23b`, CI #275 / `34107195001`: **FAILURE**, never rerun.

The handwritten generated-style Rust benchmark reference did not exactly mirror actual emitter ordering/shape. Its result remains preserved:

- correctness PASS;
- normalized LLVM equal `false`;
- exact executable equal `false`;
- ratio `1.000707082`;
- final verdict FAIL;
- basis `timing-median-ratio`.

### Accepted corrected parity

Corrected head `69bc2d1b15db1bd841b85e8a508c156dc689550d`, CI #276 / `34108814832`: **SUCCESS**.

Accepted evidence:

- exact reference/generated-Rust integration lock PASS;
- differential correctness PASS;
- normalized LLVM IR equal `true`;
- exact executable equal `true`;
- binary size `2,267,072 B` on both sides;
- reference median `16,506,786 ns`;
- Evolution median `16,520,050 ns`;
- ratio `1.000803548`;
- timing-only verdict FAIL retained visibly;
- final verdict PASS;
- basis `byte-identical-binary-parity`.

Correctness PASS plus byte-identical executables is accepted stronger deterministic runtime parity evidence under #4/#5. No compiler/lowering/codegen semantics were changed merely to obtain a friendlier benchmark result.

Final PR head #277 and merged-main #278 kept all previous Ubuntu runtime/performance gates green.

## Language specification

`docs/LANGUAGE_SPEC_V0.md` is the current implemented-language source. It now includes the completed Enums v0 grammar, semantics, ownership, static Rust lowering, diagnostics/source maps and accepted performance evidence.

Vision-only ergonomics remain outside the current spec until separately implemented and proven.

## Current active roadmap state

There is no selected implementation feature after Enums v0 yet.

Master roadmap #1 says the next Phase-1 step is to atomize one new P0 weakness from #6 under parent #2 and then repeat the project methodology:

1. define the measurable weakness and user scenario;
2. identify layer/root cause;
3. define semantics/correctness and safety boundaries;
4. assign ZERO / EXPLICIT / MANAGED cost expectations where relevant;
5. define source diagnostics/tooling expectations;
6. design tests and #4/#5 differential performance evidence where applicable;
7. create a focused P0 issue;
8. only then start implementation branches.

Do not treat broad categories such as collections, errors, borrow ergonomics, scripting, methods, generics or async as one ticket.

## ZERO-cost boundary

Current accepted Core-language slices must not silently introduce hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.

Future syntax/ergonomics must fail closed until semantics, safety, codegen and performance/cost behavior are explicitly established.

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
