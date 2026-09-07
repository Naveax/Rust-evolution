# Rust Evolution — Project State

Last verified update: **2026-09-07**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Latest verified code-bearing `main` baseline: `795461c53f896c2223443cdb022f340a5032a0bd`
- Baseline source: PR **#71** squash merge
- Post-merge main CI **#302** / run `34158551578`: **SUCCESS**

A later docs-only handoff merge may advance `main` beyond the code-bearing baseline above. Always verify the live branch head before starting code. A documentation commit cannot contain its own future squash SHA, a small inconvenience imposed by causality.

## Completed P0 — Move diagnostics v0 (#69)

Issue **#69** is **completed**. PR **#71** is merged.

Delivered behavior:

- move-only record/enum bindings retain compile-time move provenance while unavailable;
- the invalid reuse remains the primary Evolution source span;
- at most one bounded related Evolution-source location identifies the move origin;
- direct moves and the existing enum argument/return/owned-match contexts retain source-native cause information;
- continuing branch provenance is selected deterministically by source order;
- terminal paths remain excluded from continuing-state joins;
- repeat-body move failures retain the responsible body source location;
- exact same-type reinitialization clears stale provenance;
- lexical scope exit forgets provenance with the binding;
- existing single-span lexical/parser/rustc-remap rendering remains compatible;
- public `LowerError { message, span }` remains compatible for this bounded slice;
- no ownership accept/reject semantics changed;
- no generated-program runtime metadata was added;
- generated-program runtime cost remains **ZERO**.

### Final exact-head evidence

Final PR head:

- `74f6a5955d46bd9620045bd39e9703383ae30679`

Final PR CI:

- CI **#301** / run `34141025715`: **SUCCESS**
- Ubuntu: fmt, Clippy, workspace tests, fast edit-run turnaround evidence, benchmark smoke, runtime-repeat, control-flow, logical operators, functions, block-locals, Records v0, Enums v0 and release build all passed;
- Windows/macOS: fmt, Clippy, workspace tests, benchmark smoke and release build passed;
- exactly one PR run existed for the final SHA/workflow/input.

Generated-Rust preservation evidence from the final Ubuntu Enums artifact:

- artifact: `evo-bench-enums-ubuntu-latest`;
- artifact id: `10027015101`;
- artifact digest: `sha256:4d27ca15beb78e9edd50cefaab314f9071d5e6f2bfdff4e49aebf4215407e2ac`;
- `generated.rs`: **1240 bytes**;
- `generated.rs` SHA-256: `61f5f5c99c47196605ae2e461ee589b72a722c4ed5107c6b5fca353795100d83`;
- this exactly matches the accepted pre-#69 Enums generated-Rust baseline from main CI #291;
- correctness: **PASS**;
- normalized LLVM IR equality: `true`;
- exact executable equality: `true`;
- binary size: **2,267,072 bytes** on both sides;
- final verdict: **PASS** via `byte-identical-binary-parity`.

Raw timing remains retained rather than hidden:

- reference median: `16,489,932 ns`;
- Evolution median: `16,558,059 ns`;
- observed ratio: `1.004131430`;
- timing-only verdict: `FAIL`.

Under D-006, byte-identical executables after correctness PASS are deterministic runtime-parity evidence. Scheduler timing noise remains visible but does not turn the same executable bytes into different generated runtime behavior.

### Retained failed-head evidence

Failed/intermediate heads remain evidence and were not rerun merely for color:

- CI #292 / run `34131329043` and CI #295 / run `34132081700`: obsolete pre-concurrency-policy evidence;
- CI #298 / run `34139263767`: rustfmt-only failure after synchronization;
- CI #299 / run `34140117995`: Clippy dead-code failure because Records privately includes the shared move-state file containing enum-only diagnostic reason variants;
- CI #300 / run `34140487296`: Windows-only checkout-CRLF reference-fixture failure; generated Rust itself remained LF.

Each root cause was fixed on a new SHA. Old red runs remain evidence instead of being cosmetically retried.

## Post-merge verification

Squash merge:

- `795461c53f896c2223443cdb022f340a5032a0bd`

Post-merge main CI:

- CI **#302** / run `34158551578`: **SUCCESS**
- Ubuntu repeated fmt, Clippy, workspace tests, turnaround evidence, benchmark smoke, every existing runtime/performance gate and release build successfully;
- Windows and macOS quality/test/benchmark-smoke/release jobs also passed.

Issue #69 closed automatically as **completed** from the merged PR.

## Current implemented language state

`docs/LANGUAGE_SPEC_V0.md` is the implemented-language source of truth.

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
- deterministic source-native move-origin related diagnostics for accepted ownership semantics;
- direct static Rust lowering;
- generated-line source maps and rustc remapping;
- native `check`, `emit-rust`, `build`, `run`, `fmt` workflows;
- verified persistent `evo run` compile caching;
- differential correctness/performance infrastructure and retained artifacts.

The current `string` value semantics remain **static/literal**. Do not silently treat the type as a general owned runtime string.

## Prior accepted P0 — Fast edit-run v0 (#67)

Issue #67 remains completed and PR #68 merged.

Key accepted evidence:

- final PR #68 head `b26a84819e8a89f27220ebd2eb16d4d91d513094`;
- PR CI #288 / run `34123896927`: **SUCCESS**;
- feature merge `17206a702b497731bd5174a0e011225711492d3e`;
- post-feature main CI #289 / run `34124829348`: **SUCCESS**;
- docs handoff merge `ac61b3d36f62a12a9f82df7abe75317b6cbcc7d0`;
- handoff post-main CI #291 / run `34125967329`: **SUCCESS**;
- controlled cold median **69.258 ms**;
- warm median **14.972 ms**;
- observed warm speedup **4.626x**;
- measured warm rustc compile count **0**.

Fast edit-run remains tooling state only and does not alter generated-program semantics.

## Prior accepted core milestones

Records v0 remains the accepted ZERO-cost nominal product-type baseline with direct static Rust lowering and by-value ownership.

Enums v0 milestone #50 and final child #62 remain completed. Its accepted differential evidence retains correctness PASS and exact executable equality. Historical timing evidence remains visible under the benchmark policy.

## Next bounded P0 research

There is currently no open feature PR and no successor atomic P0 issue yet.

The strongest researched candidate is **source-native deterministic typo/symbol suggestions**:

- roadmap: #1 Phase 3.2 Suggested fixes;
- weakness source: #6 complex diagnostics/refactoring cost;
- layer: semantic analysis / diagnostics / developer experience;
- current source-native failure points already exist for unknown locals/functions, nominal types/constructors, record fields and enum variants;
- candidate sets can remain context-specific rather than creating a global fuzzy namespace;
- current identifiers are ASCII, allowing deterministic bounded edit-distance matching without Unicode-identifier redesign;
- the bounded experiment should emit a suggestion only for a conservative unique best candidate within a fixed threshold;
- tied, distant, invisible-scope or wrong-namespace candidates must produce no suggestion;
- suggestion metadata is compile-time only and must not change accepted generated Rust or generated-program runtime behavior.

Do not turn this into LSP completion, parser correction, global spellchecking, namespace redesign, or an excuse to start Result/Option/owned-string/collection semantics in the same issue. Humans have already demonstrated that vague tickets expand quite adequately without compiler assistance.

## ZERO-cost boundary

Current accepted Core-language slices must not silently introduce hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.

Diagnostics metadata is compile-time state. Move provenance and any future typo/symbol suggestion metadata must not emit persistent runtime metadata or alter accepted generated-program behavior.

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
