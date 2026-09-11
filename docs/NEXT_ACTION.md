# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-11**

## Stable predecessor gate

Immutable-reference surface research #102 / PR #103 is complete and merged to `main` as:

`cc7e7002bd1dd9726e0fd6bcf3d73687fddc0e15`

The accepted verdict remains **SURFACE-CANDIDATE / PUNCTUATION-AMPERSAND** with 19 semantic research cases and zero compile-expectation mismatches. Durable evidence remains in `docs/IMMUTABLE_REFERENCE_SURFACE_RESEARCH.md`.

## Active production — #104 / PR #105

Issue #104 and draft PR #105 implement the bounded production successor on branch `feature/immutable-reference-surface-v0`.

Implemented production behavior now includes:

- lexer/formatter/parser support for `&T` and `&expr`;
- explicit syntax, semantic and lowered immutable-reference types/expressions;
- nominal-record reference parameters and return contracts;
- first-class local reference bindings;
- deterministic single-source provenance across direct returns, local forwarding, function forwarding, recursion and same-source branches;
- source-native rejection of ambiguous multi-source returned-reference signatures and references escaping a local owner;
- owner move/reinitialization rejection while a possibly-live reference remains;
- bounded final-use liveness so an owner can move or be reinitialized after the final proven reference use;
- conservative control-flow handling: nested block uses keep outer references live through the complete statement, then release them when no later use remains;
- scalar reads through immutable references;
- source-native rejection of moving nominal record fields through a reference, with no implicit clone;
- interoperability with the existing inferred call-duration `SharedBorrow` without producing `&&T`;
- direct safe Rust `&T` / `&expr` codegen with no runtime reference machinery.

Permanent lowering/codegen/parser/formatter/lexer tests cover these contracts. The historical immutable-reference research workflow is retired; production proof belongs to normal CI and permanent tests.

## Explicit non-goals

The active v0 feature does not add mutable references, nested references, primitive reference types, reference fields in records/enums, generalized or user-written lifetimes, multi-owner lifetime solving, self-referential structures, hidden clone/copy, allocation, RC/GC, runtime borrow tables, unsafe lifetime widening, or invented `'static` references.

## Immediate sequence

1. Synchronize `LANGUAGE_SPEC_V0`, this file, `PROJECT_STATE`, and PR #105 with the implemented behavior.
2. Remove all development bootstrap workflows from the feature branch.
3. Track only the natural normal CI for the exact documentation-synchronized user-authored head; never duplicate a queued/in-progress run for the same SHA/workflow/input.
4. Require Ubuntu, Windows and macOS CI, workspace tests, Clippy, formatter checks, release build and existing performance gates to succeed on that exact head.
5. Live-check PR #105 head/base/mergeability and changed files after the final exact-head gate.
6. Squash-merge PR #105 only from the validated exact head.
7. Track natural post-merge `main` CI on the merge SHA; close #104 completed only after the required post-merge gate succeeds.
8. Re-read the live roadmap/issues before atomizing any generalized lifetime or mutable-reference successor.

## Production contracts

Rust remains pinned to **1.98.0**, edition 2024, opt-level 3 and codegen-units 1. Existing build/run caches, source mapping, diagnostic remapping, inferred call-duration shared borrowing and runtime parity-or-better contracts remain unchanged.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Failed SHAs remain evidence and are not rerun merely for a better color.
