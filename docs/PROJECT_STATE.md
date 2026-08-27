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
- First implementation slice **#51 — Enums parser: declarations, qualified variants and case-match surface**

Why this follows Records:

- Records established nominal user types, typed aggregate layout, source-spanned declarations and move analysis.
- Enums add closed alternatives and exhaustive control flow without requiring collections, generics or managed runtime machinery.
- This foundation is required before useful `Option` / `Result` ergonomics and richer error handling.

## Enums v0 current syntax experiment

The first parser slice deliberately keeps the surface small:

```text
enum MaybeInt
    None
    Some int
end

value = MaybeInt.Some(41)
match value
case MaybeInt.Some(x)
    print x
case MaybeInt.None
    print 0
end
```

Parser-v0 decisions for #51:

- `enum`, `match`, `case` are explicit keywords;
- variants are unit or one typed payload only;
- construction is fully qualified as `Enum.Variant(...)`;
- `match` is statement-only;
- arms begin with explicit `case` for deterministic recovery;
- patterns are fully enum-qualified;
- unit patterns and one-payload binding patterns only;
- no wildcard, guards, nested arbitrary patterns, generics or Option/Result sugar yet;
- semantic lowering remains fail-closed until nominal enum semantics/exhaustiveness/ownership land.

Enums v0 is also a **ZERO** cost-class target: ordinary Rust enums/matches, no hidden allocation/boxing/clone/dispatch/runtime metadata.

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