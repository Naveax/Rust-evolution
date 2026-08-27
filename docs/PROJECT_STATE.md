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

The parser supports source-spanned enum declarations, structured qualified constructors, statement-only match/case patterns, bounded recovery and canonical formatting.

## Completed semantic umbrella — #54

**#54 — Enums semantics: nominal typing, constructors and exhaustive match** is closed completed.

Merged delivery slices:

1. **PR #55 — nominal declaration validation**
   - squash merge `ca9641d6c7c57ab603cda8b6a4a091f50cfd625d`
   - post-merge main CI #221 / run `33098011627`: **SUCCESS**
2. **#56 / PR #58 — resolved variants and constructor typing**
   - squash merge `875b4d8fc255d6699b4f89b5e5c769d8dd34383b`
   - final PR CI #229 / run `33103924169`: **SUCCESS**
   - post-merge main CI #230 / run `33104226016`: **SUCCESS**
3. **#57 / PR #59 — exhaustive match typing and arm scopes**
   - final PR head `8bb8e7b56f92910c64943d470455e5820bbca346`
   - final PR CI #235 / run `33113751852`: **SUCCESS**
   - squash merge `e698b7f094863f017b3a29ad0210b638b6bd6a3f`
   - post-merge main CI #236 / run `33113950293`: **SUCCESS**

## Stable semantic behavior on `main`

Current authoritative semantic baseline:

`e698b7f094863f017b3a29ad0210b638b6bd6a3f`

Validated behavior includes:

- source-spanned nominal enum schemas;
- duplicate enum/variant and namespace collision rejection;
- builtin/record/enum payload-reference validation;
- direct/indirect by-value nominal layout cycle rejection without hidden indirection;
- structured exact `Enum.Variant(...)` resolution;
- unit/payload constructor arity checks;
- static payload expression typing through literals/operators/nested constructors/locals/functions/records;
- statically known nominal enum match scrutinees;
- arm enum/variant membership validation;
- duplicate-arm rejection;
- deterministic exhaustive coverage in declaration order;
- unit/payload match binding shape checks;
- typed lexical payload bindings with sibling-scope isolation and no post-match leakage;
- source-native semantic failures before rustc;
- a non-executable resolved match sidecar retaining structured enum/variant identity, source spans, typed payload-binding metadata and exhaustive-only return-path summary;
- explicit fail-closed runtime boundary before enum ownership and Rust enum/match codegen.

Post-merge CI #236 preserved every existing Ubuntu runtime/performance gate including Records v0 and the Windows/macOS quality/test/release matrix.

## Active ownership child — #60

**#60 — Enums v0 ownership + match payload extraction** is now active.

Staging:

- branch: `work/enums-ownership-v0`
- exact base: `e698b7f094863f017b3a29ad0210b638b6bd6a3f`
- no feature PR yet; first create a coherent ownership-infrastructure slice

### Ownership architecture

The existing executable Records lowerer uses `ValueType`, Records `SemanticType`, `Analyzer` and `MoveTracker`. Enum programs are intentionally rejected before that executable Analyzer. Therefore #60 should not begin by punching `Enum` through the executable Records type path.

Instead:

1. extract the availability/reinitialization/control-flow-join mechanics from Records `MoveTracker` into a diagnostics-free generic state core;
2. keep Records `MoveTracker` as a compatibility wrapper with unchanged diagnostics and behavior;
3. reuse that state core from a separate enum pre-codegen ownership validator under `record_environment::enums_impl`;
4. reuse the already-proven enum static type environment and resolved match sidecar rather than creating a competing enum type model.

Conservative v0 ownership target:

- nominal enum values are move-only regardless of payload;
- record/enum payload bindings are move-only and scalar/static payloads trivially reusable;
- by-value enum reads, arguments, returns and constructor payload uses consume move-only values;
- matching an owned enum consumes the whole scrutinee;
- payload ownership follows the declared payload type;
- same-type explicit reinitialization restores availability;
- whole-enum reuse after consuming match fails source-natively;
- partial reuse/partial moves are not inferred;
- no implicit clone, boxing, borrow/reference inference or dynamic runtime machinery.

The current match sidecar retains only `all_arms_return`. #60 may extend each resolved arm with terminal/continuing metadata because ownership joins need to ignore terminal arms individually when some arms return and others continue.

## Remaining Enums v0 queue

After #60:

1. **#61 — static Rust enum/match codegen + source maps**
   - executable lowered enum/match IR;
   - direct static Rust enum definitions, constructors and `match`;
   - generated-source mapping and native correctness corpus.
2. **#62 — differential performance parity + final spec sync**
   - runtime-dependent Enums v0 differential benchmark against equivalent Rust;
   - #4/#5 evidence;
   - final `LANGUAGE_SPEC_V0.md` synchronization only after correctness/ownership/codegen/performance are accepted.

## Parent #50 state

Completed:

- syntax/parser/formatter surface;
- nominal/type semantics;
- constructor semantics;
- exhaustive static match semantics;
- current cross-platform quality matrix and preservation of all prior runtime gates.

Still open:

- explicit enum ownership/move model (#60);
- executable static Rust enum/match lowering and source maps (#61);
- native correctness corpus (#61);
- dedicated Enums performance parity evidence (#62);
- final stable language spec synchronization (#62).

## Deliberate current boundary

Do not add in #60:

- Rust enum/match codegen;
- generated enum source-map snapshots;
- dedicated Enums performance gate;
- final language spec sync;
- generics, guards, wildcard/or/nested arbitrary patterns, Option/Result sugar.

Enums v0 remains a **ZERO** cost-class target: eventually ordinary static Rust enums/matches, never hidden allocation, boxing, clone, dispatch or runtime metadata.

## Current stable baseline

- `main`: `e698b7f094863f017b3a29ad0210b638b6bd6a3f`
- post-merge main CI #236 / run `33113950293`: **SUCCESS**
- parser child #51: completed
- semantic umbrella #54: completed
- ownership child #60: active on `work/enums-ownership-v0`
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
