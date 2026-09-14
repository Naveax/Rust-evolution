# Weak edge surface v0 research

Status: **RESEARCH IN PROGRESS**

Parent: #125. Predecessor: #118 / PR #120, accepted as `SPLIT-RESEARCH` on exact verified main `efc7e5fe85a699270bc01e6cc6329fa286733766`.

This track studies an explicit one-thread non-owning edge equivalent to safe `std::rc::Weak<T>`. It does not add production syntax.

## Surface families

The executable probe compares:

- contextual words: `weak Item`, `downgrade owner`, `upgrade edge`;
- generic-looking `Weak<Item>`, which currently lacks a generic type AST and is formatted through comparison-style angle tokens;
- punctuation is deferred unless a compelling non-colliding spelling exists.

The leading candidate is contextual words because these spellings can remain lexer identifiers and be parsed only in bounded new grammar positions, preserving ordinary identifier/call compatibility.

## Semantic matrix

Pinned Rust 1.98 evidence covers downgrade without strong-count increment, weak-count lifecycle, live/dead upgrade, weak cycle breaking, all-strong cycle retention, weak drop behavior, move-only weak handles, explicit weak duplication, weak versus lexical-reference lifetime, payload deep clone distinction, `RefCell` composition, Arc/Weak concurrency separation, borrow-instead control, and dead-target failure without fabricated validity.

## Non-negotiable boundaries

- no hidden downgrade, upgrade, strong-owner duplication, or payload clone;
- no GC or global ownership map;
- `&T`, `shared T`, and a future weak handle remain distinct;
- interior mutation remains explicit composition;
- Arc/Weak remains a separate cross-thread model;
- direct safe Rust lowering is required for any later implementation candidate.

Final exact SHA, artifact digest, and successor decision are recorded only after exact-head normal CI and the dedicated research workflow succeed with zero unexplained mismatches.
