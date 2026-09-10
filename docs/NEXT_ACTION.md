# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-10**

## Current verified main before active PR

Stable `main` before PR #88:

- `b14173c8add554963df7ccb0f48de41e72fa3136`
- squash merge from PR #86 / #85 link-time research
- post-merge CI #365 / run `34362873485`: **SUCCESS** on Ubuntu, Windows and macOS
- post-merge Link-time research #6 / run `34362873679`: **SUCCESS**
- Rust toolchain: **1.98.0**

Completed build work already retained on `main`:

- #79 / PR #81: verified exact unchanged-build artifact reuse;
- #82 / PR #84: rustc incremental research, **REJECT / DEFER**;
- #85 / PR #86: link-time attribution, **FOLLOW-UP-CANDIDATE**;
- durable predecessor reports: `docs/INCREMENTAL_BUILD_RESEARCH.md` and `docs/LINK_TIME_RESEARCH.md`.

## Active completion PR — #88 / issue #87

Issue **#87 — linker candidate v0** has reached its research decision.

Active PR:

- **#88 — `research: compare current lld path with bounded linker candidates`**
- branch: `research/linker-candidate-v0`
- accepted code/evidence head before documentation synchronization: `852bc56eeeb69123b5a7f9c120338ab2c966a35b`

Accepted exact-head validation:

- normal CI #369 / run `34453683622`: **SUCCESS** on Ubuntu, Windows and macOS;
- dedicated Linker candidate research #4 / run `34453683646`: **SUCCESS**;
- artifact: `evo-linker-candidate-research-ubuntu-24.04`;
- artifact id: `10142574260`;
- digest: `sha256:d550ce688c1979c103df8ab3d2260ec136552672268d32f5be5d683e4c3ab7e1`;
- correctness: **PASS**;
- report JSON independently parses successfully.

Durable report: `docs/LINKER_CANDIDATE_RESEARCH.md`.

## #87 final research decision

**REJECT / DEFER linker replacement under the current single-file architecture/toolchain setup.**

Final controlled medians:

| Case | Current lld full | GNU ld full | Mold full | Mold vs lld |
| --- | ---: | ---: | ---: | ---: |
| `enums-v0` | 113.077 ms | 165.612 ms | 122.321 ms | 1.08175 / ~8.17% slower |
| `logical-operators-v0` | 111.557 ms | 163.700 ms | 119.199 ms | 1.06850 / ~6.85% slower |

Supporting evidence:

- current lld link-child median: 24.727 / 24.457 ms;
- mold link-child median: 33.401 / 32.316 ms;
- every measured arm invokes the linker driver exactly once;
- relative MAD values are comfortably below the 0.10 instability ceiling;
- baseline and mold `DT_NEEDED` names match exactly;
- pinned candidate: Ubuntu Noble `mold=2.30.0+dfsg-1build1`;
- aggregate verdict: `MOLD-REJECT-OR-DEFER`.

The candidate fails the **total build** gate before runtime-corpus adoption validation. Do not expand this mold configuration into the seven-case runtime suite and do not change the production linker.

## Immediate sequence

1. Keep PR #88 on the single documentation-synchronized final head created after the accepted `852bc56e...` evidence.
2. Track the natural exact-head normal CI and Linker candidate research workflows. Do not create duplicates for the same SHA/workflow/input.
3. Require final PR #88 normal CI to be green on Ubuntu, Windows and macOS.
4. Require the final-head dedicated candidate workflow to remain green and retain its artifact.
5. Update PR #88 / #87 with final-head provenance if the docs head changes the artifact SHA.
6. Squash-merge PR #88 with expected-head protection only after both final-head workflows are accepted.
7. Track the natural post-merge `main` CI; do not start #89 before it is SUCCESS.
8. Close #87 as completed research with **REJECT / DEFER** retained.
9. Re-read live `main`, open PR/issue/branch state and active Actions.
10. Only then create the #89 research branch from the exact verified `main` SHA.

## Gated successor P0 — #89

Issue **#89 — `P0 research release optimization cost v0: measure opt-level 3 build/runtime tradeoff`** is open and gated on PR #88 merge + green post-merge main CI.

Why #89 is next:

- frontend/check/emit cost is already small relative to native rustc work;
- exact unchanged builds are solved by #79;
- tested persistent incremental approaches are rejected/deferred by #82;
- the current Rust 1.98 `cc -> lld` path beats the tested GNU ld and mold alternatives;
- final #87 evidence leaves roughly 86–88 ms outside the direct linker-child median;
- production explicitly uses `-C opt-level=3`.

## First #89 research sequence

1. Re-read live production rustc invocation from `crates/evo-cli/src/main.rs`; do not rely on remembered flags.
2. Preserve Rust 1.98.0, edition 2024, codegen-units 1, current lld path, source/path policy and no incremental state.
3. Compare current `opt-level=3` against a bounded `opt-level=2` candidate on `enums-v0` and `logical-operators-v0`.
4. Measure total compile+link latency first. Candidate must show a stable material win, initially at least 5% and 5 ms on every initial case, before runtime expansion.
5. Retain raw samples, median/min/max/p95, relative MAD, exact output correctness and binary size.
6. If opt2 clears the build gate, expand to the seven-case runtime corpus.
7. Runtime acceptance then requires both:
   - Evolution opt2 <= equivalent reference Rust opt2 under #4;
   - Evolution opt2 does not show a repeatable runtime regression versus current production-equivalent Evolution opt3.
8. Any stable runtime regression rejects the production candidate even when compilation is faster.
9. Production `RUST_OPT_LEVEL=3` remains unchanged until a separate proven implementation issue exists.

## Production contracts that remain unchanged

- Evolution syntax and semantics;
- generated Rust;
- ownership/type rules;
- Rust 1.98.0 production toolchain;
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
