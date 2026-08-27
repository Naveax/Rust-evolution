# Rust Evolution — Project State

Last verified update: **2026-08-27**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Records v0 final feature baseline: `ce3018d158d2ce4084a9e569b8eebac6eeb51f8f`.
- Records post-merge CI #197 / run `33074128274`: **SUCCESS**.
- Records parent #41 and child issues #43, #46, #47 are completed.

## Completed Records v0 milestone

Records v0 remains the accepted ZERO-cost nominal product-type baseline: static Rust structs/field access, by-value move tracking, no hidden allocation/boxing/GC/RC/clone/dynamic dispatch/runtime metadata, with its dedicated differential performance gate preserved.

## Enums v0 parser milestone completed

Parent:

- **#50 — Enums v0: nominal sum types + exhaustive static matching**

Completed parser / formatter child:

- **#51 — Enums parser: declarations, qualified variants and case-match surface**
- declaration slice PR #52 merged as `f6796fa8f9f87530b98de0e13bf636fa95c2254a`
- constructor/match slice PR #53 squash-merged as `c454fcfe5811a9122b8dfeefad7f4eb4c22c8afa`
- PR #53 final CI #215 / run `33091101594`: **SUCCESS**
- post-merge main CI #216 / run `33091396504`: **SUCCESS**
- issue #51 closed completed

Ubuntu #216 passed format, Clippy, workspace tests, benchmark smoke, runtime repeat, control-flow, logical operators, Functions v0, Block Locals v0, Records v0 and release build. Windows/macOS passed format, Clippy, workspace tests, benchmark smoke and release build.

The stable parser/formatter surface includes:

- exact-boundary `enum`, `match`, `case` keywords;
- source-spanned enum declarations and unit/single-payload variants;
- structured `Enum.Variant(...)` constructor AST;
- statement-only source-spanned `match` with explicit `case` arms;
- fully qualified unit and one-payload-binding patterns;
- bounded nested recovery with sibling-case/end preservation;
- canonical/idempotent enum constructor and match formatting;
- CLI fail-closed regressions proving unsupported enum execution stops at Evolution source spans before rustc.

No enum runtime semantics were introduced by #51.

## Active semantic child — #54

**#54 — Enums semantics: nominal typing, constructors and exhaustive match**

Working staging branch:

`work/enums-semantics-v0`

The branch was reset to the actual parser merge baseline `c454fcfe5811a9122b8dfeefad7f4eb4c22c8afa` before semantic work began.

Current staging changes, not yet accepted on `main`:

- introduced an internal `TypeEnvironment` wrapper while preserving `RecordEnvironment` as a temporary compatibility alias;
- existing Records type resolution, constructor validation and field lookup remain delegated unchanged;
- added enum declaration semantic validation before the existing enum fail-closed gate;
- duplicate enum names reject source-natively;
- duplicate variants within one enum reject source-natively;
- variant names remain scoped by enum;
- record/enum nominal namespace collisions reject source-natively;
- enum/function name collisions reject source-natively;
- valid enum programs still fail closed before semantic lowering/codegen.

These staging changes still require their own PR CI. They must not be treated as stable behavior merely because the parser baseline is green.

## #54 semantic direction

The next accepted semantic layer should establish one nominal type graph:

- builtin scalar types;
- record nominal types;
- enum nominal types;
- resolved enum variant payloads;
- deterministic record/enum/function namespace rules;
- by-value layout-cycle rejection across records and enums;
- exact enum/variant constructor resolution and payload arity/type checking;
- enum-typed match scrutinees;
- arm enum membership, duplicate-arm rejection and exhaustiveness;
- typed payload bindings scoped to one arm;
- structured lowered enum schemas/constructors/matches.

## Deliberate #54 boundary

Do not add yet:

- enum ownership/move/reinitialization rules;
- payload extraction partial-move behavior;
- Rust enum/match codegen;
- source-map codegen snapshots;
- dedicated Enums performance gate;
- generics, wildcard/guards/or-patterns/nested arbitrary patterns, Option/Result sugar.

Any runtime-facing enum surface must remain explicitly fail-closed until its later child work lands.

Enums v0 remains a **ZERO** cost-class target: ordinary static Rust enums/matches, no hidden allocation, boxing, clone, dispatch or runtime metadata.

## Current stable baseline

- `main`: `c454fcfe5811a9122b8dfeefad7f4eb4c22c8afa`
- post-merge CI #216 / run `33091396504`: **SUCCESS**
- parser child #51: completed
- active semantic child: #54

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
