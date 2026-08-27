# Rust Evolution — Project State

Last verified update: **2026-08-27**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Records v0 final feature baseline: `ce3018d158d2ce4084a9e569b8eebac6eeb51f8f`
- Records post-merge CI #197 / run `33074128274`: **SUCCESS**

Records v0 remains the accepted ZERO-cost nominal product-type baseline: static Rust structs/field access, by-value move tracking, no hidden allocation/boxing/GC/RC/clone/dynamic dispatch/runtime metadata, with its differential performance gate preserved.

## Enums v0 milestone — #50

Parent: **#50 — Enums v0: nominal sum types + exhaustive static matching**

Completed parser surface:

- parser/formatter child #51 completed
- PR #52 merged as `f6796fa8f9f87530b98de0e13bf636fa95c2254a`
- PR #53 squash-merged as `c454fcfe5811a9122b8dfeefad7f4eb4c22c8afa`
- PR #53 final CI #215 / run `33091101594`: **SUCCESS**
- post-merge main CI #216 / run `33091396504`: **SUCCESS**

The parser supports source-spanned enum declarations, structured qualified constructors, statement-only match/case patterns, bounded recovery and canonical formatting. Runtime enum execution is still intentionally fail-closed.

## Semantic umbrella — #54

**#54 — Enums semantics: nominal typing, constructors and exhaustive match**

Delivery slices:

1. **PR #55 — nominal declaration validation — MERGED**
2. **#56 / PR #58 — resolved variants and constructor typing — MERGED**
3. **#57 / PR #59 — exhaustive match typing and arm scopes — ACTIVE / FINAL DOCS VALIDATION**

Ownership, executable Rust codegen and dedicated Enums performance remain separate following children.

## Landed semantic slice — PR #55

Squash merge:

`ca9641d6c7c57ab603cda8b6a4a091f50cfd625d`

Validation:

- final PR CI #220 / run `33097848047`: **SUCCESS**
- post-merge main CI #221 / run `33098011627`: **SUCCESS**

Stable behavior includes duplicate enum/variant rejection, explicit nominal namespaces, builtin/record/enum payload reference validation and direct/indirect by-value nominal cycle rejection without hidden boxing.

## Landed semantic slice — #56 / PR #58

Squash merge:

`875b4d8fc255d6699b4f89b5e5c769d8dd34383b`

Validation:

- final PR CI #229 / run `33103924169`: **SUCCESS**
- post-merge main CI #230 / run `33104226016`: **SUCCESS**
- Ubuntu #230 preserved every existing runtime/performance gate including Records v0
- Windows/macOS preserved the quality/test/release matrix

Stable constructor semantics on `main`:

- resolved enum schemas retain structured enum/variant identity, optional payload type and source spans;
- semantic payload typing distinguishes int/bool/string, record nominal types and enum nominal types without changing Records move tracking;
- exact `Enum.Variant(...)` resolution;
- zero arguments for unit variants and exactly one for payload variants;
- payload expression type checking through literals/operators/nested constructors/locals/functions/records;
- invalid constructors stop source-natively before rustc;
- valid enum execution remains fail-closed before ownership/codegen.

## Active semantic slice — #57 / PR #59

PR: **#59 — type-check Enums v0 exhaustive matches**

Branches:

- PR branch: `feature/enums-match-typing-v0`
- staging: `work/enums-match-typing-v0`

Verified code head before final docs sync:

`a5270d2a86b1103a431382325c5d77752ccebcf1`

Implemented match semantics:

- scrutinee must have a statically known nominal enum type;
- arm qualifiers must name the scrutinee enum;
- every arm variant must resolve in the enum schema;
- duplicate variant arms are rejected;
- exhaustive coverage requires every declared variant exactly once;
- non-exhaustive diagnostics list missing variants deterministically in declaration order;
- unit variants reject payload bindings;
- payload variants require one binding under the frozen parser surface;
- payload bindings receive the declared payload type;
- bindings are lexical to one arm only;
- sibling arm scopes are independent;
- arm locals do not leak after the match;
- payload bindings obey the current no-shadowing policy;
- nested `if` / `repeat` / `match` semantic scopes remain deterministic;
- invalid match programs fail at Evolution source spans before rustc.

