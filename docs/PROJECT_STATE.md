# Rust Evolution — Project State

Last verified update: **2026-09-08**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, `docs/NEXT_ACTION.md`, `docs/LANGUAGE_SPEC_V0.md`, and the active issue/PR/Actions before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Latest verified code-bearing `main`: `07e85b3a60739f2d1f25caed0fcb622dd8894861`
- Source: PR **#81** squash merge
- Final PR head: `4288ddcfccf07fcab60d27e9677685b213005ae2`
- Final PR CI **#338** / run `34214947823`: **SUCCESS** on Ubuntu, Windows and macOS
- Post-merge main CI **#339** / run `34215424678`: **SUCCESS** on Ubuntu, Windows and macOS
- Completed P0: **#79 verified build artifact reuse v0**
- Active successor P0 research: **#82 changed-source incremental build v0**

Always verify live GitHub before acting. A later docs-only merge may advance `main` beyond the code-bearing SHA above without changing compiler behavior.

## Completed P0 — Verified build artifact reuse v0 (#79)

Issue **#79** is completed. PR **#81 — `feat: reuse verified artifacts for unchanged evo build`** is merged.

Accepted behavior:

- `evo build` keeps full lexer/parser/lowering/codegen on every invocation before cache lookup;
- verified unchanged-build reuse uses a separate local `build-cache-v0` tooling cache;
- exact Evolution source, generated Rust and compiler/configuration identity are verified on hit;
- a key/hash locates candidates but is not proof of identity;
- completion marker and regular non-symlink cached native artifact are required;
- verified hits materialize to the requested output path without rustc compilation;
- different output paths may reuse the same verified artifact;
- `evo build <file> --no-cache` and `evo build <file> <output> --no-cache` bypass lookup/publication;
- corrupt, incomplete, missing, mismatched or unusable cache state fails closed to normal compilation;
- unavailable cache storage falls back to normal compilation;
- publication is best-effort after a successful normal compile;
- bounded pruning/stale staging cleanup remain tooling-only;
- accepted `run-cache-v0` behavior remains independent;
- generated Rust and generated-program runtime semantics/cost remain unchanged.

The #76 build-latency harness now invokes `evo build ... --no-cache`, preserving its original uncached attribution semantics after build-cache reuse became the normal build path.

See `docs/BUILD_CACHE.md` and D-021.

### Final controlled Ubuntu evidence

Final PR artifact:

- name: `evo-build-cache-turnaround-ubuntu-latest`;
- id: `10051449726`;
- digest: `sha256:cf7295bf371695347300bee325e3a5a4b5a96bbf4c8789625904d66bd4c538e9`;
- source head: `4288ddcfccf07fcab60d27e9677685b213005ae2`;
- platform: `linux-x86_64`;
- fixture: `benchmarks/cases/enums-v0/evolution.evo`.

Measured values:

- cold median: **128.454 ms** across 5 samples;
- unchanged warm cached median: **17.177 ms** across 9 samples;
- cold-to-warm speedup: **7.478x**;
- accepted #76 uncached warm baseline: **96.986 ms**;
- accepted-baseline-to-cached speedup: **5.646x**;
- cold rustc compile count: **5**;
- warm rustc compile count: **0**;
- correctness: **PASS**.

Hard acceptance is exact warm rustc compile count **0** plus correct native output. Timing is supporting evidence.

Generated Rust remained exactly **1240 bytes**, SHA-256:

`61f5f5c99c47196605ae2e461ee589b72a722c4ed5107c6b5fca353795100d83`

This matches the accepted Enums v0 / #76 baseline.

### CI acceptance

Final PR CI #338:

- Ubuntu: fmt, Clippy, workspace tests, fast edit-run evidence, preserved uncached build-latency evidence, verified build-cache evidence, benchmark smoke, every existing runtime/performance gate, release build: SUCCESS;
- Windows/macOS: fmt, Clippy, workspace tests, benchmark smoke, release build: SUCCESS.

Post-merge main CI #339 repeated the same required platform/gate coverage successfully on merge SHA `07e85b3a60739f2d1f25caed0fcb622dd8894861`.

## Completed measurement predecessor — Build latency baseline v0 (#76)

The accepted #76 controlled Ubuntu values remain the comparison baseline:

- `evo check`: **1.207 ms**;
- `evo emit-rust`: **1.210 ms**;
- cold uncached `evo build`: **99.970 ms**;
- unchanged warm uncached `evo build`: **96.986 ms**;
- deterministic edited uncached build: **95.515 ms**;
- direct rustc compile/link: **94.131 ms**;
- cold/warm/edit/direct rustc count: **5/5** in each measured class.

Artifact: `evo-build-latency-ubuntu-latest`, id `10046420977`, digest `sha256:d3310910395f9c6be50200c261402bfdbb8c7ef617ed18f71b3aa7486a22a2b0`.

## Active P0 research — #82 changed-source incremental build v0

Issue **#82 — `P0 research changed-source incremental build v0: measure rustc session reuse`** is open.

Why it is next:

- #79 removes rustc from exact unchanged warm builds;
- #76's deterministic generated-Rust-changing edit still costs **95.515 ms** and invokes rustc **5/5** times;
- direct rustc compile/link is **94.131 ms**, so the remaining changed-source bottleneck is still compiler/link work rather than the roughly one-millisecond frontend.

#82 is **research-first**, not an implementation promise.

Required research boundaries:

- reuse the accepted Enums v0 fixture and deterministic result-preserving edit;
- measure the pinned Rust 1.98.0 mechanisms/flags actually used, not remembered folklore;
- isolate fresh workdir/source-path effects from reusable rustc incremental/session state;
- record prime/edit samples, correctness, rustc counts, generated Rust, state size/growth, raw data and exact toolchain identity;
- preserve diagnostics/source mapping and existing build/run cache contracts;
- no mandatory daemon/compiler service, remote cache/download, linker replacement, package/dependency redesign, multi-crate semantics, frontend redesign, hot reload or language/runtime change;
- finish with **IMPLEMENT** only if improvement is stable/material and the state/invalidation model is bounded and safe; otherwise retain a **REJECT/DEFER** result.

## Current implemented language / diagnostics state

`docs/LANGUAGE_SPEC_V0.md` remains the implemented-language source of truth. Build/run caches and incremental-build research are tooling, not language semantics.

Current accepted core includes:

- integer, boolean and current static/literal string values;
- inferred mutability and lexical block locals;
- arithmetic, comparisons and strict short-circuit logical operators;
- `input_int`, `repeat`, `if/else`;
- typed named functions with forward calls and recursion;
- nominal Records v0 with by-value ownership;
- nominal Enums v0 with unit/single-payload variants and exhaustive statement-only matching;
- explicit same-type reinitialization after moves;
- deterministic source-native move-origin diagnostics;
- deterministic bounded spelling help for supported unknown-symbol diagnostics;
- direct static Rust lowering;
- generated-line source maps and rustc remapping;
- native `check`, `emit-rust`, `build`, `run`, `fmt` workflows;
- verified persistent `evo run` compile caching;
- verified persistent exact unchanged-`evo build` artifact reuse;
- controlled build-latency/turnaround evidence infrastructure;
- differential correctness/performance infrastructure and retained artifacts.

The current `string` value semantics remain **static/literal**. Do not silently treat the type as a general owned runtime string.

## ZERO-cost boundary

Current Core-language slices must not silently introduce hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.

Diagnostics, caches, incremental-build experiments and measurement metadata are compiler/tooling state. They must not alter accepted generated-program behavior.

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
