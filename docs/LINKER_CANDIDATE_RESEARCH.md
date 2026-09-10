# Rust Evolution — Linker Candidate Research v0

Last verified evidence: **2026-09-10**

Issue: **#87**  
Research PR: **#88 — `research: compare current lld path with bounded linker candidates`**

## Decision

**REJECT / DEFER linker replacement under the current single-file architecture and Rust 1.98.0 toolchain.**

The current Ubuntu production-equivalent Rust path already uses `cc -> lld`. A reproducibly provisioned `mold` candidate and a stable GNU `ld` control were both slower in total compile+link wall time on the initial controlled corpus. The build gate therefore fails before a wider runtime experiment is justified.

No production linker setting changes as a result of this research.

## Evidence chain

Predecessor #85 established that final linking is material but not dominant:

- current linker path: `rustc -> cc -> lld`;
- roughly 23% of production-equivalent rustc wall time on the initial Enums and Logical Operators cases;
- #85 / PR #86 final decision: **FOLLOW-UP-CANDIDATE**.

#87 then compared the current path with genuinely different reproducible configurations.

Accepted code/evidence head before documentation synchronization:

- SHA: `852bc56eeeb69123b5a7f9c120338ab2c966a35b`
- normal CI #369 / run `34453683622`: **SUCCESS** on Ubuntu, Windows and macOS
- dedicated Linker candidate research #4 / run `34453683646`: **SUCCESS**
- artifact: `evo-linker-candidate-research-ubuntu-24.04`
- artifact id: `10142574260`
- digest: `sha256:d550ce688c1979c103df8ab3d2260ec136552672268d32f5be5d683e4c3ab7e1`
- report JSON independently parsed successfully

## Controlled environment

- runner: `ubuntu-24.04`
- rustc: **1.98.0**
- host: `x86_64-unknown-linux-gnu`
- production-equivalent common flags: edition 2024, opt-level 3, codegen-units 1
- baseline: Rust 1.98 default `cc -> lld`
- system-linker control: `-C linker-features=-lld -C link-self-contained=-linker`
- mold candidate: system-linker control plus `-C link-arg=-fuse-ld=mold`
- mold package: Ubuntu Noble `2.30.0+dfsg-1build1`
- warmups: 2 per arm/case
- measured samples: 9 per arm/case
- arm order rotated between samples

`mold` provisioning occurs before the timed compiler samples. Installation time is not counted as build-time improvement.

## Correctness / deployment checks

Every measured native binary must produce the committed expected stdout before its timing sample is accepted.

Accepted result:

- correctness: **PASS**
- linker-driver invocations: exactly **1 per measured sample**
- baseline and mold `DT_NEEDED` names are identical on both cases:
  - `ld-linux-x86-64.so.2`
  - `libc.so.6`
  - `libgcc_s.so.1`

The candidate therefore did not fail because it performed less work or silently removed these dynamic dependencies. It simply did not beat the current linker path.

## Final controlled medians

| Case | Arm | Full rustc median | Link-child median | Full relative MAD | Link relative MAD | Binary bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `enums-v0` | current lld | 113.077 ms | 24.727 ms | 0.0059 | 0.0090 | 4,519,880 |
| `enums-v0` | GNU ld control | 165.612 ms | 76.155 ms | 0.0077 | 0.0116 | 4,473,544 |
| `enums-v0` | mold | 122.321 ms | 33.401 ms | 0.0087 | 0.0233 | 4,536,552 |
| `logical-operators-v0` | current lld | 111.557 ms | 24.457 ms | 0.0142 | 0.0180 | 4,519,800 |
| `logical-operators-v0` | GNU ld control | 163.700 ms | 75.486 ms | 0.0084 | 0.0135 | 4,473,544 |
| `logical-operators-v0` | mold | 119.199 ms | 32.316 ms | 0.0153 | 0.0210 | 4,536,472 |

Relative MAD values are well below the experiment's 0.10 instability ceiling.

## Candidate ratios

Against the current lld total-build median:

- Enums mold: `122.321 / 113.077 = 1.08175`, about **8.17% slower**;
- Logical Operators mold: `119.199 / 111.557 = 1.06850`, about **6.85% slower**.

GNU ld is substantially slower again on both cases.

The candidate gate required a stable improvement of at least **5% and 5 ms** on every initial case. Mold does not merely miss that threshold. It regresses total build latency on both cases.

Aggregate research verdict: **`MOLD-REJECT-OR-DEFER`**.

## Why runtime corpus expansion stops here

#87 required total production-equivalent build latency to improve before spending the wider seven-case runtime corpus on a linker candidate.

That prerequisite fails. Running the larger runtime suite cannot convert a slower build candidate into a build-performance implementation candidate, so the experiment intentionally stops before runtime adoption validation.

This is not a waiver of #4. It is an earlier rejection gate.

## Production decision

Retain the current production behavior:

- Rust 1.98.0;
- edition 2024;
- opt-level 3;
- codegen-units 1;
- default `cc -> lld` linker path on the measured GNU/Linux target;
- unchanged `build-cache-v0` and `run-cache-v0` contracts;
- unchanged generated Rust, source maps and diagnostics.

Do not add a mold runtime/package dependency to Evolution production builds from this evidence.

## Revisit conditions

Linker replacement may be re-researched only when the relevant conditions materially change, for example:

- a different Rust/toolchain linker implementation materially changes the baseline;
- Evolution moves from one generated `main.rs` to a larger multi-object or package/workspace architecture where link workload shape is genuinely different;
- a new reproducible candidate exists with evidence suggesting it can beat the then-current baseline.

A future revisit must establish a fresh baseline rather than reusing these absolute timing numbers across a changed architecture or runner image.

## Successor

The next current-architecture P0 build question is **#89 — release optimization cost v0**.

Why:

- exact unchanged builds are already accelerated by verified artifact reuse;
- tested rustc incremental configurations are rejected/deferred under runtime parity;
- the current lld path already beats the tested alternative linkers;
- linker child time is only about 24–25 ms of the roughly 111–113 ms total rustc wall time in the final #87 run;
- production currently uses `-C opt-level=3`.

#89 therefore measures whether the current release optimization level carries a material compile-time premium and whether any cheaper optimization setting can preserve both #4 parity and current production runtime quality.
