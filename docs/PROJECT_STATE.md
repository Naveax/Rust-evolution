# Rust Evolution — Project State

Last verified update: **2026-09-08**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, `docs/NEXT_ACTION.md`, `docs/LANGUAGE_SPEC_V0.md`, and the active issue/PR/Actions before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Latest verified code-bearing `main`: `34f0815d0821543586cd3ae73b9b5b616a1396d3`
- Source: PR **#78** squash merge
- Post-merge main CI **#325** / run `34205500589`: **SUCCESS** on Ubuntu, Windows and macOS
- Active successor P0: **#79 verified build artifact reuse v0**

A later docs-only handoff merge may advance live `main` beyond the code-bearing SHA above. Always verify live `main`, open PRs and active Actions before implementation work.

## Completed P0 — Build latency baseline v0 (#76)

Issue **#76** is completed. PR **#78 — `bench: establish build latency attribution baseline`** is merged.

The accepted slice is measurement/attribution infrastructure only. It did **not** change `evo build` caching, generated Rust semantics, generated-program runtime behavior, linker choice, package/dependency behavior, or rustc incremental-session behavior.

### Final exact-head evidence

Final PR head:

- `96b8de7570662aa5cf5886f90ea5f0432a2c14fe`

Final PR CI:

- CI **#324** / run `34202560065`: **SUCCESS** on Ubuntu, Windows and macOS;
- Ubuntu passed fmt, Clippy, workspace tests, fast edit-run turnaround, the new build-latency evidence step, benchmark smoke, every existing runtime/performance gate and release build;
- Windows/macOS passed fmt, Clippy, workspace tests, benchmark smoke and release build.

Retained failed evidence:

- CI **#323** / run `34202332386`: rustfmt-only failure on the first harness head; Clippy/tests/evidence did not run and the failed SHA was not rerun.

### Accepted Ubuntu build-latency artifact

Artifact:

- name: `evo-build-latency-ubuntu-latest`;
- id: `10046420977`;
- digest: `sha256:d3310910395f9c6be50200c261402bfdbb8c7ef617ed18f71b3aa7486a22a2b0`;
- platform: `linux-x86_64`;
- rustc host: `x86_64-unknown-linux-gnu`;
- rustc flags: `--edition=2024 --error-format=short -C opt-level=3 -C codegen-units=1`.

Controlled fixture:

- `benchmarks/cases/enums-v0/evolution.evo`;
- committed stdin `20000000\n9\n`;
- committed expected stdout `15099959897\n`;
- deterministic edit sample changes one `sum = sum + value` expression to `sum = value + sum`, preserving result while changing generated Rust.

Accepted medians:

- `evo check`: **1.207 ms**;
- `evo emit-rust`: **1.210 ms**;
- cold `evo build`: **99.970 ms**;
- unchanged warm `evo build`: **96.986 ms**;
- edited `evo build`: **95.515 ms**;
- direct rustc compile/link of exact emitted Rust: **94.131 ms**.

Approximate build-minus-direct-rustc median signal:

- cold: **5.839 ms**;
- warm unchanged: **2.856 ms**;
- edit: **1.384 ms**.

These subtractions are retained as rough attribution signals only, not causal profiling.

Rustc compile invocation count across five measured samples per class:

- cold build: **5/5**;
- unchanged warm build: **5/5**;
- edited build: **5/5**;
- direct rustc: **5/5**.

Every measured native artifact executed the committed fixture input and matched committed expected output.

The exact retained generated Rust is **1240 bytes** with SHA-256:

- `61f5f5c99c47196605ae2e461ee589b72a722c4ed5107c6b5fca353795100d83`

This matches the existing accepted Enums v0 generated-Rust baseline.

## Evidence-driven next action

The #76 completion decision is:

- **Implement:** verified unchanged-build native artifact reuse v0;
- **Research separately:** changed-source incremental-rustc/session reuse.

Reason: unchanged warm `evo build` still invokes rustc on every sample. Direct rustc accounts for roughly **94.1 ms** of a roughly **97.0 ms** median unchanged build, while frontend/check/codegen is roughly **1.2 ms**. Optimizing the frontend first would be excellent craftsmanship applied to the wrong bottleneck, a traditional industry pastime.

The deterministic edited build also invokes rustc on every sample and remains roughly **95.5 ms**, so exact artifact reuse intentionally does not claim to solve changed-source rebuild latency.

## Active P0 — #79 verified build artifact reuse v0

Issue **#79 — `P0 verified build artifact reuse v0: unchanged evo build without rustc`** is open.

Bounded goal:

- keep full frontend validation/lowering/codegen on every `evo build` invocation;
- use a separate verified local `build-cache-v0` rather than silently changing `run-cache-v0` semantics;
- require exact Evolution source, generated Rust and compiler/configuration identity on hit;
- reject incomplete, corrupt, missing or symlink cache artifacts;
- materialize a verified cached native artifact to the requested build output path;
- add explicit `--no-cache` bypass;
- on unchanged verified warm builds, require **0 rustc compile invocations** and correct output;
- changed source/generated Rust/compiler identity must miss and compile normally;
- cache unavailability/corruption must fail closed to recompilation;
- `evo run` cache behavior remains unchanged;
- generated Rust and generated-program runtime behavior remain unchanged.

Explicit non-goals for #79:

- changed-source incremental rustc sessions;
- daemon/compiler service;
- remote cache or executable download;
- linker replacement;
- dependency/package system work;
- generated-program runtime changes.

## Current implemented language / diagnostics state

`docs/LANGUAGE_SPEC_V0.md` remains the implemented-language source of truth. #76 changed tooling measurement only, not language syntax or accepted-program semantics.

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
- controlled build-latency evidence infrastructure;
- differential correctness/performance infrastructure and retained artifacts.

The current `string` value semantics remain **static/literal**. Do not silently treat the type as a general owned runtime string.

## Prior accepted P0s

### Diagnostic suggestions v0 (#74)

- PR #75 final head `1f3fdec69f81c8d6742d3fde47d698a3df3f3543`;
- CI #319 / run `34197970982`: SUCCESS;
- squash merge `cd6dcd096af20f9f94c4f201e715ae87c848c480`;
- post-merge CI #320 / run `34198404953`: SUCCESS;
- generated Rust/native/LLVM identity remained unchanged for the accepted Enums evidence.

### Move diagnostics v0 (#69)

- PR #71 final head `74f6a5955d46bd9620045bd39e9703383ae30679`;
- CI #301 / run `34141025715`: SUCCESS;
- squash merge `795461c53f896c2223443cdb022f340a5032a0bd`;
- post-merge CI #302 / run `34158551578`: SUCCESS.

### Fast edit-run v0 (#67)

- PR #68 final head `b26a84819e8a89f27220ebd2eb16d4d91d513094`;
- CI #288 / run `34123896927`: SUCCESS;
- controlled cold median **69.258 ms**;
- warm median **14.972 ms**;
- warm rustc compile count **0**.

`evo run` cache state remains tooling-only and independent from generated-program semantics.

## ZERO-cost boundary

Current Core-language slices must not silently introduce hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.

Diagnostics, caches and build-measurement metadata are compiler/tooling state. They must not alter accepted generated-program behavior.

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
