# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-11**

## Stable gate

Current exact verified `main`:

`d13c94227f89c7ed323f26d7fd63bc5835caaccd`

This is PR #108 squash merge: `research: classify shared ownership ergonomics v0 (#108)`.

Natural exact-SHA validation:

- CI #465 / run `34616193977`: **SUCCESS** on Ubuntu, Windows and macOS;
- Ubuntu passed fmt, Clippy, workspace tests, every permanent build/cache/runtime gate and release build;
- Shared ownership ergonomics research #6 / run `34616193950`: **SUCCESS**;
- artifact `evo-shared-ownership-research-ubuntu-24.04`, id `10270430868`;
- digest `sha256:292e8a022af57068d53b602f8ffbf349ebd046b801d226ee5ef66dfdc64fcd29`;
- artifact report `git_sha` exactly matches `d13c94227f89c7ed323f26d7fd63bc5835caaccd`.

Issue #106 is closed/completed.

## Accepted #106 result — SPLIT-RESEARCH

Durable report: `docs/SHARED_OWNERSHIP_RESEARCH.md`.

The Rust 1.98 matrix retained 14 cases with zero compile-expectation mismatches and zero runtime-expectation mismatches. It deliberately split materially different models rather than approving one magic `shared` feature:

- ordinary owned control: 1;
- `BORROW-INSTEAD`: 1;
- one-thread `EXPLICIT-SHARED-CANDIDATE`: 5;
- `REQUIRES-INTERIOR-MUTABILITY-DESIGN`: 1;
- `REQUIRES-CONCURRENCY-DESIGN`: 3;
- `REQUIRES-WEAK/CYCLE-MODEL`: 1;
- `ARENA/INDEX-PREFERRED`: 1;
- `REJECT-HIDDEN-COST/AMBIGUOUS`: 1.

`SharedBorrow`, `SharedRef` and `ReferenceTracker` remain non-owning concepts. They must not be reinterpreted as shared ownership.

## Active work — #109

Issue #109 is now the active Phase 3.3 ownership-ergonomics research item:

`P0 research explicit shared handle surface v0: one-thread Rc-like nominal ownership`

The goal is to decide whether Evolution should expose a caller-visible, one-thread, reference-counted shared-owner handle equivalent to idiomatic `Rc<T>`.

This remains **research first**. No production shared-owner syntax is approved.

## Immediate sequence

1. Start from this exact verified main or from a later separately exact-SHA-validated docs-only handoff main.
2. Keep the work research-only until one surface is actually justified.
3. Compare at least three explicit source-surface families:
   - nominal wrapper spelling such as `Shared<Item>`;
   - Rust-transparent spelling such as `Rc<Item>` / `rc(...)`;
   - keyword/operator-style spelling with explicit allocation and handle duplication.
4. Measure real compatibility/parser/formatter cost. Current `TypeName` has no general generic-type node, and `<` / `>` already lex as comparison tokens.
5. Model explicit creation, inspection, handle duplication, ordinary handle move, function forwarding/return, branch/repeat clone/drop, payload borrowing, source-handle borrow conflicts, deep-clone distinction, immutable mutation rejection and cross-thread rejection.
6. Keep ordinary assignment and by-value parameter/return as **moves** of the handle. Never silently increment a refcount.
7. Treat handle duplication as an explicit semantic operation, not a MoveTracker exception and not payload `Clone`.
8. Keep payload references in the existing non-owning provenance/liveness model where possible.
9. Require any accepted codegen candidate to map directly to `Rc<T>`, `Rc::new`, `Rc::clone`, ordinary moves/borrows/drop with no wrapper/runtime ownership table.
10. Produce one final #109 verdict: `IMPLEMENT-CANDIDATE`, `DEFER`, or `REJECT`; open production implementation work only after `IMPLEMENT-CANDIDATE`.

## Architecture facts to preserve

- Parser `TypeName`: `Int`, `Bool`, `String`, `Named`, `SharedRef`; no general generic type surface.
- Lexer `<` and `>` already mean comparison tokens.
- `SemanticType` / `ValueType` currently model owned records and `SharedRef`, not shared owners.
- `SemanticType::is_trivially_reusable_v0` must not make a future shared-owner handle implicitly copyable; `Rc` clone is observable ownership work.
- `MoveTracker` already provides the correct basis for ordinary by-value handle moves.
- `ReferenceTracker` is provenance for non-owning references. A payload borrow derived from a shared handle must remain tied to the specific source handle unless later research proves a different safe model.
- Rust codegen can represent a future accepted owner type directly, but it must not add hidden `Rc::clone` calls.

## Hard boundaries

Do not fold any of these into #109:

- `Arc` / cross-thread shared ownership;
- `RefCell` or other interior mutability;
- `Mutex` / `RwLock` / synchronization;
- `Weak` production semantics or cycle solving;
- arena/index implementation;
- mutable references;
- generalized lifetime solving;
- implicit allocation;
- implicit handle duplication;
- hidden payload deep clone;
- GC/runtime ownership tables;
- unsafe lifetime or aliasing emulation;
- silent changes to existing `T`, `&T`, or inferred `SharedBorrow` semantics.

## Performance contract

Explicit shared ownership is not “zero operations”. The rule is **no overhead beyond the equivalent idiomatic Rust `Rc<T>` program**. Compare like-for-like allocation, strong-count clone/drop operations and generated Rust shape. Do not compare against a cheaper borrow/owned program doing less ownership work.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed/cancelled SHAs remain evidence and are not rerun merely for a better color.
