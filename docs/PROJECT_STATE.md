# Rust Evolution — Project State

Last verified update: **2026-09-08**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, `docs/NEXT_ACTION.md`, `docs/LANGUAGE_SPEC_V0.md`, and the active issue/PR/Actions before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Live `main` at this update: `53067b74f64fadab4b41684911401e822972bd26`
- Latest verified code-bearing merged baseline: `34f0815d0821543586cd3ae73b9b5b616a1396d3` from PR **#78**
- PR #78 post-merge main CI **#325** / run `34205500589`: **SUCCESS** on Ubuntu, Windows and macOS
- Active P0: **#79 verified build artifact reuse v0** / PR **#81**

Always re-read live GitHub before acting. Docs record the last verified state; they do not get to predict a future squash SHA, despite software documentation's occasional aspirations toward prophecy.

## Active P0 — Verified build artifact reuse v0 (#79)

PR **#81 — `feat: reuse verified artifacts for unchanged evo build`** has a fully accepted **code/evidence head**:

- head: `9fcab321d3be05b291c2d80f3949f0c975db242a`;
- CI **#337** / run `34213183199`: **SUCCESS** on Ubuntu, Windows and macOS;
- Ubuntu passed fmt, Clippy, workspace tests, fast edit-run evidence, preserved uncached build-latency evidence, verified build-cache evidence, benchmark smoke, every existing runtime/performance gate, and release build;
- Windows/macOS passed fmt, Clippy, workspace tests, benchmark smoke, and release build.

PR #81 is still **draft** at this documentation update. Durable docs create a new PR head; that exact docs head must pass one natural three-OS CI run before the PR can be marked ready and merged.

### Accepted build-cache behavior

`evo build` now has a separate local `build-cache-v0` tooling cache. The accepted behavior on the code/evidence head is:

- full lexer/parser/lowering/codegen still runs on every build before cache lookup;
- a verified hit requires exact Evolution source, exact generated Rust, exact compiler/configuration identity, a completion marker, and a regular non-symlink cached native artifact;
- the cache key is only a locator, not proof of identity;
- a hit materializes the verified native artifact to the requested output path;
- normal `evo build <file.evo> [output]` may reuse the cache by default;
- `evo build <file.evo> --no-cache` bypasses cache and uses the default output;
- `evo build <file.evo> <output> --no-cache` bypasses cache with an explicit output;
- source or compiler-fingerprint changes miss and compile normally;
- corrupt identity, missing binary, incomplete/unusable cache state, and symlink cached binaries where supported fail closed to recompilation;
- unavailable cache storage falls back to normal compilation;
- different requested output paths can reuse the same verified cached artifact;
- existing output replacement behavior remains compatible;
- cache publication is best-effort after a successful normal compile;
- `run-cache-v0` behavior is not changed by PR #81.

The accepted #76 baseline harness now calls `evo build ... --no-cache` so its historical uncached measurement semantics remain intact after cached build becomes the default.

See `docs/BUILD_CACHE.md` for the exact cache contract and accepted evidence.

### Final controlled Ubuntu artifact

Artifact:

- name: `evo-build-cache-turnaround-ubuntu-latest`;
- id: `10050970173`;
- digest: `sha256:bf949ee935a2512bd9b74726155b5f67d57a46a122af41c7377f86e3f2a2fa8e`;
- source head: `9fcab321d3be05b291c2d80f3949f0c975db242a`;
- platform: `linux-x86_64`;
- fixture: `benchmarks/cases/enums-v0/evolution.evo`.

Measured values:

- cold median: **137.783 ms** across 5 samples;
- unchanged warm cached median: **18.341 ms** across 9 samples;
- cold-to-warm speedup: **7.512x**;
- accepted #76 uncached warm baseline: **96.986 ms**;
- accepted-baseline-to-cached speedup: **5.288x**;
- cold rustc compile count: **5**;
- warm rustc compile count: **0**;
- correctness: **PASS**.

Hard acceptance is the exact **0 warm rustc compile count plus correct native output**. Timing is supporting evidence, not a semantic proof.

The retained generated Rust is still **1240 bytes** with SHA-256:

- `61f5f5c99c47196605ae2e461ee589b72a722c4ed5107c6b5fca353795100d83`

That exactly matches the accepted Enums v0 / #76 generated-Rust baseline.

## Completed P0 — Build latency baseline v0 (#76)

Issue **#76** is completed; PR **#78** is merged.

Accepted Ubuntu medians:

- `evo check`: **1.207 ms**;
- `evo emit-rust`: **1.210 ms**;
- cold uncached `evo build`: **99.970 ms**;
- unchanged warm uncached `evo build`: **96.986 ms**;
- edited uncached `evo build`: **95.515 ms**;
- direct rustc compile/link: **94.131 ms**.

Rustc compile invocation count across five measured samples was **5/5** for cold, unchanged warm, edit, and direct-rustc classes. The baseline demonstrated that direct rustc compile/link dominated unchanged build latency and justified #79 exact artifact reuse.

Accepted artifact: `evo-build-latency-ubuntu-latest`, id `10046420977`, digest `sha256:d3310910395f9c6be50200c261402bfdbb8c7ef617ed18f71b3aa7486a22a2b0`.

## Next performance problem after #79

Exact artifact reuse intentionally solves only **unchanged** builds. #76's deterministic edited-source path remained about **95.515 ms** and invoked rustc **5/5** times.

After #79 is merged and post-merge `main` CI is green, the next build/compile work should be a bounded **changed-source incremental-rustc/session research slice**, not an assumption that the exact cache somehow became incremental compilation by positive thinking.

Research must determine whether a persistent rustc/incremental session can reduce edited-source latency while preserving:

- exact generated Rust semantics;
- direct native backend behavior;
- diagnostics/source mapping;
- no mandatory daemon/runtime layer;
- explicit tooling-only cost boundaries;
- existing runtime performance gates.

Do not create a broad compiler-service architecture before measurement proves it is needed.

## Current implemented language / diagnostics state

`docs/LANGUAGE_SPEC_V0.md` remains the implemented-language source of truth. Build/run caches are tooling, not language semantics.

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
- verified persistent unchanged-`evo build` artifact reuse on PR #81's accepted code/evidence head;
- controlled build-latency and turnaround evidence infrastructure;
- differential correctness/performance infrastructure and retained artifacts.

The current `string` value semantics remain **static/literal**. Do not silently treat the type as a general owned runtime string.

## Prior accepted P0 evidence

### Diagnostic suggestions v0 (#74)

- PR #75 final head `1f3fdec69f81c8d6742d3fde47d698a3df3f3543`;
- CI #319 / run `34197970982`: SUCCESS;
- squash merge `cd6dcd096af20f9f94c4f201e715ae87c848c480`;
- post-merge CI #320 / run `34198404953`: SUCCESS.

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
