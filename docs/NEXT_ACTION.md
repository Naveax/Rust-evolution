# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-08-27**

## Active P0

**#50 — Enums v0: nominal sum types + exhaustive static matching**

Active parser child:

**#51 — Enums parser: declarations, qualified variants and case-match surface**

Current working branch:

`feature/enums-constructor-match-v0`

Records v0 is complete. Enums declaration parsing is also landed on `main`; do not reopen either merge stack unless new evidence appears.

## Verified baseline

Enums declaration parser merge:

`f6796fa8f9f87530b98de0e13bf636fa95c2254a`

Validation:

- PR #52 pre-merge CI #202 / run `33079158369`: **SUCCESS**
- post-merge `main` CI #203 / run `33079432964`: **SUCCESS**
- Ubuntu / Windows / macOS quality and release green
- Ubuntu every existing runtime gate green, including Records v0

Landed declaration surface:

- lexer keywords `enum`, `match`, `case` with prefix regressions;
- `Program.enums`;
- source-spanned enum/variant declarations;
- unit or one typed payload variant declarations;
- record/enum shared top-level type declaration region;
- enum declaration recovery and late/nested diagnostics;
- canonical enum declaration formatting;
- source-native fail-closed lowering/CLI gate before enum semantics.

## Current syntax target

### Qualified construction

```text
value = MaybeInt.None()
value = MaybeInt.Some(41)
```

### Match statement

```text
match value
case MaybeInt.Some(x)
    print x
case MaybeInt.None
    print 0
end
```

Restrictions remain:

- statement-only `match`;
- explicit `case` boundaries;
- fully qualified enum patterns;
- unit pattern or one payload-binding pattern;
- no wildcard/guards/or-patterns;
- no arbitrary nested destructuring;
- no generic enums;
- no Option/Result sugar yet.

## Resume here

1. Confirm branch `feature/enums-constructor-match-v0` still points at the verified `main` baseline before the first code push.
2. Read #50/#51 plus current parser, formatter and lowering code.
3. Add a structured qualified variant-construction expression that stores:
   - enum name;
   - variant name;
   - positional arguments;
   - source span.
4. Extend postfix parsing carefully:
   - existing `value.field` stays field access;
   - existing chained field access stays unchanged;
   - `Enum.Variant(...)` becomes qualified variant construction;
   - `Enum.Variant` without parentheses remains ordinary field-access-shaped syntax outside a case pattern;
   - retain argument count for later semantic arity validation.
5. Keep variant construction explicitly fail-closed in lowering before semantic resolution/codegen.
6. Add parser regressions for unit/payload constructors plus unchanged record constructors and field access.
7. Add statement-only `match` AST with source-spanned arms and patterns.
8. Parse each explicit `case` as either:
   - `Enum.Variant`
   - `Enum.Variant(binding)`
9. Extend parser stop/recovery logic so sibling `case` tokens and nested `if` / `repeat` / `match` blocks cannot consume the wrong `end`.
10. Diagnose top-level stray `case`, missing match expression, missing first case and missing final `end` source-natively.
11. Add formatter rules:
   - tight `Enum.Variant(...)` punctuation;
   - `case` behaves like an arm boundary for indentation;
   - canonical payload-binding pattern spacing;
   - idempotent comments/arms.
12. Add explicit lowering fail-closed handling for parsed match statements.
13. Add real CLI tests proving constructor/match programs fail at Evolution spans and never reach rustc until semantic child work lands.
14. Push atomic commits only after the previous SHA's CI has completed. Never duplicate an active SHA/workflow/input run.
15. Require all existing Ubuntu runtime gates, including Records v0, to stay green.

## Engineering constraint

Do not encode qualified enum identity as concatenated strings such as `"MaybeInt::Some"`. The parser AST must preserve enum and variant names structurally.

Do not silently reinterpret arbitrary `object.method(...)` syntax as supported methods. The only newly recognized qualified call surface in this slice is retained for later enum semantic resolution and must remain fail-closed until that resolution exists.

## Runtime / cost rule

This parser work has no accepted runtime effect. Enums v0 targets ordinary static Rust `enum` + `match` and remains cost class **ZERO**.

Do not add:

- implicit clone/copy;
- boxing;
- GC/RC;
- runtime variant maps;
- reflection metadata;
- dynamic dispatch;
- managed runtime machinery.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent work, but never create multiple active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Do not rerun an old failed SHA merely to obtain a different timing sample.
