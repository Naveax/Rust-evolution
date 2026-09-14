# Rust Evolution — Project State

Last verified update: **2026-09-14**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Current exact verified stable main: `42fc00323a1359824d0a0adf6df6c28410556b22`
- Rust toolchain: **1.98.0**
- Production flags: edition 2024, opt-level 3, codegen-units 1
- Measured GNU/Linux linker path: `rustc -> cc -> lld`
- Natural exact-SHA CI #475 / run `34816316570`: **SUCCESS** on Ubuntu, Windows and macOS

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

Verdict: **REFERENCE-SURFACE-FIRST**. Useful deterministic single-source borrowed-return signatures can rely on Rust lifetime elision while ambiguous/multi-owner/dead-owner relations fail closed. Durable report: `docs/LIFETIME_ELISION_RESEARCH.md`.

### #102 / PR #103 — immutable reference surface research

Verdict: **SURFACE-CANDIDATE / PUNCTUATION-AMPERSAND**. Selected syntax: `&T` / `&expr`. Durable report: `docs/IMMUTABLE_REFERENCE_SURFACE_RESEARCH.md`.

### #104 / PR #105 — immutable references v0

Completed. Production includes first-class bounded immutable references, deterministic single-source provenance, stored reference locals, owner move/reinitialization conflicts, bounded final-use liveness, inferred-`SharedBorrow` interoperability and direct safe Rust reference lowering without hidden allocation/clone/runtime ownership machinery.

### #106 / PR #108 — shared ownership ergonomics research

Completed with **SPLIT-RESEARCH**. Borrowing, one-thread reference-counted ownership, cross-thread ownership, interior mutability, synchronization, weak/cyclic edges and arena/index ownership remain distinct models. Durable report: `docs/SHARED_OWNERSHIP_RESEARCH.md`.

### #109 / PR #111 — explicit shared handle surface v0 research

Completed and merged as current exact verified main:

`42fc00323a1359824d0a0adf6df6c28410556b22`

Natural validation:

- CI #475 / run `34816316570`: **SUCCESS** on Ubuntu, Windows and macOS;
- Explicit shared handle surface research #8 / run `34816316546`: **SUCCESS**;
- artifact id `10337270060`;
- digest `sha256:2894d8b1c76a9ad064d18dd632d35e4e3b56a2bc159c14d2235767348c76e335`;
- verdict **IMPLEMENT-CANDIDATE / CONTEXTUAL-WORDS** with 4 surfaces, 15 semantic cases and zero compile/runtime expectation mismatches.

Accepted source surface:

```text
shared Item
share expr
dup expr
```

The three words remain contextual identifiers rather than hard lexer keywords. Durable report: `docs/EXPLICIT_SHARED_HANDLE_SURFACE_RESEARCH.md`.

## Active production implementation — #112 / PR #117

Final integration branch: `integration/explicit-shared-handle-v0-final`.

PR #117 is the consolidated production candidate. Earlier component PRs #114, #115 and #116 were closed without merge after their content was incorporated into #117; their CI/performance history remains preserved as evidence.

### Implemented shared-owner behavior

- `TypeName::SharedOwner` and corresponding semantic/lowered `SharedOwner` value categories are distinct from owned records, first-class `SharedRef`, and inferred call-duration `SharedBorrow`.
- `shared Item` is accepted only in the bounded function parameter/return type surface.
- `share expr` explicitly allocates the first shared owner and requires an owned nominal record operand.
- `dup expr` explicitly duplicates one available shared-owner handle.
- ordinary assignment, by-value parameter passing and return move the handle; they do not increment the strong count.
- a moved shared-owner local may be explicitly reinitialized with the exact same `shared T` type.
- `dup` of a moved handle is rejected rather than reviving it.
- scalar payload fields are readable through a shared owner; moving a move-only nominal payload field out is rejected instead of cloned.
- no implicit conversion exists among owned `T`, `shared T`, and `&T`.
- `r = &owner` creates an ordinary non-owning `&T`; provenance remains tied to that specific source handle.
- moving or reinitializing the source handle while a dependent immutable reference may still be live is rejected source-natively.
- a different duplicated handle may move independently.
- bounded final-use analysis permits source-handle move/reinitialization after the final proven reference use.
- branch and repeat ownership joins preserve the existing conservative move rules.
- source-native diagnostics distinguish moved shared-handle reuse, invalid `share`, invalid `dup`, and live payload-reference/source-handle conflicts; deterministic related reference-origin notes are preserved where available.

### Direct safe Rust lowering

The implemented mapping is direct:

```text
shared Item -> std::rc::Rc<__EvoRecord_Item>
share expr  -> std::rc::Rc::new(expr)
dup expr    -> std::rc::Rc::clone(&expr)
```

