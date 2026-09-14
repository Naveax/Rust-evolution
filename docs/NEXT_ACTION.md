# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-14**

## Stable gate

Current exact verified `main`:

`0f90cca266c346941ed6453e5a2be065f4f37a07`

This is docs-only PR #110 merged after the completed #106 / PR #108 shared-ownership research chain.

Natural exact-SHA validation:

- CI #467 / run `34617261754`: **SUCCESS** on Ubuntu, Windows and macOS;
- Ubuntu passed fmt, Clippy, workspace tests, every permanent build/cache/runtime gate and release build.

No production shared-owner syntax exists on this stable main yet.

## #109 accepted result — IMPLEMENT-CANDIDATE / CONTEXTUAL-WORDS

Research PR: #111.

Durable report: `docs/EXPLICIT_SHARED_HANDLE_SURFACE_RESEARCH.md`.

Accepted research evidence head:

`0ce2e006b16e5452f460f0c4fbab5c5cb057fc3b`

Exact-head evidence:

- normal CI #471 / run `34618281936`: **SUCCESS** on Ubuntu, Windows and macOS;
- dedicated Explicit shared handle surface research #4 / run `34618282024`: **SUCCESS**;
- artifact `evo-explicit-shared-handle-surface-research-ubuntu-24.04`, id `10271057861`;
- digest `sha256:39b110a20392910ac886e0f83e98d22c5c99daa685aa6dc303649d7be70e0fa8`;
- report `git_sha` exactly matches the evidence head;
- 4 surface families;
- 15 Rust semantic cases;
- 0 compile-expectation mismatches;
- 0 runtime-expectation mismatches.

Accepted candidate spelling:

```text
shared Item
share owner
dup owner
```

Intended direct Rust mapping:

```text
shared Item -> std::rc::Rc<Item>
share expr  -> Rc::new(expr)
dup expr    -> Rc::clone(&expr)
```

The words remain contextual identifiers rather than hard lexer keywords.

## Active successor — #112

Issue #112:

`P0 implement explicit shared handle v0: contextual shared/share/dup over Rc<T>`

The issue is intentionally open now so the bounded production contract is ready, but its **production start gate is not satisfied until PR #111 merges and the resulting exact main SHA passes natural CI plus the dedicated surface research workflow**.

Do not cut the production branch from the pre-merge research head.

## Immediate sequence

1. Finish PR #111 with durable docs on the research branch.
2. Require final PR #111 exact head to pass normal three-OS CI and the dedicated surface research workflow.
3. Squash-merge PR #111 using expected-head protection.
4. Track only the natural post-merge exact-main CI/research runs; do not manually duplicate them.
5. Close #109 only after those natural main runs succeed and the artifact report matches the merge SHA.
6. Cut the #112 production branch only from that verified main SHA.
7. Implement the bounded frontend first: contextual `shared Item`, `share expr`, `dup expr`, formatter idempotence, and compatibility tests proving normal identifier/call uses such as `share(...)` and `dup(...)` remain unchanged.
8. Add a distinct shared-owner semantic/lowered type family. Do not reuse `SharedBorrow`, `SharedRef`, or `ReferenceTracker` as ownership state.
9. Keep shared-owner values move-only under ordinary assignment/parameter/return; `dup` is the only owner-duplication operation in this slice.
10. Extend payload-borrow provenance minimally so a borrow remains tied to the particular source handle, including bounded final-use behavior.
11. Lower directly to idiomatic safe Rust `Rc<T>`, `Rc::new`, `Rc::clone`, moves, borrows and drops with no wrapper/runtime ownership table or hidden clone.
12. Promote the 15 research semantic categories plus contextual-identifier compatibility into permanent production tests.
13. Retire or convert the historical surface-research workflow before production syntax changes make its pre-production assumptions false.
14. Update `LANGUAGE_SPEC_V0`, `PROJECT_STATE`, and `NEXT_ACTION` only after production behavior is real and validated.
15. After #112 merge, require natural exact-main CI success before closing the implementation issue.

## Architecture facts to preserve

- Current parser `TypeName`: `Int`, `Bool`, `String`, `Named`, `SharedRef`; no general generic type surface.
- Lexer `<` and `>` are comparison tokens. The research rejected paying generic infrastructure merely for this feature.
- `shared`, `share`, and `dup` currently lex as ordinary identifiers; keep that compatibility property.
- `SemanticType` / `ValueType` currently model owned records and `SharedRef`, not shared owners.
- A future shared-owner handle is not trivially reusable. `Rc` owner duplication is observable refcount work and must remain explicit.
- `MoveTracker` is the basis for ordinary handle moves.
- `ReferenceTracker` remains non-owning provenance/liveness. Payload borrows through a shared handle should reuse it only as reference provenance, not as an ownership graph.
- Existing owned `T`, immutable `&T`, and inferred call-duration `SharedBorrow` semantics must not change silently.

## Hard boundaries for #112

Do not fold these into the first shared-owner implementation:

- `Arc` / cross-thread shared ownership;
- `Send`/`Sync`-equivalent capability system;
- `RefCell` / interior mutability;
- `Mutex` / `RwLock` / synchronization;
- `Weak` production semantics or cycle solving;
- arena/index ownership;
- mutable references;
- generalized/user-written lifetime solving;
- general generic type syntax;
- implicit allocation;
- implicit owner duplication;
- hidden payload deep clone;
- implicit owned/reference/shared-owner conversions;
- GC/runtime ownership tables;
- unsafe emulation.

## Performance contract

The comparison baseline is idiomatic Rust `Rc<T>` performing the same ownership work. `share` may allocate once; `dup` may perform one non-atomic strong-count increment. Evolution must add no extra allocation, clone, atomic upgrade, lock, runtime borrow checker, ownership map or wrapper indirection.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed/cancelled SHAs remain evidence and are not rerun merely for a better color.
