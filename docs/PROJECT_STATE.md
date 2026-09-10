# Rust Evolution — Project State

Last verified update: **2026-09-10**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Stable `main` before active PR #94: `c8ec7d397a50772ed500c4717dc340a4d00936d4`
- Rust toolchain: **1.98.0**
- production flags: edition 2024, opt-level 3, codegen-units 1
- measured GNU/Linux linker path: `rustc -> cc -> lld`

Post-PR #92 validation:

- CI #378 / run `34471091448`: SUCCESS on Ubuntu, Windows and macOS;
- Compile memory research #3 / run `34471091443`: SUCCESS.

## Accepted build/compile sequence

### #76 — build latency baseline

Frontend/check/emit is roughly ~1 ms while uncached single-file native builds are dominated by rustc compile+link. Historical direct rustc median: 94.131 ms.

### #79 — verified unchanged-build reuse

`build-cache-v0` is production-accepted. Exact verified unchanged builds can materialize a native artifact without invoking rustc. Corruption/mismatch/unavailable state fails closed to normal compilation. `--no-cache` bypasses lookup/publication.

### #82 — changed-source rustc incremental research

Decision: **REJECT / DEFER**. Tested persistent rustc incremental configurations did not preserve the project runtime contract across the committed corpus. Durable report: `docs/INCREMENTAL_BUILD_RESEARCH.md`.

### #85 — link-time attribution

Current Rust 1.98 Linux path already uses `cc -> lld`. Linker-child work was roughly 23% of total production-equivalent rustc wall time on the initial controlled cases. Durable report: `docs/LINK_TIME_RESEARCH.md`.

### #87 — linker candidate experiment

Decision: **REJECT / DEFER**. Current lld beat tested GNU ld and pinned mold alternatives on total native build latency. Durable report: `docs/LINKER_CANDIDATE_RESEARCH.md`.

### #89 — release optimization cost

Decision: **REJECT / DEFER opt3 -> opt2**. Opt2 did not approach the pre-registered 5% + 5 ms total-build advancement gate. Production remains opt-level 3. Durable report: `docs/RELEASE_OPTIMIZATION_RESEARCH.md`.

### #91 — compile memory baseline

Decision: **DEFER / NO ACTION** for Evolution-side compile-memory optimization under the current architecture.

Accepted measurement showed Evolution frontend process peak RSS around 3.8-3.9 MiB while direct rustc was about 231 MiB on both initial cases. Frontend was only about 1.65-1.67% of rustc peak RSS, far below the pre-registered 64 MiB + 25% follow-up guide.

PR #92 squash-merged as `c8ec7d397a50772ed500c4717dc340a4d00936d4`; post-merge CI #378 and Compile memory research #3 succeeded. Issue #91 is closed/completed.

Durable report: `docs/COMPILE_MEMORY_RESEARCH.md`.

## Active completion — #93 / PR #94 binary size baseline

PR #94: `research: establish binary size baseline v0`  
Branch: `research/binary-size-baseline-v0`

Accepted measurement head before docs synchronization:

- `3c2565f513ef7951a7fa011879525e9e60aa469b`;
- CI #379 / run `34471973541`: **SUCCESS** on Ubuntu, Windows and macOS;
- Binary size research #1 / run `34471973624`: **SUCCESS**;
- artifact `evo-binary-size-research-ubuntu-24.04`;
- id `10149900668`;
- digest `sha256:68eea4e28b4a24008cb5ef8d7490541bef616a95ce9e29555a22516ef3583e5a`;
- correctness PASS;
- report JSON validation PASS.

### Measurement method

For each committed corpus case, reference Rust and Evolution-generated Rust are staged as canonical `benchmark.rs` in isolated work directories and compiled with the same crate name and production-equivalent Rust 1.98 settings: edition 2024, opt-level 3, codegen-units 1, default linker, no ThinLTO/stripping/panic override/incremental state.

The artifact retains exact source inputs, file sizes, SHA-256, byte equality, GNU `size -A` section data, `readelf -d`, `DT_NEEDED`, correctness, JSON/CSV/Markdown summaries and tool identities.

### Accepted #93 results

| Case | Reference bytes | Evolution bytes | Delta |
| --- | ---: | ---: | ---: |
| runtime-repeat-v0 | 4,517,168 | 4,517,168 | 0 |
| control-flow-branch-v0 | 4,517,360 | 4,517,360 | 0 |
| logical-operators-v0 | 4,517,280 | 4,517,280 | 0 |
| function-call-v0 | 4,517,344 | 4,517,344 | 0 |
| block-locals-v0 | 4,517,376 | 4,517,376 | 0 |
| records-v0 | 4,517,408 | 4,517,408 | 0 |
| enums-v0 | 4,517,376 | 4,517,376 | 0 |

All seven binaries are byte-for-byte identical between reference and Evolution. `DT_NEEDED` identity and section sizes are also identical, and generated Rust equals the committed reference source in every controlled case.

### #93 decision

**DEFER / NO ACTION for Evolution-specific binary-size optimization under the current language/corpus.**

The roughly 4.5 MB ELF footprint is ordinary Rust/toolchain baseline, not Evolution-specific overhead. No stripping/LTO/panic/linker experiment is justified by this result.

Durable report: `docs/BINARY_SIZE_RESEARCH.md`.

## Build/compile structural deferrals

Dependency-build, proc-macro cost and workspace scaling remain deferred until Evolution has a real user-program package/dependency graph. The directly actionable current single-file Phase 3.4 questions have otherwise been measured through #93.

## Gated successor — #95 borrow inference feasibility

Issue #95 is open but must not start before PR #94 merges and the resulting natural `main` CI succeeds.

#95 researches whether a narrow statically provable read-only borrow-inference subset can reduce nominal-value move friction without hidden clone/copy/boxing/RC/GC, unsafe lifetime widening, or caller-visible ownership surprises.

The research must classify positive/negative fixtures before any implementation decision and preserve ordinary Rust shared-borrow lowering plus source-native diagnostics.

## Implemented language / tooling state

`docs/LANGUAGE_SPEC_V0.md` remains the implemented-language source of truth. Current accepted core includes integer/bool/static strings, input/repeat/control flow, functions, lexical block locals, Records v0, Enums v0, by-value ownership/reinitialization, source-native move provenance diagnostics, source maps, formatter, native check/emit/build/run and verified run/build caches.

## Zero-cost / safety boundary

Core language work must not silently add hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch. Build/tooling measurement metadata must not alter generated-program behavior.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed SHAs remain evidence and are not rerun merely for a better color.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
