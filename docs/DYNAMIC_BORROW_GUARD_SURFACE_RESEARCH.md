# Dynamic borrow guard surface v0 research

Acceptance rule: the research decision is authoritative only when the executable matrix, exact-head CI, artifact provenance, merge, and natural exact-main validation recorded on issue #134 are all green. Until then, this document describes a bounded candidate, not production semantics.

Issue: #134. Parent research: #121 / PR #128.

Exact verified research start main: `0748f7672028429c962880a09471f37ada896dc5`, where generational arena v0 is merged and its natural exact-main CI, arena performance, arena surface research, append-only sequence performance, and explicit shared-owner performance gates are green.

This track is research-only. It adds no production cell, mutation, guard, or interior-mutability semantics.

## Research question

Can the first explicit single-thread dynamic-borrow slice keep runtime guards lexical and local, so Evolution can expose RefCell-like checking without immediately adding a first-class escaping guard type or generalized lifetime solver?

The leading bounded hypothesis is **LEXICAL-GUARDS-FIRST**.

## Candidate owned-cell and acquisition contract

Candidate spellings under research are contextual words, not production syntax:

```text
state = cell Item(value = 1)

fn inspect(state cell Item) int
    ...
end

borrow state as view
    ...
end

borrow_mut state as edit
    ...
end

try_borrow state as view
    ...
else
    ...
end

try_borrow_mut state as edit
    ...
else
    ...
end
```

For the bounded first slice, `cell T` is an explicit owned runtime-borrow-checked container and `cell expr` explicitly moves an owned payload into that container. Its direct Rust model is inline `RefCell<T>`; creating the cell does not itself imply heap allocation, `Rc`, `Arc`, synchronization, or owner duplication.

This bounded spelling avoids opening general Rust-like generic source syntax merely to expose `RefCell<T>`. Ordinary `T` does not implicitly become a cell because that would hide dynamic borrow state. Because `cell` remains contextual, an existing ordinary call such as `cell(7)` must continue to parse as a function call; the constructor form applies only to the bounded non-call operand shape `cell expr`. A nested `shared cell T` source algebra is not authorized here; the Rust `Rc<RefCell<T>>` cases remain composition evidence showing that ownership and dynamic borrowing are separate mechanisms.

The intended bounded contract is:

- `borrow` acquires a panicking shared dynamic guard;
- `borrow_mut` acquires a panicking exclusive dynamic guard;
- `try_borrow` and `try_borrow_mut` expose conflict through an explicit success/failure branch;
- the success binding is lexical to the body/success branch;
- leaving the block releases the dynamic borrow state;
- multiple shared guards may coexist;
- an exclusive guard conflicts with every overlapping shared or exclusive dynamic guard;
- mutation through a shared guard is rejected;
- ordinary owned mutation remains preferred when aliasing does not require runtime borrow checking;
- returned/escaping guards are not authorized by this candidate;
- no explicit `release` source statement is selected; an inner lexical scope is the bounded early-release mechanism, while the Rust `drop` control remains research evidence rather than source syntax.

The exact payload-mutation spelling through an exclusive guard is intentionally not selected here. The current language has no general field/index mutable-place surface, and #134 does not authorize smuggling one into a guard research PR. That unresolved production boundary is isolated in hard-gated successor #148; #147 does not authorize mutating payload fields or replacing payload values through an exclusive guard.

## Why lexical guards first

Rust demonstrates that useful dynamic-borrow work can remain inside a bounded scope while the guard itself carries runtime borrow-state ownership.

A lexical Evolution construct can model the important lifetime fact directly:

```text
acquire -> body -> scope end releases guard
```

That avoids pretending that an escaping guard has an ordinary owned lifetime. A returned Rust `Ref<'a, T>` or `RefMut<'a, T>` has an explicit lifetime relationship to its originating `RefCell<T>`; a production Evolution equivalent would require a separately justified first-class guard value/lifetime contract.

Therefore a returned or otherwise escaping guard is classified as **REQUIRES-GUARD-VALUE-TYPE**, not silently widened into the lexical candidate.

