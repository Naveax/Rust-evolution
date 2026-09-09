# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-09**

## Current PR / exact evidence

Active research PR: **#84 — `research: measure changed-source rustc incremental reuse`**.

Current research branch: `research/incremental-build-v0`.

Accepted code/evidence head before the final documentation sync:

- `bbdec7f5f6ec4668849212f49bc07c49196024e2`
- CI **#355** / run `34356624339`: **SUCCESS** on Ubuntu, Windows and macOS
- Ubuntu incremental research artifact: `evo-incremental-build-research-ubuntu-latest`
- artifact id: `10106092707`
- digest: `sha256:217d9849e7e88f8ee05ebd408c0c9b2afaea57f3925f42d099b040dba7eb3a9c`
- Rust toolchain: **1.98.0**

The current PR contains research/CI/docs only. It does not change production compiler, cache, language, generated-Rust, source-map, diagnostic-remapping or runtime behavior.

## #82 final decision

Issue **#82 — changed-source incremental build v0** has completed its research decision:

**REJECT / DEFER production rustc incremental state under the current architecture/toolchain configuration.**

Durable evidence: `docs/INCREMENTAL_BUILD_RESEARCH.md`.

Key findings:

1. Current production-equivalent CGU1 + persistent rustc incremental state made the deterministic edited compile path about **9.28% slower** than its matching no-incremental control.
2. CGU256 + persistent incremental state improved the matching edited compile median from **107.522 ms** to **97.167 ms**, a **1.107x** matching-control speedup.
3. `-C incremental` itself changed optimized binary/codegen identity on the controlled Enums fixture; the difference was not stale edit-history reuse.
4. The favorable Enums runtime effect did not generalize across the committed runtime corpus.
5. Final 7-case corpus aggregate decision was **REJECT**. The clearest regression was `logical-operators-v0`: candidate/reference `1.019303`, candidate/current `1.018570`.
6. The project runtime invariant does not allow compile-time wins to purchase repeatable generated-program runtime regressions.

Because the runtime gate already fails, #82 does not proceed into production invalidation, corruption fallback, race/cleanup or diagnostic/source-map integration design.

## Immediate sequence

1. Keep PR #84 on the documentation-synchronized final head and track its one natural exact-head CI run. Do not create a duplicate Action for the same SHA/workflow/input.
2. Require final PR #84 CI to be green on Ubuntu, Windows and macOS.
3. Squash-merge PR #84 with expected-head protection only after exact-head CI is green.
4. Track the one natural post-merge `main` CI run; do not start the successor branch before it is green.
5. Close #82 as **completed research with REJECT/DEFER decision** after the merge and post-merge validation are accepted.
6. Then start **#85 — P0 research link-time baseline v0: split rustc backend/codegen from linker cost** from the exact verified `main` SHA.

## Successor P0 — #85 link-time attribution

Why #85 is next:

- #76 already showed the frontend is roughly ~1 ms while native rustc compile+link dominates uncached builds;
- #79 solves exact unchanged builds by verified artifact reuse;
- #82 rejects the tested persistent rustc incremental configurations because the viable compile-time candidate violates runtime parity;
- the current direct-rustc evidence still measures **compile/codegen + link together**, so the final-link share is unknown.

Dependency-build and proc-macro roadmap items are not yet directly measurable for Evolution user programs because the current user-program compiler path is still one generated `main.rs` passed directly to rustc with no Evolution package/dependency graph.

## #85 start gate

Do **not** create a #85 branch until all of these are true:

- PR #84 merged;
- #82 final decision recorded;
- live `main` SHA re-read from GitHub;
- natural post-merge main CI is SUCCESS;
- no duplicate open issue/branch/PR covers the same link-time experiment;
- no active same-SHA/workflow Action is duplicated.

## First #85 research sequence

1. Re-read #85 and the current production rustc invocation from `crates/evo-cli/src/main.rs`.
2. Use pinned Rust 1.98.0 evidence, not remembered compiler flags.
3. Build a measurement-only harness that separates, where defensible:
   - normal production-equivalent native compile + link;
   - object/pre-link emission under equivalent optimization/codegen settings;
   - explicit final-link work using the actual observed linker/toolchain path.
4. Record exact commands, linker identity, source SHA, target/host, object/native sizes and raw timing samples.
5. Require final native output correctness before accepting timing evidence.
6. Use more than one committed program shape if the phase split appears workload-sensitive.
7. End with FOLLOW-UP EXPERIMENT only if link cost is material and intervention risk is bounded; otherwise record NO ACTION / DEFER.
8. Do not change the default linker in the measurement issue.

## Production contracts that remain unchanged

- `build-cache-v0` exact artifact reuse remains accepted.
- `run-cache-v0` remains independent.
- production rustc flags remain edition 2024, opt-level 3, codegen-units 1 unless a later separately proven change lands.
- generated Rust/source maps/diagnostic remapping remain unchanged.
- the #4 runtime parity-or-better invariant remains non-negotiable.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input.

If a run is queued or in progress, track that run ID and continue independent work. Failed SHAs remain retained evidence and are not rerun merely to obtain a friendlier color.
