# Rust Evolution — Project State

Last verified update: **2026-09-14**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Current exact verified stable main: `0f90cca266c346941ed6453e5a2be065f4f37a07`
- Rust toolchain: **1.98.0**
- Production flags: edition 2024, opt-level 3, codegen-units 1
- Measured GNU/Linux linker path: `rustc -> cc -> lld`
- Natural exact-SHA CI #467 / run `34617261754`: **SUCCESS** on Ubuntu, Windows and macOS

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

PR #108 squash-merged as:

`d13c94227f89c7ed323f26d7fd63bc5835caaccd`

Natural exact-SHA validation:

- CI #465 / run `34616193977`: **SUCCESS** on Ubuntu, Windows and macOS;
- shared ownership ergonomics research #6 / run `34616193950`: **SUCCESS**;
- main artifact id `10270430868`;
- digest `sha256:292e8a022af57068d53b602f8ffbf349ebd046b801d226ee5ef66dfdc64fcd29`;
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

The research proved that borrowing, one-thread reference-counted ownership, cross-thread ownership, interior mutability, synchronization, weak/cyclic edges and arena/index ownership must remain distinct models.

`ParameterPassingMode::SharedBorrow`, semantic/lowered `SharedRef`, and `ReferenceTracker` remain **non-owning** and must not be overloaded into shared ownership.

### #110 — docs-only handoff after #108

PR #110 synchronized the living handoff and squash-merged to current verified stable main:

`0f90cca266c346941ed6453e5a2be065f4f37a07`

Natural CI #467 / run `34617261754`: **SUCCESS** on Ubuntu, Windows and macOS.

### #109 / PR #111 — explicit shared handle surface v0 research

Issue #109 is the bounded surface successor selected by #106.

Research PR #111 compares four source families and executes a 15-case Rust 1.98 `Rc<T>` semantic corpus.

Accepted clean evidence head:

`0ce2e006b16e5452f460f0c4fbab5c5cb057fc3b`

Exact-head validation:

- normal CI #471 / run `34618281936`: **SUCCESS** on Ubuntu, Windows and macOS;
- dedicated Explicit shared handle surface research #4 / run `34618282024`: **SUCCESS**;
- artifact `evo-explicit-shared-handle-surface-research-ubuntu-24.04`;
- artifact id `10271057861`;
- digest `sha256:39b110a20392910ac886e0f83e98d22c5c99daa685aa6dc303649d7be70e0fa8`;
- report `git_sha` exactly matches the evidence head;
- 4 surface candidates;
- 15 semantic cases;
- 0 compile-expectation mismatches;
- 0 runtime-expectation mismatches.

Final research verdict: **IMPLEMENT-CANDIDATE / CONTEXTUAL-WORDS**.

Durable report: `docs/EXPLICIT_SHARED_HANDLE_SURFACE_RESEARCH.md`.

Accepted source candidate:

```text
shared Item
share owner
dup owner
```

Direct intended Rust mapping:

```text
shared Item -> std::rc::Rc<Item>
share expr  -> Rc::new(expr)
dup expr    -> Rc::clone(&expr)
```

Surface comparison result:

- `Shared<Item>`: **DEFER-GENERIC-INFRASTRUCTURE** because current parser has no general generic type AST and formatter renders `Shared < Item >` under comparison-token rules;
- `Rc<Item>`: **DEFER-RUST-COUPLED-GENERIC** for the same generic/frontend cost plus backend-specific source coupling;
- contextual words: **PREFERRED-CANDIDATE** because existing identifier tokens and spacing preserve `shared Item`, `share owner`, `dup owner` without new lexer punctuation or general generics;
- `@Item` / `@owner` / `@@owner`: **DEFER-NEW-PUNCTUATION** because `@` currently fails lexing and needs new lexer/formatter rules.

The contextual words are not accepted as hard keywords. Production parsing must preserve existing valid identifier/call uses such as `share(...)`, `dup(...)`, and bindings named `shared`, `share`, or `dup` outside the narrow new grammar contexts.

An earlier evidence head `567278829bac974fa10306979d7d08653ef52123` passed the semantic research but failed normal CI at rustfmt. It remains historical evidence and was not rerun merely for color. A temporary one-shot rustfmt bootstrap formatted the research harness on Rust 1.98 and was removed before the accepted clean head.

PR #111 remains open while durable docs are synchronized and a final exact-head CI/research gate is obtained.

