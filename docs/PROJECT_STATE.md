# Rust Evolution — Project State

Last verified update: **2026-09-07**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Latest verified code-bearing `main` baseline: `17206a702b497731bd5174a0e011225711492d3e`
- Baseline source: PR **#68** squash merge
- Post-merge main CI **#289** / run `34124829348`: **SUCCESS**

A later docs-only handoff merge may advance `main` beyond the code-bearing baseline above. Always verify the live branch head before starting code; do not pretend a self-referential documentation commit can contain its own future squash SHA, because hashes remain stubbornly uninterested in human convenience.

## Completed P0 — Fast edit-run v0 (#67)

Issue **#67** is **completed**. PR **#68** is merged.

Delivered behavior:

- `evo run <file.evo>` still performs current-source lex/parse/lower/codegen validation on every invocation;
- unchanged runs may reuse a verified persistent native binary;
- `evo run <file.evo> --no-cache` forces an uncached compile path;
- cache identity includes exact Evolution source bytes, generated Rust bytes, selected rustc command, `rustc -vV`, edition, optimization/codegen flags, OS/architecture and executable suffix;
- exact identity files are verified in addition to the cache key, so hash equality alone is not correctness proof;
- cache entries require regular non-symlink files and a completion marker;
- compilation happens in staging and successful publication uses staging-to-final rename;
- stale/incomplete/corrupt/mismatched entries fail closed to recompilation;
- concurrent misses may compile redundantly rather than execute a partial artifact;
- cache setup/fingerprint/publication optimization failures fall back to normal compilation when safe;
- `evo build` behavior is unchanged;
- no network cache, daemon, VM, GC, executable download, unsafe Rust, hidden clone/allocation/boxing/dynamic dispatch or incremental-rustc machinery was introduced.

Cache behavior and locations are documented in `docs/FAST_EDIT_RUN_CACHE.md`.

### Final exact-head evidence

Final PR head:

- `b26a84819e8a89f27220ebd2eb16d4d91d513094`

Final PR CI:

- CI **#288** / run `34123896927`: **SUCCESS**
- Ubuntu: fmt, Clippy, workspace tests, turnaround evidence + artifact upload, benchmark smoke, runtime-repeat, control-flow, logical operators, functions, block-locals, Records v0, Enums v0 and release build all passed;
- Windows/macOS: fmt, Clippy, workspace tests, benchmark smoke and release build passed.

Controlled Ubuntu turnaround evidence:

- cold samples: `5`, each with a fresh empty cache;
- warm samples: `9`, after one untimed prime;
- cold median: **69.258 ms**;
- warm median: **14.972 ms**;
- warm speedup: **4.626x**;
- cold rustc compile count: **5**;
- measured warm rustc compile count: **0**;
- verdict: **PASS**;
- artifact: `evo-fast-edit-run-turnaround-ubuntu-latest`;
- artifact id: `10019395817`;
- digest: `sha256:2cbb68765c69b231bdee08460a1a925ca336af39ac53e1833f1ac57c8920ca34`.

The speedup number is runner-specific evidence, not a universal promise. The accepted property is that verified unchanged warm runs perform zero rustc **compilations** and measure below cold-run median on the controlled evidence runner. The rustc fingerprint probe may still execute `rustc -vV`.

### Retained failed-head evidence

Failed evidence was preserved and never rerun:

- CI **#285** / run `34122517074`, head `0c9a5821...`: rustfmt-only failure in the newly added turnaround evidence test;
- CI **#286** / run `34122829838`, head `44ecf5eb...`: turnaround test itself passed (`72.323 ms` cold, `15.387 ms` warm, `4.700x`, warm compile count `0`) but artifact upload failed because the output path was relative to the integration-test working directory.

Both root causes were fixed on new SHAs. Old red runs remain evidence rather than being emotionally negotiated with a rerun button.

## Post-merge verification

Squash merge:

- `17206a702b497731bd5174a0e011225711492d3e`

Post-merge main CI:

