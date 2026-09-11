# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-11**

## Stable predecessor gate

PR #101 (`research: classify lifetime elision feasibility v0`) squash-merged to `main` as:

`3f501b51c79c39241107df1cbe098acb6ec058ad`

Natural exact-SHA post-merge validation succeeded:

- CI #410 / run `34581869543`: **SUCCESS** on Ubuntu, Windows and macOS;
- Lifetime elision research #4 / run `34581869494`: **SUCCESS**;
- post-merge artifact id `10192045985`;
- digest `sha256:2a21e3a591a33bcf07fc36bb2b2c29c2bfed15234a4a3e30fdc010e02c8e976e`;
- verdict remains **REFERENCE-SURFACE-FIRST** with zero compile-expectation mismatches.

Issue #100 is complete. The required bounded successor is the current #102 / PR #103 immutable-reference-surface research.

## Active research — #102 / PR #103

Issue #102 researches the first caller-visible immutable reference / borrowed-result surface before any production escaping-borrow implementation.

PR:

- #103 `research: classify immutable reference surface v0`;
- branch: `research/immutable-reference-surface-v0`;
- accepted research code/evidence head before documentation synchronization: `7664d28dc0865ef4442063ed702875302271087c`.

This PR remains research/design evidence only. No production parser, lowering, ownership, codegen, runtime or accepted-program behavior changes.

## Accepted research result

Immutable reference surface research #3 / run `34598646494` on exact head `7664d28dc0865ef4442063ed702875302271087c`: **SUCCESS**.

Artifact:

- `evo-immutable-reference-surface-research-ubuntu-24.04`;
- id `10263292508`;
- digest `sha256:59535a36276e7c5903d37c1b271128bf54de9e0af894d61ecb7052f52428d1f8`.

Normal CI #413 / run `34598646488` on the same exact head: **SUCCESS** on Ubuntu, Windows and macOS.

Result:

- verdict **SURFACE-CANDIDATE**;
- recommended surface **PUNCTUATION-AMPERSAND**;
- semantic cases: **19**;
- compile-expectation mismatches: **0**.

Surface comparison:

- punctuation `&T` / `&expr`: one new lexer token, zero reserved current identifiers, direct safe Rust lowering;
- keyword `ref T` / `borrow expr`: would reserve two currently-valid identifiers;
- generic-like `Ref(...)`: collides with current nominal/call-shaped identifier space and is not direct Rust syntax.

The semantic matrix also confirms the bounded ownership rule needed by the successor: owner move/reinitialization while a later reference use remains must fail, while owner move after the final reference use is valid. Multi-owner returned-reference relationships and references to dead local owners fail closed.

Durable report: `docs/IMMUTABLE_REFERENCE_SURFACE_RESEARCH.md`.

## Recommended bounded production model

The report records the production-successor recommendation without making it current language behavior:

- explicit `&T` immutable-reference type surface;
- explicit `&expr` borrow expression;
- dedicated syntax/semantic reference variants rather than reinterpreting owned `T`;
- single deterministic owner/source provenance for v0;
- first-class immutable-reference locals;
- bounded local last-use liveness so owners may move after the final proven reference use;
- conservative fail-closed behavior where liveness/provenance is ambiguous;
- existing inferred `SharedBorrow` remains a separate call-duration passing mode;
- direct safe Rust `&T` / `&expr` codegen;
- no mutable references, generalized lifetime syntax/solver, unsafe widening, hidden clone, allocation, RC/GC or runtime ownership map.

## Failed-SHA evidence retained

Initial research head `6f9f9f10c61f2b01ca44ac02a46106c8af5c604c` produced successful dedicated evidence, but normal CI #411 / run `34583343742` failed only rustfmt. That SHA was not rerun.

Format-only head `733f7efe154422ac0e5c84ce500204b62cf84f51` preserved the matrix and produced successful immutable-reference research #2 / run `34583596906`, but normal CI #412 / run `34583596924` exposed one research-harness Clippy failure: `write_reports` had 9 parameters under `-D warnings`.

The fix on `7664d28d...` is deliberately local to that research helper. It does not relax workspace lints or change the research matrix.

## Immediate sequence

1. This documentation synchronization must be one semantic-neutral commit containing `IMMUTABLE_REFERENCE_SURFACE_RESEARCH`, this file and `PROJECT_STATE`.
2. Track only the natural normal CI and Immutable reference surface research runs created for that exact documentation head; do not dispatch duplicates.
3. Require both final-head workflows **SUCCESS** and validate that the final artifact still reports `SURFACE-CANDIDATE`, `PUNCTUATION-AMPERSAND`, 19 semantic cases and zero mismatches.
4. Live-check PR #103 head/base/mergeability and changed files.
5. Squash-merge PR #103 with expected-head protection only from that validated exact head.
6. Track the natural post-merge `main` CI and Immutable reference surface research push workflow on the exact merge SHA.
7. Close #102 completed only after both exact-SHA post-merge workflows succeed.
8. Update living meta #40 with the durable report and verified main provenance.
9. Re-read live roadmap/issues/branches/PRs, then atomize the bounded production implementation successor from the accepted report.

## Successor direction after #102 closes

The next production slice should implement only the accepted immutable-reference v0 boundary: caller-visible `&T` / `&expr`, deterministic single-source provenance, source-native borrow-vs-move/reinit diagnostics, local stored references and bounded last-use liveness.

It must preserve current owned APIs and existing call-duration `SharedBorrow`. Multiple-source returned-reference relationships remain rejected until separately researched. Mutable references, generalized lifetime syntax/inference, reference fields, unsafe lifetime extension and runtime ownership machinery remain outside this successor.

## Production contracts

Rust remains pinned to **1.98.0**, edition 2024, opt-level 3 and codegen-units 1. `build-cache-v0` / `run-cache-v0`, source mapping, diagnostic remapping, bounded inferred shared borrowing and the #4 runtime parity-or-better contract remain unchanged.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Failed SHAs remain evidence and are not rerun merely for a better color.