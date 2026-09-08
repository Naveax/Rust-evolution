# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-08**

## Live baseline at this update

- live `main`: `53067b74f64fadab4b41684911401e822972bd26`
- latest verified code-bearing merged baseline: `34f0815d0821543586cd3ae73b9b5b616a1396d3` from PR #78
- PR #78 post-merge main CI #325 / run `34205500589`: **SUCCESS**
- Rust toolchain: **1.98.0**
- active issue: **#79 verified build artifact reuse v0**
- active PR: **#81 `feat: reuse verified artifacts for unchanged evo build`**

Always verify live GitHub before acting.

## Accepted PR #81 code/evidence head

Code/evidence head:

- `9fcab321d3be05b291c2d80f3949f0c975db242a`

Exact-head CI:

- CI **#337** / run `34213183199`: **SUCCESS** on Ubuntu, Windows and macOS;
- Ubuntu: fmt, Clippy, workspace tests, fast edit-run evidence, uncached build-latency baseline, verified build-cache evidence, benchmark smoke, all runtime/performance gates, release build: SUCCESS;
- Windows/macOS: fmt, Clippy, workspace tests, benchmark smoke, release build: SUCCESS.

Accepted Ubuntu cache artifact:

- name: `evo-build-cache-turnaround-ubuntu-latest`
- id: `10050970173`
- digest: `sha256:bf949ee935a2512bd9b74726155b5f67d57a46a122af41c7377f86e3f2a2fa8e`
- fixture: `benchmarks/cases/enums-v0/evolution.evo`
- cold median: **137.783 ms**
- warm cached median: **18.341 ms**
- cold-to-warm speedup: **7.512x**
- accepted #76 warm uncached baseline: **96.986 ms**
- baseline-to-cached speedup: **5.288x**
- cold rustc compile count: **5**
- warm rustc compile count: **0**
- correctness: **PASS**

Exact retained generated Rust:

- 1240 bytes
- SHA-256 `61f5f5c99c47196605ae2e461ee589b72a722c4ed5107c6b5fca353795100d83`
- matches accepted Enums v0 / #76 evidence.

The #76 baseline harness now invokes `evo build ... --no-cache`, so its warm/cold/edit numbers remain explicitly **uncached** after PR #81 makes verified cache reuse the default build behavior.

## Current PR #81 finalization sequence

PR #81 is still draft at the time these durable docs are being written. The documentation commit itself creates a new exact PR head.

Follow this sequence exactly:

1. Read PR #81 live head and find the one natural CI run for that exact docs head.
2. Do **not** create a duplicate run while that SHA/workflow/input is queued or in progress.
3. Require all three OS jobs green.
4. Require Ubuntu's verified build-cache evidence, preserved uncached build-latency baseline, and every existing runtime/performance gate green.
5. Download the final-head `evo-build-cache-turnaround-ubuntu-latest` artifact and confirm:
   - `warm_rustc_compile_count = 0`;
   - `correctness = PASS`;
   - generated Rust SHA-256 remains `61f5f5c99c47196605ae2e461ee589b72a722c4ed5107c6b5fca353795100d83`;
   - warm median remains materially below the accepted #76 **96.986 ms** uncached baseline.
6. Update the #79 acceptance comment if the final docs-head artifact values differ from the code/evidence-head values above.
7. Update PR #81 body from draft-status language to final accepted evidence.
8. Mark PR #81 ready for review only after the exact docs head is green.
9. Squash merge PR #81 with `expected_head_sha` equal to the verified final docs head.
10. Capture the squash merge SHA.
11. Track the natural `push` CI on that exact `main` merge SHA; require SUCCESS before treating the new `main` as stable.
12. Close #79 as completed if the PR did not close it automatically, and update meta issue #40.
13. Refresh durable handoff state if the squash merge SHA/post-main run cannot be represented accurately before merge.

Failed earlier SHAs remain evidence; do not rerun them merely for a green color.

## Accepted build-cache contract

The implemented v0 contract is documented in `docs/BUILD_CACHE.md` and D-021.

Key invariants:

- full frontend validation/lowering/codegen every `evo build` invocation;
- separate `build-cache-v0` from `run-cache-v0`;
- exact source + generated Rust + compiler/config identity verification;
- completion marker and regular non-symlink native artifact;
- cache key is a locator only;
- safe output materialization;
- `--no-cache` bypass;
- corruption/unavailability fails closed to normal compilation;
- generated Rust and generated-program runtime semantics/cost unchanged;
- no daemon, remote executable download, incremental-rustc session, linker replacement, or package-system work in this slice.

## Successor after #79

After PR #81 merge and post-merge main CI are verified green, the next Build / Compile P0 should be **research-first changed-source incremental compilation**.

Evidence source: #76 edited build median **95.515 ms**, rustc compile count **5/5**.

Bound the next issue around measurement and feasibility before implementation. It should answer:

1. Can a persistent rustc incremental/session directory materially reduce the deterministic edited-source fixture latency?
2. What exact invalidation identity is required?
3. Can this stay tooling-only without a mandatory compiler daemon/service?
4. How are generated-Rust source mapping and rustc diagnostics preserved?
5. What disk-state cleanup/race/corruption rules are needed?
6. Does the measured gain justify implementation complexity versus exact cache reuse alone?

Do **not** fold dependency/package graphs, remote caches, linker replacement, daemon architecture, language runtime changes, or frontend redesign into that first research slice.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input.

If a run is queued or in progress, track that run ID and continue independent work. Failed SHAs remain retained evidence.
