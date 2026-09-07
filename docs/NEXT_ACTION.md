# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-07**

## Current state

Enums v0 parent: **#50 — nominal sum types + exhaustive static matching**
Final child: **#62 — differential performance parity + final language spec sync**

PR #65 is merged.

- final PR head: `1c946af9d15946163b77a92f5d29c89f73469be2`
- final PR CI #277 / run `34112029258`: **SUCCESS**
- squash merge: `6c7bc8a4376966775728765444002f0b6774cd31`
- post-merge main CI #278 / run `34114143728`: **ACTIVE**
- Rust toolchain: **1.98.0**

Do not close #62 or #50 until #278 succeeds on exact merge SHA `6c7bc8a4376966775728765444002f0b6774cd31`.

## Accepted Enums v0 performance evidence

The first benchmark head `2c898fa6b845f12f78de99356441cf98afe0e23b`, CI #275 / `34107195001`, is a retained **FAILURE** and must not be rerun. The handwritten generated-style Rust reference did not exactly mirror emitter ordering/shape, so deterministic LLVM/binary parity was invalid there.

Corrected head `69bc2d1b15db1bd841b85e8a508c156dc689550d`, CI #276 / `34108814832`: **SUCCESS**.

Accepted artifact evidence:

- correctness: **PASS**
- exact reference/generated-Rust lock: **PASS**
- normalized LLVM IR equal: `true`
- exact executable equal: `true`
- binary size: `2,267,072 B` on both sides
- reference median: `16,506,786 ns`
- Evolution median: `16,520,050 ns`
- observed ratio: `1.000803548`
- timing-only verdict: **FAIL** retained visibly
- final verdict: **PASS**
- basis: `byte-identical-binary-parity`

No compiler/lowering/codegen semantics were changed to obtain the corrected benchmark result.

## Language state

`docs/LANGUAGE_SPEC_V0.md` now documents the implemented Enums v0 surface:

- nominal `enum` declarations;
- unit and single-payload variants;
- qualified `Enum.Variant(...)` construction;
- static nominal payload typing and by-value layout-cycle rejection;
- exhaustive statement-only `match` / `case`;
- arm-local typed payload bindings;
- by-value enum ownership and exact-type reinitialization;
- direct static Rust enum/constructor/match lowering;
- source-map/diagnostic behavior;
- accepted differential parity evidence;
- explicit non-goals including generics, guards, wildcard/or/nested patterns, methods, references and runtime reflection.

## Resume sequence

1. Check CI #278 / run `34114143728` for exact merge SHA `6c7bc8a4376966775728765444002f0b6774cd31`.
2. Never rerun old #275/#276/#277 runs merely to obtain another result.
3. If #278 fails, inspect the actual failed job/log and fix the real issue on a new SHA. Do not close #62/#50.
4. If #278 succeeds:
   - update #62 and #50 with merged-main success;
   - close #62 as completed;
   - close #50 as completed;
   - update `docs/PROJECT_STATE.md` and this file from merged-main evidence;
   - update continuity issue #40;
   - then move to the next roadmap action.
5. After #50 closure, atomize one new P0 weakness from #6 under parent #2. Do not invent or start a new feature before the Enums parent is closed.

## Engineering constraints

- No hidden clone, allocation, boxing, GC/RC, runtime map, reflection metadata or dynamic dispatch.
- Do not weaken previous runtime/performance gates.
- Preserve unfavorable/noisy benchmark evidence instead of rerunning old SHAs.
- Unsupported future ergonomics remain fail-closed until separately designed and proven.

## CI rule

A running CI is work in progress, not a reason to create duplicate Actions. A failed SHA is evidence, not a slot-machine lever.
