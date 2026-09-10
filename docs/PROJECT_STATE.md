# Rust Evolution — Project State

Last verified update: **2026-09-10**

This is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, `docs/NEXT_ACTION.md`, `docs/LANGUAGE_SPEC_V0.md`, the relevant research reports, and live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Stable `main` before active PR #88: `b14173c8add554963df7ccb0f48de41e72fa3136`
- post-merge CI #365 / run `34362873485`: **SUCCESS** on Ubuntu, Windows and macOS
- current production Rust flags: edition 2024, opt-level 3, codegen-units 1
- measured GNU/Linux production-equivalent linker path: `rustc -> cc -> lld`

Always re-read live GitHub state. The active PR may advance beyond an accepted measurement SHA because durable documentation is synchronized after evidence is accepted.

## Accepted build / compile history

### #76 — native build latency baseline

Controlled predecessor established that frontend/check/emit is small relative to rustc native work and that uncached single-file builds are dominated by rustc compile+link.

Accepted historical medians included:

- `evo check`: 1.207 ms;
- `evo emit-rust`: 1.210 ms;
- cold uncached `evo build`: 99.970 ms;
- unchanged warm uncached build: 96.986 ms;
- deterministic edited uncached build: 95.515 ms;
- direct rustc compile+link: 94.131 ms.

### #79 / PR #81 — verified unchanged-build artifact reuse

Production `build-cache-v0` is accepted.

Hard behavior remains:

- full frontend validation/lowering/codegen still runs before build-cache lookup;
- exact Evolution source, generated Rust and compiler/configuration identity are verified;
- complete regular non-symlink cached artifact is required;
- verified unchanged hits can materialize the native output without invoking rustc;
- different requested output paths may reuse the same verified artifact;
- corruption/incomplete/mismatch/unavailable state fails closed to normal compilation;
- `--no-cache` bypasses lookup/publication;
- `run-cache-v0` stays independent;
- no generated-program runtime/codegen semantic change.

Durable contract: `docs/BUILD_CACHE.md` and D-021.

### #82 / PR #84 — changed-source rustc incremental research

Decision: **REJECT / DEFER production persistent rustc incremental state under the tested architecture/toolchain configuration.**

Key evidence:

- current CGU1 + persistent incremental state regressed deterministic edited compile latency;
- CGU256 + incremental could improve compile latency but changed optimized codegen identity;
- the favorable Enums runtime effect did not generalize;
- the committed seven-case runtime corpus contained repeatable regressions, including Logical Operators;
- the project does not buy compile-time convenience with generated-program runtime regression.

Durable report: `docs/INCREMENTAL_BUILD_RESEARCH.md`.

### #85 / PR #86 — link-time attribution

Decision: **FOLLOW-UP-CANDIDATE** for a bounded linker experiment, not production linker replacement.

Accepted finding:

- current Rust 1.98 GNU/Linux path already uses `cc -> lld`;
- direct linker-child work was roughly 23% of total production-equivalent rustc wall time on the initial Enums and Logical Operators cases;
- link cost was material enough to justify one controlled alternative-linker experiment.

PR #86 merged as `b14173c8add554963df7ccb0f48de41e72fa3136`.

Post-merge validation:

- CI #365 / run `34362873485`: SUCCESS on all three OSes;
- Link-time research #6 / run `34362873679`: SUCCESS.

Durable report: `docs/LINK_TIME_RESEARCH.md`.

## Active completion — #87 / PR #88 linker candidate experiment

Issue: **#87 — linker candidate v0**  
PR: **#88 — `research: compare current lld path with bounded linker candidates`**  
Branch: `research/linker-candidate-v0`

Accepted code/evidence head before documentation synchronization:

- `852bc56eeeb69123b5a7f9c120338ab2c966a35b`
- normal CI #369 / run `34453683622`: **SUCCESS** on Ubuntu, Windows and macOS
- Linker candidate research #4 / run `34453683646`: **SUCCESS**
- artifact `evo-linker-candidate-research-ubuntu-24.04`
- artifact id `10142574260`
- digest `sha256:d550ce688c1979c103df8ab3d2260ec136552672268d32f5be5d683e4c3ab7e1`
- artifact `report.json` independently parsed successfully

### #87 experiment design

