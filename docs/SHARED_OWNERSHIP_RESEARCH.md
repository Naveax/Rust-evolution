# Shared Ownership Ergonomics v0 Research

Last verified research evidence: **2026-09-11**

## Status / decision

Decision: **SPLIT-RESEARCH**.

Shared ownership is not one semantic feature. The validated Rust 1.98 matrix shows one bounded candidate family — explicit one-thread `Rc`-like shared-owner handles — while borrowing, cross-thread `Arc` ownership, interior mutability, synchronization, weak/cycle handling and arena/index ownership remain materially different safety and cost models.

No production Evolution shared-ownership syntax, semantic type, allocation policy or runtime behavior is introduced by this research.

## Scope and predecessor

Parent issue: #106 — `P0 research shared ownership ergonomics v0: explicit aliasing without hidden ownership cost`.

Verified production base before the research branch:

`3be6b212a549831c15ef5a30df2b35da6362683e`

This base already contains:

- ordinary owned nominal values;
- inferred call-duration `SharedBorrow` parameters;
- first-class immutable `SharedRef` values via `&T` / `&expr`;
- deterministic reference provenance and bounded final-use liveness;
- source-native ownership/reference diagnostics;
- direct safe Rust reference lowering.

The research intentionally does **not** overload any of those non-owning concepts into shared ownership.

## Exact evidence

Validated research source head:

`651ef4f3e9fc70e493eb59e3b1b2e7dcc7526d6a`

Validation on that exact head:

- normal CI #463 / run `34614250716`: **SUCCESS** on Ubuntu, Windows and macOS;
- Ubuntu passed fmt, Clippy, workspace tests, all permanent build/cache/runtime gates and release build;
- dedicated Shared ownership ergonomics research #4 / run `34614250773`: **SUCCESS** on Ubuntu 24.04 / Rust 1.98.0;
- artifact: `evo-shared-ownership-research-ubuntu-24.04`;
- artifact id: `10269252562`;
- artifact digest: `sha256:f33fd905c3feb550550a8fc99da75575dcce1d8a54c9ef1b1a531a68256149b5`;
- report `git_sha`: exact `651ef4f3e9fc70e493eb59e3b1b2e7dcc7526d6a`;
- cases: **14**;
- compile-expectation mismatches: **0**;
- runtime-expectation mismatches: **0**.

Pinned compiler:

```text
rustc 1.98.0 (88d9e12ae 2026-08-18)
host: x86_64-unknown-linux-gnu
LLVM version: 22.1.8
```

An earlier research run on `58fd73dee2f385e49e421cc5998f3ad665b6db90` proved the semantic matrix but recorded GitHub's pull-request synthetic merge SHA in the report and the normal CI head was not rustfmt-clean. That historical evidence was not rerun for color. Provenance and formatting were corrected on later source heads, culminating in the exact validated evidence above.

## Architecture baseline

The current production ownership model has three facts that must remain separate:

1. `ParameterPassingMode::SharedBorrow` is a call-duration non-owning parameter mode.
2. semantic/lowered `SharedRef` is a first-class non-owning immutable reference value.
3. `ReferenceTracker` tracks source provenance and possible liveness for references; it does not represent owners.

Therefore a future shared-owner handle, if accepted, requires an explicit ownership model. It must not be represented as a special `SharedBorrow`, a copied `SharedRef`, or a provenance exception.

Record fields also remain owned-only in the current production language, and Evolution has no production concurrency/interior-mutability/arena surface. The research treats those missing capabilities as real boundaries rather than pretending one aliasing abstraction solves them.

## Executable matrix

| Case | Classification | Ownership / cost model | Result |
| --- | --- | --- | --- |
| `owned-control` | `OWNED-CONTROL` | stack-only single owner; move | compile/run match |
| `borrow-control` | `BORROW-INSTEAD` | stack-only shared borrow | compile/run match |
| `rc-local-alias-clone-read-drop` | `EXPLICIT-SHARED-CANDIDATE` | one `Rc` allocation; non-atomic strong-count increment/decrement | compile/run match |
| `rc-function-forwarding` | `EXPLICIT-SHARED-CANDIDATE` | explicit handle clone, then ordinary handle move/drop | compile/run match |
| `rc-branch-repeat-clone-drop` | `EXPLICIT-SHARED-CANDIDATE` | repeated explicit non-atomic handle clone/drop | compile/run match |
| `rc-reference-derived-retained-handle` | `EXPLICIT-SHARED-CANDIDATE` | ordinary borrow derived from one live `Rc` handle | compile/run match |
| `rc-reference-derived-handle-move-conflict` | `EXPLICIT-SHARED-CANDIDATE` | borrow followed by invalid move of that source handle | expected Rust rejection matched (`E0505`) |
| `arc-cross-thread` | `REQUIRES-CONCURRENCY-DESIGN` | one `Arc` allocation; atomic refcount + thread transfer | compile/run match |
| `rc-cross-thread-rejected` | `REQUIRES-CONCURRENCY-DESIGN` | attempted thread transfer of non-`Send` `Rc` | expected Rust rejection matched (`E0277`) |
| `rc-refcell-shared-mutation` | `REQUIRES-INTERIOR-MUTABILITY-DESIGN` | `Rc` refcount + `RefCell` runtime borrow state | compile/run match |
| `arc-mutex-shared-mutation` | `REQUIRES-CONCURRENCY-DESIGN` | atomic refcount + `Mutex` lock/unlock | compile/run match |
| `rc-weak-cycle-break` | `REQUIRES-WEAK/CYCLE-MODEL` | strong/weak count maintenance | compile/run match |
| `arena-index-graph` | `ARENA/INDEX-PREFERRED` | container-owned nodes; copied stable indices, no per-edge refcount | compile/run match |
| `handle-clone-vs-deep-clone` | `REJECT-HIDDEN-COST/AMBIGUOUS` | handle refcount increment versus payload clone + second allocation | compile/run match |

