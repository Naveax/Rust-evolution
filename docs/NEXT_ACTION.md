# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-11**

## Stable predecessor gate

PR #99 (`implement: infer shared-borrow nominal parameters v0`) squash-merged to `main` as:

`355847a23faa29360e1da10bdfb2739eec6f8b6a`

Natural post-merge CI #406 / run `34577783740`: **SUCCESS** on Ubuntu, Windows and macOS, including the permanent inferred-shared-borrow runtime gate and release build. Issue #97 is closed/completed.

Production semantics remain the bounded call-duration `SharedBorrow` contract. Evolution still has no user-facing first-class reference value or returned/escaping reference surface.

## Active research — #100 / PR #101

Issue #100 asks whether useful borrowed-return relationships can omit named lifetime annotations without silently changing current owned return semantics.

PR:

- #101 `research: classify lifetime elision feasibility v0`
- branch: `research/lifetime-elision-feasibility-v0`
- accepted research code/evidence head before documentation synchronization: `f65db4a68a93240abe51d26444e70fc568a7ba02`

The research is classifier/reference evidence only. No production parser, lowering, ownership, codegen, runtime or accepted-program behavior changes.

## Accepted research result

Dedicated Lifetime elision research #2 / run `34580003076` on `f65db4a68a93240abe51d26444e70fc568a7ba02`: **SUCCESS**.

Artifact:

- `evo-lifetime-elision-research-ubuntu-24.04`;
- id `10191268492`;
- digest `sha256:40b6d338e7ed74aa7d158f32ac07a956cc2694892d3f945015ef99cf8dfad62f`.

Result:

- compile-expectation mismatches: **0**;
- `ELISION-CANDIDATE-REQUIRES-REFERENCE-TYPE`: **8** cases;
- `REQUIRES-EXPLICIT-LIFETIME-RELATION`: **2** cases;
- `UNSAFE-OR-UNREPRESENTABLE`: **1** case;
- aggregate verdict: **REFERENCE-SURFACE-FIRST**.

Rust 1.98 accepts named-lifetime-free signatures for useful single-source borrowed-return shapes, including whole-value, nested-field, forwarding and recursive cases. The result is still a borrow, however. Stored borrowed results retain owner move/reinitialization conflicts, multiple borrowed owner sources are not resolved by ordinary lifetime elision, and temporary-derived returned references fail closed.

Therefore current owned `T -> T` contracts must remain owned. Escaping borrowed results require a caller-visible immutable reference / borrowed-result type distinction before any production implementation.

Durable report: `docs/LIFETIME_ELISION_RESEARCH.md`.

## Failed-SHA evidence retained

Initial research head `6203569451abc1ea448a36eaa47e21fddbe0eed7` produced successful research evidence in Lifetime elision research #1 / run `34579682344`, but normal CI #407 / run `34579682411` failed only rustfmt on the new research test. That SHA was not rerun.

The format-only successor `f65db4a68a93240abe51d26444e70fc568a7ba02` preserves the same research matrix and decision.

## Immediate sequence

1. Require normal CI #408 / run `34580003002` on exact code/evidence head `f65db4a...` to complete **SUCCESS** on Ubuntu, Windows and macOS.
2. Attach `LIFETIME_ELISION_RESEARCH`, this file and `PROJECT_STATE` in one documentation-synchronization commit without changing research semantics.
3. Track only the natural normal CI and Lifetime elision research runs created for that exact documentation head; do not dispatch duplicates.
4. Require both final-head workflows **SUCCESS**, validate the final artifact still reports `REFERENCE-SURFACE-FIRST` with zero expectation mismatches, and live-check PR head/base/mergeability.
5. Squash-merge PR #101 with expected-head protection only from that validated head.
6. Track the natural post-merge `main` CI and Lifetime elision research push workflow on the exact merge SHA.
7. Close #100 completed only after both post-merge workflows succeed.
8. Update living meta #40 with the new durable report and verified main provenance.
9. Re-read live roadmap/issues/branches/PRs, then atomize the required successor: immutable reference surface v0 research/design before escaping borrowed-return implementation.

## Successor direction after #100 closes

The research supports a narrow next question: define a caller-visible immutable borrowed/reference value distinction while continuing to elide named lifetimes for the proven single-source case.

The successor must preserve current owned APIs and fail closed for multiple-source lifetime relationships until an explicit relation design exists. Mutable references, generalized lifetime syntax, `'static` invention, hidden clone/RC/GC/runtime ownership tracking and unsafe widening remain non-goals.

## Production contracts

Rust remains pinned to **1.98.0**, edition 2024, opt-level 3 and codegen-units 1. `build-cache-v0` / `run-cache-v0`, source mapping, diagnostic remapping, bounded inferred shared borrowing and the #4 runtime parity-or-better contract remain unchanged.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Failed SHAs remain evidence and are not rerun merely for a better color.
