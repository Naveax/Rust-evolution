# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-11**

## Stable production gate

Current verified `main` before the shared-ownership research merge:

`3be6b212a549831c15ef5a30df2b35da6362683e`

This is docs handoff PR #107. Natural exact-SHA CI #459 / run `34612808532` is **SUCCESS** on Ubuntu, Windows and macOS, including the permanent Ubuntu build/cache/runtime gates and release build.

Immutable references v0 remains the latest production ownership feature. Existing owned `T`, inferred call-duration `SharedBorrow`, first-class immutable `&T` / `&expr`, deterministic single-source provenance, stored local references, bounded final-use liveness and direct safe Rust reference lowering are unchanged.

## #106 research decision — SPLIT-RESEARCH

Issue #106 / draft PR #108 researched shared ownership without adding production syntax.

Validated exact research evidence head:

`651ef4f3e9fc70e493eb59e3b1b2e7dcc7526d6a`

Evidence:

- normal CI #463 / run `34614250716`: **SUCCESS** on Ubuntu, Windows and macOS;
- dedicated Shared ownership ergonomics research #4 / run `34614250773`: **SUCCESS**;
- artifact `evo-shared-ownership-research-ubuntu-24.04`;
- artifact id `10269252562`;
- digest `sha256:f33fd905c3feb550550a8fc99da75575dcce1d8a54c9ef1b1a531a68256149b5`;
- 14 Rust 1.98 cases;
- zero compile-expectation mismatches;
- zero runtime-expectation mismatches;
- verdict **SPLIT-RESEARCH**.

Durable findings: `docs/SHARED_OWNERSHIP_RESEARCH.md`.

The matrix proves that one generic “shared” feature would be wrong. It separates:

- borrowing, which remains preferable when one owner is sufficient;
- one-thread `Rc`-like multiple ownership as a bounded explicit candidate;
- `Arc` / cross-thread ownership as concurrency design;
- `RefCell` as interior-mutability design;
- `Mutex`-style shared mutation as synchronization design;
- `Weak` / cycles as a separate ownership-edge model;
- arena/index ownership as a distinct graph alternative;
- handle duplication from payload deep cloning.

No production shared-ownership syntax is approved by #106.

## Immediate merge sequence for #108

1. Keep PR #108 research-only: workflow + executable comparison harness + durable report/handoff docs.
2. Validate the final exact PR head with both normal three-OS CI and the dedicated shared-ownership research workflow.
3. Confirm the final diff contains no temporary bootstrap workflow and no production parser/type/codegen changes.
4. Keep historical failed/intermediate SHAs as evidence; do not rerun them for color.
5. When the exact final head is green, mark #108 ready and squash-merge with an expected-head lock.
6. After merge, require natural exact-SHA `main` normal CI **and** dedicated shared-ownership research workflow **SUCCESS** before closing #106.

## Gated successor — #109

Issue #109 is open but must **not** start before #108 merges and post-merge exact-SHA validation succeeds:

`P0 research explicit shared handle surface v0: one-thread Rc-like nominal ownership`

#109 narrows the next question to a caller-visible, one-thread, reference-counted shared-owner handle equivalent to idiomatic `Rc<T>`.

Required contracts for that research:

- compare multiple explicit source-surface families before selecting syntax;
- make initial allocation visible;
- make owner-handle duplication visible;
- ordinary assignment and by-value parameter passing move a handle rather than silently incrementing a count;
- handle duplication is distinct from payload deep clone;
- payload borrows continue through the existing non-owning reference provenance/liveness model;
- moving the particular handle that produced a live reference remains invalid even if another shared owner exists;
- any accepted codegen candidate maps directly to `Rc<T>`, `Rc::new`, `Rc::clone`, ordinary moves/borrows/drop with no extra Evolution runtime layer.

## Hard boundaries

Do not fold `Arc`, cross-thread transfer, interior mutability, `Mutex`/`RwLock`, `Weak` production semantics, cycle solving, arena/index implementation, mutable references, generalized lifetime solving, implicit handle duplication, implicit allocation, hidden deep clone, GC or runtime ownership tables into #109.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed/cancelled SHAs remain evidence and are not rerun merely for a better color.
