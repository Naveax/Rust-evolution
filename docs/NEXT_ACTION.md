# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-08-27**

## Active P0

Parent milestone: **#50 — Enums v0: nominal sum types + exhaustive static matching**

Completed:

- semantic umbrella #54
- ownership child #60 / PR #63
  - final PR CI #242 / run `33117703966`: **SUCCESS**
  - squash merge `fc611c0e92a48d15d94373998b3618f154f0d0e5`
  - post-merge main CI #243 / run `33117962228`: **SUCCESS**

Active child:

- **#61 — static Rust enum/match codegen + source maps**
- staging branch: `work/enums-codegen-v0`
- exact base: `fc611c0e92a48d15d94373998b3618f154f0d0e5`
- no feature PR yet; first produce a coherent executable-IR slice

Following child:

- **#62 — differential performance parity + final language spec sync**

## #61 delivery order

Do not jump directly from validated parser/sidecar data into Rust string emission.

1. **Structured executable IR foundation**
   - preserve record-vs-enum nominal type identity explicitly;
   - add enum schemas/variants with source spans to lowering IR;
   - keep structured enum + variant identity; no concatenated magic names;
   - preserve the existing scalar/function/Records executable path exactly;
   - keep the current enum execution gate closed until constructor/match IR is coherent.
2. **Constructor + exhaustive match IR**
   - validated enum constructors become structured expressions;
   - validated matches become structured statements with typed lexical payload bindings;
   - reuse #60 ownership decisions rather than reconstructing ownership in codegen.
3. **Direct static Rust emission + source maps**
   - ordinary Rust `enum`, direct variants and direct `match`;
   - deterministic generated names;
   - enum/variant/match line mappings under the existing source-map contract.
4. **Native correctness + generated Rust inspection**
   - unit/scalar/record payloads;
   - enum parameter/return roundtrip;
   - nested match/control flow;
   - no hidden clone/Box/runtime registry/dispatch.

## First atomic slice

The current executable IR has `ValueType::Record(String)` and `RecordType::Named(String)`. `RecordType::Named` becomes ambiguous once record fields may point to enums: existing Rust codegen assumes every named field type is `__EvoRecord_*`.

First slice should therefore:

- replace bare record-field `Named(String)` with explicit record-vs-enum nominal variants;
- add `ValueType::Enum(String)` for executable signature/value identity;
- add `EnumIr` / `EnumVariantIr` carrying payload type + spans;
- classify named record fields from the already validated record/enum namespace;
- add focused lowering tests proving record and enum nominal identities cannot be confused;
- keep `lower()` fail-closed for enum programs until constructor/match executable IR lands;
- make no Rust enum emission claim yet.

## Engineering constraints

- Enum/variant identity remains structured.
- Do not infer record-vs-enum from generated-name prefixes or a bare string.
- #60 ownership analysis remains authoritative; #61 must not silently invent another move system.
- No implicit clone, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.
- Preserve every accepted Records v0 generated-code/runtime gate.
- `LANGUAGE_SPEC_V0.md` remains unchanged until #62.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent work, but never create multiple active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Do not rerun an old failed SHA merely to obtain a different result or timing sample.