## Panicking versus fallible acquisition

The two failure models remain distinct.

Panicking forms correspond to safe idiomatic Rust `RefCell::borrow` / `borrow_mut`. An overlapping incompatible guard is an observable runtime panic.

Fallible forms correspond to `try_borrow` / `try_borrow_mut`. Conflict enters an explicit failure branch and must not panic, fabricate a guard, suppress the conflict, or invent a sentinel value.

A later production slice must preserve this distinction in semantic IR before codegen.

## Explicit shared-owner composition

`Rc<T>` and `RefCell<T>` solve different problems:

- `Rc` controls one-thread shared ownership;
- `RefCell` controls runtime shared/exclusive access.

The matrix therefore keeps `Rc<RefCell<T>>` as **EXPLICIT-RC-REFCELL-COMPOSITION**. Owner duplication remains explicit `Rc::clone`-class work and dynamic borrow acquisition remains separate `RefCell`-class work.

This research does not authorize an implicit `RefCell` inside existing `shared T`, an implicit `Rc` around a cell, hidden owner duplication, or a generalized nested source type algebra.

## Executable matrix

Pinned Rust 1.98 evidence covers at minimum:

1. ordinary owned mutation when aliasing is unnecessary;
2. owned cell plus shared guard;
3. owned cell plus exclusive guard;
4. sequential exclusive guards after release;
5. multiple simultaneous shared guards;
6. shared then exclusive panicking conflict;
7. exclusive then shared panicking conflict;
8. fallible shared acquisition conflict;
9. fallible exclusive acquisition conflict;
10. explicit guard drop restoring availability;
11. lexical scope end restoring availability;
12. a guard remaining live across multiple statements in one scope;
13. mutation through a shared guard rejected;
14. move-only guard reuse rejected;
15. returned guard requiring an explicit guard value/lifetime type;
16. explicit Rc + RefCell owner duplication and visible mutation;
17. dropping one shared owner while another retains the cell;
18. shared-cell aliasing versus payload deep clone;
19. ordinary immutable `&T` remaining distinct;
20. Arc + Mutex remaining a separate cross-thread synchronization model;
21. exclusive-owner `RefCell::get_mut` control showing that dynamic borrow checking is unnecessary when exclusive access already exists.

The source-surface probe also requires candidate words to remain ordinary identifiers outside exact candidate positions, proves that an ordinary `cell(...)` function call still parses, pins the current parser rejection boundaries for both `cell T` and `cell expr`, and verifies that the lexical/fallible guard block spellings are not accidentally accepted by the current production parser.

## Pre-registered decision

The dedicated workflow derives its verdict from measured matrix/surface predicates. A successful result records guard verdict **LEXICAL-GUARDS-FIRST** together with recommended cell surface **EXPLICIT-OWNED-CELL** (`cell T` + `cell expr`). It emits that pair only if:

- all compile/runtime expectations match;
- lexical/local guard cases cover the bounded dynamic-borrow semantics;
- panicking and fallible conflicts remain observably distinct;
- guard move/release behavior remains explicit;
- returned/escaping guard evidence demonstrates the separate first-class guard-type boundary;
- Rc ownership and RefCell dynamic borrow work remain independently visible;
- ordinary references and cross-thread synchronization remain separate;
- exact-SHA JSON, CSV, and Markdown provenance is produced.

A mismatch keeps #134 research open and requires evidence-driven revision rather than widening the language surface.

## Hard boundaries

This candidate does not authorize:

- implicit `RefCell`;
- implicit `Rc`, `Arc`, mutex, lock, or synchronization;
- hidden allocation, owner duplication, payload clone, or panic suppression;
- unsafe aliasing or lifetime extension;
- GC or a runtime ownership/borrow registry;
- generalized lifetime inference;
- returned or stored-across-owner-lifetime guard values;
- cross-thread dynamic borrowing;
- general mutable references;
- general field/index mutable-place syntax;
- payload replacement or field mutation through `borrow_mut` before successor #148 selects a bounded mutation surface;
- a production interior-mutability feature before exact-head and exact-main research evidence is green.
