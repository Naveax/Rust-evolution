# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-08-27**

## Active P0

Parent milestone: **#50 — Enums v0: nominal sum types + exhaustive static matching**

Semantic umbrella: **#54 — Enums semantics: nominal typing, constructors and exhaustive match**

Current PR: **#55 — validate Enums v0 nominal declarations**

Next semantic children:

- **#56 — resolved variants and constructor typing**
- **#57 — exhaustive match typing and arm scopes**

## Verified stable baseline

Parser / formatter PR #53 was squash-merged to `main` as `c454fcfe5811a9122b8dfeefad7f4eb4c22c8afa`.

Post-merge `main` CI #216 / run `33091396504`: **SUCCESS**. Ubuntu passed every existing runtime/performance gate including Records v0; Windows/macOS passed the quality/test/release matrix. Parser child #51 is closed completed.

## PR #55 nominal declaration slice

Current code head validated as `ccfc0f3e5cfad8e6c66c171725e3b58576c60dcf`.

CI #219 / run `33092454867`: **SUCCESS**.

Ubuntu #219 passed format, Clippy, workspace tests, benchmark smoke, runtime repeat, control-flow, logical operators, Functions v0, Block Locals v0, Records v0 and release build. Windows/macOS passed format, Clippy, workspace tests, benchmark smoke and release build.

Implemented in #55:

- internal `TypeEnvironment` boundary with existing Records storage delegated unchanged;
- `RecordEnvironment` retained only as a transitional compatibility name;
- duplicate enum-name diagnostics;
- duplicate variant diagnostics scoped per enum;
- deterministic record/enum nominal namespace policy;
- deterministic enum/function collision diagnostics;
- builtin, record and enum payload-reference validation;
- acyclic record-to-enum and enum-to-record nominal references accepted at declaration-validation level;
- unknown named payload/field types rejected in mixed nominal programs;
- direct/indirect by-value layout cycles rejected across records and enums;
- valid enum execution remains fail-closed before unsupported semantic lowering/codegen.

The next docs-synchronized #55 head must receive its own CI before merge. Do not rerun #219 for a newer SHA.

## Resume here

1. Inspect PR #55 head and its CI. If the current docs-only head is newer than `ccfc0f3e...`, follow that run rather than rerunning #219.
2. Merge #55 only after the current head is green, then verify the post-merge `main` CI.
3. Update #54 nominal-environment checklist only after the merged `main` evidence exists.
4. Start #56 from the actual #55 squash-merge commit, not from pre-merge staging ancestry.
5. In #56, create resolved enum schemas containing enum name, variant name, optional resolved payload type and source spans.
6. Extend the semantic type model to distinguish builtin scalars, records and enums nominally without changing Records runtime behavior.
7. Validate `Enum.Variant(...)` exactly:
   - enum exists;
   - variant belongs to that enum;
   - unit variant has zero arguments;
   - payload variant has exactly one argument;
   - payload argument type matches;
   - constructor expression receives the nominal enum type.
8. Keep constructor execution fail-closed before ownership/codegen. Do not silently route it into Rust generation.
9. After #56 lands, continue #57 with enum-typed match scrutinees, variant membership, duplicate-arm rejection, deterministic exhaustiveness and arm-local typed payload bindings.
10. Keep ownership/move semantics, Rust enum/match codegen and Enums performance work outside #56/#57.
11. Preserve every Records v0 and earlier quality/runtime/performance gate.

## Engineering constraints

Enum and variant identity must remain structured. Do not encode `Enum.Variant` as a concatenated magic name.

Do not infer arbitrary method-call semantics from `object.member(...)`.

Do not invent enum ownership rules merely to make type checking convenient. Unsupported runtime behavior must remain fail-closed.

Enums v0 remains cost class **ZERO**: no implicit cloning, hidden boxing, GC/RC, runtime variant maps, reflection metadata, dynamic dispatch or managed-runtime machinery.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent work, but never create multiple active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Do not rerun an old failed SHA merely to obtain a different result or timing sample.
