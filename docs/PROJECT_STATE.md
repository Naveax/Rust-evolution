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

Records v0 is now part of the accepted experimental language surface on `main`.

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

The next Core data-model milestone is:

- Parent issue **#50 — Enums v0: nominal sum types + exhaustive static matching**
- Active parser child **#51 — Enums parser: declarations, qualified variants and case-match surface**
- Declaration slice **PR #52** merged as `f6796fa8f9f87530b98de0e13bf636fa95c2254a`.
- PR #52 validation: **CI #202 / run `33079158369` — SUCCESS** on Ubuntu, Windows and macOS.
- Merge validation: **main CI #203 / run `33079432964` — SUCCESS**.
- Current working branch: **`feature/enums-constructor-match-v0`**.

The landed declaration slice includes:

- exact-boundary lexer keywords `enum`, `match`, `case` with prefix identifier regressions;
- source-spanned `Program.enums`, enum declarations and unit/single-payload variants;
- shared record/enum top-level type declaration region;
- declaration recovery, late/nested declaration diagnostics;
- canonical/idempotent declaration formatting;
- explicit fail-closed lowering and real CLI source-span regression before enum semantics/codegen.

## Current Enums v0 implementation target

Continue #51 with the remaining parser/formatter surface:

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

Current parser-v0 decisions:

- construction is fully qualified as `Enum.Variant(...)`;
- plain `Enum.Variant` outside `case` remains ordinary field-access-shaped syntax;
- existing record constructors and chained field access must remain unchanged;
- `match` is statement-only;
- arms begin with explicit `case` for deterministic recovery;
- patterns are fully enum-qualified;
- unit patterns and one-payload binding patterns only;
- no wildcard, guards, nested arbitrary patterns, generics or Option/Result sugar yet;
- parser AST retains enum name, variant name, argument/binding and precise source spans;
- semantic lowering remains fail-closed for constructor/match surfaces until nominal enum typing, exhaustiveness and ownership land.

Enums v0 remains a **ZERO** cost-class target: ordinary static Rust enums/matches, no hidden allocation/boxing/clone/dispatch/runtime metadata.

## Current validation baseline

Before the active branch:

- `main` head: `f6796fa8f9f87530b98de0e13bf636fa95c2254a`;
- main CI #203 / run `33079432964`: SUCCESS;
- Ubuntu #203 passed format, Clippy, workspace tests, benchmark smoke, every previous runtime/performance gate including Records v0, and release build;
- Windows/macOS #203 quality/test/release jobs passed.

Every active-branch push must preserve the same baseline. One CI run per pushed SHA; do not rerun an already-running or failed SHA merely to obtain a different sample.

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
