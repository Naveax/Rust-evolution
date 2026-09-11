# Lifetime Elision Feasibility v0 Research

Status: **ACCEPTED RESEARCH — REFERENCE-SURFACE-FIRST**

Issue: #100  
PR: #101  
Rust toolchain: **1.98.0**

## Decision summary

Rust 1.98 can omit named lifetime parameters for a useful class of **single-source borrowed-return** signatures. That does not make the returned value owned. The caller still receives a reference tied to an owner, and storing that result creates real borrow-vs-move/reinitialization constraints.

The accepted decision is therefore:

**REFERENCE-SURFACE-FIRST**

Evolution must not silently reinterpret an existing owned return contract such as `fn identity(item Item) Item` as a borrowed return. Before escaping borrowed results can become a production language feature, Evolution needs a caller-visible immutable reference / borrowed-result type distinction. Named lifetime syntax can remain absent for the proven single-source subset.

This research changes no production syntax, accepted-program semantics, ownership behavior, generated Rust, or runtime behavior.

## Accepted evidence

Accepted research code/evidence head before documentation synchronization:

`f65db4a68a93240abe51d26444e70fc568a7ba02`

Validation:

- Lifetime elision research #2 / run `34580003076`: **SUCCESS** on Ubuntu 24.04;
- Rust `1.98.0 (88d9e12ae 2026-08-18)`, LLVM 22.1.8;
- artifact `evo-lifetime-elision-research-ubuntu-24.04`;
- artifact id `10191268492`;
- digest `sha256:40b6d338e7ed74aa7d158f32ac07a956cc2694892d3f945015ef99cf8dfad62f`;
- JSON validation: **PASS**;
- compile-expectation mismatches: **0**;
- `ELISION-CANDIDATE-REQUIRES-REFERENCE-TYPE`: **8** cases;
- `REQUIRES-EXPLICIT-LIFETIME-RELATION`: **2** cases;
- `UNSAFE-OR-UNREPRESENTABLE`: **1** case;
- aggregate verdict: **REFERENCE-SURFACE-FIRST**.

Normal three-OS CI for this exact code/evidence head is CI #408 / run `34580003002`; documentation synchronization is attached only after that run completes successfully.

## What the matrix proves

### No escaping lifetime relation

The following remain ordinary owned results and need no borrowed-result model:

- borrowed nominal input -> scalar field value;
- borrowed nominal input -> computed scalar value.

Both compile as ordinary Rust and carry no returned reference.

### Existing owned semantics stay owned

Two controls deliberately preserve ownership:

- fresh nominal construction returned by value;
- current owned nominal identity `T -> T`.

A reference feature must not retroactively change these contracts. Source compatibility and ownership meaning are part of the language semantics, not formatting trivia.

### Useful single-source elision cases

Rust 1.98 accepts named-lifetime-free signatures for:

- one borrowed nominal input -> borrowed whole-value result;
- one borrowed outer nominal input -> borrowed nested nominal field;
- immediate inspection of a borrowed result;
- a borrowed result stored in a local;
- forwarding a single-source borrowed result through another function;
- recursive forwarding where the same sole owner remains the source.

These cases demonstrate that **named lifetime annotations are not required** for a useful subset.

They also demonstrate that an explicit borrowed/reference result distinction is still required. `&T` and `T` are not interchangeable API contracts.

### Stored-result conflict controls

With a stored borrowed result still live, Rust rejects:

- moving the owner;
- reinitializing the owner.

Lifetime-name elision does not erase borrow checking. Any Evolution reference surface must track enough source-level ownership state to reject the same conflicts without leaking generated-Rust diagnostics.

### Multiple-owner ambiguity

Rust rejects elided returned-reference signatures with two possible borrowed input lifetimes in the tested forms:

- a function that returns the first of two borrowed nominal inputs without an explicit relation;
- a branch that may return a borrow derived from either of two different owners.

These cases require an explicit lifetime relationship or a later deterministic language rule. They are outside the proposed first reference-surface slice.

### Invalid owner source

