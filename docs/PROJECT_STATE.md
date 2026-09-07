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
- PR remains draft until the final docs-synchronized head is green.

### Proven implementation layers

1. **Executable structured IR promotion**
   - validated schemas, constructors, exhaustive matches, typed arm bindings and #60 ownership decisions are promoted into `evo-lowering::Program`;
   - structured executable nominal types distinguish `Record` from `Enum`;
   - enum/variant semantic identity remains structured.

2. **Borrowed codegen view**
   - codegen consumes a borrowed read-only view of private executable enum IR;
   - no semantic re-resolution or compiler-side clone.

3. **Static Rust enum/match emitter**
   - deterministic `__EvoEnum_*` / `__EvoVariant_*` names;
   - ordinary static Rust enum definitions before functions/main;
   - builtin, record and enum payloads lower to ordinary Rust types;
   - direct variant construction and exhaustive Rust `match`;
   - lexical payload pattern bindings;
   - line-level mappings for enum definitions, variants, constructor-owning statements, matches and arms;
   - generated-code inspection rejects `.clone()`, `Box`, `Rc`, `Arc`, runtime maps, dynamic dispatch and reflection-style metadata.

4. **Strict enum-program frontend semantics**
   - enum-enabled programs do not rely on rustc to discover frontend type errors that the legacy scalar/Records analyzer would normally catch;
   - validation covers function names/signatures/call arity/types, return types/terminal paths, record construction/fields, assignment types, operator operands, `if`/`repeat` operand types and unsupported whole nominal print/equality.

5. **Production dispatch and native execution**
   - legacy scalar/Records programs retain the existing emitter;
   - enum-enabled `Program` values dispatch to the static enum emitter;
   - CLI `check`, `emit-rust`, `build` and `run` use the same production path.

### Final implementation/acceptance CI evidence

- CI #267 / run `34096854858`: **SUCCESS** after correcting an unreachable parser-invalid test from #266; #266 was not rerun.
- CI #268 failed only a Clippy dead-code warning after production dispatch; fixed on a new SHA and not rerun.
- corrected production head `cc10319f61ef51abe843039671152b6d176c5e29` passed CI #269 / run `34097330115`: **SUCCESS**.
- acceptance head `c0becf2e8d358aa3315816e0990f0a6ca6c708eb` failed CI #270 only at rustfmt; #270 was not rerun.
- rustfmt-corrected head `5fa923227962c89d23a68deee4d990b237e3c325` passed CI #271 / run `34097977337`: **SUCCESS** on Ubuntu/Windows/macOS; Ubuntu preserved every existing runtime/performance gate.
- final repeat/ZERO-cost/docs-pre-sync feature head `3f293c5db66d7fb503c56af535bfa57523af3332` passed CI #272 / run `34098463593`: **SUCCESS** on Ubuntu/Windows/macOS; Ubuntu again preserved every existing runtime/performance gate.

### #61 acceptance proven on the feature head

The current proven corpus includes:

- unit-only enums and matching every unit variant;
- scalar payload enum native execution;
- acyclic record payload extraction;
- enum parameter/return roundtrip;
- payload binding use inside its lexical arm;
- nested `if` + nested `match` behavior;
- `repeat` + enum construction + exhaustive match composition;
- invalid non-exhaustive match build fails before rustc and produces no native binary;
- source-map checks for enum declaration, variant, constructor-owning statement, match and arm lines;
- raw rustc fallback remains covered for unmapped generated diagnostics;
- direct builtin/record/enum payload Rust types;
- no hidden clone, boxing, RC, runtime map, dynamic dispatch or reflection metadata;
- existing Records/scalar/function quality/runtime/performance gates remain green.

## Current pre-merge state

Feature head `3f293c5db66d7fb503c56af535bfa57523af3332` is fully green in CI #272.

Staging is now ahead only with final evidence synchronization in `docs/NEXT_ACTION.md` and this file. Production code and acceptance tests are unchanged from the #272-proven feature head.

Before merge:

1. confirm staging vs `3f293c5db66d7fb503c56af535bfa57523af3332` is docs-only;
2. fast-forward `feature/enums-codegen-v0` to staging with `force=false`;
3. allow exactly one CI run for that docs-synchronized SHA;
4. on green, update #61 and PR #64 evidence, mark ready, confirm exact head/mergeability, and squash-merge with the exact expected head SHA.

After merge:

1. record the actual squash merge SHA;
2. verify the post-merge `main` push CI to completion;
3. only then close #61 and update parent #50 codegen/native/source-map checklist from merged-main evidence;
4. update durable docs to the actual merge SHA/post-merge CI;
5. start #62 from the actual #64 squash merge SHA.

## Remaining Enums v0 queue

1. Merge **#61** and verify post-merge `main` CI.
2. **#62 — differential performance parity + final language spec sync**.

#62, not #61, owns the dedicated Enums differential benchmark and final `LANGUAGE_SPEC_V0.md` synchronization.

## Parent #50 state

Syntax, nominal/type semantics, exhaustive match semantics and ownership are already merged on `main`. Direct executable static Rust enum/match codegen, native correctness and executable source maps are now proven on PR #64 but must not be marked as merged-main acceptance until #64 is merged and its `main` push CI succeeds.

Dedicated Enums differential performance evidence and final language specification synchronization remain #62.

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
