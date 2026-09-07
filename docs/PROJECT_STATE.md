# Rust Evolution — Project State

Last verified update: **2026-09-07**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Current authoritative `main`: `fc611c0e92a48d15d94373998b3618f154f0d0e5`
- Post-merge main CI #243 / run `33117962228`: **SUCCESS**

Records v0 remains the accepted ZERO-cost nominal product-type baseline: static Rust structs/field access, by-value move tracking, no hidden allocation/boxing/GC/RC/clone/dynamic dispatch/runtime metadata, with its differential performance gate preserved.

## Enums v0 milestone — #50

Completed and merged before #61:

- parser/formatter child #51;
- semantic umbrella #54, including constructor and exhaustive match typing;
- ownership child #60 / PR #63:
  - squash merge `fc611c0e92a48d15d94373998b3618f154f0d0e5`;
  - post-merge main CI #243 / run `33117962228`: **SUCCESS**.

#60 remains the ownership authority for Enums v0. Enum/record payloads and enum values use explicit by-value move semantics; scalar payloads remain reusable; exhaustive owned match consumes the scrutinee; branch/repeat merges are conservative; no implicit clone or partial nominal field move is invented.

## Active child — #61 static enum/match codegen

- Issue: **#61 — P0 enums codegen: static Rust enum/match lowering and source maps**
- PR: **#64 — `feat: promote Enums v0 into executable IR`**
- Feature branch: `feature/enums-codegen-v0`
- Staging branch: `work/enums-codegen-v0`
- Base on `main`: `fc611c0e92a48d15d94373998b3618f154f0d0e5`
- PR remains draft until final acceptance/docs head is green.

### Proven implementation layers

1. **Executable structured IR promotion**
   - validated schemas, constructors, exhaustive matches, typed arm bindings and #60 ownership decisions are promoted into `evo-lowering::Program`;
   - structured executable nominal types distinguish `Record` from `Enum`;
   - enum/variant semantic identity remains structured;
   - CI #260: **SUCCESS**.

2. **Borrowed codegen view**
   - codegen consumes a borrowed read-only view of private executable enum IR;
   - no semantic re-resolution or compiler-side clone;
   - #261/#262 exposed only formatting/direct-harness lint issues; corrected head passed CI #263 / run `34094640786`: **SUCCESS**.

3. **Static Rust enum/match emitter**
   - deterministic `__EvoEnum_*` / `__EvoVariant_*` names;
   - ordinary static Rust enum definitions;
   - builtin, record and enum payloads lower to ordinary Rust types;
   - direct variant construction and exhaustive Rust `match`;
   - lexical payload pattern bindings;
   - line-level mappings for enum definitions, variants, statements, matches and arms;
   - inspection tests reject hidden `.clone()`, `Box`, `Rc`, `Arc` and runtime maps;
   - corrected emitter head passed CI #265 / run `34095123460`: **SUCCESS**.

4. **Strict enum-program static semantics**
   - enum-enabled programs do not rely on rustc to discover frontend type errors that the legacy scalar/Records analyzer would normally catch;
   - explicit validation covers function names/signatures/call arity/types, return types/terminal paths, record construction/fields, assignment types, operator operands, `if`/`repeat` operand types and unsupported whole nominal print/equality;
   - #266 failed only an unreachable parser-invalid test case; corrected head passed CI #267 / run `34096854858`: **SUCCESS**.

5. **Production dispatch and first native execution**
   - legacy scalar/Records programs retain the existing legacy Rust emitter;
   - enum-enabled `Program` values dispatch to the static enum emitter;
   - CLI `check`, `emit-rust`, `build` and `run` use the same production path;
   - #268 exposed one legacy-emitter dead-code Clippy warning after dispatch; corrected head `cc10319f61ef51abe843039671152b6d176c5e29` passed CI #269 / run `34097330115`: **SUCCESS**;
   - #269 included native enum CLI execution and preserved all existing Ubuntu runtime/performance gates.

### Current acceptance head

Feature branch currently targets:

- `5fa923227962c89d23a68deee4d990b237e3c325`
- CI #271 / run `34097977337` is authoritative for the expanded native correctness and public source-map acceptance corpus.

The preceding head `c0becf2e8d358aa3315816e0990f0a6ca6c708eb` failed CI #270 only at `cargo fmt --check`; #270 was not rerun. `5fa923227...` contains the rustfmt corrections only.

Expanded acceptance tests cover:

- unit-only enum and matching every unit variant;
- scalar payload enum native execution;
- acyclic record payload extraction;
- enum parameter/return roundtrip;
- payload binding use inside an arm;
- nested `if` + nested `match` behavior;
- invalid non-exhaustive match build fails before rustc and produces no binary;
- public generated-code/source-map checks for enum declaration, variant, constructor-owning statement, match and arm lines;
- direct builtin/record/enum payload Rust types and no hidden clone/boxing/runtime maps.

Staging is intentionally ahead of the feature branch with:

- `be4dc4f09259b93d8ed85705bcecff99b3c5dbb1` — native `repeat` + enum construction + exhaustive match composition;
- refreshed `docs/NEXT_ACTION.md`;
- this project-state update.

Do not fast-forward the feature branch to staging while CI #271 is active.

## #61 completion sequence

1. Resolve CI #271 on exact SHA `5fa923227962c89d23a68deee4d990b237e3c325` without rerunning it.
2. If green, fast-forward PR #64 branch to the current staging head with `force=false`.
3. Let the docs/repeat-synchronized head receive exactly one new CI run.
4. Only after that final CI is green:
   - update #61 checklist from actual evidence;
   - update PR #64 body with final SHA/run evidence;
   - mark PR ready and confirm mergeability/head SHA;
   - squash-merge with expected head SHA.
5. Verify post-merge `main` push CI before closing #61 or advancing #50.
6. Start #62 from the actual #64 squash merge SHA only after main CI is green.

## Remaining Enums v0 queue

1. Finish **#61** acceptance, merge and post-merge verification.
2. **#62 — differential performance parity + final language spec sync**.

#62, not #61, owns the dedicated Enums differential benchmark and final `LANGUAGE_SPEC_V0.md` synchronization.

## ZERO-cost boundary

Enums v0 remains a ZERO-cost-class target: ordinary static Rust enum values and matches, with no hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.

Do not add generics, guards, wildcard/or/arbitrary nested patterns, references/borrow inference, methods, derives or runtime reflection as collateral work.

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
