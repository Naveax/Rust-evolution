# Interior mutability ergonomics v0 research

Status: **RESEARCH IN PROGRESS**

Parent: #121

This research isolates single-thread dynamic borrow checking from shared ownership. It does not approve any Evolution syntax or production semantics.

## Problem boundary

`Rc<T>` answers who owns a value. `RefCell<T>` answers whether shared/exclusive access may proceed at runtime. These are different mechanisms, costs, and failure modes and must not be collapsed into one hidden feature.

The research therefore compares:

- ordinary exclusive owned mutation;
- owned `RefCell<T>` with explicit runtime borrow guards;
- explicit `Rc<RefCell<T>>` composition where owner duplication and dynamic borrowing remain separate operations;
- `Arc<Mutex<T>>` as a concurrency/synchronization contrast only;
- hidden allocation, hidden runtime checking, hidden owner duplication, and hidden synchronization as rejected designs.

## Executable matrix

The pinned Rust 1.98 matrix covers at least:

1. ordinary exclusive mutable owner control;
2. owned immutable dynamic borrow;
3. owned mutable dynamic borrow;
4. sequential mutable borrows;
5. immutable then mutable overlap conflict;
6. mutable then immutable overlap conflict;
7. simultaneous immutable borrows;
8. fallible `try_borrow` conflict;
9. fallible `try_borrow_mut` conflict;
10. guard release restoring availability;
11. explicit `Rc<RefCell<T>>` duplicate then mutation visibility;
12. independent surviving shared owner after another handle moves/drops;
13. handle duplication versus payload deep clone;
14. `Arc<Mutex<T>>` concurrency boundary;
15. ordinary owned mutation classified as cheaper when dynamic borrowing buys nothing.

Each case records its ownership model, allocation model, dynamic borrow-state work, runtime success or failure expectation, output/error expectation, and final classification.

## Decision classes

- `OWNED-MUT-INSTEAD`
- `REFCELL-CANDIDATE`
- `EXPLICIT-RC-REFCELL-COMPOSITION`
- `REQUIRES-CONCURRENCY-DESIGN`
- `REJECT-HIDDEN-COST`

Aggregate result remains `SPLIT-RESEARCH` unless the exact-head evidence proves a narrower production candidate with explicit dynamic checking and direct safe Rust lowering.

## Safety and cost invariants

Any production-plausible successor must preserve the following:

- dynamic conflicts stay dynamic and observable rather than being presented as compile-time guarantees;
- panicking and fallible borrow operations remain semantically distinct;
- guard lifetime controls when runtime borrow state is released;
- no implicit `RefCell`, `Rc`, `Arc`, lock, allocation, owner duplication, payload clone, synchronization, GC, runtime ownership registry, or unsafe aliasing emulation;
- generated Rust remains direct and safe;
- runtime comparison is against equivalent idiomatic Rust performing the same dynamic borrow work.

## Evidence gate

The dedicated workflow uses Rust 1.98.0 and requires:

- exact source SHA provenance;
- at least 15 cases;
- zero unexplained expectation mismatches;
- coverage of ordinary mutation, dynamic borrow success/conflict behavior, explicit `Rc<RefCell<T>>` composition, concurrency separation, and hidden-cost rejection;
- CSV/JSON/Markdown artifacts retained as evidence.

Final observations, exact SHA, artifact digest, and successor verdict are added only after the exact-head workflow and normal three-OS CI succeed.
