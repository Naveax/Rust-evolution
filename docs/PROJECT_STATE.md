# Rust Evolution — Project State

Last verified update: **2026-09-11**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Current exact verified stable main: `d13c94227f89c7ed323f26d7fd63bc5835caaccd`
- Rust toolchain: **1.98.0**
- Production flags: edition 2024, opt-level 3, codegen-units 1
- Measured GNU/Linux linker path: `rustc -> cc -> lld`
- Natural exact-SHA CI #465 / run `34616193977`: **SUCCESS** on Ubuntu, Windows and macOS
- Natural exact-SHA shared ownership research #6 / run `34616193950`: **SUCCESS**
- Main shared-ownership artifact id `10270430868`, digest `sha256:292e8a022af57068d53b602f8ffbf349ebd046b801d226ee5ef66dfdc64fcd29`

## Build / compile sequence

- #76: build latency baseline established; single-file native builds are rustc-dominated.
- #79 / PR #81: `build-cache-v0` accepted for verified unchanged-build artifact reuse.
- #82 / PR #84: changed-source rustc incremental research **REJECT / DEFER**.
- #85 / PR #86: link-time attribution established current `cc -> lld` path and link cost.
- #87 / PR #88: GNU ld / mold candidate experiment **REJECT / DEFER**.
- #89 / PR #90: opt-level 3 -> 2 release optimization candidate **REJECT / DEFER**.
- #91 / PR #92: compile-memory baseline **DEFER / NO ACTION**.
- #93 / PR #94: binary-size baseline **DEFER / NO ACTION**; controlled reference/Evolution binaries were byte-identical across the seven-case corpus.

Dependency-build, proc-macro cost and workspace scaling remain deferred until Evolution has a real package/dependency graph.

## Ownership-ergonomics chain

### #95 / PR #96 — borrow inference feasibility

Verdict: **IMPLEMENT-CANDIDATE**. A bounded local classifier can infer call-duration shared borrows for read-only nominal parameters without generalized lifetime solving.

### #97 / PR #99 — inferred shared-borrow nominal parameters v0

Implemented `Owned` / `SharedBorrow` parameter modes. `SharedBorrow` is call-duration and non-owning; it does not create a stored or escaping reference.

### #100 / PR #101 — lifetime-elision feasibility

Verdict: **REFERENCE-SURFACE-FIRST**. Useful deterministic single-source borrowed-return signatures can rely on Rust lifetime elision, while ambiguous/multi-owner/dead-owner relations fail closed. Durable report: `docs/LIFETIME_ELISION_RESEARCH.md`.

### #102 / PR #103 — immutable reference surface research

Verdict: **SURFACE-CANDIDATE / PUNCTUATION-AMPERSAND**. Selected caller-visible syntax: `&T` / `&expr`. Durable report: `docs/IMMUTABLE_REFERENCE_SURFACE_RESEARCH.md`.

### #104 / PR #105 — immutable references v0

Issue #104 is closed/completed. PR #105 squash-merged as `5e3c111a903dbe717f374fcbe6fb2e93ab6864c1`; exact post-merge CI #457 succeeded.

Production behavior includes:

- lexer/parser/formatter support for `&T` and `&expr`;
- explicit immutable-reference semantic/lowered value types;
- first-class local reference bindings;
- deterministic single-source provenance through forwarding/calls/recursion/same-source branches;
- fail-closed multi-owner/dead-local return rejection;
- owner move/reinitialization conflicts while dependent references remain live;
- bounded final-use liveness allowing an owner move/reinitialization after the final proven reference use;
- scalar reads but no move-only nominal field extraction through a shared reference;
- interoperability with inferred call-duration `SharedBorrow` without accidental `&&T`;
- direct ordinary safe Rust reference lowering;
- no hidden allocation, wrapper reference object, clone, RC/GC, runtime ownership map, invented `'static`, or unsafe lifetime widening.

### #107 — post-implementation handoff

Docs PR #107 merged as `3be6b212a549831c15ef5a30df2b35da6362683e`; exact post-merge CI #459 succeeded.

### #106 / PR #108 — shared ownership ergonomics v0 research

Issue #106 is **closed/completed**.

PR #108 squash-merged to current verified `main`:

`d13c94227f89c7ed323f26d7fd63bc5835caaccd`

Natural exact-SHA validation:

- CI #465 / run `34616193977`: **SUCCESS** on Ubuntu, Windows and macOS;
- shared ownership ergonomics research #6 / run `34616193950`: **SUCCESS**;
- main artifact id `10270430868`;
- digest `sha256:292e8a022af57068d53b602f8ffbf349ebd046b801d226ee5ef66dfdc64fcd29`;
- report `git_sha` exactly matches current `main`;
- 14 cases;
- 0 compile-expectation mismatches;
- 0 runtime-expectation mismatches.

Final verdict: **SPLIT-RESEARCH**.

Durable report: `docs/SHARED_OWNERSHIP_RESEARCH.md`.

Classification result:

