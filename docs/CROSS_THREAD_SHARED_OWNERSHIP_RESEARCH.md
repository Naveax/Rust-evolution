# Cross-thread shared ownership v0 research

Status: **SPLIT-RESEARCH**

Parent: #123

This track isolates immutable cross-thread owner sharing from one-thread `Rc`, dynamic interior mutability, synchronization, weak edges, and payload cloning. It does not add Evolution production syntax or runtime ownership machinery.

## Decision

The evidence supports **explicit Arc-like immutable cross-thread ownership as a distinct candidate**, but it does not support one generic shared-ownership mode.

Accepted classification:

- `ARC-CANDIDATE` for explicit atomic reference-counted immutable ownership;
- `RC/OWNED-INSTEAD` where cross-thread shared ownership buys nothing;
- `REQUIRES-SEND-SYNC-CAPABILITY-DESIGN` for payload/thread capability rules;
- `REQUIRES-SYNCHRONIZATION-DESIGN` for `Mutex`, `RwLock`, atomics, and shared mutation;
- `REQUIRES-WEAK-CYCLE-MODEL` for `Weak` edges;
- `REJECT-HIDDEN-COST` for implicit atomic upgrades, duplication, synchronization, or payload cloning.

A future Arc-like surface therefore remains separate from locks, mutable shared state, Weak edges, and the existing one-thread `shared T` / `Rc<T>` contract.

## Exact executable evidence

Validated research source head:

`188f804aad3e2844c9932339711908c956ae55de`

Dedicated Cross-thread shared ownership research run `34842039994`: **SUCCESS** on pinned Rust 1.98.0.

Artifact:

- name: `evo-cross-thread-shared-ownership-research-ubuntu-24.04`;
- id: `10346138106`;
- digest: `sha256:7b154b27662af87ccfa97f366c41286c622024d1fffa3a0087be26bdd9627d65`;
- report `git_sha`: exact match to the validated source head;
- cases: **15**;
- expectation mismatches: **0**.

Observed classification counts:

- `ARC-CANDIDATE`: **6**;
- `RC/OWNED-INSTEAD`: **2**;
- Send/Sync-capability boundary: **2**;
- synchronization boundary: **3**;
- Weak/cycle boundary: **1**;
- hidden-cost rejection: **1**.

The later branch-only Clippy cleanup only marks captured case stdout as diagnostic-only; it does not alter the Rust cases, classifications, expected compile/runtime behavior, or aggregate decision. Final PR merge still requires exact-head normal CI and dedicated research on the final docs head.

## Matrix boundary

The pinned Rust 1.98 corpus proves:

- an ordinary owned Send value can simply move to another thread without shared ownership;
- `Rc<T>` cannot silently cross the thread boundary;
- `Arc<T>` explicit clone/read/join and atomic strong-count lifecycle work as a distinct ownership model;
- by-value Arc forwarding moves the handle; retaining the caller copy requires explicit duplication;
- final strong-owner drop destroys the payload;
- ordinary references borrowed from an Arc remain normal local references;
- wrapping a non-Send/non-Sync payload in Arc does not manufacture thread safety;
- `Arc<Mutex<T>>`, `Arc<RwLock<T>>`, and Arc plus atomic payload introduce synchronization semantics beyond ownership;
- Arc `Weak` is a separate edge model;
- owner-handle cloning is not payload deep cloning;
- single-thread aliasing should keep the cheaper `Rc`/owned model rather than pay atomic count work.

## Non-negotiable boundaries

Any later production candidate must preserve all of these:

- no silent `Rc -> Arc` upgrade;
- no implicit atomic refcounting;
- no hidden lock or atomic payload conversion;
- no hidden owner duplication;
- no deep clone masquerading as an owner clone;
- no bypass or fabrication of Rust thread-transfer/share capability rules;
- no implicit Weak edge semantics;
- no GC/runtime ownership registry;
- no unsafe emulation;
- performance comparison only against equivalent idiomatic `Arc` work.

## Successor boundary

This result is **not** permission to implement cross-thread mutation or generic concurrency. A production-plausible successor must first define an explicit Arc-like source surface together with a bounded capability model sufficient to reject payloads that Rust itself cannot safely transfer/share. Synchronization remains a separate later track.