- CI **#289** / run `34124829348`: **SUCCESS**
- Ubuntu repeated the turnaround evidence, benchmark smoke, all existing runtime/performance gates and release build successfully;
- macOS and Windows quality/benchmark-smoke/release jobs also passed.

Issue #67 closed automatically as **completed** from the merged PR.

## Current implemented language state

`docs/LANGUAGE_SPEC_V0.md` remains the implemented-language source of truth.

Current accepted core includes:

- integer, boolean and current static/literal string values;
- inferred mutability and lexical block locals;
- arithmetic, comparisons and strict short-circuit logical operators;
- `input_int`;
- `repeat`, `if/else`;
- typed named functions with forward calls and recursion;
- nominal Records v0 with by-value ownership;
- nominal Enums v0 with unit/single-payload variants and exhaustive statement-only matching;
- explicit same-type reinitialization after moves;
- direct static Rust lowering;
- source maps and source-native diagnostics;
- native `check`, `emit-rust`, `build`, `run`, `fmt` workflows;
- verified persistent `evo run` compile caching;
- differential correctness/performance infrastructure and retained artifacts.

The current `string` value semantics are still **static/literal**; do not silently treat the type as a general owned runtime string.

## Prior accepted core milestones

### Records v0

Records v0 remains the accepted ZERO-cost nominal product-type baseline with direct static Rust lowering, explicit by-value ownership, source-native diagnostics and its differential performance gate preserved.

### Enums v0 — #50

Enums v0 milestone **#50** is completed. Final child **#62** is completed.

Final delivery evidence:

- PR #65 final head `1c946af9d15946163b77a92f5d29c89f73469be2`;
- PR CI #277 / run `34112029258`: **SUCCESS**;
- squash merge `6c7bc8a4376966775728765444002f0b6774cd31`;
- post-merge main CI #278 / run `34114143728`: **SUCCESS**.

Accepted corrected Enums parity evidence from CI #276 / run `34108814832` retained exact executable equality and deterministic runtime parity. The earlier unfavorable #275 result also remains preserved.

## Current active roadmap state

The next bounded P0 has now been selected and atomized:

- **#69 — P0 move diagnostics v0: source-native move provenance and recovery guidance**
- parent umbrella: #2
- weakness source: #6 ownership learning / complex type-system errors / move-refactoring cost
- roadmap link: #1 Phase 3.3 move diagnostics

### Why #69 was selected

The current ownership engine has real move semantics, but `MoveState` stores only `value_type + available: bool`. Once a move-only value becomes unavailable, the compiler loses the source span/reason that caused the move. A later error can identify the invalid use but cannot point back to the move origin.

The current diagnostics renderer also has a one-primary-span surface only.

#69 is therefore deliberately diagnostics-only:

- preserve the existing accept/reject semantics;
- preserve generated Rust bytes for accepted programs;
- retain deterministic move provenance through direct consumption, branch joins, repeat analysis and owned enum matching;
- clear stale provenance on exact-type reinitialization;
- add a bounded related/secondary Evolution-source location renderer;
- keep generated-program runtime cost **ZERO** and leave #4/#5 runtime semantics unchanged.

## #69 start gate

Do **not** start feature implementation until the docs-only post-fast-edit-run handoff has its own exact-head CI and is merged.

After that gate:

1. verify live `main`, open PRs/issues and active CI;
2. create a focused feature branch from the verified handoff baseline;
3. implement structured move provenance without changing ownership semantics;
4. add related-span diagnostic rendering while preserving single-span compatibility;
5. add Records/Enums/function/match/if/repeat/reinitialization provenance tests;
6. prove accepted generated Rust remains unchanged;
7. run the normal three-OS CI and existing Ubuntu runtime/performance gates;
8. merge only from exact-head green evidence.

## ZERO-cost boundary

Current accepted Core-language slices must not silently introduce hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.

Diagnostics metadata is compile-time state. #69 must not emit persistent runtime metadata or alter accepted generated program behavior.

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
