# Cross-thread shared ownership v0 research

Status: **RESEARCH IN PROGRESS**

Parent: #123

This track isolates immutable cross-thread owner sharing from one-thread `Rc`, dynamic interior mutability, synchronization, weak edges, and payload cloning.

## Core distinction

`Arc<T>` changes the ownership mechanism itself: strong/weak count maintenance is atomic and the payload must satisfy Rust's thread-transfer/share capability rules for the intended use. `Mutex`, `RwLock`, and atomic payloads add separate synchronization semantics. None of these costs may appear implicitly.

## Executable matrix

The pinned Rust 1.98 corpus covers:

- owned thread transfer when shared ownership is unnecessary;
- `Rc<T>` cross-thread rejection;
- `Arc<T>` explicit clone/read/join and count lifecycle;
- by-value Arc forwarding and explicit duplicate-before-forwarding;
- final-owner destruction;
- ordinary references derived from an Arc handle inside one thread;
- rejection of Arc thread transfer when the payload does not satisfy Rust's thread-safety requirements;
- `Arc<Mutex<T>>`, `Arc<RwLock<T>>`, and Arc plus atomic payload as synchronization boundaries;
- Arc `Weak` as a separate weak-edge model;
- owner-handle duplication versus payload deep clone;
- single-thread cases where atomic ownership cost is unjustified.

## Decision classes

- `ARC-CANDIDATE`
- `RC/OWNED-INSTEAD`
- `REQUIRES-SEND-SYNC-CAPABILITY-DESIGN`
- `REQUIRES-SYNCHRONIZATION-DESIGN`
- `REQUIRES-WEAK-CYCLE-MODEL`
- `REJECT-HIDDEN-COST`

The aggregate remains `SPLIT-RESEARCH` until exact-head evidence justifies any narrower production successor.

## Non-negotiable boundaries

- no silent `Rc -> Arc` upgrade;
- no implicit atomic refcounting;
- no hidden lock or atomic payload conversion;
- no hidden owner duplication;
- no deep clone masquerading as an owner clone;
- no bypass of Rust thread-safety rules;
- no GC/runtime ownership registry;
- runtime comparison must use equivalent idiomatic `Arc` work.

The branch intentionally remains research-only until the #112 production gate has merged and passed its natural exact-main validation.
