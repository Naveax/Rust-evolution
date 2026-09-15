# Rust Evolution — Project State

Last verified update: **2026-09-15**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Exact verified stable main before active production #140: `532e88586a252e72d7eb9923148b07e2ab94883e`
- Rust toolchain: **1.98.0**
- Production flags: edition 2024, opt-level 3, codegen-units 1
- Natural exact-main CI #555 / run `34957074327`: **SUCCESS** on Ubuntu 24.04, Windows and macOS
- Natural exact-main Collection surface research #6 / run `34957074432`: **SUCCESS**
- Natural exact-main Explicit shared owner performance #43 / run `34957074499`: **SUCCESS**

`532e885...` is PR #139 squash merge. Issue #132 collection-surface research is closed/completed with the accepted **APPEND-ONLY-FIRST** result.

## Build / compile sequence

- #76: build latency baseline established; single-file native builds are rustc-dominated.
- #79 / PR #81: verified unchanged-build cache accepted.
- #82 / PR #84: changed-source rustc incremental research **REJECT / DEFER**.
- #85 / PR #86: link-time attribution established the current `rustc -> cc -> lld` path.
- #87 / PR #88: alternative linker experiment **REJECT / DEFER**.
- #89 / PR #90: opt-level 3 -> 2 candidate **REJECT / DEFER**.
- #91 / PR #92: compile-memory baseline **DEFER / NO ACTION**.
- #93 / PR #94: binary-size baseline **DEFER / NO ACTION**; controlled Evolution/reference binaries were byte-identical across the accepted corpus.

Dependency-build, proc-macro cost and workspace scaling remain deferred until Evolution has a real package/dependency graph.

## Ownership ergonomics implemented

### Inferred shared-borrow parameters

`Owned` and call-duration `SharedBorrow` parameter modes are distinct. SharedBorrow is non-owning and does not create a stored or escaping reference.

### First-class immutable references

`&T` / `&expr` are implemented for the bounded nominal slice with deterministic provenance, stored reference locals, source-owner move/reinitialization conflicts, bounded final-use liveness and direct safe Rust reference lowering.

### Explicit one-thread shared owners

Production source surface:

```text
shared Item
share expr
dup owner
```

Direct generated Rust mapping remains ordinary safe `Rc<T>`:

```text
shared Item -> std::rc::Rc<Item>
share expr  -> Rc::new(expr)
dup expr    -> Rc::clone(&expr)
```

Locked invariants:

- shared owners are distinct from owned values, first-class immutable references, and inferred SharedBorrow parameters;
- allocation and owner duplication are explicit only;
- ordinary assignment, by-value calls and returns move handles without hidden count increments;
- payload references remain tied to the specific source handle;
- moving/reinitializing that source handle rejects while its dependent reference may still be live;
- bounded final-use analysis releases the source after the final proven reference use;
- move-only payload extraction through shared ownership rejects instead of cloning;
- no hidden `Arc`, `RefCell`, synchronization, GC, global ownership registry or unsafe ownership emulation.

The permanent Explicit shared owner performance workflow remains a regression gate for changes that touch the relevant language/compiler surface.

## Cyclic / graph ownership research sequence

### Arena/generational handles — #126 / PR #131 completed

Durable report: `docs/ARENA_GENERATIONAL_HANDLES_RESEARCH.md`.

Accepted result:

- aggregate gate: **REQUIRES-COLLECTION-SURFACE**;
- graph-identity recommendation: **GENERATIONAL-HANDLE-CANDIDATE**;
- plain numeric indices are acceptable only while removal/reuse cannot silently rebind identity;
- reusable/removable slots require generation checking;
- independent node lifetime remains a shared-owner problem rather than an arena problem.

### Collection surface — #132 / PR #139 completed

Durable report: `docs/COLLECTION_SURFACE_RESEARCH.md`.

Accepted result:

- verdict: **APPEND-ONLY-FIRST**;
- recommended surface: **CONTEXTUAL-SEQUENCE-TYPE-CANDIDATE**;
- append/growth plus checked lookup maps directly to ordinary safe Rust storage;
- append-only numeric indices remain logically stable across physical `Vec` reallocation;
- shifting removal can silently rebind identity, while hole-preserving removal introduces an explicit occupancy model;
- live element references must block conflicting container growth/move until bounded final-use release proves them dead;
- removal, holes, generations and reusable arena slots remain later explicit layers.

## Active production — #140 append-only sequence v0

Branch:

`feature/append-only-sequence-v0`

Validated implementation head before documentation synchronization:

`95d847cfe0b6d6d052b096748360915fe1bbcdc6`

Development evidence:

- Dev sequence semantics v0 #4 / run `34982930824`: **SUCCESS**;
- focused parser/lowering/codegen/generated-Rust tests: **PASS**;
- workspace tests: **SUCCESS**;
- workspace Clippy `-D warnings`: **SUCCESS**;
- append-only differential benchmark: correctness **true**, normalized LLVM IR equal **true**, exact binary equal **true**, stable **true**, ratio **0.996626508**, verdict **PASS** by byte-identical-binary parity.

Production source surface:

```text
items = seq Item()
append items, Item(value = 1)

lookup items, 0 as item
    print item.value
else
    print 0
end
```

Implemented invariants:

- `seq`, `append`, `lookup`, and `as` remain contextual identifiers;
- `seq T` is an owned move-only append-only sequence;
- supported v0 element types are scalars, declared nominal records, and explicit `shared Record`; nested sequence/reference element types remain rejected;
- empty construction lowers to `Vec::<T>::new()`;
- append lowers to direct `Vec::push` and moves move-only payloads without hidden clone;
- lookup converts the signed index with `usize::try_from`, uses direct `Vec::get`, and requires an explicit success/failure branch;
- negative and out-of-range indices take the `else` branch;
- scalar elements bind by value; move-only record/shared-owner elements bind through an immutable element reference;
- live move-only element references block sequence growth, move, and reinitialization source-natively;
- existing bounded last-use analysis releases that conflict after the final proven reference use;
- shared-owner element lookup keeps `&Rc<T>` behavior without hidden `Rc::clone`;
- sequence parameters become mutable in generated Rust only when grown;
- unused move-only lookup bindings do not artificially pin the sequence;
- generated Rust remains ordinary safe `Vec<T>` / `push` / `get` with no `unsafe`, `RefCell`, lock, GC, registry, or custom runtime.

Permanent regression workflow: `.github/workflows/append-only-sequence-performance.yml`.

## Separate / deferred research tracks

Do not silently fold these into append-only sequence semantics:

- explicit Weak/cycle-edge surface;
- interior mutability;
- cross-thread shared ownership / `Arc` / synchronization;
- mutable references;
- generalized/user-written lifetime solving;
- general generic type syntax;
- sequence removal/pop/delete;
- hole reuse and generation-checked reusable arena slots;
- shared-owner record fields / enum payloads;
- hidden allocation policy, owner duplication, deep clone, GC or runtime ownership maps.

## Current operational sequence

1. Synchronize implementation-backed language and handoff docs for #140.
2. Open the #140 production PR from `feature/append-only-sequence-v0`.
3. Require one exact final PR head to pass normal CI plus the Append-only sequence performance gate and all other naturally triggered ownership/research regressions.
4. Review the final PR diff and merge only with expected-head protection.
5. Require natural exact-main postmerge CI and append-only sequence performance before closing #140 completed.
6. Return to the generation-checked arena-handle candidate from #126; removal/reuse/generation remains a separate explicit layer.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed/cancelled historical SHAs remain evidence and are not rerun merely for color. CI running does not block independent source/docs work.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
