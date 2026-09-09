# Link-time attribution research v0

Issue: #85

Research PR: #86

Successor experiment: #87

## Decision

**FOLLOW-UP-CANDIDATE.**

The current Rust 1.98.0 Ubuntu single-file native build spends a material, repeatable share of wall time in the final linker-driver child path. The evidence justifies one bounded experiment against the current linker baseline, but it does **not** justify a production linker change by itself.

The current baseline already uses **lld**. A future experiment must not describe ordinary lld adoption as a new optimization.

## Scope

This research is measurement/tooling only. It does not change:

- Evolution syntax or accepted-program semantics;
- ownership/type rules;
- generated Rust;
- production rustc flags;
- production linker configuration;
- `build-cache-v0` or `run-cache-v0`;
- source maps or rustc diagnostic remapping;
- generated-program runtime behavior or thresholds.

## Predecessor evidence

#76 established that the frontend is small relative to uncached native compilation and measured direct rustc compile+link as one combined cost.

#79 solved exact unchanged builds with verified native artifact reuse.

#82 tested persistent rustc incremental state for changed-source rebuilds and finished REJECT/DEFER because the compile-time candidate that improved rebuild latency violated the project runtime parity contract on the committed corpus.

That left one current-architecture question: how much of direct rustc native build time is final linking?

## Accepted exact-head evidence

Accepted PR #86 head:

`121581ed1c105dd30cfdcfeb4567a3a988f20c23`

Validation:

- normal CI #361 / run `34361156009`: **SUCCESS** on Ubuntu, Windows and macOS;
- dedicated Link-time research #2 / run `34361156018`: **SUCCESS**;
- artifact `evo-link-time-research-ubuntu-latest`;
- artifact id `10107871908`;
- digest `sha256:62d454a95fd2a1f0ec7ba3c92b08582a29cb9a237bda8cf78b0113818ccc4162`;
- rustc 1.98.0 / LLVM 22.1.8;
- host `x86_64-unknown-linux-gnu`;
- link driver `cc` 13.3.0;
- 7 measured samples per arm/case;
- exact-output correctness: **PASS**;
- exactly one linker-driver invocation per instrumented sample.

The earlier head `95c28970ec1f3ab742c0a59a178b20d91809bcab` retained useful first-run timing evidence but normal CI #360 failed only rustfmt. That SHA was not rerun. The format-only correction produced the accepted exact head above.

## Method

Two committed workloads were used:

- `benchmarks/cases/enums-v0`;
- `benchmarks/cases/logical-operators-v0`.

For each case, the research harness measures:

1. normal production-equivalent rustc compile + native link;
2. `rustc --emit=obj` under the same successful-build optimization/codegen flags;
3. a normal full rustc build with a transparent PATH-shadow `cc` wrapper;
4. wall time spent by the real linker-driver child invoked through that wrapper;
5. linker invocation count;
6. native/object sizes;
7. exact committed output correctness.

The wrapper forwards rustc's exact linker argv to the real PATH-resolved `cc`. It does not substitute a different linker.

`rustc --print link-args` and wrapper argv are retained as evidence of the actual link path.

## Production-equivalent flags

The measured Rust compilation settings remain:

- edition 2024;
- opt-level 3;
- codegen-units 1;
- current target/host defaults for the final link.

## Results

| Case | Full rustc | `--emit=obj` | Full-object rough | Instrumented full | Direct link child | Link/full | Classification |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `enums-v0` | 115.158 ms | 85.330 ms | 29.829 ms | 117.355 ms | 26.784 ms | 23.259% | MATERIAL |
| `logical-operators-v0` | 113.687 ms | 84.238 ms | 29.449 ms | 115.818 ms | 26.393 ms | 23.215% | MATERIAL |

Supporting stability from retained raw samples:

- Enums full relative MAD: about 0.46%;
- Enums link-child relative MAD: about 1.33%;
- Logical full relative MAD: about 0.24%;
- Logical link-child relative MAD: about 0.36%.

Transparent wrapper overhead versus the normal full-build median is only about 2.1 ms on both cases. Normal and instrumented native binary sizes are identical within each case.

The measured link share is therefore materially above the predeclared 8 ms / 10% research guide on both initial cases and is not explained by wrapper overhead.

## Attribution caveat

`full rustc median - object emission median` is retained only as a rough supporting signal. It subtracts separately sampled compiler modes and is **not causal profiling**.

The primary attribution evidence is the child wall time measured inside the transparent driver wrapper. The rough subtraction lands in the same order of magnitude, which is useful corroboration rather than proof by arithmetic.

## Actual linker path

The retained Rust 1.98.0 linker argv contains:

`-fuse-ld=lld`

The Ubuntu path is therefore:

`rustc -> cc driver -> lld`

This matches the stable Rust target behavior for `x86_64-unknown-linux-gnu`, where lld use is already the default. A successor must compare against this actual baseline rather than proposing lld as though it were absent.

## What #85 proves

#85 proves that, on the controlled Ubuntu runner and these two committed single-file workloads:

- final link work is observable directly;
- the direct link child consumes about 26.4-26.8 ms;
- that is about 23% of normal full rustc wall time;
- the signal is stable enough to justify a bounded follow-up experiment;
- the baseline already uses lld.

## What #85 does not prove

#85 does **not** prove that:

- another linker is faster overall;
- changing the linker preserves runtime quality;
- a Linux-only linker is acceptable for Evolution generally;
- a candidate preserves deployment/dynamic-dependency behavior;
- isolated linker micro-timing equals total user-visible build improvement;
- production should change its default linker.

## Successor gate

Issue #87 owns the next experiment:

`P0 experiment linker candidate v0: beat current Rust 1.98 lld path without runtime or deployment regression`

Do not create its implementation/research branch until:

1. PR #86 is merged;
2. the resulting natural `main` CI is SUCCESS;
3. live main/open issue/PR/Action state is re-read;
4. no duplicate candidate experiment exists.

#87 must compare the exact current `cc -> lld` baseline with a genuinely different reproducible link path/configuration. A candidate is interesting only if it materially improves **total production-equivalent native build latency** while preserving correctness, #4 runtime parity, binary/deployment behavior and a bounded platform story.

A candidate such as `mold` may be researched only if its exact provision/version is reproducible. It is not preinstalled on the exact GitHub Ubuntu runner image used for #85, so availability must not be assumed.

## CI operations

Do not create duplicate active Actions for the same SHA/workflow/input. Failed SHAs remain retained evidence and are not rerun merely to obtain a friendlier color.
