# Rust Evolution — Project State

Last verified update: **2026-08-27**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Records v0 final feature baseline on `main`: `ce3018d158d2ce4084a9e569b8eebac6eeb51f8f` (`perf: complete Records v0 parity acceptance`).
- Final Records post-merge CI: **#197 / run `33074128274` — SUCCESS** on Ubuntu, Windows and macOS.
- Ubuntu #197 passed format, Clippy, workspace tests, benchmark smoke, release build, every older runtime gate, and the dedicated Records v0 performance gate.
- Records parent #41 and child issues #43, #46, #47 are completed.

## Completed Records v0 milestone

Records v0 is part of the accepted experimental language surface on `main`.

Implemented and validated:

- nominal record declarations and named record types;
- exact named constructors with deterministic schema-order lowering;
- builtin and acyclic record-valued fields;
- zero-field constructors;
- typed direct/chained scalar field access;
- record parameters and return values;
- by-value move tracking with source-native reuse-after-move diagnostics;
- same-type explicit reinitialization;
- conservative ownership joins across `if`;
- loop-carried move safety across `repeat`;
- explicit rejection of whole-record print/equality and record-valued partial field moves;
- static Rust structs, struct literals and direct field access;
- record declaration/field source mapping plus constructor/access owning-statement mapping;
- real CLI/native process coverage;
- canonical formatter/spec support;
- dedicated Ubuntu differential performance gate.

Records v0 cost class is **ZERO**: no hidden allocation, boxing, GC/RC, implicit clone, dynamic dispatch, runtime object map or reflection metadata.

Dedicated parity evidence from CI #190 / run `33071967025`, artifact `9646205940`:

- correctness: PASS;
- normalized LLVM IR equality: true;
- exact executable equality: true;
- binary size: `2,267,104 B / 2,267,104 B`;
- raw median ratio: `1.000226357`;
- final verdict: PASS;
- verdict basis: `byte-identical-binary-parity`.

## Active P0 — Enums v0

Parent issue:

- **#50 — Enums v0: nominal sum types + exhaustive static matching**

Parser / formatter child:

- **#51 — Enums parser: declarations, qualified variants and case-match surface**
- declaration slice PR #52 merged as `f6796fa8f9f87530b98de0e13bf636fa95c2254a`
- PR #52 validation: CI #202 / run `33079158369` — SUCCESS
- post-merge declaration validation: main CI #203 / run `33079432964` — SUCCESS
- remaining constructor/match slice implemented in **PR #53** on `feature/enums-constructor-match-v0`
- parser code head `acb27b1f54c7e695d46c5395a4d84c6d02cb136c` validated by **CI #214 / run `33090709840` — SUCCESS**

Next semantic child:

- **#54 — Enums semantics: nominal typing, constructors and exhaustive match**
- deliberate boundary: type semantics/lowering/diagnostics only; ownership, Rust enum/match codegen and performance remain later work

## Enums parser / formatter surface implemented in PR #53

Accepted syntax:

```text
value = MaybeInt.None()
value = MaybeInt.Some(41)

match value
case MaybeInt.Some(x)
    print x
case MaybeInt.None
    print 0
end
```

Implemented properties:

- exact-boundary lexer keywords `enum`, `match`, `case` with prefix-identifier regressions;
- source-spanned enum declarations and unit/single-payload variants;
- structured qualified constructor AST with enum name, variant name, arguments and source span;
- existing record constructors, `value.field` and chained field access preserved;
- plain `Enum.Variant` remains field-access-shaped syntax outside `case`;
- statement-only `match` with source-spanned arms/patterns;
- fully qualified unit and one-payload-binding patterns;
- sibling `case` arm boundaries and nested `if` / `repeat` / `match` recovery;
- source-native diagnostics for stray `case`, missing match expression, missing first case, malformed payload binding and missing final `end`;
- canonical/idempotent formatter coverage for enum declarations, constructors, match/case indentation, payload bindings and comments;
- explicit fail-closed lowering for enum declarations, constructors and match statements before enum semantic/codegen support;
- real CLI gates proving unsupported enum execution stops at Evolution source spans and never reaches rustc.

## PR #53 validation evidence

Code head `acb27b1f54c7e695d46c5395a4d84c6d02cb136c`:

- CI **#214 / run `33090709840` — SUCCESS**;
- Ubuntu: format, Clippy, workspace tests, benchmark smoke, runtime repeat gate, control-flow gate, logical-operator gate, Functions v0 gate, Block Locals v0 gate, Records v0 gate and release build all SUCCESS;
- Windows/macOS: format, Clippy, workspace tests, benchmark smoke and release build SUCCESS.

Any later docs-only synchronization commit still requires its own CI before PR #53 merge. Never rerun #214 for a newer SHA.

## Next implementation target — #54 nominal semantics

After PR #53 lands on `main`, continue with a shared nominal type environment rather than extending Records-only special cases.

Required semantic direction:

- `SemanticType` must represent builtin scalars, records and enums nominally;
- record/enum/function namespace policy must be deterministic and tested;
- enum declarations create schemas with variant identity, optional resolved payload type and source spans;
- duplicate enum/variant names and unknown payload types reject source-natively;
- direct/indirect by-value nominal layout cycles reject without hidden boxing;
- `Enum.Variant(...)` resolves exactly one declared variant and validates zero/one payload arity + type;
- constructor expressions evaluate to the nominal enum type;
- `match` scrutinee must be enum-typed;
- arms must belong to the scrutinee enum, be duplicate-free and exhaustive;
- payload bindings receive the declared payload type and are lexical to one arm;
- sibling arm scopes remain independent;
- lowered enum schemas/constructors/matches retain structured identity and spans;
- ownership semantics and Rust codegen remain explicitly fail-closed until their own child work.

Enums v0 remains a **ZERO** cost-class target: ordinary static Rust enums/matches, no hidden allocation, boxing, clone, dispatch or runtime metadata.

## Current stable baseline

Until PR #53 merges:

- stable `main` parser baseline: `f6796fa8f9f87530b98de0e13bf636fa95c2254a`;
- main CI #203 / run `33079432964`: SUCCESS;
- PR #53 code validation CI #214: SUCCESS.

After PR #53 merge, the actual merge commit and post-merge main CI become authoritative and must replace this temporary merge-candidate evidence in the next durable update.

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
