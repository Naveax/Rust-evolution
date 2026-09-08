# Rust Evolution — Project State

Last verified update: **2026-09-08**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, `docs/NEXT_ACTION.md`, `docs/LANGUAGE_SPEC_V0.md`, and the active issue/PR/Actions before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Latest verified code-bearing `main`: `cd6dcd096af20f9f94c4f201e715ae87c848c480`
- Source: PR **#75** squash merge
- Post-merge main CI **#320** / run `34198404953`: **SUCCESS** on Ubuntu, Windows and macOS
- Open feature PRs at this handoff point: none
- Next atomic P0: **#76 build latency baseline v0**

A later docs-only handoff merge may advance `main` beyond the code-bearing SHA above. Always verify live `main` before new implementation work. Documentation still has not defeated causality.

## Completed P0 — Diagnostic suggestions v0 (#74)

Issue **#74** is completed. PR **#75 — `feat: add deterministic source-native diagnostic suggestions`** is merged.

Delivered behavior:

- conservative deterministic one-edit matcher for current ASCII identifiers;
- supported edit shapes: insertion, deletion, substitution and adjacent transposition;
- suggestions require one unique best candidate inside the fixed conservative threshold;
- equal-best ties, distant names, one-character guesses and empty candidate sets stay silent;
- candidate lookup stays namespace-specific rather than searching one global fuzzy symbol pool;
- local suggestions are lexical-scope-aware;
- function suggestions use declared functions only;
- record type/constructor/constructor-field/field suggestions use record-specific candidates;
- enum type/constructor/variant/match-variant suggestions use enum-specific candidates;
- primary diagnostic message/span remain stable; suggestion is bounded additional `help:` text;
- a dedicated compile-time help sidecar is keyed by exact primary `(message, span)` identity;
- stale/mismatched help state is dropped and cannot contaminate unrelated diagnostics;
- move-origin related locations from #69 and textual help can coexist;
- parser autocorrection, automatic source edits and LSP/code actions remain out of scope;
- ownership accept/reject semantics are unchanged;
- no generated-program runtime metadata or cost was introduced.

### Final exact-head evidence

Final PR head:

- `1f3fdec69f81c8d6742d3fde47d698a3df3f3543`

Final PR CI:

- CI **#319** / run `34197970982`: **SUCCESS** on Ubuntu, Windows and macOS;
- Ubuntu passed fmt, Clippy, workspace tests, fast edit-run turnaround evidence, benchmark smoke, runtime repeat, control-flow, logical operators, Functions v0, Block locals v0, Records v0, Enums v0 and release build;
- Windows/macOS passed fmt, Clippy, workspace tests, benchmark smoke and release build.

Retained failed/intermediate heads were not rerun merely for color:

- CI #314 / run `34191629058`: rustfmt-only failure;
- CI #316 / run `34192026716`: rustfmt-only failure in the new process test;
- CI #317 / run `34197292048`: acceptance-test failure proving `MabyInt -> MaybeInt` is outside the one-edit boundary; test corrected to one-edit `MaybInt -> MaybeInt`, matcher threshold intentionally unchanged.

### Generated-Rust preservation

The Ubuntu Enums benchmark artifact from accepted pre-feature main CI #304 was compared with the feature artifact from compiler head `20eb7210336d53827b89b2481e0f9cde38867a00`.

Exact equality evidence:

- `generated.rs` SHA-256 on both sides: `61f5f5c99c47196605ae2e461ee589b72a722c4ed5107c6b5fca353795100d83`;
- generated native executable SHA-256 on both sides: `d2b172767e1ca9267173322ca2d2eb3bc9941361c93f3f64056d6c81d05d0431`;
- generated LLVM IR SHA-256 on both sides: `0c82e4c394ab5edc7f32d21a9f48800ada2ed169c695c04b8d66c677eecc533f`.

Commits after that compiler head through the final PR head changed only `crates/evo-cli/tests/diagnostic_suggestions.rs`, so compiler/codegen bytes remained unchanged after the comparison point.

## Post-merge verification

Squash merge:

- `cd6dcd096af20f9f94c4f201e715ae87c848c480`

Post-merge main CI:

- CI **#320** / run `34198404953`: **SUCCESS**;
- Ubuntu repeated all quality, turnaround, benchmark, runtime/performance and release gates successfully;
- Windows and macOS quality/test/benchmark-smoke/release jobs passed.

Issue #74 closed automatically as **completed** from the merged PR.

## Current implemented language / diagnostics state

