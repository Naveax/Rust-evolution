# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-11**

## Stable predecessor gate

PR #96 (`research: classify borrow inference feasibility v0`) merged to `main` as:

`cfc19e3308a505a45c970e883dffc72a8bf0b81b`

Post-merge validation succeeded:

- CI #386 / run `34481370952`: **SUCCESS** on Ubuntu, Windows and macOS;
- Borrow inference research #5 / run `34481370964`: **SUCCESS**;
- #95 closed/completed with verdict **IMPLEMENT-CANDIDATE**.

The accepted research rule is local and deliberately non-transitive: nominal parameters may become call-duration shared borrows only when their own body has at least one direct `Inspect`, zero `Consume` uses and no reinitialization. Nested calls remain consuming boundaries while classifying the caller.

## Active completion — #97 / PR #99

Issue #97 implements inferred shared-borrow nominal parameters v0.

PR:

- #99 `implement: infer shared-borrow nominal parameters v0`
- branch: `feature/inferred-shared-borrow-parameters-v0`
- latest accepted implementation/benchmark head before documentation synchronization: `f33b513188add2c6755263f6c7e1079072ec9d7b`

Implemented contract:

- internal parameter modes are `Owned` and `SharedBorrow`;
- only nominal record/enum parameters satisfying the accepted local classifier become `SharedBorrow`;
- classification is non-transitive and requires no fixpoint/lifetime solving;
- a top-level nominal caller local passed to an already-decided `SharedBorrow` parameter is inspected rather than moved;
- temporaries/subexpressions keep ordinary owned evaluation and are borrowed only at the call boundary;
- lowered/executable IR carries the passing mode before Rust rendering;
- Rust codegen emits ordinary `&T` parameters and `&expr` arguments;
- record-only and enum-integrated paths use the same rule;
- owned return, owned match, consuming nested calls, reinitialization and unsupported escaping-reference cases remain by-value/fail-closed;
- no implicit clone/copy, boxing, RC/GC, runtime ownership map, stored borrow value, generalized lifetime or mutable-borrow inference was introduced.

## Targeted correctness evidence

Committed tests cover:

- repeated calls on one read-only record local;
- borrow then later owned move;
- branch/repeat/nested record inspection;
- temporary call arguments;
- enum-integrated reuse and temporary calls;
- owned return/reinitialization boundaries;
- the deliberately non-transitive forwarding rule;
- exact benchmark-reference/generated-Rust equality.

Exact head `f33b513188add2c6755263f6c7e1079072ec9d7b` passed workspace tests on Ubuntu, Windows and macOS in CI #404 / run `34576556404`.

## Permanent runtime differential gate

New case: `benchmarks/cases/inferred-shared-borrow-v0`.

The fixture performs 5,000,000 iterations of repeated read-only calls on one nominal value and later reassigns that owned value. The locked reference uses explicit idiomatic Rust shared borrowing. A committed `evo-bench` test requires `reference.rs` to mirror generated Rust exactly, preventing irrelevant source-order/layout differences from becoming timing noise.

Accepted CI #404 Ubuntu evidence on `f33b513188add2c6755263f6c7e1079072ec9d7b`:

- correctness: **PASS**;
- normalized LLVM IR equal: **true**;
- exact executable bytes equal: **true**;
- reference median: **5,305,283 ns**;
- Evolution median: **5,286,876 ns**;
- observed ratio: **0.996530440**;
- stable measurement: **true**;
- timing verdict: **PASS**;
- final verdict: **PASS**;
- verdict basis: `byte-identical-binary-parity`.

Artifact:

- `evo-bench-inferred-shared-borrow-ubuntu-latest`;
- id `10189953214`;
- digest `sha256:8fcee06fa8df815e0aaf156649d787cf59459f0d829151811f19ff9ae48d7955`.

All pre-existing Ubuntu runtime gates also remained **PASS** in the same run, and the release build succeeded.

## Failed-SHA evidence retained

Failed or superseded heads are evidence and are not rerun merely to make history green.

- CI #394 / run `34487001305`: initial enum-integrated head failed rustfmt only.
- CI #396 / run `34494690100`: formatting passed; Clippy exposed stale API/dead-code integration issues.
- CI #399 / run `34495384328`: exposed legacy direct-include integration harnesses missing the new crate-root bridge.
- CI #402 / run `34496493149` on `64b30b6ddf280adfb04b578ed675331df065c658`: all existing Ubuntu runtime gates before the new case passed; the new benchmark had correctness/stability PASS but its manually written Rust reference differed from generated Rust in function/helper ordering. That produced non-identical IR/binaries and a timing-only ratio `1.006804464`, so the gate correctly failed. The SHA was not rerun.
- The reference was then aligned exactly with generated static Rust and protected by a permanent exact-reference test. The corrected head `f33b513...` produced identical IR/binaries and PASS evidence in CI #404.

## Immediate sequence

1. Synchronize `LANGUAGE_SPEC_V0`, `BENCHMARKING`, `PROJECT_STATE`, this file, PR #99, #97 and living meta #40 with the accepted `f33b513...` evidence.
2. Keep the documentation-synchronized PR head fixed and track only its natural CI; do not create duplicate runs.
3. Require final-head normal CI **SUCCESS** on Ubuntu, Windows and macOS, including the new shared-borrow runtime gate and all existing gates.
4. Confirm the final-head shared-borrow artifact still reports correctness PASS and byte-identical parity.
5. Squash-merge PR #99 only from that exact validated head.
6. Track the natural post-merge `main` CI on the exact merge SHA.
7. Close #97 completed only after post-merge `main` CI succeeds.
8. Update #40 with merge/post-merge provenance, then re-read live roadmap/issues before starting a successor.

## Production contracts

Rust remains pinned to **1.98.0**, edition 2024, opt-level 3 and codegen-units 1. `build-cache-v0` / `run-cache-v0`, source mapping, diagnostic remapping and the #4 runtime parity-or-better contract remain unchanged.

The new feature changes only the proven nominal-parameter ownership subset. Explicit Evolution `&` syntax, returned/escaping references, generalized lifetimes, mutable borrow inference, transitive/fixpoint borrow inference, stored references, automatic cloning and ownership runtimes remain outside v0.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Failed SHAs remain evidence and are not rerun merely for a better color.
