# Binary Size Research v0

Last verified update: **2026-09-10**

Issue: #93  
PR: #94 `research: establish binary size baseline v0`

## Decision

**DEFER / NO ACTION for Evolution-specific native binary-size optimization under the current language/corpus.**

Across all seven committed representative cases, reference Rust and Evolution-generated Rust compile to **byte-for-byte identical native binaries** when source-path/crate-name identity noise is controlled and both sides use the same production-equivalent compiler settings.

Observed Evolution-specific binary delta: **0 bytes in every case**.

## Accepted measurement head

`3c2565f513ef7951a7fa011879525e9e60aa469b`

Validation:

- CI #379 / run `34471973541`: **SUCCESS** on Ubuntu, Windows and macOS;
- Binary size research #1 / run `34471973624`: **SUCCESS**;
- artifact `evo-binary-size-research-ubuntu-24.04`;
- artifact id `10149900668`;
- digest `sha256:68eea4e28b4a24008cb5ef8d7490541bef616a95ce9e29555a22516ef3583e5a`;
- correctness: **PASS**;
- JSON validation: **PASS**.

## Controlled method

Platform/toolchain:

- GitHub `ubuntu-24.04`;
- Rust **1.98.0**;
- edition 2024;
- opt-level 3;
- codegen-units 1;
- current default linker path;
- no ThinLTO;
- no stripping;
- no panic-strategy override;
- no rustc incremental state.

To avoid false binary differences caused by source path or crate identity, both reference Rust and Evolution-generated Rust are copied into isolated work directories under the canonical source name `benchmark.rs` and compiled with the same crate name `evo_binary_size_case`.

This intentionally reuses the existing benchmark infrastructure's compile-identity principle while keeping the production build flags for this baseline. The benchmark harness's ThinLTO setting is **not** imported into this research.

For each case the retained artifact contains:

- committed reference Rust;
- exact Evolution-generated Rust;
- native file sizes and deltas;
- SHA-256 and byte-equality result;
- GNU `size -A` raw section output;
- parsed section deltas;
- `readelf -d` output;
- `DT_NEEDED` dependency identity;
- source-equality signal;
- exact-output correctness result;
- Rust/binutils identity;
- Markdown, JSON and CSV summaries.

## Results

| Case | Reference bytes | Evolution bytes | Delta | Overhead | Binary equal | DT_NEEDED equal | Source equal |
| --- | ---: | ---: | ---: | ---: | --- | --- | --- |
| `runtime-repeat-v0` | 4,517,168 | 4,517,168 | 0 | 0.0000% | true | true | true |
| `control-flow-branch-v0` | 4,517,360 | 4,517,360 | 0 | 0.0000% | true | true | true |
| `logical-operators-v0` | 4,517,280 | 4,517,280 | 0 | 0.0000% | true | true | true |
| `function-call-v0` | 4,517,344 | 4,517,344 | 0 | 0.0000% | true | true | true |
| `block-locals-v0` | 4,517,376 | 4,517,376 | 0 | 0.0000% | true | true | true |
| `records-v0` | 4,517,408 | 4,517,408 | 0 | 0.0000% | true | true | true |
| `enums-v0` | 4,517,376 | 4,517,376 | 0 | 0.0000% | true | true | true |

All parsed section deltas are zero because the binaries are identical. Dynamic dependency identity is also identical in every case.

The roughly 4.5 MB absolute ELF size is therefore ordinary Rust/toolchain baseline footprint for these builds, not an Evolution-specific deployment penalty.

## Pre-registered decision guide

Before measurement, #93 recorded:

- **DEFER / NO ACTION** if byte-identical where source shape is locked and every non-identical case stays within both 1.0% and 16 KiB Evolution-specific overhead with no dependency growth;
- **FOLLOW-UP-CANDIDATE** only if both thresholds are exceeded in at least two representative cases, or a clearly attributable hidden dependency/runtime footprint appears;
- **EXPAND / INVESTIGATE** for mixed/near-threshold cases.

The observed result is stronger than the DEFER condition: all seven cases are byte-identical with zero dependency growth.

## Interpretation

This baseline establishes that the current accepted Evolution syntax/features do not add binary footprint relative to their committed idiomatic Rust references when compile identity and flags are controlled.

That result is consistent with the project's static lowering design: generated Rust for the current corpus is identical to the committed reference source under the controlled comparison, so identical rustc inputs produce identical executables.

A future feature can reopen binary-size work only if it introduces a new source/codegen shape, monomorphization behavior, dependency/runtime support, or another concrete reason for footprint divergence. Absolute Rust executable size alone is not evidence of an Evolution defect.

## Deferred build/compile items

Dependency-build, proc-macro cost and workspace scaling remain structurally deferred until Evolution user programs have a real package/dependency graph. Measuring Cargo/workspace behavior that Evolution cannot yet express would manufacture a benchmark instead of measuring the product.

## Successor

The next current-architecture P0 research item is #95: **borrow inference feasibility v0** under Phase 3.3 Ownership ergonomics.

#95 is gated on PR #94 merging and the resulting natural `main` CI succeeding. It must not begin from an unverified predecessor SHA.

## Production contracts unchanged

This research changes no:

- Evolution syntax or semantics;
- ownership/type rules;
- generated-program behavior;
- production rustc flags;
- linker behavior;
- build/run cache contract;
- source maps/diagnostic remapping;
- runtime performance thresholds;
- runtime allocation/clone/boxing/dynamic-dispatch behavior.
