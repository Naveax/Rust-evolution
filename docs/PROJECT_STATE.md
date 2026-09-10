# Rust Evolution — Project State

Last verified update: **2026-09-10**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Stable `main` before active PR #96: `733900781ea61f5684209570a35e9f63df71a7d4`
- Rust toolchain: **1.98.0**
- production flags: edition 2024, opt-level 3, codegen-units 1
- measured GNU/Linux linker path: `rustc -> cc -> lld`

Post-PR #94 validation:

- CI #381 / run `34474194185`: **SUCCESS** on Ubuntu, Windows and macOS;
- Binary size research #3 / run `34474194189`: **SUCCESS**;
- #93 closed/completed.

## Accepted build / compile sequence

### #76 — build latency baseline

Frontend/check/emit is roughly ~1 ms while uncached single-file native builds are dominated by rustc compile+link. Historical direct rustc median: 94.131 ms.

### #79 — verified unchanged-build reuse

`build-cache-v0` is production-accepted. Exact verified unchanged builds can materialize a native artifact without invoking rustc. Corruption/mismatch/unavailable state fails closed to normal compilation. `--no-cache` bypasses lookup/publication.

### #82 — changed-source rustc incremental research

Decision: **REJECT / DEFER**. Tested persistent rustc incremental configurations did not preserve the project runtime contract across the committed corpus. Durable report: `docs/INCREMENTAL_BUILD_RESEARCH.md`.

### #85 — link-time attribution

Current Rust 1.98 Linux path already uses `cc -> lld`. Linker-child work was a material component of total production-equivalent rustc wall time on initial controlled cases. Durable report: `docs/LINK_TIME_RESEARCH.md`.

### #87 — linker candidate experiment

Decision: **REJECT / DEFER**. Current lld beat tested GNU ld and pinned mold alternatives on total native build latency. Durable report: `docs/LINKER_CANDIDATE_RESEARCH.md`.

### #89 — release optimization cost

Decision: **REJECT / DEFER opt3 -> opt2**. Opt2 did not approach the pre-registered advancement gate. Production remains opt-level 3. Durable report: `docs/RELEASE_OPTIMIZATION_RESEARCH.md`.

### #91 — compile memory baseline

Decision: **DEFER / NO ACTION** for Evolution-side compile-memory optimization under the current architecture. Evolution frontend process peak RSS was only about 1.65-1.67% of direct-rustc peak on the accepted initial cases. Durable report: `docs/COMPILE_MEMORY_RESEARCH.md`.

### #93 — binary-size baseline

Decision: **DEFER / NO ACTION** for Evolution-specific binary-size optimization under the current language/corpus.

PR #94 squash-merged as `733900781ea61f5684209570a35e9f63df71a7d4`.

Post-merge:

- CI #381 / run `34474194185`: SUCCESS on Ubuntu, Windows and macOS;
- Binary size research #3 / run `34474194189`: SUCCESS;
- artifact id `10150883697`;
- digest `sha256:f391425df8a6c31ef22cb6998be8706a583bfe22f626fb1bb617f0df75792df3`.

Across the seven committed cases, controlled reference/Evolution binaries are byte-for-byte identical, every file-size delta is 0 bytes, parsed section deltas are zero, `DT_NEEDED` identity is unchanged and generated Rust equals committed reference source.

Durable report: `docs/BINARY_SIZE_RESEARCH.md`. Issue #93 is closed/completed.

## Build / compile structural boundary

Dependency-build, proc-macro cost and workspace scaling remain deferred until Evolution has a real user-program package/dependency graph. The directly actionable current single-file Phase 3.4 questions are measured through #93.

Active P0 work has returned to Phase 3.3 ownership ergonomics.

## Active completion — #95 / PR #96 borrow inference feasibility

Issue #95 researches whether a narrow statically local read-only nominal-parameter subset can be inferred as a call-duration shared borrow without generalized lifetime solving or hidden runtime mechanisms.

PR #96: `research: classify borrow inference feasibility v0`  
Branch: `research/borrow-inference-feasibility-v0`

Accepted measurement head before documentation synchronization:

`b3ef92be7e273bc4a27812d24733b87c12bd81c3`

Accepted validation:

- CI #384 / run `34478349183`: **SUCCESS** on Ubuntu, Windows and macOS;
- Borrow inference research #3 / run `34478349203`: **SUCCESS**;
- artifact `evo-borrow-inference-research-ubuntu-24.04`;
- artifact id `10152500994`;
- digest `sha256:3ba6965b0ffac03d9e1b74bd82152c4c0cbcf47a5d502f694910483e7f13b6fd`;
- report JSON validation: **PASS**;
- aggregate verdict: **IMPLEMENT-CANDIDATE**.

### Existing ownership seam

Enum-integrated ownership analysis already records `Inspect` and `Consume` uses with source spans and nominal types, then preserves those modes into ownership/executable IR. Record move tracking likewise has distinct inspect and consume operations.

Current function arguments remain consuming boundaries, which is the source of the measured ergonomic friction.

### Accepted classifier rule

A parameter is a local shared-borrow candidate only when:

