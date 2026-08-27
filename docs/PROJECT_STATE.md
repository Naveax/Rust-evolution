# Rust Evolution — Project State

Last verified update: **2026-08-27**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Current `main` head before PR #48 merge: `d90278e5134f3fdb1765e76fb8fdd78e6feda615` (`feat: validate nominal record semantics (#44)`).
- Records parser/tooling landed in #45 (`529a9c84b326400409582340cb70ef5780332839`).
- Lexical block locals v0 landed in #39 (`272d3a36a6236370eb4abbd0b7325f67f231de41`).
- Active Records implementation PR: **#48**, branch `feature/records-typed-lowering-v0`.
- Latest validated PR #48 head: `97dae87c42108571e6dfd0c0f87b0010bded97e9`.
- Authoritative PR-head validation: CI **#188**, run **33069876396**, **SUCCESS** on Ubuntu/Windows/macOS.
- PR #48 is ready for review and mergeable. An automated merge attempt from the assistant was blocked by the product safety layer; do not interpret that as a repository/CI failure.

## Durable continuation infrastructure

GitHub is the durable source of truth. Chat memory is optional context.

Read order:

1. `AGENTS.md`
2. `docs/PROJECT_STATE.md`
3. `docs/NEXT_ACTION.md`
4. active issue/PR/Actions referenced there

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.

## Current implemented language surface

Implemented on `main` before PR #48:

- UTF-8 source and source spans;
- integer (`i64`), boolean and literal/static string values;
- bindings, inferred mutability and lexical block locals;
- arithmetic, comparisons and strict short-circuit boolean operators;
- `print`, `input_int`, `repeat`, `if / else / end`;
- typed named functions with static calls, forward calls and direct recursion;
- source-native lexer/parser/lowering diagnostics and bounded recovery;
- deterministic formatter;
- generated-Rust source mappings and rustc diagnostic remapping;
- native `check`, `emit-rust`, `build`, `run`, `fmt` workflows;
- differential correctness/performance harness and existing runtime gates;
- Records v0 parser/formatter surface and semantic declaration foundation.

Implemented and validated on PR #48:

- lowered nominal record schema IR retaining declaration/field order and spans;
- production `ValueType::Record(name)` identity;
- record types in function parameters and return values;
- exact named constructors with deterministic schema-order lowering;
- zero-field `Name()` record constructors;
- typed scalar/chained field access;
- by-value record move semantics with reuse-after-move diagnostics;
- same-type explicit reinitialization of moved record locals;
- conservative `if` ownership joins;
- `repeat` loop-carried move safety;
- explicit rejection of whole-record print/equality and record-valued partial field moves;
- static Rust record structs, struct literals, direct field access and nominal record signatures;
- valid Records v0 programs through real CLI `check`, `emit-rust`, native `build` and execution;
- source-native invalid-schema and moved-record diagnostics before rustc.

The native PR #48 process corpus builds and runs a record program with output `42`.

## Records v0 cost and safety contract

Records v0 is a **ZERO** cost-class feature.

The landed path adds no hidden:

- heap allocation solely for records;
- `Box`, `Rc`, `Arc`, GC or managed runtime;
- `.clone()` insertion;
- dynamic dispatch or trait-object object model;
- runtime field maps / `HashMap` object representation;
- reflection/type metadata.

Recursive by-value record layouts are rejected rather than implicitly boxed.

## Active P0

Parent issue: **#41 — Records v0: typed product data**

Completed typed-lowering/ownership child: **#46** via PR **#48** once merged.

Remaining acceptance child: **#47 — Records v0 zero-cost Rust codegen / differential parity**.

PR #48 already lands the static Rust codegen and first native process slice from #47, but #47 remains open because final Records v0 evidence is not complete.

### Remaining #47 work

1. Add record-specific generated-source mapping regressions and map record declaration/field lines where meaningful.
2. Expand the real CLI/native process corpus for nested records, chained access, zero-field construction, record return roundtrips and explicit reinitialization.
3. Add a runtime-dependent Records v0 differential benchmark that cannot be optimized into a constant-only trivial case.
4. Retain generated Rust evidence and compare normalized LLVM, binary size and exact executable equality.
5. Enforce an Ubuntu Records v0 performance gate under the existing hard performance contract.
6. Only after mapping + benchmark evidence is green, update `docs/LANGUAGE_SPEC_V0.md` with accepted Records v0 semantics and limitations.
7. Close #47 and parent #41 only after final evidence and post-merge validation.

The exact continuation sequence is maintained in `docs/NEXT_ACTION.md`.

## Performance contract

Core/native zero-cost work remains governed by:

`T_evolution <= T_reference_rust`

Correctness comes first. Exact executable equality may establish deterministic runtime parity while still retaining raw timing evidence.

Current Ubuntu CI runtime gates cover:

- runtime input / repeat / reassignment;
- control-flow branches;
- logical operators;
- functions v0;
- block locals v0.

CI #188 confirms all existing gates remain green on the current Records v0 implementation head. A dedicated Records v0 gate is still required by #47 before parent #41 is accepted.

## Explicit Records v0 limitations

Current v0 deliberately does not provide:

- whole-record `print`;
- whole-record equality;
- partial move of a record-valued field;
- implicit borrow/reference inference;
- recursive by-value self-referential records;
- methods/impl blocks, classes/inheritance, reflection, enums/pattern matching or default fields.

Unsupported operations fail closed rather than inserting hidden runtime machinery.

## Handoff invariant

Every significant merge or incomplete stopping point must keep `PROJECT_STATE.md`, `NEXT_ACTION.md`, issue/PR evidence and durable decisions synchronized with GitHub reality.

The repository is the project memory. The chat transcript is not.