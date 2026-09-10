# Release Optimization Cost Research v0

Issue: #89  
PR: #90  
Phase: 3.4 Build / Compile — Release optimization cost  
Toolchain: Rust 1.98.0

## Decision

**REJECT / DEFER lowering the production optimization level from `-C opt-level=3` to `-C opt-level=2` under the current single-file architecture/toolchain setup.**

The candidate does not provide a material total native build improvement. Production remains at `opt-level=3`.

The seven-case runtime corpus was intentionally not executed because the pre-registered build advancement gate failed first. Runtime validation is an adoption gate, not a consolation prize for a compile-time candidate that did not improve compilation materially.

## Predecessor state

The experiment started only after PR #88 merged and the exact post-merge main state was green:

- verified base `main`: `5646ad45dd3d6640d147a7fc40bbbda891a540d2`;
- post-merge CI #371 / run `34456073733`: **SUCCESS** on Ubuntu, Windows and macOS;
- post-merge Linker candidate research #6 / run `34456073754`: **SUCCESS**;
- #87 closed/completed with linker replacement **REJECT / DEFER**.

## Pre-registered experiment contract

Before measurement, #89 fixed the initial comparison:

- cases: `enums-v0`, `logical-operators-v0`;
- current arm: edition 2024, `opt-level=3`, codegen-units 1, current default linker path, no incremental state;
- candidate arm: identical except `opt-level=2`;
- 2 warmups per arm/case;
- 9 measured compile+link samples per arm/case;
- alternating arm order;
- median as the primary build metric;
- raw samples plus min/max/p95 and relative MAD retained;
- exact committed stdout correctness required after every compile.

The candidate could advance only if **both** initial cases independently showed:

- correctness PASS;
- stable measurements with max relevant relative MAD <= 0.10;
- at least **5%** lower total compile+link median;
- at least **5 ms** lower total compile+link median.

Thresholds were recorded before measurement and were not moved afterward.

## Accepted code/evidence head

Accepted measurement head:

`0e46279fb1038a90e1aced9dd268a0e61c45a1b1`

Validation:

- normal CI #373 / run `34456762370`: **SUCCESS** on Ubuntu, Windows and macOS;
- Release optimization research #2 / run `34456762372`: **SUCCESS**;
- artifact: `evo-release-optimization-research-ubuntu-24.04`;
- artifact id: `10143822084`;
- artifact digest: `sha256:8b6ea8af1b1e65a321b4d958adab646b29517181621566531f7a22e524a2622e`;
- artifact `report.json`: parsed successfully;
- correctness: **PASS**.

The first PR head `fa096dae66415aec178de9c0ef3f3995df8c8eed` failed only `cargo fmt --check` and was not rerun. Formatting was corrected on the accepted head above.

## Results

| Case | opt3 current median | opt2 candidate median | Saved | Improvement | opt3 rel MAD | opt2 rel MAD | opt3 bytes | opt2 bytes | Verdict |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `enums-v0` | 109.425 ms | 109.036 ms | 0.389 ms | 0.36% | 0.003925 | 0.006854 | 4,519,880 | 4,519,832 | `BUILD-GATE-FAIL` |
| `logical-operators-v0` | 109.784 ms | 109.726 ms | 0.058 ms | 0.05% | 0.008338 | 0.009040 | 4,519,800 | 4,519,736 | `BUILD-GATE-FAIL` |

Aggregate verdict: **`REJECT-DEFER`**.

The candidate is stable, but stability only makes the negative result clearer. The observed improvement is sub-1% and sub-1 ms on both cases, far below both pre-registered materiality thresholds.

Binary-size changes are tiny supporting observations and do not alter the build decision.

## Runtime gate disposition

The pre-registered plan required the seven-case runtime corpus only after the build gate passed on both initial cases. That condition was not met.

Therefore this experiment does **not** claim anything about opt2 runtime parity or superiority. It simply does not need to ask that question for production adoption because opt2 already fails the compile-time reason for considering the change.

If a future toolchain or architecture materially changes the opt2/opt3 compile-time gap, runtime validation must still enforce both:

1. Evolution candidate <= equivalent reference Rust under matching compiler conditions according to #4;
2. Evolution candidate must not show a repeatable runtime regression versus current production-equivalent Evolution behavior.

## Production impact

None.

This research does not change:

- Evolution syntax or semantics;
- generated Rust;
- ownership/type behavior;
- Rust 1.98.0 toolchain pin;
- production `opt-level=3`;
- codegen-units 1;
- linker behavior;
- `build-cache-v0` or `run-cache-v0`;
- source mapping / diagnostic remapping;
- generated-program runtime behavior;
- #4 runtime thresholds.

## Successor

Issue #91, **compile memory baseline v0**, is the next current-architecture Phase 3.4 research item. It is gated on PR #90 merge plus a green natural post-merge `main` CI.

The successor measures frontend and rustc peak RSS before proposing any memory optimization. Dependency-build, proc-macro and workspace-scale work remain deferred until Evolution has an actual user-program package/dependency graph.
