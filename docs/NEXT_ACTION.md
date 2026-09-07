# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-07**

## Verified merged feature baseline

Fast edit-run v0 **#67 is completed** and PR **#68 is merged**.

Latest verified code-bearing `main` baseline:

- `17206a702b497731bd5174a0e011225711492d3e`
- PR #68 squash merge
- post-merge CI **#289** / run `34124829348`: **SUCCESS**
- Rust toolchain: **1.98.0**

A later docs-only handoff merge may advance the live `main` SHA. Before starting code, read the live branch head and active Actions rather than assuming this document can predict its own future merge hash.

## Final fast edit-run evidence

Final PR head:

- `b26a84819e8a89f27220ebd2eb16d4d91d513094`

Final PR CI:

- CI **#288** / run `34123896927`: **SUCCESS**

Controlled Ubuntu turnaround evidence:

- cold median: **69.258 ms** across 5 fresh-cache samples;
- warm median: **14.972 ms** across 9 measured warm samples after one untimed prime;
- warm speedup: **4.626x**;
- cold rustc compile count: **5**;
- measured warm rustc compile count: **0**;
- verdict: **PASS**;
- artifact id `10019395817`;
- digest `sha256:2cbb68765c69b231bdee08460a1a925ca336af39ac53e1833f1ac57c8920ca34`.

The cache is developer-tooling state only. Accepted generated Rust bytes and generated-program runtime semantics were not changed.

Retained failed-head evidence remains visible:

- #285 / `34122517074`: rustfmt-only failure;
- #286 / `34122829838`: turnaround measurement passed but artifact upload path failed.

Neither failed SHA was rerun.

## Current implemented language state

`docs/LANGUAGE_SPEC_V0.md` is still the implemented-language source of truth.

Relevant current ownership behavior:

- Records v0 and Enums v0 are nominal and by-value;
- move-only nominal locals become unavailable after consuming reads/calls/returns/matches according to existing lowering rules;
- exact same-type reinitialization restores availability;
- `if`, `repeat`, and exhaustive `match` use conservative ownership joins;
- terminal branches are excluded from continuing-state joins;
- no implicit clone, borrow, reference inference, boxing or managed runtime is inserted.

## Next active P0

**#69 — P0 move diagnostics v0: source-native move provenance and recovery guidance**

Parent: #2

Weakness source: #6 ownership learning curve / complex type-system errors / move-refactoring cost.

Roadmap: #1 Phase 3.3 move diagnostics.

### Root cause already verified

Current `crates/evo-lowering/src/move_state.rs` stores each binding as essentially:

```text
value_type + available: bool
```

When a move-only binding becomes unavailable, the source move location/reason is discarded.

Current `crates/evo-diagnostics/src/lib.rs` exposes a one-primary-span `render_error` surface. A use-after-move error can point at the invalid use but cannot structurally point back to the Evolution source that consumed the value.

## Start gate

The docs-only post-fast-edit-run handoff must first:

1. receive its own exact-head CI;
2. remain docs-only;
3. merge cleanly to `main`;
4. have its post-merge main state verified.

Only then start #69 implementation.

## #69 implementation sequence

After the handoff gate is green:

1. verify live `main`, working diff, open PRs/issues and active CI;
2. create `feature/move-diagnostics-v0` from the verified baseline;
3. keep the first code patch focused on structured move provenance, not renderer cosmetics;
4. extend move-state entries so unavailable move-only bindings retain a deterministic Evolution-source span and reason;
5. preserve current availability/merge/reinitialization semantics exactly;
6. propagate structured provenance through the Records/Enums ownership path;
7. add a bounded related/secondary-span diagnostic renderer while keeping existing single-span rendering compatible;
8. wire CLI lowering diagnostics to the related source location without exposing generated Rust;
9. add focused unit + process-level tests;
10. prove accepted generated Rust remains byte-for-byte unchanged;
11. run normal exact-head CI and merge only after all gates pass.

## Required #69 test matrix

At minimum cover:

- direct record move then reuse;
- direct enum move then reuse;
- by-value function argument consumption;
- owned enum `match` consumption;
- one continuing `if` branch moves the value;
- terminal branch move does not poison continuation;
- both continuing branches move the same value with deterministic provenance selection;
- reinitialization clears old provenance and later moves record the new site;
- repeat-body move identifies the body source responsible for later-iteration invalidity;
- child-scope provenance disappears with the child binding;
- missing-binding/type-mismatch diagnostics do not become fake move diagnostics;
- related-span renderer handles UTF-8, tabs and zero-width spans;
- single-span lexical/parser/rustc-remap rendering remains compatible.

## Non-negotiable #69 boundaries

- **No ownership semantic changes.** Existing valid/invalid program classification must remain the same except richer diagnostics.
- **No generated Rust changes for accepted programs.** If generated bytes change, stop and explain why before treating the work as diagnostics-only.
- **No runtime cost.** Diagnostic provenance is compile-time state only.
- No borrowing, lifetimes, references, partial moves, clone insertion, Copy inference, field-level move semantics expansion, LSP protocol work or generalized diagnostics-framework redesign.
- Existing #4/#5 Ubuntu runtime/performance gates must remain green even though no new runtime benchmark is required for a diagnostics-only slice.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input.

If a run is queued or in progress, track that run ID and continue independent work. Failed SHAs are retained evidence; do not rerun them merely to obtain a friendlier color.