## Active production successor — #112

Issue #112 is open:

`P0 implement explicit shared handle v0: contextual shared/share/dup over Rc<T>`

Its scope is already bounded, but the **production branch must not be cut until PR #111 merges and the resulting exact main SHA passes natural three-OS CI plus the dedicated surface research workflow**.

The first implementation slice is one-thread immutable nominal shared ownership only.

Required production contracts:

- `shared Item` is a distinct shared-owner type category, not `SharedRef` or `SharedBorrow`;
- `share expr` is explicit initial allocation;
- `dup expr` is explicit owner-handle duplication;
- assignment and by-value function parameter/return move the handle;
- no hidden refcount increment occurs on ordinary use;
- payload borrow remains ordinary non-owning reference provenance tied to the particular source handle;
- source-handle move/reinitialization conflicts while that reference may be live;
- another duplicated handle may move independently;
- bounded final-use permits source-handle move after the final proven reference use;
- mutation through immutable shared ownership remains unsupported;
- cross-thread ownership remains unsupported;
- codegen maps directly to safe Rust `Rc<T>`, `Rc::new`, `Rc::clone`, moves, borrows and drops;
- no Evolution wrapper/runtime ownership table, hidden deep clone, `Arc`, interior mutability, locks or unsafe emulation.

The accepted 15 research categories plus contextual-identifier compatibility must become permanent production tests. The historical surface-research workflow must be retired or converted before production syntax makes its pre-production assumptions false.

## Current architecture facts relevant to #112

### Frontend

Parser `TypeName` currently contains:

- `Int`
- `Bool`
- `String`
- `Named(String)`
- `SharedRef(Box<TypeName>)`

There is no general generic-type AST/parser today. `<` and `>` are lexer comparison tokens.

`shared`, `share`, and `dup` currently lex as ordinary identifiers. The accepted implementation should exploit that as contextual grammar rather than reserving new hard keywords.

### Semantic / lowered types

`SemanticType` and public lowering `ValueType` model primitive values, owned records and `SharedRef`; there is no shared-owner value category.

The production successor needs a distinct shared-owner type family. A shared-owner handle must not become trivially reusable: `Rc` duplication is observable ownership work.

### Move tracking

`MoveTracker` already has the correct fundamental model for ordinary shared-handle assignment/parameter passing: the handle is move-only unless explicitly duplicated.

`dup` should inspect one available shared handle and produce another move-only handle. It must not special-case ordinary reads into hidden copies.

### Reference provenance

`ReferenceTracker` records non-owning provenance and possible liveness. A payload borrow derived through a shared handle should reuse this model where possible while remaining tied to the particular source handle.

Another owner does not erase the source-handle borrow relation. The accepted Rust matrix rejects moving the source handle during a live payload borrow and allows moving a different duplicated handle.

### Rust codegen

Current codegen renders owned/reference value types directly. The accepted shared-owner type should map directly to `std::rc::Rc<T>` plus explicit `Rc::new` / `Rc::clone` operations. Hidden clone insertion is forbidden.

## Explicit current non-goals

Not approved by #112:

- `Arc` / cross-thread shared ownership;
- `Send`/`Sync`-equivalent capability system;
- interior mutability (`RefCell` etc.);
- locks/synchronization (`Mutex`, `RwLock`);
- `Weak` production semantics or cycle solving;
- arena/index implementation;
- mutable references;
- generalized/user-written lifetime solver;
- general generic type syntax;
- implicit shared ownership inference;
- implicit allocation;
- implicit owner duplication;
- hidden payload deep clone;
- implicit owned/reference/shared-owner conversions;
- GC/runtime ownership tables;
- unsafe emulation;
- silent changes to existing owned `T`, immutable `&T`, or inferred `SharedBorrow` contracts.

## Implemented language / tooling state

`docs/LANGUAGE_SPEC_V0.md` remains the implemented-language source of truth. The production core currently includes integer/bool/static strings, input/repeat/control flow, functions, lexical block locals, Records v0, Enums v0, by-value ownership/reinitialization, inferred call-duration shared-borrow parameters, first-class bounded immutable references, source-native move/reference diagnostics, source maps, formatter, native check/emit/build/run and verified run/build caches.

Shared-owner syntax remains research-only until #112 production work lands.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed SHAs remain evidence and are not rerun merely for a better color.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
