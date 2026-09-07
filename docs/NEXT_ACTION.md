# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-07**

## Active P0

Parent milestone: **#50 — Enums v0: nominal sum types + exhaustive static matching**

Stable `main` baseline:

- #61 / PR #64 squash merge: `a7b8c08a71283cffa15216c7359078f1c8a34873`
- post-merge main CI #274 / run `34106421537`: **SUCCESS**
- Ubuntu preserved every existing runtime/performance gate; Windows/macOS preserved format, Clippy, workspace tests, benchmark smoke and release build
- Rust toolchain: **1.98.0**

Completed Enums children:

- #51 parser/formatter
- #54 semantic umbrella
- #60 ownership
- #61 executable static Rust codegen + source maps + native correctness

Active final child:

- **#62 — differential performance parity + final language spec sync**
- feature branch: `feature/enums-performance-v0`
- staging branch: `work/enums-performance-v0`
- exact branch base: `a7b8c08a71283cffa15216c7359078f1c8a34873`

## #62 first slice

Staging currently contains:

1. `benchmarks/cases/enums-v0/`
   - runtime-dependent `input_int` workload;
   - `20,000,000` iterations, seed `9`;
   - two scalar-payload variants;
   - enum construction through `classify`, by-value enum return, exhaustive match and payload binding on every hot-loop iteration;
   - equivalent generated-style Rust reference with the same layout, algorithm, input and output;
   - expected stdout `15099959897`;
   - benchmark policy matching Records v0: warmup=3, samples=13, max relative MAD=0.15.
2. Native process regression proving an enum local can be explicitly reinitialized with the same enum type after a consuming call, then successfully reused in a native binary.
3. Ubuntu CI `Enums v0 performance gate` plus uploaded benchmark artifact.

The language spec is intentionally unchanged until benchmark correctness/performance evidence is green.

## Immediate resume sequence

1. Compare current `work/enums-performance-v0` against exact base `a7b8c08a71283cffa15216c7359078f1c8a34873`.
2. Fast-forward `feature/enums-performance-v0` to staging with `force=false` and open a draft PR tracking #62.
3. Let exactly one CI run validate the first slice. Do not manually rerun failed/active SHAs.
4. If CI fails:
   - inspect the actual first failing job/log;
   - if the Enums benchmark runs, retain its reported correctness/timing verdict even when unfavorable;
   - fix the real issue on a new staging SHA; never rerun the old SHA just for another timing sample.
5. If the Enums gate is green:
   - download/inspect the benchmark artifact;
   - record differential correctness, normalized LLVM/executable parity where reported, binary sizes, raw samples, median/p95/MAD and final ratio/verdict;
   - update #62 evidence only from the artifact/run.
6. Only after accepted benchmark evidence:
   - synchronize `docs/LANGUAGE_SPEC_V0.md` with the already-proven Enums behavior;
   - update #50 remaining native/performance/spec checklist;
   - run a final docs-synchronized CI;
   - merge #62 with expected-head SHA and verify post-merge `main` CI before closing #50.

## Performance contract

#4/#5 remain non-negotiable:

- correctness before timing;
- equivalent task/input/output/layout/flags;
- same pinned rustc path;
- stable repeated measurements;
- repeatable `T_evolution / T_reference_rust > 1.00` is FAIL;
- noisy results are INCONCLUSIVE, never silently PASS;
- deterministic stronger parity evidence such as equivalent normalized IR/binary may supersede noisy wall-clock timing only under the existing harness contract.

## Engineering constraints

- No benchmark that makes the reference Rust intentionally worse.
- No hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.
- Keep enum/variant identity structured and static.
- Preserve all existing Records/scalar/function runtime gates.
- Do not change accepted Enums semantics merely to improve a benchmark result.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent staging/docs work, but never create multiple active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Never rerun an old failed SHA merely to obtain another result or timing sample.