Returning a reference derived from a fresh temporary/local owner is rejected. There is no valid caller-owned lifetime to attach to the result. Evolution must fail closed here and must never invent `'static` or widen lifetime validity.

## Classification matrix

| Case | Classification | Rust result |
| --- | --- | --- |
| borrowed input -> scalar field | `NO-LIFETIME-RELATION` | compiles |
| borrowed input -> computed scalar | `NO-LIFETIME-RELATION` | compiles |
| fresh nominal owned return | `OWNED-SEMANTICS-MUST-STAY-OWNED` | compiles |
| current owned nominal identity | `OWNED-SEMANTICS-MUST-STAY-OWNED` | compiles |
| single-source borrowed whole return | `ELISION-CANDIDATE-REQUIRES-REFERENCE-TYPE` | compiles |
| single-source borrowed nested field | `ELISION-CANDIDATE-REQUIRES-REFERENCE-TYPE` | compiles |
| immediate borrowed-result inspection | `ELISION-CANDIDATE-REQUIRES-REFERENCE-TYPE` | compiles |
| stored borrowed-result local | `ELISION-CANDIDATE-REQUIRES-REFERENCE-TYPE` | compiles |
| owner move while borrowed result live | `ELISION-CANDIDATE-REQUIRES-REFERENCE-TYPE` | rejected |
| owner reinitialization while borrowed result live | `ELISION-CANDIDATE-REQUIRES-REFERENCE-TYPE` | rejected |
| two borrowed inputs, return first without relation | `REQUIRES-EXPLICIT-LIFETIME-RELATION` | rejected |
| branch chooses between two owners | `REQUIRES-EXPLICIT-LIFETIME-RELATION` | rejected |
| single-source forwarded borrowed return | `ELISION-CANDIDATE-REQUIRES-REFERENCE-TYPE` | compiles |
| single-source recursive borrowed return | `ELISION-CANDIDATE-REQUIRES-REFERENCE-TYPE` | compiles |
| borrowed return from temporary | `UNSAFE-OR-UNREPRESENTABLE` | rejected |

The workflow artifact retains the full per-case metadata and compiler stderr summaries in `report.json`, `report.md`, and `raw-matrix.csv`.

## Required successor boundary

The next design/research slice should establish an **explicit immutable reference surface v0** before attempting escaping-borrow implementation.

That successor should answer, at minimum:

1. how source syntax distinguishes owned `T` from borrowed `&T`-equivalent values;
2. how immutable reference values may be stored in locals;
3. how source-native diagnostics track owner move/reinitialization conflicts;
4. how single-source output lifetimes are elided internally without user-written lifetime names;
5. how current owned APIs remain source- and semantics-compatible;
6. how multiple-source relationships fail closed until an explicit lifetime-relation design exists.

The first reference surface must not add mutable references, generalized lifetime syntax, implicit clone/copy, `Rc`/`Arc`/GC, unsafe lifetime widening, hidden ownership maps, or invented `'static` values.

## Failed-SHA evidence retained

Initial research head:

`6203569451abc1ea448a36eaa47e21fddbe0eed7`

- Lifetime elision research #1 / run `34579682344`: **SUCCESS** with the same `REFERENCE-SURFACE-FIRST` result, zero compile-expectation mismatches, and a retained artifact (`10191069517`, digest `sha256:d97af7e415cfe0d27ce3515531a2aa54dae4feba1f5c685796ed4689d2be0c56`);
- normal CI #407 / run `34579682411` failed only `cargo fmt --all -- --check` on the new research test;
- that SHA was not rerun;
- formatting was corrected on new head `f65db4a68a93240abe51d26444e70fc568a7ba02` without changing the research matrix or decision logic.

## Final research decision

**REFERENCE-SURFACE-FIRST**.

Named lifetime burden can be avoided for useful single-source borrowed-return signatures, but escaping borrowed results cannot safely be hidden behind existing owned type syntax. A first-class immutable borrowed/reference result distinction is the required predecessor to any production borrowed-return feature.
