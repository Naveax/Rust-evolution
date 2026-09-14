# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-14**

## Stable gate

Current exact verified `main`:

`c5ccc21d7bf23d8daec2de36dc635c0f85313605`

Natural validation on that exact SHA:

- CI #512 / run `34834431873`: **SUCCESS** on Ubuntu 24.04, Windows and macOS;
- Explicit shared owner performance #12 / run `34834431791`: **SUCCESS**;
- performance artifact id `10343513286`;
- digest `sha256:fb6714e7f09ca0047c70435b21129128033d8235732ca1d9f82b6fdeab9716bf`;
- artifact head SHA exactly matches verified main.

## Completed production milestone — #112 / PR #117

The bounded one-thread explicit shared-owner slice is production behavior:

```text
shared Item
share expr
dup owner
```

Direct safe Rust mapping:

```text
shared Item -> std::rc::Rc<Item>
share expr  -> Rc::new(expr)
dup expr    -> Rc::clone(&expr)
```

Key contracts remain locked:

- shared owners are a distinct move-only value category;
- ordinary assignment, by-value calls and returns move handles without hidden count increments;
- allocation and owner duplication are explicit only;
- payload immutable references remain ordinary non-owning references tied to the specific source handle;
- no implicit owned/reference/shared-owner conversions;
- no `Arc`, `RefCell`, synchronization, `Weak`, GC, global ownership runtime or unsafe emulation;
- record/enum storage of shared owners remains outside v0;
- enum-bearing unsupported combinations fail closed before executable codegen.

## Active research queue

### #118 — cyclic / graph ownership boundaries

PR #120 is parked closed only while its research branch is reconstructed cleanly on verified main.

Accepted dedicated evidence already exists on historical exact research head `4c6a7e4f1d24388d3081e28ca88c73f142f88a68`:

- Cyclic graph ownership research #12 / run `34832221728`: **SUCCESS**;
- artifact id `10342831245`;
- digest `sha256:5ebf114519378c431f6cee33181de569ef5d4cd6e54e58f09cf3e502b43612ba`;
- verdict: **SPLIT-RESEARCH**;
- 15 cases;
- compile/runtime expectation mismatches: 0 / 0.

The clean persistent research diff is five files only: dedicated workflow, main matrix, retention controls, arena/generation controls and durable report.

Gated successors exist but must not start before #118 merges and its natural exact-main normal CI plus dedicated research workflow succeed:

- #125 — explicit `Weak` edge surface research;
- #126 — arena/generational graph-handle research.

### #121 — interior mutability ergonomics

PR #122 is parked closed with its branch preserved. The clean research diff contains only the dedicated workflow, Rust 1.98 matrix, direct guard-state controls and durable report. Shared ownership and dynamic borrow state remain separate models.

### #123 — cross-thread shared ownership

PR #124 is parked closed with its branch preserved. The clean research diff contains only the dedicated workflow, Arc/thread-capability matrix and durable report. `Arc`, Send/Sync-like capability, locks/atomics and Weak remain separate concerns; no automatic `Rc -> Arc` upgrade is permitted.

## Execution order

1. Merge this docs-only handoff after exact-head normal CI.
2. Require the resulting docs-only `main` SHA to pass natural normal CI.
3. Reconstruct PR #120's five research-only files onto that exact verified main, reopen #120, and require exact-head normal CI plus dedicated cyclic-graph research evidence.
4. While #120 Actions run, reconstruct #122 and #124 onto the same verified main while keeping them closed so they do not flood the runner queue.
5. Merge/validate #118 only after its exact-head gates pass; close #118 only after natural postmerge exact-main normal CI and dedicated cyclic research both pass.
6. Only then unlock #125 and #126.
7. Validate #121 and #123 independently and create production successors only for bounded accepted candidates.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. If one gate is queued or running, advance independent work and return later. Historical failed/cancelled SHAs remain evidence rather than targets for cosmetic reruns.
