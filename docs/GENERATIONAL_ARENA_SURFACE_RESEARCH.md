# Generational arena surface v0 research

Status: **RUNTIME-ARENA-ID-GENERATIONAL-CANDIDATE / CONTEXTUAL-ARENA-HANDLE-CANDIDATE**

Parent: #142

Prerequisites:

- #126 / PR #131 established generation-checked handles as the graph-identity candidate once a bounded collection surface existed;
- #132 / PR #139 selected append-only indexed storage as the first collection layer;
- #140 / PR #141 shipped and verified bounded append-only `seq T`, satisfying the collection prerequisite without introducing general generics.

This research resolves the remaining reusable-slot identity question. It does not authorize or implement production arena syntax, mutable references, a general generic type system, GC, a global handle registry, per-edge reference counting, `RefCell`, locking, or unsafe pointer identity.

## Decision

The executable matrix selects a bounded runtime identity model with three copyable fields:

```text
(arena identity, slot index, generation)
```

The accepted candidate keeps storage arena-owned, uses direct indexed slots plus an O(1) free-index stack, and rejects stale or cross-arena handles before payload access.

Candidate-family classification:

| Candidate | Classification | Reason |
| --- | --- | --- |
| `(index, generation)` without arena identity | **REJECT** | Two distinct arenas can contain the same slot index and generation while owning different payloads. The handle is therefore not sufficient identity across arena values. |
| `(arena identity, index, generation)` | **ACCEPT CANDIDATE** | Cross-arena use is rejected, stale generations fail, handle copies require no owner-count traffic, and direct safe Rust storage remains practical. |
| statically instance-scoped handles | **DEFER** | Current Evolution nominal/type provenance cannot encode one runtime arena instance into storable handle types without a broader dependent/generic scope mechanism. |
| non-reusing tombstone slots | **CONTROL** | It avoids stale-target rebinding by never reusing holes, but storage grows with removals and does not solve reusable-slot identity. |
| explicit shared owners | **CONTROL** | Values whose lifetime must be independent of the arena remain an explicit shared-ownership problem rather than arena identity. |

## Research surface family

The matrix keeps the candidate words contextual rather than adding lexer keywords:

```text
items = arena Item()
insert items, Item(value = 1) as h

lookup items, h as item
    print item.value
else
    print 0
end

remove items, h as removed
    print removed.value
else
    print 0
end
```

Candidate type forms:

```text
arena Item
handle Item
```

These forms are research candidates only. Production syntax remains gated on a separate implementation issue after #142 closes.

Outside their exact contextual positions, `arena`, `handle`, `insert`, and `remove` remain ordinary identifiers. The surface therefore does not require a general `Arena<T>` / `Handle<T>` generic grammar merely to expose the bounded v0 family.

## Runtime identity and reuse

The accepted runtime candidate uses arena-local storage plus explicit arena identity:

- insertion into an unused tail slot returns its index and current generation;
- removal moves the payload out exactly once;
- successful removal invalidates the old handle;
- reusable slots increment generation before entering the free-index stack;
- a subsequent insertion may reuse that slot in O(1) time and returns the fresh generation;
- checked lookup requires arena identity, bounds, occupancy, and generation to match;
- a stale generation cannot resolve a reused payload;
- a handle from another arena is rejected even when index and generation happen to be identical.

The matrix deliberately constructs the cross-arena collision control with the same `(index, generation)` in two arenas. The candidate without arena identity can therefore cross-resolve the wrong payload, while the three-field handle rejects the same access.

## Exhaustion and wraparound

Generation wrap is fail-closed rather than cyclic:

- if a live slot at generation `u64::MAX` is removed, that slot is permanently retired;
- retired slots never return to the free list;
- an old handle therefore cannot become valid again through generation wrap;
- the arena identity source must likewise reject exhaustion rather than silently wrap and recreate an old identity.

No hidden epoch table or process-global identity registry is introduced to manufacture uniqueness.

## Ownership and borrow boundaries

The candidate stays aligned with the current Evolution ownership model and ordinary safe Rust borrowing:

- arena storage owns live payloads;
- arena drop destroys every remaining live payload exactly once;
- removal transfers a move-only payload without implicit clone;
- lookup of an explicit shared-owner payload borrows the stored owner and does not perform hidden `Rc::clone`;
- copying a handle copies three plain machine-word-class fields and performs no strong/weak reference-count update;
- a live element reference blocks conflicting removal;
- a live element reference blocks insertion/growth/reuse-capable mutation;
- a live element reference blocks moving the arena while the reference may still be used;
- a live element reference blocks reinitializing the arena while that reference may still be used;
- ordinary bounded final-use / non-lexical-lifetime behavior permits the conflicting operation after the final proven reference use.

This research does not add mutable references. Exclusive arena operations remain explicit owner operations.

## Direct Rust cost model

The runtime candidate is intentionally ordinary, inspectable safe Rust:

- slot storage is direct `Vec`-class indexed storage;
- vacancy is explicit in each slot;
- one free-index stack supplies constant-time reuse;
- lookup performs arena-id comparison, vector bounds checking, occupancy checking, and generation comparison;
- handle copy is fixed-size scalar copying;
- no per-handle allocation or refcount operation is required.
- a deterministic equivalent-work control executes the candidate and an independent idiomatic Rust generational-slot reference through the same seven insert/get/remove/reuse/stale-get operations;
- both produce the same checksum, slot count, free-list state, operation count, and three-machine-word-class handle size.

The research model contains no `unsafe`, global handle table, per-handle reference counting, tracing GC, `RefCell`, lock, or pointer-identity shortcut.

## Rejected shortcuts

The following are explicitly outside the accepted candidate:

- plain reusable numeric indices with no generation;
- `(index, generation)` identity with silent cross-arena acceptance;
- generation wrap that can resurrect stale handles;
- process-global handle registries;
- per-edge or per-handle hidden reference counting;
- hidden payload clone;
- hidden `Rc::clone` during lookup;
- tracing GC;
- compiler-inserted `RefCell` or synchronization;
- unsafe raw-pointer graph identity;
- general generic syntax introduced solely for this bounded surface;
- pretending arena ownership gives a payload lifetime independent of the arena.

## Evidence contract

The dedicated pinned workflow is `.github/workflows/generational-arena-surface-research.yml` and executes the ignored matrix test in `crates/evo-lowering/tests/generational_arena_surface_research.rs` on Rust 1.98.0.

The report must retain exact commit provenance and validate, at minimum:

- zero expectation mismatches;
- the selected runtime and surface classifications;
- cross-arena rejection and the no-arena-identity counterexample;
- stale-handle invalidation and fresh-generation reuse;
- generation-wrap retirement and arena-id fail-closed policy;
- move-only and explicit shared-owner controls;
- native borrow-boundary compile controls;
- absence of the rejected hidden runtime mechanisms.

Exact final-head workflow and artifact identifiers are recorded in the PR/issue completion evidence rather than embedded here, so this durable decision document does not need a content change merely to rotate CI run numbers.

## Successor boundary

No production arena implementation is authorized until one exact final PR head passes normal multi-platform CI, the dedicated generational-arena research workflow, and all naturally triggered collection/ownership regression gates, then merges and passes the required exact-main postmerge gates.

After #142 closes, the bounded production successor may implement the contextual `arena T` / `handle T` family with explicit insertion, checked lookup, checked removal, runtime arena identity, generation-checked reusable slots, and direct safe Rust lowering. Removal/reuse semantics must remain explicit; general generics, mutable references, independent-lifetime masquerading, hidden clone/refcount, GC, registries, locks, and unsafe identity machinery remain separate work.