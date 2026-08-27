# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-08-27**

## Active P0

**#50 — Enums v0: nominal sum types + exhaustive static matching**

First implementation slice:

**#51 — Enums parser: declarations, qualified variants and case-match surface**

Records v0 is complete on `main`; do not reopen its merge stack unless new evidence appears.

## Accepted baseline

Records final feature merge:

`ce3018d158d2ce4084a9e569b8eebac6eeb51f8f`

Final Records validation:

- CI #197 / run `33074128274`: **SUCCESS**
- Ubuntu / Windows / macOS quality and release green
- Ubuntu all previous runtime gates green
- Ubuntu `Records v0 performance gate` green
- #41, #43, #46, #47 completed

Dedicated Records parity evidence remains:

- correctness PASS
- normalized LLVM equal
- exact executable bytes equal
- binary size `2,267,104 B / 2,267,104 B`
- raw timing ratio `1.000226357`
- final verdict PASS via `byte-identical-binary-parity`

## #51 syntax target

### Enum declaration

```text
enum MaybeInt
    None
    Some int
end
```

Only unit variants and one typed payload variant form are in the first parser slice.

### Qualified variant construction

```text
value = MaybeInt.None()
value = MaybeInt.Some(41)
```

The enum qualifier is mandatory. Do not introduce bare global variant constructors in v0.

### Match statement

```text
match value
case MaybeInt.Some(x)
    print x
case MaybeInt.None
    print 0
end
```

First-slice restrictions:

- statement-only `match`;
- explicit `case` arm boundaries;
- fully qualified patterns;
- unit pattern or one payload-binding pattern only;
- no wildcard;
- no guards;
- no arbitrary nested destructuring;
- no generic enums;
- no `Option` / `Result` sugar yet.

## Resume here

1. Confirm `main` head and latest CI are still green before branching.
2. Create `feature/enums-parser-v0` from current `main`.
3. Read #50 and #51 plus current lexer/parser/formatter/lowering code.
4. Add exact-boundary lexer keywords:
   - `enum`
   - `match`
   - `case`
5. Add prefix regressions so `enumerate`, `matcher`, `casework` remain identifiers.
6. Extend public parser AST with:
   - `Program.enums`;
   - source-spanned enum/variant declarations;
   - optional single payload type;
   - qualified variant construction expression;
   - match statement / match arms / source-spanned patterns.
7. Use the record declaration-region policy for enums: records/enums before executable top-level statements; preserve current function-placement compatibility.
8. Extend postfix parsing without regressing field access:
   - `value.field` remains field access;
   - `Enum.Variant(...)` becomes qualified variant construction;
   - plain `Enum.Variant` remains ordinary field-access-shaped syntax outside a `case` pattern.
9. Parse `match` arms with explicit `case` stop boundaries and existing nested block semantics.
10. Update recovery depth/stop logic so nested `match` / `if` / `repeat` do not consume sibling cases or wrong `end`s.
11. Add formatter support for enum declarations, qualified constructors and match/case indentation.
12. Keep lowering fail-closed for any enum-bearing executable surface until the semantic child slice lands. Parsed enums must never disappear silently into generated Rust.
13. Add lexer/parser/formatter/CLI fail-closed tests from #51.
14. Run one CI per pushed SHA. While CI runs, work independent tasks; never dispatch/rerun the same active SHA/workflow/input.
15. Require all existing Ubuntu performance gates, including Records v0, to remain green even though this parser slice has no runtime feature impact.

## First implementation constraints

Do not add enum runtime semantics in the parser slice merely because the AST makes it tempting.

The parser PR should prove syntax, recovery, formatting and fail-closed safety. Nominal enum type collection, exhaustiveness and move ownership belong in the next child slice after the parser surface is green.

Do not add:

- implicit clone/copy;
- boxing;
- runtime variant maps;
- reflection metadata;
- dynamic dispatch;
- generic payloads;
- methods/traits/derive;
- match guards/wildcards/or-patterns.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent work, but never create multiple active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Do not rerun an old failed SHA just to obtain a different timing sample.

## Performance rule

Enums v0 targets ordinary static Rust enum/match lowering and remains cost class **ZERO**.

The dedicated enum performance gate comes only after parser + nominal semantics + ownership + codegen are proven. All prior gates remain mandatory throughout.