# Incremental build research v0

Status: **REJECT / DEFER for production**

Issue: #82  
Research PR: #84  
Toolchain: Rust **1.98.0**

## Purpose

This document records why persistent rustc incremental state is not being adopted in the current Rust Evolution native build path.

The research was intentionally evidence-first. It did not change production compiler, cache, language, generated Rust, source-map, diagnostic-remapping or runtime behavior.

## Existing build baseline

The predecessor build work established two important facts:

- #76 deterministic edited uncached `evo build`: **95.515 ms** median, rustc invoked for every measured build;
- #79 verified exact unchanged-build artifact reuse: unchanged warm build can avoid rustc entirely.

The remaining question was whether persistent rustc state could materially improve a build after generated Rust changes.

## Slice 1: current production codegen-unit strategy

Accepted CI #343 / run `34351155816`, head `0e27b5156b242e62fe9f39f1ae015eddac629dff`.

Artifact:

- id `10103828414`
- digest `sha256:e9d90bc37e46e4df3505958dd5d0b0b5c48dbdb35196f15e2b9f1b1c13c7ea24`

Edited medians:

| Arm | Median |
| --- | ---: |
| current `evo build --no-cache` | 110.835 ms |
| stable direct rustc, no incremental | 110.763 ms |
| stable direct rustc, persistent incremental, CGU1 | 121.040 ms |

Result: persistent incremental state with the current production `codegen-units=1` strategy was about **9.28% slower** than its matching stable-path control.

Decision: **reject CGU1 incremental state**.

## Slice 2: isolate codegen-unit granularity

Accepted CI #346 / run `34352132387`, head `781a88db475a44d8e5d71a615c505fb4b0f71075`.

Artifact:

- id `10104176757`
- digest `sha256:530f361dbdbabeafb196376d12521ade7c2e28fb80b30871f036f4e152262f2d`

Edited medians:

| Arm | Median |
| --- | ---: |
| stable direct rustc, CGU256, no incremental | 107.522 ms |
| stable direct rustc, CGU256, persistent incremental | 97.167 ms |

The matching-control speedup was **1.107x**, about **9.63% lower median latency**. Incremental state grew from roughly 19 files / 966.6 KiB after prime to 38 files / 1.936 MiB after the edit.

This justified runtime-quality research, not production adoption, because production used CGU1 and a different CGU/incremental configuration can change optimized output.

## Slice 3: single-fixture runtime quality

Accepted CI #350 / run `34354081684`, head `494657909fdf49c3cc90c1e45434c8745612fd5f`.

Artifact:

- id `10105032923`
- digest `sha256:c82203ebe5b6bbcfc387d999ffa8c85a5a8aa741277a8857ade267deb4666eb9`

Enums-v0 runtime medians:

| Arm | Median |
| --- | ---: |
| reference Rust, CGU1 | 16.539 ms |
| current Evolution generated Rust, CGU1 | 16.530 ms |
| Evolution generated Rust, CGU256, no incremental | 16.543 ms |
| Evolution generated Rust, CGU256 + incremental | 13.337 ms |

The candidate was about 19% faster on this fixture, but its binary differed from the matching CGU256 no-incremental binary. That result could not safely be generalized.

## Slice 4: isolate incremental mode from edit-history reuse

Accepted CI #352 / run `34354989080`, head `90fc59af42d5d70b50bb8456949d10ac117432ba`.

Artifact:

- id `10105425903`
- digest `sha256:a0f24eabb0cc3b78830fe55f297ac0afe633e1dcaa363d084d3ef72f697eb446`

Controlled same-path medians were approximately:

- baseline CGU256, no incremental: **16.450 ms**;
- edited CGU256, no incremental: **16.449 ms**;
- baseline CGU256, fresh incremental prime: **13.252 ms**;
- edited CGU256, reused incremental session: **13.263 ms**.

Binary identity showed:

- baseline no-incremental == edited no-incremental;
- incremental prime == incremental edited/reuse;
- no-incremental != incremental-mode binary;
- about **3,618,404** byte positions differed.

