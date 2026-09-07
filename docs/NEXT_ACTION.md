# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-07**

## Active P0

Parent milestone: **#50 — Enums v0: nominal sum types + exhaustive static matching**

Stable `main` baseline:

- ownership child #60 / PR #63 squash merge: `fc611c0e92a48d15d94373998b3618f154f0d0e5`
- post-merge main CI #243 / run `33117962228`: **SUCCESS**
- Rust toolchain: **1.98.0**

Active child:

- **#61 — static Rust enum/match codegen + source maps**
- PR: **#64 — `feat: promote Enums v0 into executable IR`**
- feature branch: `feature/enums-codegen-v0`
- staging branch: `work/enums-codegen-v0`

Following child:

- **#62 — differential performance parity + final language spec sync**

## Proven #61 implementation

### Executable lowering

- validated enum schemas, constructors, exhaustive matches, typed arm-local bindings and explicit #60 ownership decisions are promoted into `evo-lowering::Program`;
- executable nominal types distinguish Records from Enums without name-prefix guessing;
- enum/variant semantic identity remains structured.

### Static Rust codegen

- codegen consumes a borrowed read-only view of validated executable enum IR;
- deterministic `__EvoEnum_*` / `__EvoVariant_*` identifiers;
- ordinary static Rust enum definitions emitted before functions/main;
- builtin, record and enum payloads lower to ordinary static Rust types;
- direct variant construction and exhaustive Rust `match` with lexical payload bindings;
- source mappings cover enum declarations, variants, constructor-owning statements, matches and arms;
- generated-code inspection rejects hidden `.clone()`, `Box`, `Rc`, `Arc`, runtime maps, dynamic dispatch and reflection-style metadata.

### Strict frontend semantics and production dispatch

- enum-enabled programs validate function calls/signatures/returns, record construction/fields, assignment types, operator operands and control-flow operand types before codegen;
- scalar/Records programs retain the legacy emitter;
- enum-enabled `Program` values dispatch to the static enum emitter;
- CLI `check`, `emit-rust`, `build` and `run` share the production path.

## CI history relevant to the final layer

- CI #267 / run `34096854858`: **SUCCESS** after removing an unreachable parser-invalid test case from #266; #266 was not rerun.
- CI #268 failed only a Clippy dead-code warning after production dispatch; it was not rerun.
- corrected production head `cc10319f61ef51abe843039671152b6d176c5e29` passed CI #269 / run `34097330115`: **SUCCESS**.
- expanded acceptance head `c0becf2e8d358aa3315816e0990f0a6ca6c708eb` failed CI #270 only at `cargo fmt --check`; #270 was not rerun.
- rustfmt-corrected acceptance head `5fa923227962c89d23a68deee4d990b237e3c325` passed CI #271 / run `34097977337`: **SUCCESS** on Ubuntu, Windows and macOS; Ubuntu preserved every existing runtime/performance gate.
- final repeat/ZERO-cost/docs-pre-sync feature head `3f293c5db66d7fb503c56af535bfa57523af3332` passed CI #272 / run `34098463593`: **SUCCESS** on Ubuntu, Windows and macOS; Ubuntu again preserved every existing runtime/performance gate.

## #61 acceptance now proven on the feature head

The feature-head corpus covers:

- unit-only enum and matching every unit variant;
- scalar payload enum native execution;
- acyclic record payload extraction;
- enum parameter/return roundtrip;
- payload binding use inside its arm;
- nested `if` + nested `match` behavior;
- `repeat` + enum construction + exhaustive match composition;
- invalid non-exhaustive match fails before rustc and produces no binary;
- source-map lines for enum declaration, variant, constructor-owning statement, match and arm;
- unmapped rustc failures retain raw stderr through the existing CLI regression;
- direct builtin/record/enum Rust payload types;
- no hidden clone, boxing, RC, runtime map, dynamic dispatch or reflection metadata.

## Current staging state

Feature head `3f293c5db66d7fb503c56af535bfa57523af3332` is fully green in CI #272.

Staging is now being advanced only for this final evidence synchronization in `docs/NEXT_ACTION.md` and `docs/PROJECT_STATE.md`. Production code and acceptance tests are unchanged from the #272-proven head.

## Resume sequence

1. Compare the current staging head against `3f293c5db66d7fb503c56af535bfa57523af3332` and confirm the delta is documentation-only.
2. Fast-forward `feature/enums-codegen-v0` to that staging head with `force=false`.
3. Let the docs-synchronized head receive exactly one fresh CI run. Do not manually rerun old SHAs or create duplicate active runs.
4. If that final docs-synchronized CI is green:
   - update #61 checklist only from actual test/CI evidence;
   - update PR #64 body with the exact final head SHA and CI run;
   - mark PR #64 ready;
   - re-fetch PR metadata, confirm mergeability and exact head SHA;
   - squash-merge with `expected_head_sha` equal to that exact final head.
5. Record the returned squash merge SHA and verify the post-merge `main` push CI to completion.
6. Only after post-merge `main` CI succeeds:
   - close #61 completed;
   - update parent #50 codegen/native/source-map checklist from merged-main evidence;
   - update durable docs to the actual merge SHA and post-merge CI;
   - create/start #62 from the actual #64 squash merge SHA.

## Engineering constraints

- Enum/variant identity remains structured; never concatenate semantic identity into magic names.
- Record-vs-enum nominal kind remains explicit.
- #60 ownership analysis remains authoritative; #61 must not invent another move model.
- No implicit clone, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch.
- Preserve every accepted Records v0 generated-code/runtime gate.
- `LANGUAGE_SPEC_V0.md` remains intentionally unchanged until #62.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent staging/docs work, but never create multiple active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Never rerun an old failed SHA merely to obtain another result.
