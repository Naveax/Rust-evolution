# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-10**

## Stable main before active PR #96

- `733900781ea61f5684209570a35e9f63df71a7d4`
- PR #94 / #93 binary-size research merge
- post-merge CI #381 / run `34474194185`: **SUCCESS** on Ubuntu, Windows and macOS
- post-merge Binary size research #3 / run `34474194189`: **SUCCESS**
- #93: closed/completed with **DEFER / NO ACTION** for Evolution-specific binary-size optimization
- Rust toolchain: **1.98.0**

## Active completion — #95 / PR #96

Issue **#95 — borrow inference feasibility v0** has reached an accepted research decision.

PR:

- #96 `research: classify borrow inference feasibility v0`
- branch: `research/borrow-inference-feasibility-v0`
- accepted measurement head before documentation synchronization: `b3ef92be7e273bc4a27812d24733b87c12bd81c3`

Accepted validation:

- CI #384 / run `34478349183`: **SUCCESS** on Ubuntu, Windows and macOS;
- Borrow inference research #3 / run `34478349203`: **SUCCESS**;
- artifact `evo-borrow-inference-research-ubuntu-24.04`;
- artifact id `10152500994`;
- digest `sha256:3ba6965b0ffac03d9e1b74bd82152c4c0cbcf47a5d502f694910483e7f13b6fd`;
- report JSON validation **PASS**;
- aggregate verdict **IMPLEMENT-CANDIDATE**.

Accepted research result:

- **6** representative `SAFE-LOCAL-INFERENCE-CANDIDATE` cases;
- **2** of those reproduce real current move friction;
- owned return, nested current call boundary, owned match scrutinee, parameter reinitialization and repeat-read-then-owned-return remain **KEEP-BY-VALUE**;
- returned/escaping borrow requires a lifetime model;
- mutable borrowing requires an explicit borrow contract;
- stored/overlapping inferred borrow versus move remains unsupported/ambiguous in v0.

The accepted rule is deliberately local and non-transitive: a nominal parameter qualifies only when its own body has at least one current `Inspect`, zero current `Consume` uses and no reinitialization. Nested calls remain consuming boundaries while classifying the caller. No fixpoint or generalized lifetime solving is part of v0.

Durable report: `docs/BORROW_INFERENCE_RESEARCH.md`.

## Failed-SHA evidence retained

Initial research head `5b3a3ee2b5041bba6010c66ea7264fa3522fd9e9` was not rerun:

- CI #382 / run `34475611657` failed only `cargo fmt --check` on one research-test formatting difference;
- Borrow inference research #1 / run `34475611799` successfully executed the semantic test and printed the same `IMPLEMENT-CANDIDATE`, 6-candidate / 2-friction result;
- that dedicated workflow then failed because report output used a package-relative path while validation/upload expected workspace-root `target/`.

Formatting and artifact-path defects were corrected on new SHAs. Exact accepted head `b3ef92be...` completed test + JSON validation + artifact upload successfully.

## Immediate sequence

1. Keep PR #96 on one documentation-synchronized final head.
2. Track only the natural exact-head normal CI and Borrow inference research workflows; do not duplicate them.
3. Require normal CI green on Ubuntu, Windows and macOS.
4. Require the final-head borrow-inference workflow green and retain its artifact.
5. Confirm the final-head artifact still reports `IMPLEMENT-CANDIDATE`, 6 safe local candidates, 2 current move-friction cases and fail-closed negative boundaries.
6. Update PR #96 / #95 and living meta #40 with exact final-head provenance.
7. Squash-merge PR #96 using expected-head protection only after both exact-head workflows are green.
8. Track the natural post-merge `main` CI and Borrow inference research push workflow on the exact merge SHA.
9. Close #95 completed only after both post-merge gates succeed and the research artifact still supports the accepted decision.
10. Re-read live main/branch/PR/Actions state and search for duplicate/external #97 work.
11. Only then create the #97 implementation branch from exact verified main.

## Gated successor — #97

Issue **#97 — inferred shared-borrow nominal parameters v0** is open but must not start before PR #96 merges and both natural post-merge workflows succeed.

Bounded implementation contract:

- internal parameter passing modes: `Owned` and `SharedBorrow`;
- `SharedBorrow` only for locally proven read-only nominal parameters;
- classification remains non-transitive in v0;
- call ownership analysis inspects a caller local when the already-decided callee parameter is `SharedBorrow`;
- semantic IR carries the passing mode before Rust rendering;
- Rust codegen lowers to ordinary `&T` parameters and `&expr` call arguments;
- later by-value move after a completed shared-borrow call remains valid;
- current owned return/match/reinitialization/consume cases remain by-value;
- no stored or escaping borrow values.

Implementation must add representative differential/runtime evidence and preserve the existing #4 parity-or-better contract.

## Production contracts unchanged by #95 research

PR #96 is research evidence only. It does not change:

- Evolution syntax or accepted-program semantics;
- current by-value ownership behavior;
- generated Rust for production programs;
- Rust 1.98.0;
- edition 2024;
- opt-level 3;
- codegen-units 1;
- linker behavior;
- build-cache-v0 / run-cache-v0;
- source mapping and rustc diagnostic remapping;
- #4 runtime parity-or-better contract;
- zero-cost boundary: no hidden clone/copy, allocation, boxing, RC/GC, runtime ownership maps, reflection or dynamic dispatch.

## Build/compile structural deferrals

Dependency-build, proc-macro cost and workspace scaling remain deferred until Evolution user programs have a real package/dependency graph. Current single-file Phase 3.4 work is measured through #93; active P0 work has returned to Phase 3.3 ownership ergonomics.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Failed SHAs remain evidence and are not rerun merely for a better color.
