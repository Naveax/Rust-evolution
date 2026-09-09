# Rust Evolution — Project State

Last verified update: **2026-09-09**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, `docs/NEXT_ACTION.md`, `docs/LANGUAGE_SPEC_V0.md`, and the active issue/PR/Actions before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Latest verified stable `main` before active PR #86: `c1ba2f2776a3b00bb5833a525f94e7b3e0cfa16f`
- Post-merge main CI #359 / run `34358142270`: **SUCCESS** on Ubuntu, Windows and macOS
- Completed unchanged-build feature: **#79 / PR #81**
- Completed changed-source incremental research: **#82 / PR #84**, production decision **REJECT / DEFER**
- Active link-attribution research completion PR: **#86**, tracking **#85**
- Gated successor experiment: **#87**

Always verify live GitHub before acting. An active PR may advance beyond the accepted evidence SHA due documentation synchronization.

## Accepted unchanged-build behavior — #79

PR #81 implemented verified persistent native artifact reuse for exact unchanged `evo build` inputs.

Accepted behavior remains:

- full lexer/parser/lowering/codegen runs before build-cache lookup;
- build reuse uses separate local `build-cache-v0` tooling state;
- exact Evolution source, generated Rust and compiler/configuration identity are verified;
- completion marker and regular non-symlink native artifact are required;
- verified hits materialize the requested output without rustc compilation;
- different output paths may reuse the same verified artifact;
- explicit `--no-cache` bypasses lookup/publication;
- corrupt/incomplete/mismatched/unavailable cache state fails closed to normal compilation;
- `run-cache-v0` remains independent;
- generated Rust and generated-program runtime semantics remain unchanged.

Durable contract: `docs/BUILD_CACHE.md` and D-021.

## Accepted build-latency predecessor — #76

The controlled single-file baseline established:

- `evo check`: **1.207 ms**;
- `evo emit-rust`: **1.210 ms**;
- cold uncached `evo build`: **99.970 ms**;
- unchanged warm uncached `evo build`: **96.986 ms**;
- deterministic edited uncached build: **95.515 ms**;
- direct rustc compile+link: **94.131 ms**;
- rustc invoked for every measured cold/warm/edit/direct build.

The frontend is therefore not the dominant uncached native-build cost.

## #82 changed-source incremental research — completed

Durable report: `docs/INCREMENTAL_BUILD_RESEARCH.md`.

Final production decision:

**REJECT / DEFER production rustc incremental state under the current architecture and Rust 1.98.0 configuration.**

Accepted conclusions:

- production-equivalent CGU1 + persistent incremental state made the matching deterministic edited compile path about **9.28% slower**;
- CGU256 + persistent incremental state improved matching edited compile latency by about **9.63%**, enough to justify deeper runtime research;
- enabling `-C incremental` changed optimized codegen identity;
- the favorable Enums runtime effect did not generalize across the committed seven-case runtime corpus;
- the clearest runtime regression was `logical-operators-v0`, about **1.93% slower than reference** and **1.86% slower than current Evolution**.

The project does not trade generated-program runtime regressions for compile-time convenience.

No production cache/compiler/language/source-map/diagnostic/runtime behavior changed from #82.

## #85 link-time attribution — final research decision pending merge

Issue **#85 — `P0 research link-time baseline v0: split rustc backend/codegen from linker cost`** is implemented as measurement-only PR **#86 — `research: attribute native build time to linker work`**.

Durable report on the PR branch: `docs/LINK_TIME_RESEARCH.md`.

Accepted evidence head before final documentation synchronization:

`121581ed1c105dd30cfdcfeb4567a3a988f20c23`

Validation:

- normal CI #361 / run `34361156009`: **SUCCESS** on Ubuntu, Windows and macOS;
- dedicated Link-time research #2 / run `34361156018`: **SUCCESS**;
- artifact `evo-link-time-research-ubuntu-latest`;
- artifact id `10107871908`;
- digest `sha256:62d454a95fd2a1f0ec7ba3c92b08582a29cb9a237bda8cf78b0113818ccc4162`;
- rustc 1.98.0 / LLVM 22.1.8;
- host `x86_64-unknown-linux-gnu`;
- driver `cc` 13.3.0;
- exact-output correctness **PASS**;
- 7 measured samples per arm/case;
- exactly one linker-driver invocation per instrumented sample.

