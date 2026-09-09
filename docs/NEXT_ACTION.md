# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-08**

## Verified merged baseline

Verified build artifact reuse v0 **#79 is completed** and PR **#81 is merged**.

Latest verified code-bearing `main`:

- `07e85b3a60739f2d1f25caed0fcb622dd8894861`
- PR #81 final head `4288ddcfccf07fcab60d27e9677685b213005ae2`
- final PR CI **#338** / run `34214947823`: **SUCCESS** on Ubuntu, Windows and macOS
- post-merge main CI **#339** / run `34215424678`: **SUCCESS** on Ubuntu, Windows and macOS
- Rust toolchain: **1.98.0**

A later docs-only handoff merge may advance live `main` beyond the code-bearing SHA above. Always verify live GitHub, open PRs and active Actions before implementation/research work.

## Accepted #79 build-cache evidence

Final Ubuntu artifact:

- `evo-build-cache-turnaround-ubuntu-latest`
- artifact id `10051449726`
- digest `sha256:cf7295bf371695347300bee325e3a5a4b5a96bbf4c8789625904d66bd4c538e9`
- fixture `benchmarks/cases/enums-v0/evolution.evo`

Controlled final-head values:

- cold median: **128.454 ms**
- unchanged warm cached median: **17.177 ms**
- cold-to-warm speedup: **7.478x**
- accepted #76 warm uncached baseline: **96.986 ms**
- accepted-baseline-to-cached speedup: **5.646x**
- cold rustc compile count: **5**
- warm rustc compile count: **0**
- correctness: **PASS**

Exact retained generated Rust:

- 1240 bytes
- SHA-256 `61f5f5c99c47196605ae2e461ee589b72a722c4ed5107c6b5fca353795100d83`
- matches accepted Enums v0 / #76 evidence.

Hard acceptance is zero warm rustc compilation plus correct native output. Timing is supporting evidence.

The #76 build-latency harness now invokes `evo build ... --no-cache`, preserving its original uncached baseline semantics after cached build became the default path.

## Active P0 research — #82

Issue **#82 — P0 research changed-source incremental build v0: measure rustc session reuse** is open.

Parent: #2

Weakness source: #6 Build / Compile.

Roadmap: #1 Phase 3.4 warm build / incremental build.

Evidence source:

- exact unchanged builds are now solved by #79: warm cached **17.177 ms**, rustc count **0**;
- #76 deterministic generated-Rust-changing edit remains **95.515 ms** with rustc invoked **5/5** times;
- direct rustc compile/link baseline is **94.131 ms**.

#82 is research/measurement first. It is **not** permission to ship production incremental-build state merely because rustc has an option with an encouraging name.

## Start gate for #82 branch work

This docs-only handoff must first be merged and its natural post-merge `main` CI must be green.

Only then:

1. Verify live `main`, #82, open PRs and all queued/in-progress Actions.
2. Confirm there is no duplicate research/incremental issue or active branch/PR covering the same experiment.
3. Create a focused research branch from the exact verified `main` SHA.
4. Do not alter production `build-cache-v0` behavior as part of the first experiment.

## First #82 research sequence

1. Inspect the pinned Rust **1.98.0** toolchain directly and record the exact supported rustc incremental/session mechanisms and flags. Do not infer them from memory or another rustc release.
2. Reuse the accepted #76 Enums v0 fixture and deterministic result-preserving edit:
   - fixture: `benchmarks/cases/enums-v0/evolution.evo`;
   - stdin: `20000000\n9\n`;
   - expected stdout: `15099959897\n`;
   - edit one `sum = sum + value` to `sum = value + sum`;
   - require generated Rust to change while native output remains correct.
3. Build a research-only harness that isolates experiment arms rather than changing several variables at once:
   - current fresh-workdir / no persistent compiler-session baseline;
   - stable generated-source/work path without reusable incremental state, if needed to isolate path effects;
   - persistent rustc incremental/session state with otherwise equivalent successful-build flags.
4. Preserve the existing frontend path and measure it separately where relevant. Do not skip validation/lowering/codegen to manufacture a favorable rebuild number.
5. Record exact rustc command lines/flags and `rustc -vV` for every arm.
6. Count rustc compile invocations independently of timing.
7. Execute every measured native artifact with the committed fixture input and require exact expected output before accepting timing evidence.
8. Retain baseline and edited generated Rust artifacts so source-map/diagnostic compatibility can be reviewed.
9. Record persistent state evidence when applicable:
   - directory size;
   - high-level file count/growth;
   - reuse across edits;
   - corruption/mismatch behavior where safely testable;
   - cleanup/invalidation requirements.
10. Emit raw samples CSV, JSON and Markdown evidence. Keep unfavorable/outlier samples visible.
11. Compare edited rebuild medians against the accepted #76 **95.515 ms** edit baseline and against an appropriate direct-rustc arm.
12. Review diagnostics/source mapping explicitly. A latency win that breaks source-native rustc remapping is not an accepted win.
13. Keep normal three-OS fmt/Clippy/workspace validation and every existing Ubuntu turnaround/runtime/performance gate green for any research PR that changes repository code.

## #82 decision gate

The research may finish either way.

### IMPLEMENT

Atomize a separate production implementation issue only if evidence shows:

- stable, material changed-source improvement;
- correct native output;
- a bounded compiler-state/invalidation identity;
- safe corruption/mismatch fallback;
- acceptable race/cleanup behavior;
- preserved diagnostics/source mapping;
- no mandatory daemon/service requirement;
- complexity justified by measured gain.

### REJECT / DEFER

Record the negative result and stop if the gain is weak/unstable, toolchain-fragile, requires disproportionate persistent state, needs a mandatory daemon/compiler service, or breaks diagnostics/source mapping.

A roadmap checkbox is not a performance result.

## Explicit non-goals for #82

Do not fold these into the research slice:

- compiler daemon/server architecture;
- remote cache or executable download;
- linker replacement;
- Cargo/dependency graph or package-system work;
- multi-crate incremental semantics;
- frontend redesign/micro-optimization;
- hot reload/runtime code replacement;
- Evolution language/runtime changes;
- changes to accepted `build-cache-v0` or `run-cache-v0` contracts unless a later implementation issue explicitly proves and scopes them.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input.

If a run is queued or in progress, track that run ID and continue independent work. Failed SHAs remain retained evidence and are not rerun merely to obtain a friendlier color.
