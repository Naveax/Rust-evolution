# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-09**

## Current verified main

Verified stable `main` before active PR #86:

- `c1ba2f2776a3b00bb5833a525f94e7b3e0cfa16f`
- squash merge from PR #84
- post-merge CI #359 / run `34358142270`: **SUCCESS** on Ubuntu, Windows and macOS
- Rust toolchain: **1.98.0**

#82 changed-source rustc incremental research is closed/completed with a **REJECT / DEFER** production decision. Durable evidence: `docs/INCREMENTAL_BUILD_RESEARCH.md`.

## Active completion PR — #86 / issue #85

Issue **#85 — link-time baseline v0** has reached its research decision.

Active PR:

- **#86 — `research: attribute native build time to linker work`**
- branch `research/link-time-baseline-v0`
- accepted evidence head before final docs synchronization: `121581ed1c105dd30cfdcfeb4567a3a988f20c23`

Accepted validation:

- normal CI #361 / run `34361156009`: **SUCCESS** on Ubuntu, Windows and macOS;
- dedicated Link-time research #2 / run `34361156018`: **SUCCESS**;
- artifact `evo-link-time-research-ubuntu-latest`;
- artifact id `10107871908`;
- digest `sha256:62d454a95fd2a1f0ec7ba3c92b08582a29cb9a237bda8cf78b0113818ccc4162`.

Durable report: `docs/LINK_TIME_RESEARCH.md`.

## #85 decision

**FOLLOW-UP-CANDIDATE.**

Controlled exact-head medians:

| Case | Full rustc | Direct linker-child | Link/full |
| --- | ---: | ---: | ---: |
| `enums-v0` | 115.158 ms | 26.784 ms | 23.259% |
| `logical-operators-v0` | 113.687 ms | 26.393 ms | 23.215% |

The signal is stable, exact-output correctness passes, and every instrumented sample contains exactly one linker-driver invocation.

Important baseline identity:

- driver: `cc` 13.3.0;
- retained argv includes `-fuse-ld=lld`;
- current Ubuntu Rust 1.98.0 path is therefore **`rustc -> cc -> lld`**.

Do not create a successor whose premise is "switch to lld". The project is already measuring lld.

## Immediate sequence

1. Keep PR #86 on its documentation-synchronized final head and track the natural exact-head workflows created by the docs commits. Do not create duplicate runs.
2. Require final PR #86 normal CI to be green on Ubuntu, Windows and macOS.
3. Require the final-head dedicated Link-time research workflow to remain green and retain its artifact.
4. Squash-merge PR #86 with expected-head protection only after both exact-head workflows are accepted.
5. Track the one natural post-merge `main` CI. Do not create the #87 branch before that run is SUCCESS.
6. Close #85 as completed research with the **FOLLOW-UP-CANDIDATE** decision retained.
7. Re-read live main, #87, open PRs/branches and active Actions.
8. Only then create the #87 experiment branch from the exact verified main SHA.

## Successor P0 — #87

Issue **#87 — `P0 experiment linker candidate v0: beat current Rust 1.98 lld path without runtime or deployment regression`** is open and gated on #86 merge + green post-merge main CI.

The experiment is research-first. It must compare the exact current `cc -> lld` baseline with at least one genuinely different reproducible link path/configuration.

Candidate discovery may include:

- stable `-C linker-features=-lld` as a non-lld control;
- a different linker such as `mold` only if its exact version/provisioning can be reproduced;
- another bounded link configuration that changes real link behavior rather than renaming the same lld path.

The exact GitHub Ubuntu runner image used for #85 does not list `mold` as preinstalled. Do not assume availability or make production depend on an ad-hoc network download.

## #87 acceptance sequence

1. Preserve the current Rust 1.98.0 baseline: edition 2024, opt-level 3, codegen-units 1, `x86_64-unknown-linux-gnu`, current `cc -> lld` link path.
2. Record exact candidate version, provisioning source and actual linker argv before measuring it.
3. Start with the same #85 cases: `enums-v0` and `logical-operators-v0`.
4. Measure total production-equivalent native compile+link latency and direct linker-child latency separately.
5. Require exact native output correctness for every candidate artifact.
6. Retain raw samples, median/min/max/p95 or derivable equivalents, and a stability signal such as relative MAD.
7. A candidate only advances if it materially reduces **total build latency**, not merely isolated linker time.
8. If it wins, expand to the committed seven-case runtime corpus and enforce #4 runtime parity-or-better.
9. Inspect binary size, startup/deployment/dynamic dependencies and link-failure behavior before any production proposal.
10. Explicitly address Windows/macOS before a global default-linker proposal.
11. End with IMPLEMENT-CANDIDATE, EXPAND/ITERATE, or REJECT/DEFER from retained evidence.

## Production contracts that remain unchanged

PR #86 and #85 are measurement only. They do not change:

- Evolution syntax or semantics;
- generated Rust;
- ownership/type rules;
- production rustc flags;
- production linker configuration;
- `build-cache-v0` or `run-cache-v0`;
- source mapping or diagnostic remapping;
- runtime thresholds;
- package/dependency behavior.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input.

If a run is queued or in progress, track that exact run ID and continue independent work. Failed SHAs remain retained evidence and are not rerun merely to obtain a friendlier color.
