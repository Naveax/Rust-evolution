# Rust Evolution — Project State

Last verified update: **2026-09-10**

This is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, `docs/NEXT_ACTION.md`, `docs/LANGUAGE_SPEC_V0.md`, relevant research reports, and live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Stable `main` before active PR #90: `5646ad45dd3d6640d147a7fc40bbbda891a540d2`
- post-merge CI #371 / run `34456073733`: **SUCCESS** on Ubuntu, Windows and macOS
- post-merge Linker candidate research #6 / run `34456073754`: **SUCCESS**
- production Rust flags: edition 2024, opt-level 3, codegen-units 1
- measured GNU/Linux production-equivalent linker path: `rustc -> cc -> lld`

Always re-read live GitHub state. Active PRs may advance beyond accepted measurement SHAs when durable documentation is synchronized after evidence is accepted.

## Accepted build / compile history

### #76 — native build latency baseline

Historical accepted medians:

- `evo check`: 1.207 ms;
- `evo emit-rust`: 1.210 ms;
- cold uncached `evo build`: 99.970 ms;
- unchanged warm uncached build: 96.986 ms;
- deterministic edited uncached build: 95.515 ms;
- direct rustc compile+link: 94.131 ms.

The frontend is small relative to native rustc work on the accepted single-file fixture.

### #79 / PR #81 — verified unchanged-build artifact reuse

Production `build-cache-v0` is accepted. Exact verified unchanged hits can materialize the native output without invoking rustc while normal frontend validation/lowering/codegen still runs. Corruption/mismatch/incomplete state fails closed to normal compilation. `run-cache-v0` remains independent.

### #82 / PR #84 — changed-source rustc incremental research

Decision: **REJECT / DEFER** persistent rustc incremental state under the tested architecture/toolchain configuration.

CGU256 + incremental could improve compile latency, but wider runtime evidence violated the project's parity contract. Durable report: `docs/INCREMENTAL_BUILD_RESEARCH.md`.

### #85 / PR #86 — link-time attribution

Decision: **FOLLOW-UP-CANDIDATE** for one bounded linker experiment.

Current Rust 1.98 GNU/Linux path already uses `cc -> lld`; direct linker-child work was roughly 23% of total production-equivalent rustc wall time on the initial controlled cases. Durable report: `docs/LINK_TIME_RESEARCH.md`.

### #87 / PR #88 — linker candidate experiment

Decision: **REJECT / DEFER** linker replacement under the current single-file architecture/toolchain setup.

PR #88 merged as `5646ad45dd3d6640d147a7fc40bbbda891a540d2`.

Post-merge validation:

- CI #371 / run `34456073733`: **SUCCESS** on Ubuntu, Windows and macOS;
- Linker candidate research #6 / run `34456073754`: **SUCCESS**;
- issue #87: closed/completed.

Accepted candidate evidence showed current lld faster than both GNU ld and pinned Ubuntu Noble mold. Durable report: `docs/LINKER_CANDIDATE_RESEARCH.md`.

## Active completion — #89 / PR #90 release optimization cost

Issue: **#89 — release optimization cost v0**  
PR: **#90 — `research: measure opt3 versus opt2 build cost`**  
Branch: `research/release-optimization-cost-v0`

Accepted code/evidence head before documentation synchronization:

`0e46279fb1038a90e1aced9dd268a0e61c45a1b1`

Validation:

- normal CI #373 / run `34456762370`: **SUCCESS** on Ubuntu, Windows and macOS;
- Release optimization research #2 / run `34456762372`: **SUCCESS**;
- artifact `evo-release-optimization-research-ubuntu-24.04`;
- artifact id `10143822084`;
- digest `sha256:8b6ea8af1b1e65a321b4d958adab646b29517181621566531f7a22e524a2622e`;
- correctness: **PASS**;
- artifact `report.json` parsed successfully.

### #89 experiment design

Controlled Ubuntu 24.04 / Rust 1.98.0. Initial cases:

- `enums-v0`;
- `logical-operators-v0`.

Arms differ only in optimization level:

