# Compile memory research v0

Issue: #91  
PR: #92  
Accepted measurement head: `3e2c3e390ff7c90ce61088b6c54540dce9dbf27f`

## Decision

**DEFER / NO ACTION for Evolution-side compile-memory optimization under the current single-file architecture.**

The frontend/check and emit-rust processes are tiny relative to direct rustc. The pre-registered #91 Evolution-side hotspot guide is not met on either initial committed case.

## Method

Controlled platform: GitHub `ubuntu-24.04`, pinned Rust 1.98.0.

Primary accepted metric:

- GNU `/usr/bin/time` `%M` maximum resident set size;
- unit: KiB;
- scope: the directly measured process;
- 1 warmup + 5 measured samples per arm/case.

Measured arms:

1. `evo check`;
2. `evo emit-rust`;
3. direct `rustc` compiling the exact emitted Rust with production-equivalent flags: edition 2024, opt-level 3, codegen-units 1.

The harness deliberately does **not** report GNU time process RSS as concurrent whole-process-tree peak RSS. Full `evo build --no-cache` tree peak remains unavailable until a separate process-tree/cgroup method is proven.

## Accepted evidence

- CI #376 / run `34470282815`: **SUCCESS** on Ubuntu, Windows and macOS;
- Compile memory research #1 / run `34470282845`: **SUCCESS**;
- artifact: `evo-compile-memory-research-ubuntu-24.04`;
- artifact id: `10149215121`;
- digest: `sha256:e7fbfff77bed760784cf5ca9c2d57da5a0df206e670e6fc402349c563be7b12a`;
- correctness: **PASS**;
- report JSON validation: **PASS**.

| Case | Arm | Median peak RSS | Min | Max | Share of direct rustc |
| --- | --- | ---: | ---: | ---: | ---: |
| `enums-v0` | check | 3,932 KiB | 3,884 | 3,988 | 1.660% |
| `enums-v0` | emit-rust | 3,964 KiB | 3,864 | 3,996 | 1.674% |
| `enums-v0` | direct rustc | 236,816 KiB | 236,656 | 238,852 | 100% |
| `logical-operators-v0` | check | 3,916 KiB | 3,884 | 3,976 | 1.654% |
| `logical-operators-v0` | emit-rust | 3,912 KiB | 3,864 | 3,936 | 1.652% |
| `logical-operators-v0` | direct rustc | 236,748 KiB | 236,684 | 239,120 | 100% |

Direct rustc is about 231 MiB peak RSS on both cases. Frontend phases are about 3.8-3.9 MiB and roughly 1.65-1.67% of direct-rustc peak.

## Pre-registered triage guide

Before first measurement #91 recorded:

- FOLLOW-UP-CANDIDATE only if an Evolution-controlled frontend/check or emit-rust phase is at least 64 MiB and at least 25% of direct-rustc median peak RSS on both cases, or equivalent evidence reveals another controllable hotspot;
- DEFER / NO ACTION if frontend phases remain below both guides while direct rustc dominates;
- EXPAND-CORPUS for mixed or too-small signals.

Both cases clearly satisfy the DEFER / NO ACTION branch.

## Scope boundary

No production compiler flag, linker, cache, generated Rust, language semantic, ownership/type rule, diagnostic/source-map behavior or generated-program runtime behavior changed in this research.

## Next roadmap action

Phase 3.4 dependency-build, proc-macro and workspace-scaling work remains structurally deferred until Evolution user programs have a package/dependency graph. The next directly measurable current-architecture item is #93 binary-size baseline v0.
