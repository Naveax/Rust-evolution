# Rust Evolution — Project State

Last verified update: **2026-08-27**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- `main` head after Records production-lowering merge: `9f55b3ac44f17c52cbda5f5d6a5a626075e9a6e8` (`feat: add Records v0 typed lowering and static codegen`).
- Post-merge `main` CI #195 / run `33073491890`: **SUCCESS** on Ubuntu, Windows, and macOS.
- Ubuntu #195 passed format, Clippy, workspace tests, benchmark smoke, all pre-Records runtime gates, and release build.
- PR #48 is merged and typed-lowering/ownership issue #46 is closed as completed.

## Active P0

**Parent #41 — Records v0: typed product data**

Only the parity/acceptance PR remains:

### PR #49 — Records v0 parity / acceptance evidence

- Branch: `feature/records-parity-v0`
- Target after unstacking: `main`
- Pre-clean validation head: `007655f3056616df26c01941e21a93ecce2e8f1b`.
- CI #194 / run `33073047724`: **SUCCESS** on Ubuntu, Windows, and macOS with the Ubuntu Records v0 performance gate green.
- Because PR #48 was squash-merged, the old stacked branch ancestry diverges from `main` even though its base tree content is equivalent. Normalize PR #49 onto `main` before final review so GitHub does not re-display the already-merged #48 history.
- Preserve the accepted PR #49 tree contents; ancestry cleanup is not a semantic rewrite.

## Records v0 accepted behavior

The merged production foundation plus PR #49 contain:

- nominal record declarations and named record types;
- validated field definitions retaining schema order and source spans;
- exact named constructors with deterministic declaration-order lowering;
- zero-field `Name()` constructors;
- typed direct/chained scalar field access;
- record parameters and return values;
- Rust-style by-value record moves with source-native reuse-after-move diagnostics;
- same-type explicit reinitialization after move;
- conservative `if` ownership joins;
- `repeat` loop-carried move safety;
- explicit rejection of whole-record print/equality and record-valued partial field moves;
- static Rust structs, struct literals, direct field access, and by-value record signatures;
- CLI `check`, `emit-rust`, `build`, and native execution for valid record programs;
- record declaration/field source mapping plus statement-level constructor/access mapping regressions;
- native process coverage for nested records, chained access, zero-field record roundtrip, record return, explicit reinitialization, and rejected builds producing no binary;
- `docs/LANGUAGE_SPEC_V0.md` describing the accepted Records v0 grammar, semantics, codegen, source-map policy, ownership limits, and parity evidence.

## Zero-cost evidence

Records v0 remains a **ZERO** cost-class feature. The generated path adds no hidden allocation, boxing, GC/RC, implicit clone, dynamic dispatch, runtime object map, or reflection metadata.

Dedicated `records-v0` differential evidence from CI #190 / run `33071967025`, artifact `9646205940`:

- correctness: **PASS**;
- normalized LLVM IR equality: **true**;
- exact executable equality: **true**;
- binary size: **2,267,104 B / 2,267,104 B**;
- median reference time: **18,806,589 ns**;
- median Evolution time: **18,810,846 ns**;
- observed timing ratio: **1.000226357**;
- timing-only verdict: **FAIL**;
- final verdict: **PASS**;
- verdict basis: **`byte-identical-binary-parity`**.

The raw timing remains visible, but byte-identical executables after correctness PASS establish deterministic runtime parity under the project benchmark policy.

## Validation chain

- PR #48 head `7d72530169cd8180033056a70730eb6523e9d6ed`: CI #189 / run `33071591641` green.
- PR #48 squash merge: `9f55b3ac44f17c52cbda5f5d6a5a626075e9a6e8`.
- Post-merge main CI #195 / run `33073491890`: green.
- PR #49 benchmark head: CI #190 / run `33071967025` green.
- PR #49 expanded native/source-map corpus: CI #192 / run `33072503739` green.
- PR #49 accepted language spec: CI #193 / run `33072815054` green.
- PR #49 pre-clean current tree: CI #194 / run `33073047724` green.

## Remaining P0 lifecycle work

No additional Records v0 implementation slice should be invented before the lifecycle is closed.

1. Normalize PR #49 branch ancestry onto `main` while preserving the validated parity tree and updated handoff docs.
2. Retarget PR #49 to `main`.
3. Verify the resulting PR diff contains only the Records parity benchmark/gate, source-map/native regressions, spec, and handoff changes.
4. Track the single CI run for the cleaned head; do not rerun or duplicate it.
5. Mark PR #49 ready after green CI.
6. Squash-merge PR #49 through the normal authorized repository path.
7. Verify post-merge `main` CI including the new Ubuntu Records v0 performance gate.
8. Close #47 and parent #41 only after final `main` evidence is green.
9. Refresh durable handoff one last time on `main`, then select the next roadmap feature from current GitHub state.

## Durable continuation infrastructure

Read order:

1. `AGENTS.md`
2. `docs/PROJECT_STATE.md`
3. `docs/NEXT_ACTION.md`
4. active issue/PR/Actions referenced there

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.

## Handoff invariant

Every significant merge or incomplete stopping point must keep `PROJECT_STATE.md`, `NEXT_ACTION.md`, issue/PR evidence, and durable decisions synchronized with GitHub reality.

The repository is the project memory. The chat transcript is not.