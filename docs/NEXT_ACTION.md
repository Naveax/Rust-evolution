# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-10**

## Stable main before active PR #94

- `c8ec7d397a50772ed500c4717dc340a4d00936d4`
- PR #92 / #91 compile-memory research merge
- post-merge CI #378 / run `34471091448`: **SUCCESS** on Ubuntu, Windows and macOS
- post-merge Compile memory research #3 / run `34471091443`: **SUCCESS**
- Rust toolchain: **1.98.0**

## Active completion — #93 / PR #94

Issue **#93 — binary size baseline v0** has reached a research decision.

PR:

- #94 `research: establish binary size baseline v0`
- branch: `research/binary-size-baseline-v0`
- accepted measurement head before documentation sync: `3c2565f513ef7951a7fa011879525e9e60aa469b`

Accepted validation:

- CI #379 / run `34471973541`: **SUCCESS** on Ubuntu, Windows and macOS;
- Binary size research #1 / run `34471973624`: **SUCCESS**;
- artifact id `10149900668`;
- digest `sha256:68eea4e28b4a24008cb5ef8d7490541bef616a95ce9e29555a22516ef3583e5a`;
- correctness **PASS**;
- JSON validation **PASS**.

Accepted corpus result:

| Case | Reference bytes | Evolution bytes | Delta |
| --- | ---: | ---: | ---: |
| runtime-repeat-v0 | 4,517,168 | 4,517,168 | 0 |
| control-flow-branch-v0 | 4,517,360 | 4,517,360 | 0 |
| logical-operators-v0 | 4,517,280 | 4,517,280 | 0 |
| function-call-v0 | 4,517,344 | 4,517,344 | 0 |
| block-locals-v0 | 4,517,376 | 4,517,376 | 0 |
| records-v0 | 4,517,408 | 4,517,408 | 0 |
| enums-v0 | 4,517,376 | 4,517,376 | 0 |

All seven reference/Evolution binaries are byte-for-byte identical. `DT_NEEDED` identity and parsed section sizes are also identical. Generated Rust equals the committed reference source in every controlled case.

Decision: **DEFER / NO ACTION** for Evolution-specific binary-size optimization under the current language/corpus.

Durable report: `docs/BINARY_SIZE_RESEARCH.md`.

## Immediate sequence

1. Keep PR #94 on one documentation-synchronized final head.
2. Track the natural exact-head normal CI and Binary size research workflows; do not duplicate them.
3. Require normal CI green on Ubuntu, Windows and macOS.
4. Require final-head binary-size workflow green and artifact retained.
5. Update PR #94 / #93 with final-head provenance.
6. Squash-merge PR #94 using expected-head protection only after both final-head workflows are green.
7. Track natural post-merge `main` CI and binary-size push workflow.
8. Close #93 completed only after post-merge main CI succeeds.
9. Re-read live main/branch/PR/Actions state.
10. Only then create #95 borrow-inference research branch from exact verified main.

## Gated successor — #95

Issue **#95 — borrow inference feasibility v0** is open and must not start before PR #94 merge + green post-merge main CI.

The first #95 slice is research/semantics classification, not implementation. It should classify representative read-only nominal-value use sites as safe local inference candidates, explicit-borrow-syntax cases, lifetime-model cases, keep-by-value cases, or unsafe/ambiguous cases.

No automatic clone/copy insertion, hidden RC/GC/boxing, generalized lifetime magic or silent ownership weakening is permitted.

## Build/compile structural deferrals

Dependency-build, proc-macro cost and workspace scaling remain deferred until Evolution user programs have a real package/dependency graph. Current single-file Phase 3.4 work is otherwise measured through binary-size baseline v0.

## Production contracts unchanged

- Evolution syntax/semantics and ownership/type rules;
- Rust 1.98.0;
- edition 2024;
- opt-level 3;
- codegen-units 1;
- current linker behavior;
- build-cache-v0 / run-cache-v0;
- source mapping and rustc diagnostic remapping;
- #4 runtime parity-or-better contract;
- no hidden runtime allocation/clone/boxing/dynamic dispatch metadata.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Failed SHAs remain evidence and are not rerun merely for color.
