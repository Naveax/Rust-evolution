# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-14**

## Stable gate

Current exact verified `main`:

`42fc00323a1359824d0a0adf6df6c28410556b22`

Natural validation on that exact SHA:

- CI #475 / run `34816316570`: **SUCCESS** on Ubuntu, Windows and macOS;
- Explicit shared handle surface research #8 / run `34816316546`: **SUCCESS**;
- artifact id `10337270060`;
- digest `sha256:2894d8b1c76a9ad064d18dd632d35e4e3b56a2bc159c14d2235767348c76e335`;
- accepted research verdict: **IMPLEMENT-CANDIDATE / CONTEXTUAL-WORDS**.

## Active final implementation — #112 / PR #117

Final integration branch: `integration/explicit-shared-handle-v0-final`.

PR #117 consolidates the complete bounded one-thread immutable shared-owner slice. The previously separate conversion-boundary, diagnostics, move/reinitialization, enum-boundary, performance and documentation lanes are included in this single final candidate.

Implemented surface and contracts:

- contextual `shared Item`, `share expr`, and `dup expr`;
- explicit safe `std::rc::Rc<T>` lowering with `Rc::new` and `Rc::clone` only where source requests allocation/duplication;
- move-only shared-owner handles with same-type explicit reinitialization;
- no implicit conversion among owned `T`, `shared T`, and `&T`;
- payload references remain ordinary non-owning references tied to the specific source handle;
- source-handle move/reinitialization conflicts while a dependent reference may still be live;
- bounded final-use release, branch/repeat ownership checks and moved-handle diagnostics;
- record-only first slice with enum-bearing shared-owner/reference use failing closed before executable enum IR/codegen;
- no `Arc`, `RefCell`, synchronization, `Weak`, GC, wrapper ownership runtime, hidden deep clone, unsafe emulation or generalized lifetime machinery.

## Accepted component performance evidence

Equivalent-`Rc` component head `c83d42dbe0e346021f1f524cf9d65f67fdbc66d3` passed Explicit shared owner performance #7 / run `34824865453`:

- correctness: PASS;
- normalized LLVM IR equal: true;
- exact executable bytes equal: true;
- binary size: 2,267,304 bytes on both sides;
- stable observed median ratio: `0.992969173`;
- final verdict: PASS;
- verdict basis: `byte-identical-binary-parity`;
- artifact id `10339932088`;
- digest `sha256:a490d5a0bc5d2cfe15c4da01b89cb45eb9e4d8aa51d8309d865a926dc60d6721`.

This component evidence does **not** replace final combined-head validation.

## Remaining completion sequence

1. Require normal CI on the final exact PR #117 head across Ubuntu, Windows and macOS.
2. Require the permanent Explicit shared owner performance workflow on that same exact head.
3. Inspect the complete final diff against verified main and resolve any review threads.
4. Squash-merge PR #117 only with expected-head protection.
5. Track the natural post-merge exact-main normal CI and Explicit shared owner performance runs without starting duplicates.
6. Close #112 only after both natural exact-main gates succeed.
7. Synchronize the living meta/handoff to the new exact verified main and then advance the next queued P0 item.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. If one gate is queued/running, advance independent work and return to that gate later. Failed SHAs remain evidence and are not rerun merely for a better color.