Classification totals:

- `EXPLICIT-SHARED-CANDIDATE`: **5**;
- `BORROW-INSTEAD`: **1**;
- `REQUIRES-INTERIOR-MUTABILITY-DESIGN`: **1**;
- `REQUIRES-CONCURRENCY-DESIGN`: **3**;
- `REQUIRES-WEAK/CYCLE-MODEL`: **1**;
- `ARENA/INDEX-PREFERRED`: **1**;
- `REJECT-HIDDEN-COST/AMBIGUOUS`: **1**;
- plus one owned control.

## Findings

### Borrow when borrowing is enough

Two read-only users of one owner need no second owner when the original owner outlives both uses. The `borrow-control` case remains `BORROW-INSTEAD`; an `Rc`-like feature must not replace ordinary `&T` merely because aliasing exists syntactically.

### One-thread shared ownership is a real, explicit cost model

The `Rc` cases prove a useful semantic family that current Evolution cannot express: independently storable owners of one allocation. That model has real operations — allocation, strong-count increment and decrement — and therefore cannot be inferred from ordinary assignment or hidden behind a borrow.

Ordinary assignment and by-value parameter passing should remain moves of a handle. Creating another owner must be an explicit operation equivalent to `Rc::clone`, not an implicit copy exception in move tracking.

### Handle duplication is not payload cloning

The handle-clone/deep-clone probe produced different allocation, identity and refcount behavior. A future source surface must make these operations impossible to confuse. One generic hidden `clone` policy would be semantically and operationally ambiguous.

### A shared owner does not erase borrow liveness

A reference derived from a particular `Rc` handle is still an ordinary borrow. Moving that same handle while its derived reference remains live is rejected by Rust even if another shared owner exists. Therefore payload references from a future shared handle should continue through Evolution's reference-provenance/liveness model rather than gaining invented lifetime widening.

### `Rc` and `Arc` are not one transparent mode

`Arc` permits cross-thread ownership by paying atomic refcount costs. `Rc` is intentionally not `Send` and the negative probe rejects its transfer. Evolution must not silently upgrade a local shared handle to `Arc`, because doing so changes both the safety contract and runtime operations.

### Shared mutation is a separate problem

`Rc<RefCell<T>>` adds runtime borrow checking. `Arc<Mutex<T>>` adds atomic ownership plus synchronization. Neither behavior follows from shared ownership alone, and neither belongs in a first immutable/shared-owner handle slice.

### Cycles require a weak-edge model

Reference counting alone does not solve cyclic ownership. The `Weak` probe demonstrates a separate edge kind and separate strong/weak count semantics. A future `Rc`-like handle must not claim cycle safety it does not possess.

### Arena/index ownership remains a real alternative

The arena/index probe centralizes node ownership in a container and copies indices instead of updating a refcount per edge. Some graph workloads should prefer this model. Current Evolution lacks the container/type surface needed to make it a production design candidate today, so it remains a separate deferred track.

## Decision

**SPLIT-RESEARCH**.

Do not implement one generic shared-ownership feature from #106. Split the useful one-thread immutable shared-owner handle into a narrower surface/semantic research successor, and keep the other cost/safety families separate.

Successor issue:

- #109 — `P0 research explicit shared handle surface v0: one-thread Rc-like nominal ownership`.

#109 is gated on #108 merging and natural exact-SHA `main` validation. It must compare multiple explicit source surfaces and preserve the key contracts proven here:

- shared ownership is distinct from borrowing;
- initial allocation is caller-visible;
- handle duplication is caller-visible;
- ordinary assignment/parameter passing moves the handle rather than silently incrementing a count;
- handle duplication and payload deep clone are distinct operations;
- payload references keep ordinary reference liveness/provenance;
- codegen, if later accepted, maps directly to idiomatic `Rc<T>` operations without an extra Evolution runtime layer.

## Explicitly separate / deferred tracks

The following are not approved by this decision and need their own prerequisites or research before production work:

- cross-thread `Arc`-like ownership and `Send`/`Sync`-equivalent rules;
- interior mutability;
- locking / synchronized shared mutation;
- `Weak` production semantics and cycle diagnostics;
- arena/index ownership and graph ergonomics;
- mutable references;
- generalized lifetime solving;
- implicit shared-owner inference;
- hidden allocation, handle duplication, deep clone, synchronization, GC or runtime ownership maps.

## Production impact

None. This report records research evidence only. Existing owned `T`, inferred call-duration `SharedBorrow`, first-class immutable `&T`, move diagnostics, reference provenance/liveness and generated Rust contracts are unchanged.
