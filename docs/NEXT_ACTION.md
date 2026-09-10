# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-10**

## Current verified main before active PR

Stable `main` before PR #90:

- `5646ad45dd3d6640d147a7fc40bbbda891a540d2`;
- squash merge from PR #88 / #87 linker-candidate research;
- post-merge CI #371 / run `34456073733`: **SUCCESS** on Ubuntu, Windows and macOS;
- post-merge Linker candidate research #6 / run `34456073754`: **SUCCESS**;
- Rust toolchain: **1.98.0**.

Completed build work retained on `main`:

- #79 / PR #81: verified exact unchanged-build artifact reuse;
- #82 / PR #84: rustc incremental research, **REJECT / DEFER**;
- #85 / PR #86: link-time attribution, **FOLLOW-UP-CANDIDATE**;
- #87 / PR #88: alternative linker experiment, **REJECT / DEFER**; current Rust 1.98 `cc -> lld` remains production behavior.

Durable predecessor reports:

- `docs/INCREMENTAL_BUILD_RESEARCH.md`;
- `docs/LINK_TIME_RESEARCH.md`;
- `docs/LINKER_CANDIDATE_RESEARCH.md`.

## Active completion PR — #90 / issue #89

Issue **#89 — release optimization cost v0** has reached its research decision.

Active PR:

- **#90 — `research: measure opt3 versus opt2 build cost`**;
- branch: `research/release-optimization-cost-v0`;
- accepted code/evidence head before documentation synchronization: `0e46279fb1038a90e1aced9dd268a0e61c45a1b1`.

Accepted validation:

- normal CI #373 / run `34456762370`: **SUCCESS** on Ubuntu, Windows and macOS;
- Release optimization research #2 / run `34456762372`: **SUCCESS**;
- artifact: `evo-release-optimization-research-ubuntu-24.04`;
- artifact id: `10143822084`;
- digest: `sha256:8b6ea8af1b1e65a321b4d958adab646b29517181621566531f7a22e524a2622e`;
- correctness: **PASS**;
- artifact JSON parses successfully.

Durable report added by the documentation synchronization commit: `docs/RELEASE_OPTIMIZATION_RESEARCH.md`.

## #89 decision

**REJECT / DEFER lowering production optimization from opt3 to opt2 under the current single-file architecture/toolchain setup.**

Controlled accepted medians:

| Case | opt3 current | opt2 candidate | Saved | Improvement | Verdict |
| --- | ---: | ---: | ---: | ---: | --- |
| `enums-v0` | 109.425 ms | 109.036 ms | 0.389 ms | 0.36% | `BUILD-GATE-FAIL` |
| `logical-operators-v0` | 109.784 ms | 109.726 ms | 0.058 ms | 0.05% | `BUILD-GATE-FAIL` |

Both arms are stable; relevant relative MAD values are below 0.01. The pre-registered advancement gate required at least **5% and 5 ms** total compile+link reduction on **both** initial cases.

The candidate misses that gate by a wide margin. Per the pre-registered plan, do **not** run the seven-case runtime corpus for opt2 and do not change production `opt-level=3`.

## Immediate sequence

1. Keep PR #90 on the single documentation-synchronized final head created after accepted head `0e46279f...`.
2. Track the natural exact-head normal CI and Release optimization research workflows. Do not create duplicate runs.
3. Require final PR #90 normal CI to be green on Ubuntu, Windows and macOS.
4. Require the final-head dedicated research workflow to remain green and retain its artifact.
5. Update PR #90 / #89 with final-head provenance and confirm the final artifact still reports `REJECT-DEFER`.
6. Squash-merge PR #90 with expected-head protection only after both exact-head workflows are accepted.
7. Track the natural post-merge `main` CI and dedicated research push workflow.
8. Close #89 as completed research with **REJECT / DEFER** retained only after post-merge `main` CI succeeds.
9. Re-read live `main`, #91, branches/PRs and active Actions.
10. Only then create the #91 research branch from the exact verified `main` SHA.

## Gated successor P0 — #91

Issue **#91 — `P0 research compile memory baseline v0: attribute frontend and rustc peak RSS`** is open and gated on PR #90 merge + green post-merge main CI.

Why this is next:

- unchanged build latency is already solved by verified artifact reuse;
- tested rustc incremental configurations were rejected/deferred;
- current lld beats tested linker alternatives;
- opt2 gives no material total-build improvement versus opt3;
- current architecture still has a directly measurable Phase 3.4 resource question: compile-time memory.

## First #91 research sequence

1. Prove the memory measurement method before treating any RSS number as evidence.
2. Controlled platform: Ubuntu 24.04, Rust 1.98.0, `x86_64-unknown-linux-gnu`.
3. Preserve production-equivalent edition 2024, opt3, CGU1, current linker, no incremental state.
4. Start with `enums-v0` and `logical-operators-v0`.
5. Separate at least frontend/check, emit-rust and direct rustc peak RSS.
6. Measure full `evo build --no-cache` only if the mechanism defensibly accounts for child/grandchild compiler processes; otherwise mark whole-tree attribution unavailable.
7. Retain at least 5 samples per accepted arm, median/min/max, raw RSS, supporting wall time, exact generated Rust, correctness, tool identity, JSON/CSV/Markdown.
8. If the two cases are too small to distinguish useful memory behavior, expand with a committed deterministic supported-language stress fixture rather than an artificial uncommitted Rust blob.
9. End with FOLLOW-UP-CANDIDATE, EXPAND-CORPUS, or DEFER / NO ACTION from evidence.

Dependency-build, proc-macro and workspace-scaling roadmap items remain structurally deferred until Evolution has a real user-program package/dependency graph.

## Production contracts that remain unchanged

- Evolution syntax and semantics;
- generated Rust;
- ownership/type rules;
- Rust 1.98.0;
- edition 2024;
- opt-level 3;
- codegen-units 1;
- current linker behavior;
- `build-cache-v0` and `run-cache-v0`;
- source mapping and rustc diagnostic remapping;
- #4 runtime parity-or-better threshold;
- no hidden clone/allocation/boxing/dynamic dispatch/runtime metadata.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input.

If a run is queued or in progress, track that exact run ID and continue independent work. Failed SHAs remain retained evidence and are not rerun merely to obtain a friendlier color.
