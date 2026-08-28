# Rust Evolution — Project State

Last verified update: **2026-08-28**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Current authoritative `main`: `fc611c0e92a48d15d94373998b3618f154f0d0e5`
- Post-merge main CI #243 / run `33117962228`: **SUCCESS**

Records v0 remains the accepted ZERO-cost nominal product-type baseline: static Rust structs/field access, by-value move tracking, no hidden allocation/boxing/GC/RC/clone/dynamic dispatch/runtime metadata, with its differential performance gate preserved.

## Enums v0 milestone — #50

Parent: **#50 — Enums v0: nominal sum types + exhaustive static matching**

Completed and merged:

- parser/formatter child #51; post-merge main CI #216: **SUCCESS**
- semantic umbrella #54:
  - PR #55 nominal declarations; post-merge main CI #221: **SUCCESS**
  - #56 / PR #58 resolved variants + constructor typing; post-merge main CI #230: **SUCCESS**
  - #57 / PR #59 exhaustive match typing + arm scopes; squash merge `e698b7f094863f017b3a29ad0210b638b6bd6a3f`; post-merge main CI #236 / run `33113950293`: **SUCCESS**
- ownership child #60 / PR #63:
  - final PR head `19a17cd9a9fe62baa0bb4e1adae4ae8bfe8bff4a`
  - final PR CI #242 / run `33117703966`: **SUCCESS**
  - squash merge `fc611c0e92a48d15d94373998b3618f154f0d0e5`
  - post-merge main CI #243 / run `33117962228`: **SUCCESS**

Semantic umbrella #54 and ownership child #60 are closed completed.

## Proven ownership behavior on `main`

Enums v0 ownership now has merged-main evidence for:

- nominal enum values are move-only regardless of scalar/static payload;
- enum/record payload bindings are move-only while int/bool/string remain reusable;
- by-value local reads, function arguments/returns and constructor payload uses consume move-only values;
- exact same-type reinitialization restores availability;
- owned exhaustive match consumes the whole scrutinee;
- every arm begins from the same post-scrutinee-consumption ownership state;
- continuing `if` / `match` exits merge conservatively while terminal branches/arms do not poison continuation;
- repeat later-iteration safety and conservative zero-iteration behavior are preserved;
- non-reusable nominal field move-out is explicitly rejected instead of inventing partial-move or implicit-clone semantics;
- Records v0 ownership diagnostics/runtime behavior remain unchanged.

CI #243 preserved every existing Ubuntu runtime/performance gate including Records v0. Windows/macOS preserved format, Clippy, workspace tests, benchmark smoke and release build.

## Active child — #61 executable enum IR + Rust codegen + source maps

Issue: **#61 — Enums v0 static Rust enum/match codegen**

Staging:

- branch: `work/enums-codegen-v0`
- exact base: ownership squash merge `fc611c0e92a48d15d94373998b3618f154f0d0e5`
- first staging handoff commit: `f2cfbb80652cc260efb2dfc033563e5f69fe2a9f`
- no feature PR yet; create one only after the first coherent IR/codegen slice is proven enough to review

### #61 delivery shape

Do not jump from validated parser/semantic sidecars directly into Rust string emission. Deliver #61 in atomic layers:

1. **Executable structured IR promotion**
   - promote validated enum schemas with source spans into lowering IR;
   - preserve structured enum/variant identity;
   - make executable nominal types distinguish records from enums without generated-name guessing;
   - preserve accepted #60 ownership decisions instead of re-running a competing ownership model;
   - keep unsupported executable enum codegen fail-closed while IR structure is introduced.
2. **Constructor + exhaustive match IR**
   - validated constructors retain enum/variant identity and payload expression;
   - match IR retains validated enum/variant identity, arm spans and typed lexical payload bindings;
   - return-path/control-flow structure remains explicit.
3. **Static Rust emission + source maps**
   - emit ordinary Rust `enum`, direct variant constructors and direct exhaustive `match`;
   - deterministic generated identifiers;
   - source mapping for enum declarations/variants/match structural lines/arms under the existing line mapper.
4. **Native correctness / generated-code inspection**
   - unit/scalar/record payloads, enum parameter/return roundtrip and nested control flow;
   - generated Rust inspection proving no clone/box/runtime dispatch;
   - preserve every existing Records/runtime gate.

### First IR slice boundary

The first code slice should be deliberately smaller than executable enum lowering:

- introduce `EnumIr` / `EnumVariantIr` with spans;
- expose enum payload types as a structured executable value type;
- distinguish record nominal references from enum nominal references in record/enum schema IR;
- derive these types from the already-proven `ResolvedPayloadType::{Record, Enum}` semantic environment, not from spelling heuristics;
- leave `ExprKind::EnumConstruct`, `StmtKind::Match` executable lowering and Rust emission for the following slice;
- keep existing Records/scalar/function output unchanged.

The current `RecordType::Named(String)` is not sufficient once executable record fields may contain enum values. Codegen must never infer `__EvoRecord_*` vs `__EvoEnum_*` from a bare name string.

### Gate separation requirement

`record_environment::reject_enum_declarations()` currently combines two responsibilities:

1. run enum pre-codegen semantic + ownership validation;
2. reject enum execution because executable lowering/codegen is not implemented.

#61 should separate semantic validation from execution gating. This allows structured enum IR to be introduced without accidentally removing the fail-closed boundary before constructor/match emission is ready.

### Ownership authority

Do not bolt enum ownership into the existing Records executable `Analyzer` as a second model. #60's pre-codegen ownership validator is authoritative. #61 executable lowering consumes validated syntax/type identity and produces IR; it must not reconstruct move decisions independently.

## Remaining Enums v0 queue

1. **#61 — static Rust enum/match executable IR + codegen + source maps + native correctness**
2. **#62 — differential performance parity + final spec sync**

#62 owns the runtime-dependent Enums differential benchmark, #4/#5 performance evidence and final `LANGUAGE_SPEC_V0.md` synchronization.

## Parent #50 state

Completed and merged on `main`:

- syntax/parser/formatter surface;
- nominal/type semantics;
- constructor semantics;
- exhaustive static match semantics;
- explicit by-value enum ownership and match payload extraction.

Still open:

- executable structured enum/match lowering and direct static Rust emission (#61);
- executable source maps and native correctness corpus (#61);
- dedicated Enums performance parity evidence (#62);
- final stable language specification synchronization (#62).

## Deliberate boundary

Enums v0 remains a **ZERO** cost-class target: ordinary static Rust enums/matches, never hidden allocation, boxing, clone, dispatch or runtime metadata.

Do not add generics, guards, wildcard/or/nested arbitrary patterns, references/borrow inference, methods, derives or runtime reflection as collateral work.

`LANGUAGE_SPEC_V0.md` intentionally remains behind executable Enums work until #62 because codegen/native/performance acceptance is not complete.

## Durable continuation infrastructure

Read order:

1. `AGENTS.md`
2. `docs/PROJECT_STATE.md`
3. `docs/NEXT_ACTION.md`
4. `docs/LANGUAGE_SPEC_V0.md`
5. active issue/PR/Actions referenced there

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.

## Handoff invariant

Every significant merge or incomplete stopping point must keep `PROJECT_STATE.md`, `NEXT_ACTION.md`, issue/PR evidence and durable decisions synchronized with GitHub reality.

The repository is the project memory. The chat transcript is not.
