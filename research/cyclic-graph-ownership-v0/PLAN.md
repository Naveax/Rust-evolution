# Cyclic graph ownership ergonomics v0 — research plan

Parent: #118

Status: **PREPARATION ONLY**. This branch is intentionally based on the last verified main before #112 finishes. Do not treat it as final evidence and do not open the final research PR until #112 has merged and its natural exact-main CI plus explicit-shared-owner performance gate are green.

## Question

Compare bounded one-thread graph/cycle ownership models without pretending that one abstraction can hide materially different runtime and safety contracts.

The research must keep these families distinct:

- borrow-only control;
- strong `Rc` DAG ownership;
- explicit `Weak` edges;
- arena/index ownership;
- interior-mutability contrast;
- cross-thread `Arc` contrast;
- address-sensitive/self-referential contrast.

No candidate may introduce hidden GC, an Evolution ownership registry, implicit deep clone, implicit strong/weak conversion, synchronization, unsafe emulation, or a generalized lifetime solver.

## Executable matrix

| # | Case | Expected class | Key observation |
| ---: | --- | --- | --- |
| 1 | owned tree control | `OWNED-CONTROL` | plain by-value ownership remains cheapest when aliases/cycles are unnecessary |
| 2 | borrow-only traversal | `BORROW-INSTEAD` | read-only traversal does not need shared owners |
| 3 | explicit `Rc` DAG | `STRONG-RC-DAG` | multiple independent owners pay explicit strong-count work |
| 4 | strong `Rc` cycle | `REJECT-STRONG-CYCLE` | strong-only cycles retain ownership and are not cycle-safe |
| 5 | `Weak` back-edge | `WEAK-EDGE-CANDIDATE` | weak edge breaks ownership cycle without becoming an owner |
| 6 | upgrade after final strong drop | `WEAK-EDGE-CANDIDATE` | dead weak target is observable as failed upgrade |
| 7 | strong/weak count operations | `WEAK-EDGE-CANDIDATE` | count changes are explicit cost, not a free reference |
| 8 | arena/index DAG | `ARENA-INDEX-CANDIDATE` | container owns nodes; edges copy indices instead of touching refcounts |
| 9 | arena/index cycle | `ARENA-INDEX-CANDIDATE` | cycles can be data relationships without ownership cycles |
| 10 | stale index/generation boundary | `ARENA-INDEX-CANDIDATE` | removal needs an explicit stale-handle policy and may require generation metadata |
| 11 | equivalent graph traversal | `PARITY-CONTROL` | candidate representations must produce identical traversal semantics before performance comparison |
| 12 | edge/handle duplicate vs payload clone | `REJECT-HIDDEN-COST` | aliasing and deep cloning are different operations |
| 13 | `Rc<RefCell<T>>` shared mutation | `REQUIRES-INTERIOR-MUTABILITY-DESIGN` | runtime borrow state is a separate feature/cost model |
| 14 | `Arc` cross-thread graph | `REQUIRES-CONCURRENCY-DESIGN` | atomic ownership/thread transfer must not be a silent upgrade |
| 15 | address-sensitive self-reference move conflict | `SELF-REFERENTIAL-SEPARATE` | pinning/address stability is distinct from cyclic graph ownership |

Every case records compile expectation, runtime expectation when applicable, observed stdout/stderr, ownership model, allocation model, and caller-visible ownership operations.

## Evidence fields

The eventual machine-readable report must contain at least:

- exact source `git_sha`;
- `rustc -Vv`;
- case count;
- compile-expectation mismatch count;
- runtime-expectation mismatch count;
- classification counts;
- strong/weak operation notes;
- allocation model;
- stale/dead-target behavior;
- whether the case needs runtime borrow checking, atomics, synchronization, unsafe, generation counters, or hidden state;
- final split/adopt/defer decision basis.

## Acceptance discipline

A candidate is not production-worthy merely because its Rust probe compiles. A production successor needs a bounded source surface, deterministic source-native diagnostics, direct safe Rust lowering, explicit cost, correctness parity and, where runtime work exists, a differential benchmark against idiomatic Rust performing the same ownership operations.

If the matrix confirms multiple viable but materially different models, split successors rather than inventing a generic graph-owner abstraction.