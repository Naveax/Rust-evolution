# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-08-27**

## Active P0

Parent milestone: **#50 — Enums v0: nominal sum types + exhaustive static matching**

Completed semantic umbrella: **#54 — Enums semantics: nominal typing, constructors and exhaustive match**

Completed semantic slices on `main`:

- **PR #55 — nominal declaration validation**
  - squash merge `ca9641d6c7c57ab603cda8b6a4a091f50cfd625d`
  - post-merge main CI #221 / run `33098011627`: **SUCCESS**
- **#56 / PR #58 — resolved variants and constructor typing**
  - squash merge `875b4d8fc255d6699b4f89b5e5c769d8dd34383b`
  - post-merge main CI #230 / run `33104226016`: **SUCCESS**
- **#57 / PR #59 — exhaustive match typing and arm scopes**
  - final PR head `8bb8e7b56f92910c64943d470455e5820bbca346`
  - final PR CI #235 / run `33113751852`: **SUCCESS**
  - squash merge `e698b7f094863f017b3a29ad0210b638b6bd6a3f`
  - post-merge main CI #236 / run `33113950293`: **SUCCESS**

Active child:

- **#60 — Enums v0 ownership + match payload extraction**
- staging branch: `work/enums-ownership-v0`
- staging base: exact #59 squash merge `e698b7f094863f017b3a29ad0210b638b6bd6a3f`
- no feature PR exists yet; create it after the first coherent ownership slice is code-complete

Following children:

1. **#61 — static Rust enum/match codegen + source maps**
2. **#62 — differential performance parity + final language spec sync**

## Verified stable baseline

Current `main` baseline for #60:

`e698b7f094863f017b3a29ad0210b638b6bd6a3f`

Post-merge main CI #236 is authoritative for the completed semantic layer. Ubuntu passed format, Clippy, workspace tests, benchmark smoke, runtime repeat, control-flow, logical operators, Functions v0, Block Locals v0, Records v0 and release build. Windows/macOS passed format, Clippy, workspace tests, benchmark smoke and release build.

#57 and semantic umbrella #54 are closed completed from this merged-main evidence. Parent #50 remains open for ownership, codegen, native correctness, source maps, performance and final spec sync.

## Proven semantic substrate available to #60

The merged semantic layer provides:

- resolved enum schemas with structured enum/variant identity and source spans;
- `ResolvedPayloadType` covering int/bool/string, record nominal types and enum nominal types;
- ownership-free static expression typing through locals, function parameters/returns, record constructors/fields and enum constructors;
- exhaustive match validation;
- typed lexical payload bindings;
- a non-executable `MatchEnvironment` sidecar retaining structured arm identity, typed payload bindings, spans and exhaustive-only `all_arms_return`;
- explicit fail-closed execution before enum ownership/Rust codegen.

## #60 ownership design boundary

Do **not** force enum ownership directly into executable `ValueType` / Records `SemanticType` as the first step. The production lowerer still intentionally rejects enum execution before the existing Records `Analyzer`, and Records diagnostics/runtime behavior are already accepted evidence.

### Slice 1 — reusable move-state core, zero behavior change

Extract the availability/reinitialization/join mechanics currently embedded in Records `MoveTracker` into a diagnostics-free generic move-state core.

Requirements:

- exact type identity retained for reinitialization checks;
- available/moved state retained per lexical binding;
- generic consume / inspect / reinitialize operations;
- two-way continuing-branch join semantics preserved;
- repeat fixed-point safety preserved;
- add a multi-arm continuing-branch merge primitive suitable for exhaustive match;
- Records `MoveTracker` remains a wrapper and preserves existing public behavior and **existing diagnostic wording**;
- every existing Records ownership unit/process test remains unchanged and green.

### Slice 2 — enum pre-codegen ownership pass

Add a separate enum ownership validator under the existing `enums_impl` pre-codegen semantic path.

It should reuse the proven static type environment and match sidecar rather than invent a second language type system.

Conservative v0 rule:

- nominal enum values are move-only regardless of scalar/static payload; no implicit Copy inference;
- record and enum payload bindings are move-only, scalars remain trivially reusable;
- by-value reads/arguments/returns/constructor payloads consume move-only values;
- matching an owned enum consumes the whole scrutinee;
- payload binding ownership follows the declared payload type;
- whole-enum reuse after consuming match is rejected source-natively;
- same-type explicit reinitialization restores availability;
- no partial reuse of a matched enum is inferred;
- no implicit clone/boxing/borrow/reference behavior.

For ownership joins, extend the match sidecar with **per-arm terminal/continuing information** if needed. `all_arms_return` alone is sufficient for function return-path summary but not sufficient to merge move state across a mix of terminal and continuing arms.

## Resume here

1. Work only on `work/enums-ownership-v0` until the first coherent #60 code slice is ready.
2. Implement the generic diagnostics-free move-state core and adapt Records `MoveTracker` as a compatibility wrapper without changing Records behavior or error messages.
3. Add focused core tests for consume/reinitialize, two-branch joins, repeat safety and N-way continuing-arm merge.
4. Inspect the diff against `e698b7f0...`; first slice should be ownership infrastructure/tests only, not enum codegen.
5. Create `feature/enums-ownership-v0` from the staging head and open a draft PR for #60 only after that first slice is coherent.
6. Before moving any head SHA, confirm no active Action exists for the target SHA. Never create duplicate active Actions for the same SHA/workflow/input.
7. While CI runs, continue Slice 2 on staging: enum pre-codegen ownership traversal, enum reuse-after-move, reinitialization, payload extraction ownership and match joins.
8. If a run fails, fix the actual issue on a new SHA. Do not rerun the old failed SHA merely for a different result.
9. Keep #61 executable enum/match IR and Rust codegen out of #60.
10. Keep #62 benchmark and `LANGUAGE_SPEC_V0.md` synchronization out of #60/#61 until native behavior is proven.

## Engineering constraints

Enum and variant identity must remain structured. Do not encode `Enum.Variant` as a concatenated magic name.

Do not infer arbitrary method-call semantics from `object.member(...)`.

Do not add hidden `.clone()`, boxing, GC/RC, runtime variant maps, reflection metadata or dynamic dispatch.

Do not alter accepted Records ownership diagnostics merely to generalize internal machinery. Shared state mechanics may be extracted; user-visible Records behavior is a regression boundary.

`LANGUAGE_SPEC_V0.md` remains intentionally unsynchronized with executable Enums behavior until #62 because ownership/codegen/performance are not proven yet.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent work, but never create multiple active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Do not rerun an old failed SHA merely to obtain a different result or timing sample.