## Retained match semantic sidecar

#57 also introduces a non-executable semantic sidecar for later ownership/codegen work:

- structured enum name and variant name for each validated arm;
- match and arm source spans;
- typed payload-binding metadata;
- source-statement indexing without concatenated magic names;
- `all_arms_return`, computed only after structural exhaustiveness succeeds;
- parser ↔ resolved-sidecar invariant validation before the existing fail-closed runtime boundary.

This is semantic metadata, not Rust enum/match codegen. It creates no ownership joins and no runtime representation.

## PR #59 CI evidence

- structural head `661786a4...`: CI #231 / run `33104721456` — **SUCCESS**
- first typed-match head `a5a8fb8e...`: CI #232 / run `33105128042` — failed only because one new test expected line 9 while the correct payload-binding pattern span was line 10; fmt/Clippy and all other relevant tests were green; failed SHA was not rerun
- corrected typed-match head `d7d6818937c1d1f353b271938ca14b643b2ac01c`: CI #233 / run `33105515592` — **SUCCESS**
- resolved-sidecar head `a5270d2a86b1103a431382325c5d77752ccebcf1`: CI #234 / run `33113246845` — **SUCCESS**
- Ubuntu #234 passed format, Clippy, workspace tests, benchmark smoke, runtime repeat, control-flow, logical operators, Functions v0, Block Locals v0, Records v0 and release build
- Windows/macOS #234 passed format, Clippy, workspace tests, benchmark smoke and release build

The staging branch is now newer only because durable handoff docs are being synchronized with #234 and the following queue. That docs-synchronized SHA must receive its own final CI after one fast-forward to PR #59.

## Following Enums v0 queue

The remaining #50 work is now atomized:

1. **#60 — Enums v0 ownership + match payload extraction**
   - explicit by-value enum move/reinitialization semantics;
   - source-native reuse-after-move;
   - payload extraction ownership and conservative control-flow joins;
   - no hidden clone/boxing.
2. **#61 — static Rust enum/match codegen + source maps**
   - executable lowered enum/match IR;
   - direct Rust enum definitions, constructors and `match`;
   - generated-source mapping and native correctness corpus.
3. **#62 — differential performance parity + final spec sync**
   - runtime-dependent Enums v0 benchmark against equivalent Rust;
   - #4/#5 parity evidence;
   - `LANGUAGE_SPEC_V0.md` synchronization only after correctness/ownership/codegen/performance are proven.

## Deliberate current boundary

Do not add in #57:

- enum move/reinitialization rules;
- payload extraction ownership/partial moves;
- Rust enum/match codegen;
- source-map codegen snapshots;
- dedicated Enums performance gate;
- generics, guards, wildcard/or/nested arbitrary patterns, Option/Result sugar.

Enums v0 remains a **ZERO** cost-class target: eventually ordinary static Rust enums/matches, never hidden allocation, boxing, clone, dispatch or runtime metadata.

## Current stable baseline

- `main`: `875b4d8fc255d6699b4f89b5e5c769d8dd34383b`
- post-merge main CI #230 / run `33104226016`: **SUCCESS**
- parser child #51: completed
- nominal declaration PR #55: merged and validated
- constructor child #56 / PR #58: merged and validated
- match child #57 / PR #59: code head #234 green, final docs-synchronized CI then merge
- ownership child #60: queued after #57/#54 merged-main completion
- codegen child #61: queued after #60
- performance/spec child #62: queued after #61

## Durable continuation infrastructure

Read order:

1. `AGENTS.md`
2. `docs/PROJECT_STATE.md`
3. `docs/NEXT_ACTION.md`
4. `docs/LANGUAGE_SPEC_V0.md`
5. active issue/PR/Actions referenced there

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.

`LANGUAGE_SPEC_V0.md` intentionally remains behind Enums parser/semantic experiments until #62 because executable ownership/codegen/performance acceptance is not yet complete.

## Handoff invariant

Every significant merge or incomplete stopping point must keep `PROJECT_STATE.md`, `NEXT_ACTION.md`, issue/PR evidence and durable decisions synchronized with GitHub reality.

The repository is the project memory. The chat transcript is not.
