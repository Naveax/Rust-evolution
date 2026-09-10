# Rust Evolution — Project State

Last verified update: **2026-09-10**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Stable `main` before active PR #92: `f122f4011537f2ed73624c95f1809ba122de924c`
- Rust toolchain: **1.98.0**
- production flags: edition 2024, opt-level 3, codegen-units 1
- measured GNU/Linux linker path: `rustc -> cc -> lld`

Post-PR #90 validation:

- CI #375 / run `34457802639`: SUCCESS on Ubuntu, Windows and macOS;
- Release optimization research #4 / run `34457802658`: SUCCESS.

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

PR #90 merged as `f122f4011537f2ed73624c95f1809ba122de924c`; issue #89 is closed/completed.

## Active completion — #91 / PR #92 compile memory baseline

PR #92: `research: establish compile memory baseline v0`  
Branch: `research/compile-memory-baseline-v0`

Accepted measurement head before docs synchronization:

- `3e2c3e390ff7c90ce61088b6c54540dce9dbf27f`;
- CI #376 / run `34470282815`: **SUCCESS** on Ubuntu, Windows and macOS;
- Compile memory research #1 / run `34470282845`: **SUCCESS**;
- artifact `evo-compile-memory-research-ubuntu-24.04`;
- id `10149215121`;
- digest `sha256:e7fbfff77bed760784cf5ca9c2d57da5a0df206e670e6fc402349c563be7b12a`;
- correctness PASS;
- report JSON validation PASS.

### Measurement method

GNU `/usr/bin/time` `%M` maximum resident set size in KiB for the directly measured process. 1 warmup + 5 measured samples per arm/case.

Arms:

1. `evo check`;
2. `evo emit-rust`;
3. direct rustc on exact generated Rust with production-equivalent flags.

GNU time process RSS is **not** treated as simultaneous whole-process-tree peak RSS. Full `evo build --no-cache` tree peak remains unavailable unless a separate cgroup/process-tree method is proven.

### Accepted #91 results

| Case | check median | emit-rust median | direct-rustc median | frontend share |
| --- | ---: | ---: | ---: | ---: |
| Enums | 3,932 KiB | 3,964 KiB | 236,816 KiB | ~1.66-1.67% |
| Logical Operators | 3,916 KiB | 3,912 KiB | 236,748 KiB | ~1.65% |

Direct rustc peaks at about 231 MiB. Frontend processes peak at about 3.8-3.9 MiB.

Pre-registered #91 guide required an Evolution-controlled phase to reach at least 64 MiB and at least 25% of direct-rustc peak on both cases for a memory-optimization follow-up. Neither case is remotely close.

### #91 decision

**DEFER / NO ACTION for Evolution-side compile-memory optimization under the current architecture.**

No production code or runtime behavior changes in this slice.

Durable report: `docs/COMPILE_MEMORY_RESEARCH.md`.

## Gated successor — #93 binary size baseline

Issue #93 is open but must not start before PR #92 merges and the resulting natural `main` CI succeeds.

#93 will measure the existing committed seven-case corpus under production-equivalent Rust 1.98 flags, retaining reference/generated binary sizes, byte identity where applicable, section sizes where defensible, dynamic dependency identity, exact correctness and machine-readable evidence.

Dependency-build, proc-macro and workspace-scaling roadmap items remain deferred until Evolution has an actual user-program package/dependency graph.

## Implemented language / tooling state

`docs/LANGUAGE_SPEC_V0.md` remains the implemented-language source of truth. Current accepted core includes integer/bool/static strings, input/repeat/control flow, functions, lexical block locals, Records v0, Enums v0, by-value ownership/reinitialization, source-native diagnostics, direct static Rust lowering, source maps, formatter, native check/emit/build/run and verified run/build caches.

## Zero-cost / safety boundary

Core language work must not silently add hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch. Build/tooling measurement metadata must not alter generated-program behavior.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed SHAs remain evidence and are not rerun merely for a better color.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