- current: edition 2024, opt-level 3, CGU1, current default linker, no incremental state;
- candidate: identical except opt-level 2.

Sampling: 2 warmups + 9 measured samples per arm/case, alternating order. Each measured compile is followed by exact committed stdout correctness.

Pre-registered advancement required both cases to show stable measurements plus at least 5% and 5 ms lower total compile+link median.

### #89 accepted results

| Case | opt3 median | opt2 median | Saved | Improvement | opt3 rel MAD | opt2 rel MAD | opt3 bytes | opt2 bytes | Verdict |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| Enums | 109.425 ms | 109.036 ms | 0.389 ms | 0.36% | 0.003925 | 0.006854 | 4,519,880 | 4,519,832 | `BUILD-GATE-FAIL` |
| Logical Operators | 109.784 ms | 109.726 ms | 0.058 ms | 0.05% | 0.008338 | 0.009040 | 4,519,800 | 4,519,736 | `BUILD-GATE-FAIL` |

Aggregate verdict: **`REJECT-DEFER`**.

The measurements are stable. Opt2 simply does not materially reduce total native build latency on either initial case.

### #89 production decision

**REJECT / DEFER lowering production optimization from opt3 to opt2.**

The seven-case runtime corpus is intentionally not executed because the candidate fails the pre-registered build gate before runtime adoption validation is justified. Production remains at `-C opt-level=3`.

Durable report added by the documentation synchronization commit: `docs/RELEASE_OPTIMIZATION_RESEARCH.md`.

## Gated successor — #91 compile memory baseline v0

Issue **#91 — `P0 research compile memory baseline v0: attribute frontend and rustc peak RSS`** is open. It must not start until PR #90 is merged and the resulting natural post-merge `main` CI is **SUCCESS**.

Why this is next:

- unchanged-build latency is solved by verified artifact reuse;
- tested incremental state is rejected/deferred;
- current lld beats tested alternative linkers;
- lowering opt3 to opt2 offers no material build win;
- compile-time memory is the next directly measurable current-architecture Phase 3.4 resource question.

The first #91 slice must prove its memory measurement method before accepting RSS values. It should separate frontend/check, emit-rust and direct-rustc peak RSS, and include full `evo build --no-cache` only if child/grandchild process memory is defensibly accounted for.

Start with Enums and Logical Operators; retain at least five samples per accepted arm plus raw RSS, median/min/max, supporting wall time, generated Rust, correctness, tool identity and JSON/CSV/Markdown evidence.

If the two current cases are too small to produce a useful memory baseline, expand with a committed deterministic fixture using supported Evolution constructs. Do not manufacture an uncommitted Rust stress blob merely to make a graph look interesting.

Dependency build, proc-macro cost and workspace scaling remain structurally deferred until Evolution user programs have a real package/dependency graph.

## Current implemented language / tooling state

`docs/LANGUAGE_SPEC_V0.md` remains the implemented-language source of truth.

Accepted core includes integer/boolean/static-string values, inferred mutability, lexical block locals, arithmetic/comparisons/logical operators, input/repeat/if, typed named functions, nominal Records and Enums, move diagnostics, direct static Rust lowering, source maps, native check/emit/build/run/fmt workflows, verified run/build caches, and controlled correctness/runtime/build research infrastructure.

The current `string` semantics remain static/literal. Do not silently treat them as a general owned runtime string.

## Zero-cost / safety boundary

Core-language work must not silently introduce hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.

Tooling/build measurement metadata must not alter accepted generated-program behavior.

## Durable continuation order

1. `AGENTS.md`
2. `docs/PROJECT_STATE.md`
3. `docs/NEXT_ACTION.md`
4. `docs/LANGUAGE_SPEC_V0.md`
5. relevant durable research report
6. live active issue / PR / Actions

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing exact run and continue independent work while it runs.

Failed SHAs remain evidence and are not rerun merely to improve the color.

Every significant merge or incomplete stopping point must keep `PROJECT_STATE.md`, `NEXT_ACTION.md`, issue/PR evidence and durable decisions synchronized with GitHub reality.

The repository is the project memory. The chat transcript is not.
