# Rust Evolution — Project State

Last verified update: **2026-09-09**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, `docs/NEXT_ACTION.md`, `docs/LANGUAGE_SPEC_V0.md`, and the active issue/PR/Actions before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Latest verified `main` before active PR #84: `1b7f355a92b27283d32731b68e3f86d5c99c264b`
- That SHA is the docs-handoff merge from PR #83; no production compiler behavior changed versus the preceding accepted build-cache main.
- Post-merge main CI #341 / run `34349824008`: **SUCCESS** on Ubuntu, Windows and macOS.
- Completed P0 build-cache feature: **#79 / PR #81**.
- Active research completion PR: **#84**, tracking **#82 changed-source incremental build v0**.
- Successor gated research issue: **#85 link-time baseline v0**.

Always verify live GitHub before acting. The active PR may advance beyond the accepted evidence SHA below due documentation synchronization.

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

The accepted controlled single-file baseline established:

- `evo check`: **1.207 ms**;
- `evo emit-rust`: **1.210 ms**;
- cold uncached `evo build`: **99.970 ms**;
- unchanged warm uncached `evo build`: **96.986 ms**;
- deterministic edited uncached build: **95.515 ms**;
- direct rustc compile+link: **94.131 ms**;
- rustc invoked for every measured cold/warm/edit/direct build.

The frontend is therefore not the dominant uncached native-build cost.

## #82 changed-source incremental build research — final decision

Issue **#82** researched persistent rustc incremental/session state before any production implementation was allowed.

Durable report: `docs/INCREMENTAL_BUILD_RESEARCH.md`.

Research PR: **#84 — `research: measure changed-source rustc incremental reuse`**.

Accepted final evidence head before documentation sync:

- `bbdec7f5f6ec4668849212f49bc07c49196024e2`
- CI #355 / run `34356624339`: **SUCCESS** on Ubuntu, Windows and macOS
- Ubuntu preserved every existing turnaround, build-cache and runtime/performance gate
- artifact `evo-incremental-build-research-ubuntu-latest`
- artifact id `10106092707`
- digest `sha256:217d9849e7e88f8ee05ebd408c0c9b2afaea57f3925f42d099b040dba7eb3a9c`

### Slice 1 — current CGU1 strategy

Edited medians:

- stable direct rustc, no incremental: **110.763 ms**;
- stable direct rustc, persistent incremental, CGU1: **121.040 ms**.

Result: CGU1 incremental state was about **9.28% slower** than its matching control.

### Slice 2 — CGU256 matching control

Edited medians:

- CGU256, no incremental: **107.522 ms**;
- CGU256, persistent incremental: **97.167 ms**.

Result: **1.107x** matching-control speedup, about **9.63% lower median**. This was enough to justify runtime-quality research but not production adoption.

### Slice 3/4 — runtime/codegen identity

On the Enums-v0 fixture, CGU256 + `-C incremental` produced a stable roughly **19% runtime improvement** versus the matching non-incremental CGU256 binary.

Isolation proved this was not stale edit-history reuse:

- baseline/edited no-incremental binaries were byte-identical;
- incremental prime/edited-reuse binaries were byte-identical;
- incremental-mode binary differed from no-incremental output at roughly **3,618,404 byte positions**.

Therefore enabling `-C incremental` itself changed optimized codegen identity.

### Slice 5 — committed runtime corpus

Seven committed performance cases were measured with exact-output correctness PASS, 3 warmups and 21 samples per arm.

| Case | Candidate/reference | Candidate/current | Verdict |
| --- | ---: | ---: | --- |
| runtime-repeat-v0 | 0.999231 | 0.998121 | PASS |
| control-flow-branch-v0 | 1.001843 | 1.001111 | FAIL |
| logical-operators-v0 | 1.019303 | 1.018570 | FAIL |
| function-call-v0 | 0.999582 | 0.999869 | PASS |
| block-locals-v0 | 1.000019 | 0.999306 | FAIL |
| records-v0 | 0.999854 | 1.000877 | FAIL |
| enums-v0 | 0.805402 | 0.804054 | PASS |

Aggregate decision: **REJECT**.

The favorable Enums effect did not generalize. `logical-operators-v0` was the clearest regression at about **1.93% slower than reference** and **1.86% slower than current Evolution**.

## #82 production decision

**REJECT / DEFER production rustc incremental state under the current architecture and Rust 1.98.0 configuration.**

Why:

1. the production-equivalent CGU1 strategy regresses changed-source compile latency;
2. the CGU256 configuration can improve compile latency but changes optimized codegen identity;
3. the runtime effect is workload-dependent;
4. the broader committed corpus violates the non-negotiable runtime parity-or-better contract.

The project does not trade generated-program runtime regressions for compiler convenience.

Because the runtime gate fails first, #82 intentionally stops before a production persistent-state design for invalidation, corruption fallback, races/cleanup or diagnostic/source-map integration.

## Production behavior after #82 research

No production behavior is changed by PR #84.

The following remain unchanged:

- Evolution language syntax and accepted-program semantics;
- ownership/type rules;
- production generated Rust;
- production rustc flags (`edition=2024`, `opt-level=3`, `codegen-units=1`);
- `build-cache-v0`;
- `run-cache-v0`;
- source mappings;
- rustc diagnostic remapping;
- runtime thresholds;
- linker choice;
- package/dependency behavior.

Direct-rustc arms in #82 are measurement-only and do not replace the production compile/remap path.

## Successor P0 — #85 link-time baseline v0

Issue **#85 — `P0 research link-time baseline v0: split rustc backend/codegen from linker cost`** is open but gated.

Why it is next:

- the frontend is already known to be small relative to native compile+link;
- exact unchanged build is solved by #79;
- the tested persistent incremental configurations are rejected/deferred by #82;
- direct rustc evidence still combines compile/codegen and final link work, so the link share is unknown.

Dependency-build/proc-macro roadmap items remain deferred until Evolution has a real user-program package/dependency graph. The current compiler still emits one `main.rs` and invokes rustc directly.

#85 must not start until:

1. PR #84 final documentation head passes exact-head CI;
2. PR #84 is squash-merged with expected-head protection;
3. the natural post-merge `main` CI is SUCCESS;
4. #82 is closed as completed research with its negative decision retained;
5. live main/open PR/issue/Action state is re-read and no duplicate work exists.

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
- controlled build-latency/turnaround evidence infrastructure;
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
