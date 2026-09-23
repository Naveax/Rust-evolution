# Weak storage surface v0 research

Acceptance rule: this document describes a research candidate only. It becomes an accepted decision only after the executable matrix, exact-head CI, dedicated pinned-Rust workflow, exact-SHA artifact provenance, merge, and natural exact-main validation are all green and recorded on issue #153.

Parent production slice: #137 / PR #155.

Exact research start main: `36022b386782775e449ae6a6eedcbbb8c8d1275f`.

The production predecessor already provides:

~~~text
weak Item
downgrade owner
upgrade edge as live
    ...
else
    ...
end
~~~

Production v0 intentionally rejects Weak record fields, enum payloads, sequence elements, arena payloads, nested storage forms, and enum-bearing Weak programs.

## Research question

What is the smallest source storage surface that makes one-thread Weak handles useful for graph edges without opening general nested-type syntax, hidden strong ownership, or accidental unification with arena handles?

The pre-registered leading result is:

**WEAK-RECORD-FIELD-CANDIDATE**

## Candidate first slice

~~~text
record Node
    next weak Node
end
~~~

The intended direct Rust representation is a fixed-size standard-library Weak handle field:

~~~rust
struct Node {
    next: std::rc::Weak<Node>,
}
~~~

This is not recursive by-value storage. `Weak<Node>` is a fixed-size handle. The research explicitly contrasts it with an illegal Rust shape such as `struct Node { next: Node }`.

## Why record field first

A record field can move a Weak handle into and with the containing record without fabricating a strong owner. Dropping the field does not drop a live payload, and moving the record does not require `Weak::clone`.

The first-slice contract therefore remains compatible with the production Weak model:

- Weak handles are move-only unless a separate explicit Weak-duplication operation is accepted later;
- a stored edge can be moved as part of ordinary record ownership;
- upgrading the edge remains checked;
- no strong count increment is hidden by storage.

## Sequence storage is a separate boundary

Rust itself can store `Vec<Weak<T>>` directly, and the matrix measures that behavior.

However, the current Evolution append-only sequence contract binds move-only elements through an immutable reference during checked lookup. For a Weak element that means the source-language success binding would naturally correspond to `&Weak<T>`.

Production Weak v0 currently accepts checked upgrade from a Weak local, not from a borrowed-Weak source category. Moving `edges[index]` out is also invalid in Rust without removal or explicit duplication.

Therefore `seq weak T` is not selected in this first research result. Supporting it honestly requires at least one separately justified decision:

- allow `upgrade` to inspect a borrowed Weak handle; or
- add an explicit Weak duplication operation; or
- add a sequence removal/move-out operation.

The research does not silently choose among those.

## Explicit Weak duplication boundary

Rust `Weak::clone` increments weak bookkeeping but not the strong count. That is a real operation and therefore cannot be hidden by ordinary source assignment or sequence lookup.

The matrix includes an explicit control showing:

- Weak clone is distinct from move;
- strong count remains unchanged;
- any future source duplication spelling must be explicit and separately accepted.

This research does not select that spelling.

## Arena identity remains distinct

`weak T` and `handle T` solve different identity problems:

- Weak is an ownership-sensitive non-owning edge to an `Rc<T>` allocation;
- arena handles identify an arena id + slot + generation.

The matrix requires them to remain distinct Rust types and does not authorize conversion or identity unification.

## Enum integration remains separate

Rust can technically place `Weak<T>` in an enum variant. That does not justify widening Evolution enum payload rules.

Current shared-owner and Weak enum payload restrictions already fail closed. This research records enum feasibility only as **ENUM-INTEGRATION-BOUNDARY** evidence and keeps enum Weak payloads unauthorized.

## Recursive-layout rule

The research distinguishes:

~~~rust
struct Bad {
    next: Bad,
}
~~~

from:

~~~rust
struct Node {
    next: std::rc::Weak<Node>,
}
~~~

The first is recursively sized and rejected by Rust. The second is a fixed-size handle and compiles.

A future production Weak record-field implementation must preserve this distinction. It must not generalize recursive by-value records merely because `weak Node` is permitted.

## Hidden-cost controls

The executable matrix requires that:

- storing a Weak field does not increment strong count;
- moving a Weak field does not call `Clone`;
- Weak storage does not deep-clone payloads;
- dropping a Weak field does not drop a live payload;
- `std::sync::Weak` remains a distinct cross-thread model;
- no Rc/Arc conversion is implied;
- no GC, registry, unsafe validity layer, or payload boxing is introduced solely to make Weak storage work.

## Source compatibility probes

The research requires:

- `weak`, `downgrade`, and `upgrade` to remain contextual identifiers;
- ordinary record/type identifiers named `weak` to keep parsing;
- current production parser to reject Weak record fields explicitly;
- current production parser to reject `seq weak T` explicitly;
- current production parser to reject Weak enum payloads explicitly;
- current production parser to reject Weak arena payloads explicitly;
- ordinary local/function Weak production syntax to remain valid.

The research branch must not accidentally make candidate storage syntax production-valid.

## Executable Rust 1.98 matrix

The matrix covers 20 controls:

1. record Weak field upgrades while a strong owner lives;
2. record Weak field fails after final strong owner dies;
3. explicit Weak back-edge avoids a strong cycle;
4. dropping Weak field does not drop live payload;
5. moving a record containing a Weak field moves the handle;
6. Weak field storage does not increment strong count;
7. moving a Weak field does not clone it;
8. explicit Weak clone is separate bookkeeping;
9. vector stores moved Weak handle without strong duplication;
10. vector checked access yields borrowed Weak handle;
11. indexed vector element cannot be moved out directly;
12. borrowed vector Weak can upgrade without moving the element;
13. arena handle identity remains distinct;
14. Weak does not convert to arena handle;
15. enum can technically carry Weak but stays a separate source boundary;
16. recursive by-value record is rejected;
17. recursive record through Weak handle is fixed-size;
18. Weak field does not deep-clone payload;
19. Arc Weak remains a separate cross-thread model;
20. Rc Weak does not convert to Arc Weak.

## Pre-registered decision

The dedicated workflow derives **WEAK-RECORD-FIELD-CANDIDATE** only if:

- all compile/runtime expectations match;
- record-field controls cover live/dead upgrade, move/drop, recursive layout, and strong-count behavior;
- sequence controls demonstrate Rust feasibility while preserving the source-level borrowed-Weak boundary;
- explicit Weak duplication remains separate and source-visible;
- arena identity remains separate;
- enum integration remains unauthorized;
- recursive by-value records remain rejected;
- no hidden clone/strong-owner traffic is observed;
- source compatibility and current fail-closed probes pass;
- exact-SHA JSON/CSV/Markdown provenance is produced.

Any unexplained mismatch yields **DEFER**.

## Hard boundaries

This research does not authorize:

- production Weak record fields by itself;
- `seq weak T`;
- Weak enum payloads;
- Weak arena payloads;
- implicit Weak duplication;
- general nested/generic type syntax;
- general record field move-out machinery;
- unification of Weak and arena handles;
- Arc/`std::sync::Weak`;
- hidden Rc clone or strong-count increment;
- implicit payload clone;
- GC/cycle collection;
- unsafe pointer validity emulation;
- process-global ownership registries;
- general boxing solely to make recursive layout compile.

A production storage implementation must be a separate issue after exact-main research acceptance.