1. its type is nominal (`Record` or `Enum`);
2. its own function body has at least one direct use classified `Inspect` under the current/pre-borrow rules;
3. it has zero `Consume` uses under those same rules;
4. it is never reinitialized/assigned;
5. classification does not depend on inferred modes of called functions.

Nested calls are deliberately consuming boundaries while classifying the caller. The rule is therefore **non-transitive** in v0 and requires no fixpoint or generalized lifetime solving.

### Accepted fixture result

| Case | Classification | Inspect | Consume | Reinit | Current lowering |
| --- | --- | ---: | ---: | --- | --- |
| `direct-field-read` | SAFE-LOCAL-INFERENCE-CANDIDATE | 1 | 0 | false | PASS |
| `double-read-current-friction` | SAFE-LOCAL-INFERENCE-CANDIDATE | 1 | 0 | false | REJECTED-CURRENT-MOVE |
| `shared-read-branches` | SAFE-LOCAL-INFERENCE-CANDIDATE | 2 | 0 | false | PASS |
| `nested-record-read` | SAFE-LOCAL-INFERENCE-CANDIDATE | 1 | 0 | false | PASS |
| `repeat-read-only` | SAFE-LOCAL-INFERENCE-CANDIDATE | 1 | 0 | false | PASS |
| `read-then-move-current-friction` | SAFE-LOCAL-INFERENCE-CANDIDATE | 1 | 0 | false | REJECTED-CURRENT-MOVE |
| `inspect-then-owned-return` | KEEP-BY-VALUE | 1 | 1 | false | PASS |
| `forward-through-current-consuming-call` | KEEP-BY-VALUE | 0 | 1 | false | PASS |
| `owned-match-scrutinee` | KEEP-BY-VALUE | 0 | 1 | false | PASS |
| `parameter-reinitialization` | KEEP-BY-VALUE | 1 | 0 | true | PASS |
| `repeat-read-then-owned-return` | KEEP-BY-VALUE | 1 | 1 | false | PASS |

Summary:

- 6 safe local candidates;
- 2 representative cases demonstrate actual current move friction;
- consume/match/reinitialization controls remain by-value/fail-closed.

Explicit outside-v0 boundaries:

- borrowed return / escaping reference: **REQUIRES-LIFETIME-MODEL**;
- mutable borrow inference: **REQUIRES-EXPLICIT-BORROW-SYNTAX**;
- stored/overlapping inferred borrow versus move: **UNSAFE/AMBIGUOUS**.

### #95 decision

**IMPLEMENT-CANDIDATE** for a narrow, non-transitive, call-duration inferred shared-borrow parameter mode.

This is a research decision only. PR #96 does not alter production syntax, ownership semantics, generated Rust, runtime behavior or performance contracts.

Durable report: `docs/BORROW_INFERENCE_RESEARCH.md`.

### Failed-SHA evidence

Initial research head `5b3a3ee2b5041bba6010c66ea7264fa3522fd9e9` was not rerun:

- CI #382 failed only formatting;
- Borrow inference research #1 successfully executed the semantic classifier and already produced the accepted 6-candidate / 2-friction result;
- JSON validation/artifact upload then failed because the report path was package-relative while the workflow expected workspace-root `target/`.

Formatting and output-path harness defects were corrected on later SHAs. Accepted head `b3ef92be...` validates the complete evidence pipeline.

## Gated implementation successor — #97

Issue #97: **inferred shared-borrow nominal parameters v0**.

It must not start until PR #96 merges and both natural post-merge workflows succeed:

- normal `main` CI;
- Borrow inference research push workflow.

No #97 implementation branch should exist before that gate.

Bounded implementation semantics:

- internal passing modes: `Owned` and `SharedBorrow`;
- candidate classification follows the accepted local/non-transitive #95 rule;
- call ownership analysis uses the already-decided callee mode, inspecting a top-level nominal caller local for `SharedBorrow` instead of moving it;
- executable/lowered IR carries the passing contract before codegen;
- Rust codegen emits ordinary `&T` parameters and `&expr` call arguments;
- borrow lifetime is the call only;
- later owned move remains valid after a shared-borrow call;
- existing owned return, match, consume, reinitialization and unsupported escaping-reference cases remain fail-closed/by-value.

Both record-only and enum-integrated lowering/codegen paths must obey the same semantic contract. Codegen must not infer borrow safety from function bodies.

#97 must add representative differential/runtime evidence and preserve the existing #4 parity-or-better contract.

## Implemented language / tooling state

`docs/LANGUAGE_SPEC_V0.md` remains the implemented-language source of truth. Current accepted core includes integer/bool/static strings, input/repeat/control flow, functions, lexical block locals, Records v0, Enums v0, by-value ownership/reinitialization, source-native move provenance diagnostics, source maps, formatter, native check/emit/build/run and verified run/build caches.

Shared-borrow inference is **not yet an implemented language feature** while #96 remains research-only and #97 gated.

## Zero-cost / safety boundary

Core language work must not silently add hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch. Borrow work must not invent `'static`, store inferred references, weaken owned-match payload rules, or leak unsupported lifetime semantics into accepted Evolution programs.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed SHAs remain evidence and are not rerun merely for a better color.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
