# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-11**

## Stable production gate

Immutable references v0 is complete.

PR #105 squash-merged to `main` as:

`5e3c111a903dbe717f374fcbe6fb2e93ab6864c1`

Issue #104 is closed/completed. Natural exact-SHA post-merge CI #457 / run `34609435365` is **SUCCESS** on Ubuntu, Windows and macOS. Ubuntu also passed the permanent build/cache/runtime gates and release build.

Production now includes explicit `&T` / `&expr`, first-class immutable-reference semantic types, deterministic single-source provenance, stored local references, bounded final-use liveness, source-native ownership diagnostics, inferred `SharedBorrow` interoperability and direct safe Rust reference lowering. Historical immutable-reference research evidence remains in `docs/IMMUTABLE_REFERENCE_SURFACE_RESEARCH.md`.

## Active research — #106

Issue #106 is the next Phase 3.3 ownership-ergonomics item:

`P0 research shared ownership ergonomics v0: explicit aliasing without hidden ownership cost`

The question is not whether Evolution can hide `Rc` or `Arc` behind ordinary assignment. It must not. The research asks whether the current language is mature enough for a **caller-visible shared-ownership contract** whose cost and semantics remain explicit and whose generated Rust maps directly to the equivalent idiomatic primitive.

Required distinctions:

- ordinary owned `T`;
- immutable borrowed `&T`;
- one-thread multiple ownership (`Rc`-like semantics);
- cross-thread multiple ownership (`Arc`-like semantics);
- interior mutability/synchronization as separate boundaries rather than implicit behavior;
- cycle/graph cases where `Weak` or arena/index ownership may be preferable.

## Immediate sequence

1. Start #106 from exact verified `main` SHA `5e3c111a903dbe717f374fcbe6fb2e93ab6864c1`.
2. Inventory what the current Records/Enums/functions/reference surface can honestly express before adding any syntax.
3. Build a research-only Rust comparison corpus for owned, borrowed, `Rc`, `Arc`, interior-mutability boundary cases, `Weak` cycle breaking and arena/index alternatives.
4. Record allocation/refcount/synchronization costs, safety boundaries and direct generated-Rust candidates. Never compare a shared-ownership candidate against a cheaper Rust program performing different ownership work.
5. Classify each case as `EXPLICIT-SHARED-CANDIDATE`, `BORROW-INSTEAD`, `REQUIRES-INTERIOR-MUTABILITY-DESIGN`, `REQUIRES-CONCURRENCY-DESIGN`, `REQUIRES-WEAK/CYCLE-MODEL`, `ARENA/INDEX-PREFERRED`, `DEFER-CURRENT-LANGUAGE-TOO-SMALL`, or `REJECT-HIDDEN-COST/AMBIGUOUS`.
6. Keep research code separate from production syntax. Do not overload `SharedBorrow` or `SharedRef`; both are non-owning concepts.
7. If meaningful executable probes are possible, validate them under normal CI and #4/#5 methodology. If the current language is too small, document the missing prerequisite instead of manufacturing a performance claim.
8. Produce a durable shared-ownership research report and one final verdict: `IMPLEMENT-CANDIDATE`, `SPLIT-RESEARCH`, `DEFER`, or `REJECT`.
9. Open a separate implementation issue only if the evidence reaches `IMPLEMENT-CANDIDATE`.

## Hard boundaries

No implicit shared ownership inference, hidden allocation, hidden handle clone, deep clone, automatic `Arc` selection, implicit locking, GC/runtime ownership table, unsafe lifetime widening, mutable-reference implementation, generalized lifetime solver, interior-mutability implementation, concurrency runtime design or silent changes to existing `T` / `&T` contracts belong in #106.

## Production contracts

Rust remains pinned to **1.98.0**, edition 2024, opt-level 3 and codegen-units 1. Existing build/run caches, source mapping, diagnostic remapping, inferred call-duration shared borrowing, first-class immutable references and runtime parity-or-better contracts remain unchanged.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed SHAs remain evidence and are not rerun merely for a better color.