Conclusion: the runtime/codegen divergence was caused by enabling `-C incremental` itself, not by stale edited-state reuse.

## Slice 5: committed runtime corpus

Final accepted research head before documentation:

- head `bbdec7f5f6ec4668849212f49bc07c49196024e2`;
- CI #355 / run `34356624339`: **SUCCESS** on Ubuntu, Windows and macOS;
- all existing Ubuntu turnaround, build-cache and runtime/performance gates remained green.

Artifact:

- `evo-incremental-build-research-ubuntu-latest`;
- id `10106092707`;
- digest `sha256:217d9849e7e88f8ee05ebd408c0c9b2afaea57f3925f42d099b040dba7eb3a9c`;
- correctness: **PASS** for every measured binary;
- 7 committed performance cases;
- 3 warmups + 21 samples per arm/case;
- aggregate candidate decision: **REJECT**.

| Case | Reference | Current CGU1 | Candidate CGU256+incremental | Candidate/reference | Candidate/current | Verdict |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| runtime-repeat-v0 | 15.080 ms | 15.097 ms | 15.068 ms | 0.999231 | 0.998121 | PASS |
| control-flow-branch-v0 | 19.963 ms | 19.978 ms | 20.000 ms | 1.001843 | 1.001111 | FAIL |
| logical-operators-v0 | 27.020 ms | 27.040 ms | 27.542 ms | 1.019303 | 1.018570 | FAIL |
| function-call-v0 | 28.126 ms | 28.118 ms | 28.114 ms | 0.999582 | 0.999869 | PASS |
| block-locals-v0 | 19.897 ms | 19.912 ms | 19.898 ms | 1.000019 | 0.999306 | FAIL |
| records-v0 | 21.296 ms | 21.274 ms | 21.293 ms | 0.999854 | 1.000877 | FAIL |
| enums-v0 | 18.712 ms | 18.743 ms | 15.071 ms | 0.805402 | 0.804054 | PASS |

The Enums-v0 gain did not generalize. The clearest regression was `logical-operators-v0`, where the candidate median was about **1.93% slower than reference** and **1.86% slower than current Evolution**.

The project runtime contract does not allow a compile-time improvement to purchase a repeatable generated-program runtime regression.

## Final decision

**REJECT / DEFER production rustc incremental state under the current architecture and Rust 1.98.0 configuration.**

Reasons:

1. CGU1 incremental state makes the changed-source compile path slower.
2. CGU256 incremental state produces a useful matching-control compile-time improvement, but changes optimized codegen identity.
3. The favorable Enums-v0 runtime effect is workload-specific.
4. The broader committed corpus contains runtime regressions and therefore fails the project's parity-or-better invariant.
5. The runtime gate fails before a production state/invalidation design is justified.

## Production behavior remains unchanged

PR #84 is research and CI evidence only.

It does **not** change:

- Evolution language syntax or semantics;
- ownership/type rules;
- generated Rust emitted by accepted production paths;
- `build-cache-v0`;
- `run-cache-v0`;
- production rustc flags;
- source mapping;
- rustc diagnostic remapping;
- runtime thresholds;
- linker choice;
- package/dependency behavior.

The direct-rustc experiment arms bypass the production diagnostic-remapping path and are measurement-only. Because the candidate already fails the runtime gate, further production corruption/race/invalidation/source-map integration work is intentionally not pursued in #82.

## Revisit condition

Do not reopen the same approach merely because a future runner produces a friendlier Enums timing.

A future incremental-build proposal needs **materially new evidence or mechanism**, including both:

- a stable changed-source build improvement;
- preserved runtime parity across the relevant committed corpus.

Only then should persistent-state invalidation, corruption fallback, race/cleanup and production diagnostic/source-map integration be researched again.

## Successor

The next current-architecture P0 build/compile research issue is **#85 — link-time baseline v0**.

Dependency-build and proc-macro roadmap items remain deferred until Evolution has a real user-program package/dependency graph to measure. #85 instead separates the currently observable direct-rustc compile/codegen cost from final link cost before anyone starts swapping linkers because that sounds fashionable.