### Method

Two committed workload shapes are measured:

- `enums-v0`;
- `logical-operators-v0`.

The research harness records:

- normal production-equivalent full rustc compile+link;
- `rustc --emit=obj` pre-link/object work;
- transparent PATH-shadow `cc` instrumentation;
- direct wall time of the real linker-driver child;
- actual linker argv;
- object/native sizes;
- exact native output correctness;
- raw CSV/JSON/Markdown evidence.

`full - object` is retained only as rough supporting attribution. The primary signal is the directly timed child path.

### Accepted link-time results

| Case | Full rustc | Object | Direct link child | Link/full | Classification |
| --- | ---: | ---: | ---: | ---: | --- |
| `enums-v0` | 115.158 ms | 85.330 ms | 26.784 ms | 23.259% | MATERIAL |
| `logical-operators-v0` | 113.687 ms | 84.238 ms | 26.393 ms | 23.215% | MATERIAL |

Stability from retained raw samples:

- Enums full relative MAD ~0.46%; link-child ~1.33%;
- Logical full relative MAD ~0.24%; link-child ~0.36%;
- transparent wrapper overhead ~2.1 ms on both cases.

Final #85 research decision:

**FOLLOW-UP-CANDIDATE.**

Final link work is a material, stable share of the current uncached/changed-source native build, so one bounded successor experiment is justified.

## Current linker identity

Retained Rust 1.98.0 linker argv includes:

`-fuse-ld=lld`

The current Ubuntu production-equivalent path is:

`rustc -> cc driver -> lld`

Therefore **lld is already the baseline**. A successor must not describe ordinary lld adoption as a new optimization.

No production linker or rustc flag changes in #85/#86.

## Gated successor — #87

Issue **#87 — `P0 experiment linker candidate v0: beat current Rust 1.98 lld path without runtime or deployment regression`** is open.

Start gate:

1. PR #86 documentation-synchronized exact head passes normal CI and dedicated link-time research;
2. PR #86 is squash-merged with expected-head protection;
3. the resulting natural `main` CI is SUCCESS;
4. #85 is closed as completed research with FOLLOW-UP-CANDIDATE retained;
5. live main/open PR/branch/Action state is re-read and no duplicate experiment exists.

Only then create the #87 branch from the exact verified main SHA.

#87 must compare the exact current `cc -> lld` baseline with at least one genuinely different reproducible link path/configuration.

Potential research directions, only if exactly provisioned and recorded:

- stable `-C linker-features=-lld` as a non-lld control;
- a genuinely different linker such as `mold` if its exact version/provisioning is reproducible;
- another bounded configuration that changes real link behavior rather than merely renaming the same lld path.

The exact GitHub Ubuntu runner image used by #85 does not list `mold` as preinstalled. Do not assume it exists and do not make production depend on an ad-hoc download.

A candidate advances only if it materially reduces **total production-equivalent native build latency** while preserving:

- exact correctness;
- #4 runtime parity-or-better;
- binary/startup/deployment behavior;
- understandable failure behavior;
- a bounded Windows/macOS story before any global production default.

## Production behavior remains unchanged

Current production contracts remain:

- Evolution language syntax and accepted-program semantics unchanged;
- ownership/type rules unchanged;
- production generated Rust unchanged;
- production rustc flags remain edition 2024, opt-level 3, codegen-units 1;
- current target-default linker behavior unchanged;
- `build-cache-v0` unchanged;
- `run-cache-v0` unchanged;
- source mappings unchanged;
- rustc diagnostic remapping unchanged;
- runtime thresholds unchanged;
- package/dependency behavior unchanged.

## Current implemented language / diagnostics state

`docs/LANGUAGE_SPEC_V0.md` remains the implemented-language source of truth. Build/run caches and build-performance research are tooling, not language semantics.

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
- controlled build-latency/turnaround/link-attribution evidence infrastructure;
- differential correctness/performance infrastructure and retained artifacts.

The current `string` value semantics remain static/literal. Do not silently treat the type as a general owned runtime string.

## ZERO-cost boundary

Current core-language slices must not silently introduce hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.

Diagnostics, caches and build-performance measurement metadata are compiler/tooling state. They must not alter accepted generated-program behavior.

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
