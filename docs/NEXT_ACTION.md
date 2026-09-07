# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-07**

## Verified merged feature baseline

Move diagnostics v0 **#69 is completed** and PR **#71 is merged**.

Latest verified code-bearing `main` baseline:

- `795461c53f896c2223443cdb022f340a5032a0bd`
- PR #71 squash merge
- post-merge CI **#302** / run `34158551578`: **SUCCESS**
- Rust toolchain: **1.98.0**

A later docs-only handoff merge may advance the live `main` SHA. Before starting code, read live `main` and active Actions rather than asking this document to predict its own future hash.

## Final move-diagnostics evidence

Final PR head:

- `74f6a5955d46bd9620045bd39e9703383ae30679`

Final PR CI:

- CI **#301** / run `34141025715`: **SUCCESS** on Ubuntu, Windows and macOS.

Final Ubuntu Enums preservation artifact:

- artifact `evo-bench-enums-ubuntu-latest`;
- id `10027015101`;
- digest `sha256:4d27ca15beb78e9edd50cefaab314f9071d5e6f2bfdff4e49aebf4215407e2ac`;
- `generated.rs`: **1240 bytes**;
- SHA-256 `61f5f5c99c47196605ae2e461ee589b72a722c4ed5107c6b5fca353795100d83`;
- exactly matches the accepted pre-#69 Enums baseline;
- correctness PASS;
- normalized LLVM IR equal;
- exact executable equality;
- both binaries 2,267,072 bytes;
- final `byte-identical-binary-parity` PASS.

Post-merge main CI:

- CI **#302** / run `34158551578`: **SUCCESS**;
- Ubuntu repeated turnaround, benchmark smoke, all existing runtime/performance gates and release build;
- Windows/macOS quality/test/benchmark-smoke/release jobs passed.

Retained failed/intermediate evidence was not rerun merely for color:

- #292 / `34131329043` and #295 / `34132081700`: obsolete pre-concurrency-policy evidence;
- #298 / `34139263767`: rustfmt-only failure;
- #299 / `34140117995`: Clippy dead-code failure from Records' private include of shared move-state reason variants;
- #300 / `34140487296`: Windows-only checkout-CRLF reference-fixture failure; generated Rust stayed LF.

## Current accepted ownership diagnostics

For existing by-value Records/Enums ownership semantics:

- invalid reuse is the primary Evolution-source error location;
- one bounded related Evolution-source location may identify the move origin;
- direct consumption and existing enum argument/return/owned-match contexts retain source-native provenance;
- continuing branch provenance selection is deterministic by source order;
- terminal paths do not poison continuing ownership state;
- repeat-body move errors identify the responsible body source;
- exact same-type reinitialization clears stale move provenance;
- lexical scope exit removes provenance with the binding;
- ordinary missing-binding/type-mismatch errors do not inherit fake move notes;
- diagnostics metadata remains compile-time only;
- accepted generated Rust and generated-program runtime behavior remain unchanged.

## Next bounded P0 to atomize

No successor issue has been opened yet. There are no open feature PRs at this handoff point.

Current researched candidate:

**P0 diagnostic suggestions v0: deterministic nearest-symbol hints for source-native unknown-name errors**

Parent: #2

Weakness source: #6 complex error messages / refactoring cost / learning ergonomics.

Roadmap: #1 Phase 3.2 — Suggested fixes.

### Verified root-cause / feasibility

Current lowering already emits source-native semantic errors at distinct resolution sites for:

- local use before definition/outside visible lexical scope;
- unknown named function;
- unknown record/nominal type;
- unknown record constructor;
- unknown record field / constructor field;
- unknown enum constructor;
- unknown enum variant;
- unknown enum/record payload type.

Candidate sets already exist in the relevant lexical scope or record/enum/function environments. A bounded suggestion layer therefore does not require grammar redesign, runtime reflection, a global symbol registry, or generated-code changes.

Current identifiers are ASCII, so v0 can use deterministic bounded edit distance without expanding Unicode identifier semantics.

### Proposed acceptance boundary

Atomize a focused issue before creating a feature branch. The issue should require at minimum:

1. context-specific candidate sets only;
2. a conservative deterministic distance/threshold policy;
3. exactly one suggestion only when there is a unique best candidate within the threshold;
4. tied candidates => no suggestion;
5. distant candidates => no suggestion;
6. invisible lexical-scope locals => no suggestion;
7. names from the wrong namespace => no suggestion;
8. accepted/invalid program classification unchanged except richer diagnostic text;
9. no parser recovery/autocorrection;
10. no LSP/completion protocol work;
11. no runtime metadata/cost;
12. accepted generated Rust unchanged;
13. cross-platform deterministic diagnostic text tests;
14. normal three-OS CI and all existing Ubuntu runtime/performance gates green.

Suggested edit cases for the deterministic matcher:

- one-character substitution;
- insertion;
- deletion;
- adjacent transposition if explicitly supported by the chosen metric;
- same-distance ambiguity;
- threshold boundary;
- empty candidate set;
- case-sensitive distinction under current identifier rules.

Suggested semantic cases:

- visible local typo;
- hidden/out-of-scope local is not suggested;
- function typo does not suggest record/enum names;
- record constructor typo uses nominal constructor candidates only;
- record field typo uses only the resolved record schema's fields;
- enum variant typo uses only the resolved enum's variants;
- nominal type typo uses the correct nominal type namespace;
- no suggestion is attached to unrelated type/ownership/parser errors.

### Diagnostics API boundary

Do not reuse the move-origin related-location sidecar to carry textual spelling suggestions. #69's related location means “this source span caused the move”. A spelling hint has different semantics and often no second source span.

Prefer a small dedicated compile-time text-help/suggestion representation or another equally bounded design. Keep `render_error` compatibility where possible and prove ordinary diagnostics remain byte-stable when no suggestion exists.

## First next action

1. Verify live `main`, open PRs/issues and active CI after the docs-only handoff merge.
2. Atomize the diagnostic-suggestions P0 issue with the boundaries above.
3. Inspect exact current unknown-symbol error sites/tests on that verified baseline.
4. Create a focused feature branch only after the issue exists.
5. Implement the matcher/helper first with deterministic unit tests.
6. Integrate one semantic namespace at a time; do not widen grammar or runtime semantics.
7. Add process-level diagnostic regressions and generated-Rust preservation evidence.
8. Use one natural exact-head CI run and merge only after all gates pass.

## Larger candidates explicitly deferred

Do not fold these into the diagnostic-suggestions slice:

- Result/Option propagation syntax;
- owned/runtime strings;
- Vec/collections;
- general error-handling semantics;
- borrowing/references/lifetime inference;
- global IDE/LSP completion;
- parser autocorrection.

Each has broader language/runtime prerequisites and deserves its own measured issue rather than hiding behind a spelling helper.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input.

If a run is queued or in progress, track that run ID and continue independent work. Failed SHAs are retained evidence; do not rerun them merely to obtain a friendlier color.
