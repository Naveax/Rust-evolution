# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-08-27**

## Active P0

**#50 — Enums v0: nominal sum types + exhaustive static matching**

Completed parser / formatter child:

**#51 — Enums parser: declarations, qualified variants and case-match surface**

Active semantic child:

**#54 — Enums semantics: nominal typing, constructors and exhaustive match**

Current semantic staging branch:

`work/enums-semantics-v0`

## Verified stable baseline

Parser / formatter PR #53 was squash-merged to `main` as:

`c454fcfe5811a9122b8dfeefad7f4eb4c22c8afa`

Validation:

- PR #53 final CI #215 / run `33091101594`: **SUCCESS**
- post-merge `main` CI #216 / run `33091396504`: **SUCCESS**
- Ubuntu #216 passed format, Clippy, workspace tests, benchmark smoke, every existing runtime/performance gate including Records v0, and release build
- Windows/macOS #216 passed format, Clippy, workspace tests, benchmark smoke and release build
- issue #51 is closed as completed

Do not reopen the parser merge stack unless new evidence appears.

## Landed Enums parser surface

The stable parser accepts the frozen v0 surface:

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

The AST preserves enum/variant identity structurally. Existing record constructors, direct/chained field access and plain `Enum.Variant` field-shaped syntax remain unchanged. Match recovery and formatter behavior are source-native and bounded. Enum declarations, constructors and match statements remain explicitly fail-closed before unsupported runtime semantics/codegen.

## #54 work in progress

Current staging work is based directly on merge commit `c454fcfe5811a9122b8dfeefad7f4eb4c22c8afa`.

Implemented on staging but not yet accepted on `main`:

1. Introduced an internal `TypeEnvironment` boundary while retaining `RecordEnvironment` as a temporary compatibility alias for Records v0 callers.
2. Kept all existing record type resolution, constructor validation and field access behavior delegated through the same record implementation.
3. Added source-native enum declaration semantic validation before the existing fail-closed gate:
   - duplicate enum names;
   - duplicate variant names within one enum;
   - record/enum nominal namespace collisions;
   - enum/function namespace collisions;
   - same variant name remains legal in different enums.
4. Valid enum programs still stop at the existing Enums semantic/codegen gate. No runtime behavior has been enabled.

The staging changes require CI validation before they become an accepted semantic baseline.

## Resume here

1. Confirm `main` still contains merge `c454fcfe5811a9122b8dfeefad7f4eb4c22c8afa` and post-merge CI #216 remains SUCCESS.
2. Inspect issue #54 and the current semantic PR/staging head before pushing. Never duplicate an active SHA/workflow/input run.
3. Finish the shared nominal type model:
   - builtin scalar types;
   - record nominal types;
   - enum nominal types;
   - deterministic record/enum/function namespace policy.
4. Resolve enum variant payload types through that shared model:
   - builtin payloads;
   - record payloads;
   - enum payloads;
   - unknown payload types rejected at Evolution spans.
5. Extend by-value layout-cycle validation across the nominal graph so record/enum cross-cycles cannot force hidden boxing.
6. Add constructor semantics:
   - exact enum + variant resolution;
   - unit variants require zero arguments;
   - payload variants require exactly one argument;
   - payload argument type checked statically;
   - constructor expression has nominal enum type.
7. Add match semantics:
   - scrutinee must be enum-typed;
   - arm variants must belong to that enum;
   - duplicate arms rejected;
   - exhaustiveness checked source-natively;
   - payload bindings receive the declared type and remain arm-local;
   - sibling arm scopes remain independent.
8. Lower enum schemas/constructors/matches into structured IR only after validation, but keep Rust codegen explicitly fail-closed in #54.
9. Keep enum ownership/move semantics, payload extraction ownership, Rust enum/match codegen and Enums performance work out of #54.
10. Preserve every Records v0 and earlier quality/runtime/performance gate.

## Engineering constraints

Do not encode qualified enum identity as concatenated strings such as `"MaybeInt::Some"`. Enum and variant identity must remain structured through parser, semantic environment and lowered IR.

Do not silently reinterpret arbitrary `object.method(...)` syntax as supported methods.

Do not guess enum ownership rules in #54. The parser and type system may know a value is an enum without deciding yet whether every read consumes it.

## Runtime / cost rule

Enums v0 remains cost class **ZERO** and targets ordinary static Rust `enum` + `match`.

Do not add implicit cloning, hidden boxing, GC/RC, runtime variant maps, reflection metadata, dynamic dispatch or managed-runtime machinery.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent work, but never create multiple active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Do not rerun an old failed SHA merely to obtain a different result or timing sample.
