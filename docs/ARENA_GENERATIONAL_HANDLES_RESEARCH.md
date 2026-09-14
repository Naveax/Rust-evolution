# Arena / generational graph handles v0 research

Status: **RESEARCH IN PROGRESS**

Parent: #126

This track isolates container-owned graph identity from reference-counted ownership. It does not add Evolution production collection syntax, graph runtime machinery, or unsafe pointer semantics.

## Research model

The executable Rust 1.98 matrix compares:

- direct owned trees where an arena would be needless overhead;
- copied plain indices for DAG/cyclic traversal;
- checked stale-index failure after removal;
- the silent identity-rebinding risk of plain slot reuse;
- `(arena, index, generation)` handles that reject stale and cross-arena identities;
- fresh-generation resolution after slot reuse;
- copyable handles with no per-edge refcount operation;
- deterministic payload destruction by container ownership;
- explicit removal/invalidation;
- traversal parity against an equivalent `Rc` control;
- container-owned mutation without hidden `RefCell` or locking;
- payload copy versus handle copy;
- independent node lifetime as a case for explicit shared ownership instead.

## Current bounded conclusion

The semantic model under test is a **GENERATIONAL-HANDLE-CANDIDATE**, but production remains gated by **REQUIRES-COLLECTION-SURFACE**. Evolution does not yet have enough general container/collection surface to expose an arena honestly without inventing hidden runtime storage.

Plain `usize`-like indices are acceptable only where removal/reuse cannot silently change identity. Once slots can be reused, generation checking or an equivalent explicit stale-identity policy is required.

## Safety / cost invariants

Any later production candidate must keep explicit:

- who owns node storage;
- whether removal exists;
- generation increment and wraparound policy;
- cross-arena identity policy;
- checked lookup failure;
- handle copy versus payload clone;
- container mutation versus interior mutability;
- independent node lifetime versus arena-tied lifetime.

No hidden per-edge refcounting, global handle table, tracing GC, lock, `RefCell`, unsafe pointer graph, silent cross-arena acceptance, or fabricated stale-target validity is permitted.

## Evidence gate

The dedicated workflow pins Rust 1.98.0, requires rustfmt-clean source, runs the exact ignored research matrix, emits exact-SHA JSON/Markdown evidence, and requires zero expectation mismatches.

Final exact head, artifact digest, observed classification counts, and any narrower successor are recorded only after exact-head normal CI plus dedicated research succeed.
