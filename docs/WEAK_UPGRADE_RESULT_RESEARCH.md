# Checked Weak upgrade result surface v0 research

Status: **RESEARCH IN PROGRESS**

Issue: #138. Production consumer: #137. Accepted Weak predecessor: #125 / PR #130.

Exact verified start main: `0748f7672028429c962880a09471f37ada896dc5`, where #144 generational arena v0 is merged and its natural exact-main CI, arena performance, arena surface research, append-only sequence performance, and explicit shared-owner performance gates are all green.

This track is research-only. It does not add production `weak T`, `downgrade`, or `upgrade` semantics.

## Research question

The existing Weak candidate requires checked recovery of a one-thread `shared T` owner. Rust naturally returns `Option<Rc<T>>`, but Evolution currently has no general generic Option/Result surface and intentionally fails closed on the relevant first-class optional-owner paths.

The research compares the four pre-registered families from #138:

1. immediate scoped success/failure branching;
2. a dedicated first-class maybe-shared value;
3. enum/shared-owner integration;
4. generic Option/Result infrastructure.

## Leading bounded candidate

The executable hypothesis is:

```text
upgrade edge as owner
    print owner.value
else
    print 0
end
```

Properties under study:

- `upgrade`, `as`, `else`, and `end` remain contextual/ordinary tokens outside the exact construct;
- the Weak handle is inspected, not consumed, by a check;
- success creates a new strong `shared T` owner with the same ownership/count semantics as idiomatic `Weak::upgrade`;
- the success binding is branch-local and follows existing `shared T` move/duplication rules;
- failure is explicit and fabricates no payload access;
- no first-class optional owner value escapes the construct;
- no generic type syntax, enum/shared-owner widening, sentinel/null convention, hidden panic, hidden strong duplication, payload clone, GC, registry, or unsafe validity machinery is introduced.

The current parser is expected to reject this spelling because it is not production syntax yet. That rejection is evidence that this branch remains research-only, not a reason to invent backend-only semantics.

## Executable matrix

Pinned Rust 1.98 controls cover at minimum:

- live upgrade with one and multiple strong owners;
- dead upgrade and explicit failure;
- independent lifetime of the newly upgraded owner;
- strong-count restoration after upgraded-owner drop;
- existing move rules for the upgraded owner;
- explicit strong duplication when caller retention is needed;
- no fabricated payload on failed upgrade;
- non-consuming repeated checks of the same Weak handle;
- checked upgrade inside a function;
- first-class Rust `Option<Rc<T>>` across statements as a control, not a source requirement;
- nested control flow;
- payload deep-clone separation;
- lexical reference separation;
- Arc/Weak concurrency separation.

The test also records current source-surface facts: contextual words lex as identifiers, ordinary `upgrade`/related identifier use parses, the scoped candidate is not silently accepted yet, and `maybe shared T`, generic `Option<shared T>`, and enum/shared-owner result paths remain fail-closed.

## Pre-registered decision

The dedicated workflow must emit exactly one #138 outcome. The current hypothesis is `SCOPED-UPGRADE-CANDIDATE` only if:

- all executable matrix expectations match;
- the scoped family covers the required Weak recovery semantics without a first-class result;
- the Weak check is non-consuming;
- current optional/generic/enum alternatives remain unnecessary for the bounded production slice;
- exact-SHA JSON, CSV, and Markdown provenance is produced.

A mismatch keeps #137 blocked and requires evidence-driven revision rather than widening the language surface.