- owned control: 1;
- `BORROW-INSTEAD`: 1;
- `EXPLICIT-SHARED-CANDIDATE`: 5;
- `REQUIRES-INTERIOR-MUTABILITY-DESIGN`: 1;
- `REQUIRES-CONCURRENCY-DESIGN`: 3;
- `REQUIRES-WEAK/CYCLE-MODEL`: 1;
- `ARENA/INDEX-PREFERRED`: 1;
- `REJECT-HIDDEN-COST/AMBIGUOUS`: 1.

The research proves that borrowing, one-thread reference-counted ownership, cross-thread ownership, interior mutability, synchronization, weak/cyclic edges and arena/index ownership must remain distinct models.

`ParameterPassingMode::SharedBorrow`, semantic/lowered `SharedRef`, and `ReferenceTracker` remain **non-owning** and must not be overloaded into shared ownership.

The useful bounded successor is explicit one-thread `Rc`-like ownership with caller-visible allocation and handle duplication.

## Active successor — #109 explicit shared handle surface v0 research

Issue #109 is open and its start gate is satisfied:

`P0 research explicit shared handle surface v0: one-thread Rc-like nominal ownership`

No production shared-owner syntax or semantic type is accepted yet.

Research must compare at least three explicit surface families and validate both surface cost and semantic fit before any implementation issue is opened.

Required semantic contract:

- create the initial shared allocation explicitly;
- duplicate an owner handle explicitly;
- ordinary assignment moves the handle;
- ordinary by-value function parameter/return moves the handle;
- explicit duplicate then forwarding/return is supported in the model;
- payload inspection does not pretend the handle and payload are the same value category;
- handle duplication and payload deep cloning are visibly distinct;
- payload borrows remain ordinary non-owning references;
- moving the specific handle that produced a live payload borrow remains invalid even if another shared owner exists;
- attempted mutation through immutable shared ownership is outside this slice;
- cross-thread transfer is outside this type contract;
- any accepted codegen candidate must lower directly to `Rc<T>`, `Rc::new`, `Rc::clone`, ordinary moves/borrows/drop with no extra Evolution runtime layer.

## Current architecture facts relevant to #109

### Frontend

Parser `TypeName` currently contains:

- `Int`
- `Bool`
- `String`
- `Named(String)`
- `SharedRef(Box<TypeName>)`

There is no general generic-type AST/parser today. `<` and `>` are already lexer comparison tokens. Therefore `Shared<Item>` / `Rc<Item>`-style spellings are possible research candidates but carry real parser/formatter/compatibility cost.

### Semantic / lowered types

`SemanticType` and public lowering `ValueType` model primitive values, owned records and `SharedRef`; there is no shared-owner value category.

A future shared-owner candidate should use a distinct ownership type family rather than a `SharedRef` variant or parameter passing mode.

`SemanticType::is_trivially_reusable_v0` currently treats all non-record semantic values as reusable. A future shared-owner handle must **not** become implicitly reusable through that shortcut: `Rc` handle duplication increments a strong count and is explicit ownership work.

### Move tracking

`MoveTracker` already has the correct fundamental model for ordinary handle assignment/parameter passing: non-trivially-reusable values are moved and become unavailable until reinitialized.

Explicit owner duplication should therefore produce a new handle value; it should not special-case ordinary reads into hidden copies.

### Reference provenance

`ReferenceTracker` currently records `Parameter` or `LocalOwner` provenance for non-owning references and blocks owner move/reinitialization while a dependent reference remains live.

A payload borrow derived through a future shared-owner handle should reuse this non-owning liveness model where possible and stay tied to the particular source handle, matching the accepted #106 Rust evidence.

### Rust codegen

Current codegen renders owned/reference value types directly and adds `&` only for explicit/shared-borrow semantics already represented by IR.

A future accepted shared-owner type can map directly to `std::rc::Rc<T>` plus explicit `Rc::new` / `Rc::clone` operations. Hidden clone insertion is forbidden.

## Explicit current non-goals

Not approved by #109:

- `Arc` / cross-thread shared ownership;
- interior mutability (`RefCell` etc.);
- locks/synchronization (`Mutex`, `RwLock`);
- `Weak` production semantics or cycle solving;
- arena/index implementation;
- mutable references;
- generalized/user-written lifetime solver;
- implicit shared ownership inference;
- implicit allocation;
- implicit handle duplication;
- hidden payload deep clone;
- GC/runtime ownership tables;
- unsafe emulation;
- silent changes to existing owned `T`, immutable `&T`, or inferred `SharedBorrow` contracts.

## Implemented language / tooling state

`docs/LANGUAGE_SPEC_V0.md` remains the implemented-language source of truth. The production core currently includes integer/bool/static strings, input/repeat/control flow, functions, lexical block locals, Records v0, Enums v0, by-value ownership/reinitialization, inferred call-duration shared-borrow parameters, first-class bounded immutable references, source-native move/reference diagnostics, source maps, formatter, native check/emit/build/run and verified run/build caches.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed SHAs remain evidence and are not rerun merely for a better color.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
