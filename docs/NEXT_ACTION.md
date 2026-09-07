# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-07**

## Stable baseline

Enums v0 milestone **#50 is completed**. Final child **#62 is completed**.

Authoritative `main`:

- `6c7bc8a4376966775728765444002f0b6774cd31`
- PR #65 squash merge
- post-merge CI #278 / run `34114143728`: **SUCCESS**
- Rust toolchain: **1.98.0**

Final PR evidence:

- final PR head `1c946af9d15946163b77a92f5d29c89f73469be2`
- CI #277 / run `34112029258`: **SUCCESS** on Ubuntu/macOS/Windows

Ubuntu #278 passed fmt, Clippy, workspace tests, benchmark smoke, runtime-repeat, control-flow, logical-operators, function-call, block-locals, Records v0, Enums v0 and release-build gates. macOS and Windows quality/release jobs also passed.

## Accepted Enums v0 performance evidence

Retained first failure:

- head `2c898fa6b845f12f78de99356441cf98afe0e23b`
- CI #275 / run `34107195001`: **FAILURE**, never rerun
- correctness PASS
- normalized LLVM equal `false`
- exact executable equal `false`
- ratio `1.000707082`
- basis `timing-median-ratio`

Corrected accepted evidence:

- head `69bc2d1b15db1bd841b85e8a508c156dc689550d`
- CI #276 / run `34108814832`: **SUCCESS**
- correctness PASS
- exact reference/generated-Rust lock PASS
- normalized LLVM equal `true`
- exact executable equal `true`
- binary size `2,267,072 B` on both sides
- reference median `16,506,786 ns`
- Evolution median `16,520,050 ns`
- ratio `1.000803548`
- timing-only verdict FAIL retained visibly
- final verdict PASS
- basis `byte-identical-binary-parity`

No compiler/lowering/codegen semantics were changed to obtain the corrected benchmark result.

## Implemented language state

`docs/LANGUAGE_SPEC_V0.md` is the current implemented-language source. Enums v0 now includes nominal declarations, unit/single-payload variants, qualified constructors, exhaustive statement-only matching, arm-local typed payload bindings, by-value ownership/reinitialization, direct static Rust lowering, source-native diagnostics/source maps and accepted performance evidence.

Unsupported future ergonomics remain outside the current spec and fail closed.

## Next active task

Master roadmap #1 now requires:

> **Atomize one new P0 weakness from #6 under parent #2.**

No next feature has been selected yet. Do not jump directly into implementation.

Selection work must:

1. inspect #6 against current implemented capabilities;
2. choose one bounded, high-impact weakness;
3. define the measurable problem and user scenario;
4. classify layer and cost class;
5. define semantics/correctness and safety boundaries;
6. define required source-native diagnostics/tooling behavior;
7. define test and differential benchmark needs under #4/#5;
8. create one focused P0 issue before implementation branches are opened.

Prefer a problem that is useful, independently measurable and small enough to preserve the current evidence discipline. Do not smuggle in generics, async, methods, borrowing, collections, error handling and scripting all at once merely because humans enjoy impossible milestones.

## Engineering constraints

- No hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch unless a future feature explicitly declares a different cost class.
- Do not weaken existing runtime/performance gates.
- Preserve unfavorable/noisy evidence rather than rerunning old SHAs.
- Every new feature must fail closed outside its declared semantics.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. A failed SHA is evidence, not a retry button with emotional support.
