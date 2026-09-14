# Interior mutability ergonomics v0 — research plan

Parent: #119

Status: **PREPARATION ONLY**. This branch is intentionally based on the last verified main while #112 finishes. Do not open the final research PR or treat this branch as accepted evidence until #112 merges and its natural exact-main normal CI plus explicit-shared-owner performance gate succeed.

## Question

Find out whether bounded one-thread interior-mutability ergonomics can improve shared-mutation workflows without hiding Rust's dynamic borrow checks or turning immutable `shared T` into a mutation loophole.

The matrix must keep these models separate:

- plain owned mutable state;
- exclusive `&mut`;
- `Cell<T>` copy-in/copy-out mutation;
- `RefCell<T>` runtime borrow guards;
- explicit composition `Rc<RefCell<T>>`;
- lock-based cross-thread mutation as a separate concurrency model.

## Executable matrix

| # | Case | Expected class | Key observation |
| ---: | --- | --- | --- |
| 1 | owned mutable local | `OWN/BORROW-INSTEAD` | no runtime borrow state when one owner is enough |
| 2 | exclusive `&mut` control | `OWN/BORROW-INSTEAD` | compile-time exclusivity is cheaper than interior mutability |
| 3 | `Cell<i64>` get/set | `CELL-CANDIDATE` | scalar copy-in/copy-out mutation has no borrow guard |
| 4 | aliased `Cell` shared refs | `CELL-CANDIDATE` | multiple shared references may update copy-like cell contents explicitly |
| 5 | `RefCell` immutable borrow | `REFCELL-CANDIDATE` | runtime borrow flag creates a guard |
| 6 | `RefCell` mutable borrow | `REFCELL-CANDIDATE` | exclusive runtime guard permits mutation |
| 7 | guard drop then mutable borrow | `REFCELL-CANDIDATE` | guard lifetime is semantically relevant |
| 8 | immutable+mutable overlap | `REQUIRES-GUARD-SURFACE-RESEARCH` | invalid dynamic overlap must remain visible |
| 9 | mutable+mutable overlap | `REQUIRES-GUARD-SURFACE-RESEARCH` | second dynamic exclusive borrow fails |
| 10 | `try_borrow` / `try_borrow_mut` | `REFCELL-CANDIDATE` | explicit fallible path avoids panic |
| 11 | `Rc<RefCell<T>>` mutation | `EXPLICIT-COMPOSITION-CANDIDATE` | shared ownership and interior mutation compose as two costs |
| 12 | one Rc handle dropped, alias survives | `EXPLICIT-COMPOSITION-CANDIDATE` | owner lifetime is independent of borrow-state semantics |
| 13 | alias vs payload deep clone | `REJECT-HIDDEN-RUNTIME-COST` | identity-sharing and independent copies differ |
| 14 | `Arc<Mutex<T>>` contrast | `REQUIRES-CONCURRENCY-DESIGN` | atomics/locks are not a silent upgrade |
| 15 | plain borrow alternative | `OWN/BORROW-INSTEAD` | interior mutability is rejected when ordinary borrowing suffices |

## Evidence requirements

Each case records:

- exact ownership model;
- allocation model;
- caller-visible operations;
- runtime borrow state: yes/no;
- dynamic failure path: none/panic/fallible result;
- expected compile result;
- expected runtime output or panic behavior;
- observed result;
- whether current Evolution can express it without a new feature.

## Design boundaries

A production candidate must not:

- make ordinary `&T` secretly runtime-checked and mutable;
- invent implicit `RefCell` allocation;
- turn `shared T` into `Rc<RefCell<T>>` automatically;
- extend a borrow guard using unsafe code;
- hide panic/fallible overlap semantics;
- silently upgrade to `Arc`, `Mutex`, or `RwLock`;
- introduce GC or a runtime ownership registry.

The research should prefer ordinary ownership/borrowing whenever the matrix shows that interior mutability adds no semantic value.