Controlled Ubuntu 24.04, Rust 1.98.0, same generated source/path policy and otherwise production-equivalent flags.

Arms:

1. current Rust 1.98 default `cc -> lld`;
2. GNU/system linker control with `-C linker-features=-lld -C link-self-contained=-linker`;
3. Ubuntu Noble `mold=2.30.0+dfsg-1build1`, selected through the same opt-out plus `-C link-arg=-fuse-ld=mold`.

The workflow provisions mold before measurement. Installation time is excluded from compile timing.

Each case uses 2 warmups and 9 rotating-order measured samples per arm.

### #87 accepted results

| Case | Arm | Full median | Link-child median | Full rel MAD | Binary bytes |
| --- | --- | ---: | ---: | ---: | ---: |
| Enums | current lld | 113.077 ms | 24.727 ms | 0.0059 | 4,519,880 |
| Enums | GNU ld | 165.612 ms | 76.155 ms | 0.0077 | 4,473,544 |
| Enums | mold | 122.321 ms | 33.401 ms | 0.0087 | 4,536,552 |
| Logical Operators | current lld | 111.557 ms | 24.457 ms | 0.0142 | 4,519,800 |
| Logical Operators | GNU ld | 163.700 ms | 75.486 ms | 0.0084 | 4,473,544 |
| Logical Operators | mold | 119.199 ms | 32.316 ms | 0.0153 | 4,536,472 |

Mold versus current lld total-build median:

- Enums: about **8.17% slower**;
- Logical Operators: about **6.85% slower**.

Correctness and deployment evidence:

- exact committed stdout: PASS for every accepted timing sample;
- exactly one linker-driver invocation per measured sample;
- baseline and mold `DT_NEEDED` names match exactly;
- all relevant relative MAD values are well below the 0.10 instability ceiling.

Aggregate verdict: **`MOLD-REJECT-OR-DEFER`**.

### #87 production decision

**REJECT / DEFER linker replacement under the current single-file architecture/toolchain setup.**

The current lld path is faster than both tested alternatives. Mold fails the total-build gate before the wider runtime corpus is justified. Production linker behavior remains unchanged.

Durable report: `docs/LINKER_CANDIDATE_RESEARCH.md`.

## Gated successor — #89 release optimization cost v0

Issue **#89 — `P0 research release optimization cost v0: measure opt-level 3 build/runtime tradeoff`** is open but must not start until PR #88 is merged and its natural post-merge `main` CI succeeds.

Why this is the next current-architecture build question:

- exact unchanged-build work is already solved by #79;
- tested rustc incremental approaches are rejected/deferred by #82;
- alternative linker paths failed to beat current lld under #87;
- in final #87 evidence, current lld consumed only about 24–25 ms of the 111–113 ms total rustc time;
- production explicitly uses `-C opt-level=3`.

#89 first compares current opt3 with bounded opt2 while holding edition, codegen-units, linker, toolchain, generated source/path policy and no-incremental behavior constant.

A lower optimization setting may advance only after a stable material total-build win. If it advances, it must satisfy both:

- #4 Evolution <= equivalent reference Rust under the same opt-level/linker/codegen conditions;
- no repeatable runtime regression versus current production-equivalent Evolution opt3.

Compile-time speed does not purchase runtime regression.

## Current implemented language / tooling state

`docs/LANGUAGE_SPEC_V0.md` remains the implemented-language source of truth.

Accepted core includes, among other current v0 behavior:

- integer, boolean and static/literal string values;
- inferred mutability and lexical block locals;
- arithmetic/comparisons/strict short-circuit logical operators;
- `input_int`, `repeat`, `if/else`;
- typed named functions including forward calls and recursion;
- nominal Records v0 with by-value ownership;
- nominal Enums v0 with unit/single-payload variants and exhaustive statement-only matching;
- explicit same-type reinitialization after moves;
- deterministic source-native move-origin diagnostics;
- bounded spelling help for supported unknown-symbol diagnostics;
- direct static Rust lowering;
- generated-line source maps and rustc diagnostic remapping;
- native `check`, `emit-rust`, `build`, `run`, `fmt` workflows;
- verified persistent run cache and exact unchanged-build cache;
- controlled correctness/runtime/build research infrastructure.

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