`docs/LANGUAGE_SPEC_V0.md` remains the implemented-language source of truth. No syntax or accepted-program semantics changed in #74.

Current accepted core includes:

- integer, boolean and current static/literal string values;
- inferred mutability and lexical block locals;
- arithmetic, comparisons and strict short-circuit logical operators;
- `input_int`;
- `repeat`, `if/else`;
- typed named functions with forward calls and recursion;
- nominal Records v0 with by-value ownership;
- nominal Enums v0 with unit/single-payload variants and exhaustive statement-only matching;
- explicit same-type reinitialization after moves;
- deterministic source-native move-origin related diagnostics;
- deterministic bounded spelling help for supported unknown-symbol diagnostics;
- direct static Rust lowering;
- generated-line source maps and rustc remapping;
- native `check`, `emit-rust`, `build`, `run`, `fmt` workflows;
- verified persistent `evo run` compile caching;
- differential correctness/performance infrastructure and retained artifacts.

The current `string` value semantics remain **static/literal**. Do not silently treat the type as a general owned runtime string.

## Prior accepted P0s

### Move diagnostics v0 (#69)

- PR #71 final head `74f6a5955d46bd9620045bd39e9703383ae30679`;
- PR CI #301 / run `34141025715`: SUCCESS;
- squash merge `795461c53f896c2223443cdb022f340a5032a0bd`;
- post-merge CI #302 / run `34158551578`: SUCCESS;
- deterministic source-native move provenance is compile-time-only diagnostics metadata;
- accepted generated Rust remained identical to the accepted Enums baseline.

### Fast edit-run v0 (#67)

- PR #68 final head `b26a84819e8a89f27220ebd2eb16d4d91d513094`;
- PR CI #288 / run `34123896927`: SUCCESS;
- feature merge `17206a702b497731bd5174a0e011225711492d3e`;
- controlled cold median **69.258 ms**;
- warm median **14.972 ms**;
- observed warm speedup **4.626x**;
- measured warm rustc compile count **0**.

The `evo run` cache remains tooling state only and does not change generated-program semantics.

Records v0 and Enums v0 remain the accepted ZERO-cost nominal data baselines with direct static Rust lowering and explicit by-value ownership.

## Active P0 — #76 build latency baseline v0

Issue **#76 — `P0 build latency baseline v0: cold, warm and edit compile attribution`** is open.

Roadmap / weakness source:

- parent: #2;
- weakness source: #6 Build / Compile;
- roadmap: #1 Phase 3.4 — Cold build baseline, Warm build baseline, Incremental build.

### Verified current build-path root cause

Current `evo build`:

1. performs normal frontend validation/lowering/codegen through `load_program`;
2. calls `compile_rust()` directly;
3. creates a fresh temporary directory and writes generated `main.rs`;
4. invokes the selected rustc with the existing edition/optimization/codegen-unit flags;
5. links directly to the requested output path;
6. removes the temporary directory.

The verified persistent cache from #67 is only used by `run_generated()` for `evo run`. `evo build` deliberately does not consult it.

Therefore unchanged `evo build` currently follows a full rustc compile/link path by control flow. #76 still measures actual invocation counts and latency rather than turning code inspection into fake benchmark data.

### #76 boundary

This slice is measurement/attribution only. It must not introduce build caching, rustc incremental sessions, a daemon, remote cache, linker replacement, dependency/package semantics, or generated-program runtime changes.

Controlled Ubuntu evidence must measure at minimum:

- frontend/check latency;
- emit latency;
- cold build latency;
- unchanged warm build latency;
- deterministic small-edit build latency;
- direct rustc compile/link latency for the exact generated Rust with matching flags;
- rustc invocation counts for cold, warm and edited builds;
- raw samples plus machine-readable and human-readable reports.

The completion output must classify the next build/compile action from evidence as Implement / Research / No-action rather than assuming a cache is automatically the answer.

## ZERO-cost boundary

Current accepted Core-language slices must not silently introduce hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.

Diagnostics and build-measurement metadata are compiler/tooling state. They must not alter accepted generated-program behavior.

## Durable continuation infrastructure

Read order:

1. `AGENTS.md`
2. `docs/PROJECT_STATE.md`
3. `docs/NEXT_ACTION.md`
4. `docs/LANGUAGE_SPEC_V0.md`
5. active issue/PR/Actions referenced there

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.

## Handoff invariant

Every significant merge or incomplete stopping point must keep `PROJECT_STATE.md`, `NEXT_ACTION.md`, issue/PR evidence and durable decisions synchronized with GitHub reality.

The repository is the project memory. The chat transcript is not.