Ordinary moves remain ordinary Rust moves. Payload borrows lower to normal safe references through the `Rc` payload. Generated-Rust compile tests cover create/dup/read, payload borrowing, by-value forwarding and same-type reinitialization after a move.

The implementation adds no wrapper ownership object, runtime ownership registry, hidden payload deep clone, `Arc`, `RefCell`, lock, GC, unsafe block, synchronization or invented lifetime widening.

### Frontend / formatter compatibility

`shared`, `share`, and `dup` remain ordinary identifier tokens outside the narrow contextual forms. Existing calls/names such as `share(...)`, `dup(...)`, and identifiers named `shared` remain compatible. Formatter coverage locks canonical `shared Item`, `share expr` and `dup expr` spelling.

### Permanent production coverage

The final candidate contains permanent parser/lowering/codegen tests for:

- contextual surface and identifier compatibility;
- scalar/nested/storage exclusions;
- move-only assignment/parameter/return behavior;
- explicit duplication in branches and repeats;
- moved-handle reuse and same-type reinitialization;
- no implicit conversion among ownership categories;
- payload-reference/source-handle conflicts and final-use release;
- direct safe Rust codegen operation shape and rustc acceptance;
- fail-closed current enum-bearing-program boundary.

The historical automated surface-research workflow is retired. Its durable report and artifacts remain preserved.

### Current enum-bearing-program boundary

The accepted v0 shared-owner slice is record-only. Programs containing enum declarations still route through the existing Enums v0 integrated semantic pipeline. Explicit reference/shared-owner types or expressions in that pipeline fail closed before executable enum IR/codegen with source-native diagnostics. #112 does not silently widen the enum pipeline, invent shared-owner enum payload/storage semantics, or add a runtime fallback.

### Equivalent-Rc performance gate

PR #117 contains a permanent `explicit-shared-owner-v0` differential benchmark plus an exact committed reference/generated-Rust lock. The Evolution and Rust programs perform the same ownership work: one `Rc` allocation, one explicit owner duplication per loop iteration, equivalent by-value forwarding, field reads and drops.

Accepted component evidence on exact head `c83d42dbe0e346021f1f524cf9d65f67fdbc66d3`:

- Explicit shared owner performance #7 / run `34824865453`: **SUCCESS**;
- artifact id `10339932088`;
- digest `sha256:a490d5a0bc5d2cfe15c4da01b89cb45eb9e4d8aa51d8309d865a926dc60d6721`;
- correctness PASS;
- normalized LLVM IR equal: true;
- exact executable bytes equal: true;
- binary size: 2,267,304 bytes on both sides;
- reference median: 828,921 ns;
- Evolution median: 823,093 ns;
- stable observed ratio: `0.992969173`;
- final verdict: PASS;
- verdict basis: `byte-identical-binary-parity`.

This component result does not replace the required final combined-head gate on PR #117.

## #112 completion rule

Before merge, the final exact PR #117 head must pass:

1. normal CI on Ubuntu, Windows and macOS;
2. the permanent Explicit shared owner performance workflow;
3. complete final diff/review-thread inspection against verified main.

Then PR #117 may be squash-merged only with expected-head protection. After merge, the natural exact-main normal CI and Explicit shared owner performance runs must both succeed without duplicate manual runs. Issue #112 closes only after those natural main gates pass, followed by living meta/handoff synchronization to the new exact verified main.

## Explicit current non-goals

Not approved by #112:

- `Arc` / cross-thread shared ownership;
- `Send`/`Sync`-equivalent capability rules;
- interior mutability (`RefCell` etc.);
- locks/synchronization (`Mutex`, `RwLock`);
- `Weak` production semantics or cycle solving;
- arena/index implementation;
- mutable references;
- generalized/user-written lifetime solving;
- general generic type syntax;
- implicit shared ownership inference;
- implicit allocation;
- implicit owner duplication;
- hidden payload deep clone;
- implicit owned/reference/shared-owner conversions;
- shared-owner record fields or enum payloads;
- nested/general shared-owner type algebra;
- GC/runtime ownership tables;
- unsafe emulation;
- silent changes to existing owned `T`, immutable `&T`, or inferred `SharedBorrow` contracts.

## Implemented language / tooling state

`docs/LANGUAGE_SPEC_V0.md` is the implemented-language source of truth. Stable main currently includes integer/bool/static strings, input/repeat/control flow, functions, lexical block locals, Records v0, Enums v0, by-value ownership/reinitialization, inferred call-duration shared-borrow parameters, first-class bounded immutable references, source-native move/reference diagnostics, source maps, formatter, native check/emit/build/run and verified run/build caches.

The explicit shared-owner surface is implemented in PR #117 but is not stable-main behavior until that PR merges and natural post-merge validation succeeds.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed SHAs remain evidence and are not rerun merely for a better color. CI running does not block independent queue work.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
