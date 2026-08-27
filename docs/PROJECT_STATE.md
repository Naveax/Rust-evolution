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

1. **PR #55 — nominal declaration validation — MERGED**
2. **#56 / PR #58 — resolved variants and constructor typing — ACTIVE**
3. **#57 — exhaustive match typing and arm scopes — NEXT**

Ownership/codegen/performance remain later work.

## Landed semantic slice — PR #55

PR #55 squash merge:

`ca9641d6c7c57ab603cda8b6a4a091f50cfd625d`

Validation:

- final PR CI #220 / run `33097848047`: **SUCCESS**
- post-merge main CI #221 / run `33098011627`: **SUCCESS**
- Ubuntu #221 preserved every existing runtime/performance gate including Records v0
- Windows/macOS preserved the quality/test/release matrix

Stable #55 behavior on `main`:

- internal `TypeEnvironment` boundary while Records storage/ownership behavior remains unchanged;
- duplicate enum-name rejection;
- duplicate variant rejection within one enum, with variant names reusable across enums;
- record/enum nominal namespace collision rejection;
- enum/function namespace collision rejection;
- builtin, record and enum payload-reference validation;
- acyclic record-to-enum and enum-to-record nominal references accepted at declaration-validation level;
- unknown named payload/field types rejected source-natively in mixed nominal programs;
- direct/indirect record/enum by-value layout cycles rejected without hidden boxing;
- valid enum programs still stop at the Enums ownership/codegen fail-closed boundary.

## Active constructor semantic slice — #56 / PR #58

PR: **#58 — type-check Enums v0 variant constructors**

Branches:

- PR branch: `feature/enums-constructor-typing-v0`
- staging: `work/enums-constructor-typing-v0`

Current implementation direction:

- resolved enum schemas retain enum/variant identity, optional resolved payload type and source spans;
- semantic payload view distinguishes int/bool/string, record nominal types and enum nominal types without touching Records move tracking;
- exact `Enum.Variant(...)` resolution rejects unknown enum and unknown variant;
- unit variants require zero arguments;
- payload variants require exactly one argument;
- literal/operator/nested-enum payload types are checked;
- an ownership-free constructor typing view propagates local binding types, function parameter/return types, named record constructor types and record field types into payload validation;
- invalid constructors fail at Evolution source spans before rustc;
- real CLI regressions cover unknown variants and wrong payload types;
- match payload bindings remain explicitly deferred to #57;
- valid enum execution remains fail-closed before enum ownership and Rust codegen.

## PR #58 CI evidence

- resolved schemas `1791baac...`: CI #222 / run `33098431733` — **SUCCESS**
- enum/variant identity + arity `9335fdc0...`: CI #223 / run `33098652712` — **SUCCESS**
- first payload typing `702fa089...`: CI #224 / run `33099006049` — failed only on a new test's incorrect expected line; production diagnostic correctly pointed to line 13, fmt/Clippy were green
- corrected payload span `6c0342ec...`: CI #225 / run `33099288196` — **SUCCESS**
- first full payload-propagation integration `4c4f1d2f...`: CI #226 / run `33099643034` — Clippy found an unused validation wrapper and `&mut Vec` API; both fixed on a new SHA, no rerun
- code head `e8f84fb2c2c193f4d30c8b513653edf53bdee604`: CI #227 / run `33099872021` is the active validation at this handoff point

Do not duplicate #227 while it is active. Staging may be ahead with docs-only commits; those require their own final CI after the code head is proven green.

## Following slice — #57 exhaustive match typing

After #56 lands on `main`:

- require enum-typed scrutinees;
- validate arm enum/variant membership;
- reject duplicate arms;
- require deterministic exhaustive coverage;
- type payload bindings from variant schemas;
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

- `main`: `ca9641d6c7c57ab603cda8b6a4a091f50cfd625d`
- post-merge main CI #221 / run `33098011627`: **SUCCESS**
- parser child #51: completed
- nominal declaration PR #55: merged and validated on main
- semantic umbrella #54: active
- constructor child #56 / PR #58: active
- match child #57: queued after #56

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
