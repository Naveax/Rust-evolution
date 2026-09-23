# Exclusive guard mutation surface v0 research

Acceptance rule: this document describes a research candidate only. It becomes an accepted decision only after the executable matrix, exact-head CI, dedicated pinned-Rust workflow, artifact provenance, merge, and natural exact-main validation are all green and recorded on issue #148.

Issue: #148. Parent accepted research: #134 / PR #147.

Exact research start main: `b3b3eebe5c1d41b47e26e3bb2bc00db5a34c32c9`.

Accepted parent contract:

- explicit owned `cell T` / `cell expr`;
- lexical `borrow`, `borrow_mut`, `try_borrow`, and `try_borrow_mut` guards;
- returned/escaping guards remain outside the bounded surface;
- #134 deliberately did not select payload mutation syntax through an exclusive guard.

This track is research-only. It does not add production parser, lowering, formatter, codegen, or runtime semantics.

## Research question

What is the narrowest mutation operation that makes an exclusive lexical dynamic-borrow guard useful without silently introducing a general mutable-reference or mutable-place language?

The leading bounded hypothesis is **WHOLE-PAYLOAD-REPLACE-CANDIDATE**.

## Candidate surface

Research candidate:

~~~text
borrow_mut state as edit
    replace edit, Item(value = 2)
end
~~~

The same replacement operation could appear inside the success branch of `try_borrow_mut`.

The intended bounded contract is:

- the target must be the current lexical exclusive guard binding;
- the replacement expression is evaluated and consumed as the new payload;
- the old payload is dropped exactly once according to ordinary Rust assignment semantics;
- a shared `borrow` guard cannot use the operation;
- the exclusive guard remains live after replacement until lexical scope exit;
- later guards observe the replacement;
- no payload clone, owner duplication, allocation, lock, registry, or dynamic dispatch is implied by replacement itself.

Direct Rust mapping:

~~~rust
let mut edit = state.borrow_mut();
*edit = replacement;
~~~

## Why whole-payload replacement first

Whole-payload replacement requires mutation authority but does not require a general assignment-place grammar. It is intentionally not claimed to subsume partial updates: preserving an unrelated move-only field while changing one field is measured as mutable-place evidence, because reconstructing the whole payload without clone/move-place machinery is not equivalent.

By contrast, allowing:

~~~text
edit.value = 2
edit.inner.value = 2
edit.items[0] = 2
~~~

requires the language to define mutable field/index places, nested place traversal, and the interaction of those places with ordinary owned values, references, guards, collections, and future mutable references. Rust can perform these operations through `RefMut<T>`, but that does not make the source-language grammar free.

The matrix therefore classifies direct field, nested-field, and index mutation as **MUTABLE-PLACE-BOUNDARY** evidence. Their Rust feasibility is measured, but this research does not authorize them.

## Explicit update comparison

A separate source statement such as:

~~~text
update edit, Item(value = 2)
~~~

can lower to the same Rust assignment, but it adds a second mutation operation without a distinct ownership or cost model.

The executable control uses a Rust helper that merely performs `*slot = value`. If whole-payload replacement already supplies the bounded semantics directly, the additional update wrapper is not selected for the first slice.

This is a surface comparison, not a claim that future update operations are invalid.

## Move and drop behavior

Replacement must preserve ordinary ownership behavior.

The matrix checks that:

- replacing a move-only payload drops the old payload once;
- the new payload is consumed into the cell;
- replacement does not invoke `Clone`;
- replacing an `Rc` payload does not implicitly increment its strong count;
- moving the exclusive guard binding invalidates the old binding;
- mutation through the moved-to guard remains possible;
- an escaping `RefMut<'a, T>` still demonstrates the separate first-class guard/lifetime boundary.

## Dynamic-borrow behavior remains unchanged

Mutation does not weaken the accepted #134 runtime-borrow contract.

The matrix retains controls for:

- overlapping shared then exclusive acquisition panicking;
- fallible exclusive acquisition entering explicit failure instead of panicking;
- later shared acquisition observing a completed replacement;
- explicit `Rc<RefCell<T>>` composition keeping owner duplication visible;
- `Arc<Mutex<T>>` remaining a separate cross-thread synchronization model;
- `RefCell::get_mut` showing that runtime dynamic borrowing is unnecessary when the cell itself is already exclusively owned.

## Source compatibility probes

Candidate words remain contextual identifiers.

The research probes require:

- `replace`, `with`, `update`, and `set` to remain ordinary identifier tokens outside exact candidate positions;
- ordinary calls using those names to continue parsing;
- `replace edit with expr` to remain rejected by the current production parser;
- direct field assignment to remain rejected by the current production parser;
- the explicit-update candidate to remain rejected by the current production parser.

The research must not accidentally make any candidate syntax production-valid.

## Executable Rust 1.98 matrix

Pinned Rust evidence covers 22 cases:

1. scalar whole-payload replacement;
2. record whole-payload replacement;
3. sequential replacements;
4. later shared guard observing replacement;
5. replaced move-only payload drop count;
6. no implicit clone during replacement;
7. no implicit `Rc` owner duplication;
8. shared guard replacement compile rejection;
9. overlapping shared/exclusive conflict;
10. fallible exclusive conflict;
11. direct field mutation as mutable-place evidence;
12. nested-field mutation as mutable-place evidence;
13. index mutation as mutable-place evidence;
14. field mutation preserving an unrelated move-only field without cloning it;
15. explicit update helper as a wrapper comparison;
16. mutation through a moved-to guard;
17. moved guard reuse rejection;
18. returned exclusive guard lifetime boundary;
19. explicit `Rc<RefCell<T>>` composition;
20. dropping one shared owner while another retains the cell;
21. `Arc<Mutex<T>>` concurrency boundary;
22. exclusive-owner `RefCell::get_mut` control.

## Pre-registered decision

The dedicated workflow derives **WHOLE-PAYLOAD-REPLACE-CANDIDATE** only if:

- all compile/runtime expectations match;
- whole-payload replacement cases cover scalar, nominal, move/drop, conflict, and later-read behavior;
- field/nested/index mutation remains explicitly classified as a broader mutable-place boundary;
- the explicit-update comparison adds no distinct runtime mechanism;
- guard movement and escape boundaries remain visible;
- `Rc` composition and cross-thread synchronization remain separate;
- no hidden clone/refcount traffic is observed by the controls;
- source compatibility probes pass;
- exact-SHA JSON, CSV, and Markdown provenance is produced.

The recommended research surface is:

~~~text
replace guard, expr
~~~

A mismatch yields **DEFER** rather than widening the surface.

## Hard boundaries

This research does not authorize:

- production `cell` or guard syntax by itself;
- direct field mutation through guards;
- nested mutable-place traversal;
- index mutation;
- general mutable references;
- generalized assignment-place infrastructure;
- escaping or returned guard values;
- implicit `RefCell`, `Rc`, `Arc`, mutex, lock, clone, or allocation;
- hidden panic suppression;
- unsafe aliasing or lifetime extension;
- GC or a runtime ownership/borrow registry;
- cross-thread dynamic borrowing.

A production mutation slice may begin only after this research and its parent #134 have complete exact-main acceptance evidence.
