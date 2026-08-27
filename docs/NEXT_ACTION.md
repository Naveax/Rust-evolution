# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-08-27**

## Active P0

**#50 — Enums v0: nominal sum types + exhaustive static matching**

Parser / formatter child:

**#51 — Enums parser: declarations, qualified variants and case-match surface**

Semantic child prepared next:

**#54 — Enums semantics: nominal typing, constructors and exhaustive match**

Parser merge candidate:

- PR **#53 — feat: add Enums v0 constructor and match parser surface**
- branch `feature/enums-constructor-match-v0`
- code head validated as `acb27b1f54c7e695d46c5395a4d84c6d02cb136c`
- CI **#214 / run `33090709840` — SUCCESS**
- Ubuntu passed format, Clippy, workspace tests, benchmark smoke, every existing runtime/performance gate including Records v0, and release build
- Windows/macOS passed format, Clippy, workspace tests, benchmark smoke and release build

Records v0 is complete. Enums declaration parsing is landed on `main`; the remaining #51 constructor/match surface is implemented in PR #53 and remains fail-closed before enum semantics/codegen.

## Implemented parser surface in #51

### Qualified construction

```text
value = MaybeInt.None()
value = MaybeInt.Some(41)
```

Implemented properties:

- structured `EnumConstruct` AST stores enum name, variant name, positional arguments and source span;
- existing `value.field` and chained field access remain field access;
- plain `Enum.Variant` without parentheses remains field-access-shaped syntax outside `case`;
- record constructors remain unchanged;
- constructor lowering fails closed before enum semantic support.

### Match statement

```text
match value
case MaybeInt.Some(x)
    print x
case MaybeInt.None
    print 0
end
```

Implemented properties:

- statement-only `match`;
- explicit `case` arm boundaries;
- fully qualified unit and one-payload-binding patterns;
- source-spanned match statements, arms and patterns;
- sibling `case` terminates the current arm;
- nested `if` / `repeat` / `match` preserves the correct `end` boundary;
- stray `case`, missing match expression, missing first case, malformed payload binding and missing final `end` are diagnosed source-natively with bounded recovery;
- match lowering fails closed before enum semantic support.

Formatter coverage includes canonical/idempotent qualified constructors, `match` / `case` indentation, payload-binding spacing and comment/arm boundaries.

CLI regressions prove enum declarations, constructors and match statements stop at Evolution source spans and do not reach rustc.

## Resume here

1. Inspect PR #53 and its current head before changing anything. If a newer docs-only head exists, follow that run instead of re-running #214.
2. If PR #53 is not merged, require one green CI for its current head, then merge it. Do not rerun an old failed or already-running SHA.
3. After merge, verify the post-merge `main` CI before treating #51 as complete on the stable branch.
4. Close/update #51 only after the merged `main` evidence exists.
5. Start #54 from the merged `main` head. A pre-merge staging branch must be fast-forwarded/recreated from the actual merge baseline before it becomes authoritative.
6. Build one shared nominal type environment rather than layering enum special cases on the Records-only environment:
   - builtin scalar types;
   - record nominal types;
   - enum nominal types;
   - deterministic record/enum/function namespace policy.
7. Add enum schema/variant validation:
   - duplicate enum names;
   - duplicate variants;
   - builtin/record/enum payload type resolution;
   - unknown payload types;
   - direct/indirect by-value nominal layout-cycle rejection.
8. Add constructor semantics:
   - exact enum + variant resolution;
   - unit variant requires zero arguments;
   - payload variant requires exactly one argument;
   - payload type checking;
   - constructor expression has nominal enum type.
9. Add match semantics:
   - scrutinee must be enum-typed;
   - arm variants must belong to that enum;
   - duplicate arms rejected;
   - exhaustiveness checked source-natively;
   - payload bindings receive the declared type and remain arm-local;
   - sibling arm scopes remain independent.
10. Keep ownership, Rust enum/match codegen and Enums performance work out of #54. Unsupported runtime surfaces must remain explicitly fail-closed at the next boundary.
11. Preserve every Records v0 and earlier quality/runtime/performance gate.

## Engineering constraints

Do not encode qualified enum identity as concatenated strings such as `"MaybeInt::Some"`. Enum and variant identity remain structured through parser, semantic environment and lowered IR.

Do not silently reinterpret arbitrary `object.method(...)` syntax as supported methods.

Do not guess ownership semantics while implementing #54. Enum move/reinitialization and payload-extraction ownership belong to a later child unless a minimal dependency is proven.

## Runtime / cost rule

Enums v0 remains cost class **ZERO** and targets ordinary static Rust `enum` + `match`.

Do not add:

- implicit clone/copy assumptions;
- boxing;
- GC/RC;
- runtime variant maps;
- reflection metadata;
- dynamic dispatch;
- managed runtime machinery.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent work, but never create multiple active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Do not rerun an old failed SHA merely to obtain a different result or timing sample.
