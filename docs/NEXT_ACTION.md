# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume the project from here without prior conversation history.

Last verified update: **2026-08-27**

## Active P0

**Parent issue #41 — Records v0: typed product data**

Completed typed-lowering/ownership child: **#46**

Remaining acceptance child: **#47 — Records v0 zero-cost Rust codegen / differential parity**

Current implementation PR: **#48 — `feat: add typed record lowering IR`**

Branch: `feature/records-typed-lowering-v0`

Validated head:

`97dae87c42108571e6dfd0c0f87b0010bded97e9`

Authoritative validation:

- CI run ID: **33069876396**
- run number: **188**
- conclusion: **SUCCESS**
- Ubuntu / Windows / macOS: fmt, Clippy, workspace tests and release path green
- Ubuntu: all existing runtime gates green
- real CLI/native Records v0 process test builds and runs with output `42`

PR #48 is no longer draft and is mergeable. An assistant-triggered merge attempt was blocked by the product safety layer, not by GitHub repository state or CI.

## What PR #48 already proves

- nominal lowered record types and schemas;
- exact named construction with deterministic schema field order;
- zero-field constructor resolution;
- typed/chained scalar field access;
- record values in function parameters and returns;
- by-value record move semantics with no implicit clone;
- source-native reuse-after-move diagnostics;
- same-type explicit reinitialization;
- conservative `if` ownership merge;
- `repeat` loop-carried move safety;
- explicit fail-closed behavior for whole-record print/equality and record-valued partial field moves;
- static Rust `struct`/literal/direct-field codegen;
- valid record programs through `check`, `emit-rust`, native `build` and execution;
- no runtime record map, boxing, GC/RC, reflection metadata or dynamic dispatch.

## Resume here

Do **not** start enums, collections, error-handling sugar or another language feature yet. Finish Records v0 acceptance first.

### Step 1 — land PR #48

1. Re-read PR #48 head and CI #188.
2. If the head is still `97dae87c42108571e6dfd0c0f87b0010bded97e9` (or a descendant containing only verified handoff docs) and CI is green, merge PR #48 using the repository's normal squash-merge convention.
3. Verify post-merge `main` CI.
4. Close #46 as completed after the merge if GitHub did not auto-close it from the PR body.

Do not rerun an already-running workflow and do not create duplicate Actions for the same SHA/workflow/input.

### Step 2 — continue #47 from updated `main`

After PR #48 is merged, create/continue a dedicated #47 branch from the new `main` head. Keep the next slice evidence-driven and atomic.

Implementation order:

1. **Record-specific source mapping**
   - map generated record declaration lines to declaration/field spans where useful;
   - add constructor/access mapping regressions under the existing source-map policy;
   - verify backend-owned rustc diagnostics remap cleanly when applicable.

2. **Expand real CLI/native correctness corpus**
   - nested acyclic record construction;
   - chained scalar access;
   - zero-field record construction;
   - record parameter + record return roundtrip;
   - explicit record reinitialization after move;
   - rejected record source produces no native binary.

3. **Dedicated Records v0 differential benchmark**
   - use runtime input so constant folding cannot erase the workload;
   - Evolution and reference Rust must use the same record layout, algorithm, inputs, outputs and release flags;
   - correctness must pass before any performance verdict.

4. **Codegen evidence**
   - retain generated Rust artifact;
   - compare normalized LLVM;
   - report binary sizes;
   - report exact executable byte equality when achieved;
   - retain raw timing even when exact equality establishes deterministic parity.

5. **CI performance gate**
   - add the Ubuntu Records v0 gate;
   - preserve the hard `T_evolution / T_reference <= 1.00` rule unless exact executable equality establishes parity;
   - keep all older runtime gates green.

6. **Specification and closure**
   - only after source-map + benchmark evidence is green, update `docs/LANGUAGE_SPEC_V0.md` with Records v0 syntax, nominal typing, constructor/access rules, move semantics and explicit limitations;
   - update `docs/PROJECT_STATE.md`, this file and parent #41;
   - close #47 and #41 after final CI/post-merge validation.

## Records v0 explicit limitations to preserve

- no whole-record `print`;
- no record equality;
- no partial move of record-valued fields;
- no implicit clone/copy insertion;
- no implicit borrow/reference inference;
- no heap/self-referential by-value recursion;
- no runtime object dictionaries/reflection machinery.

These are deliberate v0 boundaries. Do not weaken them merely to make a test convenient.

## Performance rule

For ZERO-cost/native core work:

`T_evolution <= T_reference_rust`

Prefer stronger codegen evidence in this order:

1. correctness PASS;
2. generated Rust inspection;
3. normalized LLVM equality;
4. exact executable equality;
5. otherwise stable hard runtime ratio gate.

The next accepted language feature is chosen only after Records v0 parent #41 is closed.