# Cyclic graph ownership ergonomics v0 research

Parent: #118

Status: **SPLIT-RESEARCH**

This research separates cyclic/graph ownership models instead of treating reference counting as a universal graph solution. It introduces no Evolution production syntax or runtime ownership machinery.

## Accepted executable evidence

Historical exact research head:

`4c6a7e4f1d24388d3081e28ca88c73f142f88a68`

Dedicated pinned-Rust evidence:

- Cyclic graph ownership research #12 / run `34832221728`: **SUCCESS**;
- Rust: **1.98.0**;
- artifact `evo-cyclic-graph-ownership-research-ubuntu-24.04`;
- artifact id `10342831245`;
- digest `sha256:5ebf114519378c431f6cee33181de569ef5d4cd6e54e58f09cf3e502b43612ba`;
- artifact/report git SHA exactly matches the research head;
- cases: **15**;
- compile expectation mismatches: **0**;
- runtime expectation mismatches: **0**.

Observed classification counts:

- weak-edge candidates: **4**;
- arena/index candidates: **4**;
- borrow-instead controls: **1**;
- strong-cycle-retention/leak control: **1**;
- interior-mutability boundary: **1**;
- concurrency boundary: **1**;
- self-referential separate problem: **1**;
- hidden-cost rejection: **1**;
- plus the owned acyclic control.

The final PR head after rebasing onto the latest verified `main` must still pass normal three-OS CI and the dedicated workflow before #118 can merge. The historical artifact is decision evidence, not a substitute for that integration gate.

## Models classified

The pinned Rust corpus and direct controls distinguish:

- ordinary owned acyclic trees;
- borrow-only traversal where a container already owns all nodes;
- explicit strong `Rc` DAG ownership;
- strong `Rc` cycles and their retention behavior;
- explicit `Weak` back edges, downgrade/upgrade, and strong/weak count lifecycle;
- arena/index DAG and cyclic graphs;
- stale plain-index behavior and generation-checked handles;
- equivalent traversal results across candidate models;
- edge/handle duplication versus payload deep clone;
- `Rc<RefCell<T>>` as an interior-mutability boundary;
- `Arc` as a concurrency boundary;
- address-sensitive self-reference as a separate problem.

Direct tests additionally prove that:

1. a two-node all-strong `Rc` cycle retains both nodes after external owners drop;
2. replacing the back edge with `Weak` permits deterministic destruction;
3. a plain copied index can silently refer to a new occupant after slot reuse;
4. an `(index, generation)` handle can reject that stale identity while a fresh generation resolves the reused slot.

## Decision

Aggregate verdict: **SPLIT-RESEARCH**.

`Weak` edges and arena/generational handles are not two spellings for one feature. They differ in storage ownership, identity, dead-target behavior, removal semantics, per-edge cost, and API shape.

The evidence also rejects treating any of these as implicit upgrades of the production `shared T` / `Rc<T>` owner model.

## Successors

Two narrower research successors preserve the split:

- #125 — explicit `Weak` edge surface v0;
- #126 — arena/generational graph handles v0.

Both successors are gated: **do not start their branches until #118 merges and the resulting exact `main` passes natural normal CI plus the dedicated cyclic-graph research workflow.**

## Safety / cost boundaries

Any later accepted successor must keep explicit:

- who owns node storage;
- whether an edge owns, weakly observes, or indexes a target;
- final-strong-owner behavior;
- stale/dead target behavior;
- strong/weak count operations for reference-counted edges;
- generation or equivalent stale-handle policy for reusable arena slots;
- mutation as a separate interior-mutability model;
- cross-thread ownership as a separate concurrency model.

Rejected shortcuts remain hidden GC, hidden runtime ownership graphs, implicit strong/weak conversion, implicit deep clone, hidden `RefCell`, hidden `Arc`, locks, unsafe pointer emulation, and fabricated target validity.
