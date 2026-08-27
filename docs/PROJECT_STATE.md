# Rust Evolution — Project State

Last verified update: **2026-08-27**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Records v0 final feature baseline: `ce3018d158d2ce4084a9e569b8eebac6eeb51f8f`
- Records post-merge CI #197 / run `33074128274`: **SUCCESS**
- Records parent #41 and child issues #43, #46, #47 are completed

## Completed Records v0 milestone

Records v0 remains the accepted ZERO-cost nominal product-type baseline: static Rust structs/field access, by-value move tracking, no hidden allocation/boxing/GC/RC/clone/dynamic dispatch/runtime metadata, with its dedicated differential performance gate preserved.

## Completed Enums parser milestone

Parent: **#50 — Enums v0: nominal sum types + exhaustive static matching**

Parser / formatter child #51 is completed.

- declaration slice PR #52 merged as `f6796fa8f9f87530b98de0e13bf636fa95c2254a`
- constructor/match slice PR #53 squash-merged as `c454fcfe5811a9122b8dfeefad7f4eb4c22c8afa`
- PR #53 final CI #215 / run `33091101594`: **SUCCESS**
- post-merge main CI #216 / run `33091396504`: **SUCCESS**

The stable parser supports source-spanned enum declarations, structured qualified constructors, statement-only match/case patterns, bounded recovery and canonical formatting. Enum execution remains intentionally fail-closed before unsupported runtime semantics/codegen.

## Active semantic umbrella — #54

**#54 — Enums semantics: nominal typing, constructors and exhaustive match**

Delivery is split into atomic slices:

1. **PR #55 — nominal declaration validation**
2. **#56 — resolved variants and constructor typing**
3. **#57 — exhaustive match typing and arm scopes**

Ownership/codegen/performance remain later work.

## PR #55 current validated slice

PR branch: `feature/enums-semantics-v0`

Validated code head: `ccfc0f3e5cfad8e6c66c171725e3b58576c60dcf`

CI #219 / run `33092454867`: **SUCCESS**.

Ubuntu #219 passed format, Clippy, workspace tests, benchmark smoke, all existing runtime/performance gates including Records v0, and release build. Windows/macOS passed format, Clippy, workspace tests, benchmark smoke and release build.

Implemented on #55:

- internal `TypeEnvironment` semantic boundary;
- existing record environment retained behind delegated storage with a transitional `RecordEnvironment` compatibility name;
- duplicate enum-name rejection;
- duplicate variant rejection within one enum;
- same variant name allowed across different enums;
- record/enum nominal namespace collision rejection;
- enum/function namespace collision rejection;
- builtin/record/enum payload-reference validation;
- acyclic record-to-enum and enum-to-record references accepted at declaration-validation level;
- unknown named types rejected source-natively in mixed nominal programs;
- direct and indirect record/enum by-value layout cycles rejected without hidden boxing;
- valid enum programs still stop at the existing Enums semantic/codegen fail-closed gate.

A later docs-synchronized #55 head still requires its own final CI before merge. Until #55 lands on `main`, the items above are PR evidence rather than stable language behavior.

## Next slice — #56 resolved variants + constructor typing

After #55 merges, start #56 from the actual squash-merge baseline.

Required direction:

- resolved enum schemas retain enum name, variant name, optional resolved payload type and source spans;
- semantic type model distinguishes scalars, records and enums nominally;
- enum/function signatures resolve deterministically where required for constructor expression type checking;
- `Enum.Variant(...)` resolves exactly one enum + variant;
- unit variants require zero arguments;
- payload variants require exactly one argument;
- payload expression is statically type checked;
- constructor expression receives the nominal enum type;
- unknown enum/variant/wrong arity/wrong payload type reject at Evolution source spans;
- constructor execution remains fail-closed before ownership/codegen.

## Following slice — #57 exhaustive match typing

After #56 lands:

- require enum-typed scrutinees;
- validate arm enum/variant membership;
- reject duplicate arms;
- require deterministic exhaustive coverage;
- type payload bindings from variant payloads;
- keep bindings lexical to one arm and sibling scopes independent;
- keep ownership joins and Rust match codegen out of the slice.

## Deliberate semantic boundary

Do not add yet:

- enum move/reinitialization rules;
- payload extraction partial-move behavior;
- Rust enum/match codegen;
- source-map codegen snapshots;
- dedicated Enums performance gate;
- generics, guards, wildcard/or/nested arbitrary patterns, Option/Result sugar.

Enums v0 remains a **ZERO** cost-class target: ordinary static Rust enums/matches, no hidden allocation, boxing, clone, dispatch or runtime metadata.

## Current stable baseline

- `main`: `c454fcfe5811a9122b8dfeefad7f4eb4c22c8afa`
- post-merge CI #216 / run `33091396504`: **SUCCESS**
- parser child #51: completed
- semantic umbrella #54: active
- nominal declaration PR #55: code head green in CI #219, final docs-synchronized head pending

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
