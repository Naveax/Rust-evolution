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

## Proven #61 layers

### Executable lowering

- valid Enums v0 programs are promoted into `evo-lowering::Program` with structured enum/variant identity, source spans, typed arm-local bindings and explicit #60 ownership decisions;
- executable nominal types distinguish Records from Enums without name-prefix guessing;
- CI #260 proved the first full executable-IR promotion layer.

### Borrowed codegen view

- codegen receives a borrowed, read-only structured view of the validated executable enum IR;
- no semantic re-resolution or compiler-side clone is required;
- after format/harness fixes in #261/#262, CI #263 / run `34094640786`: **SUCCESS**.

### Static Rust emitter

- deterministic `__EvoEnum_*` and `__EvoVariant_*` identifiers;
- direct ordinary Rust enum definitions, static payload types, direct variant constructors and exhaustive `match`;
- line-level mappings for enum declarations, variants, statements, matches and arms;
- generated-code inspection rejects hidden `.clone()`, `Box`, `Rc`, `Arc` and runtime maps;
- after #264 rustfmt correction, CI #265 / run `34095123460`: **SUCCESS**.

### Strict enum-program static semantics

Because enum-enabled programs bypass the legacy scalar/Records analyzer, #61 now performs explicit source-native validation for function calls/signatures/returns, record construction/fields, assignment types, operator operands and control-flow operand types before codegen.

- #266 failed only because one test attempted parser-invalid top-level `return`; the unreachable test case was removed on a new SHA, not rerun;
- CI #267 / run `34096854858`: **SUCCESS**.

### Production codegen dispatch + first native corpus

- scalar/Records programs retain the legacy emitter;
- enum-enabled `Program` values dispatch directly to the static enum emitter;
- CLI `check`, `emit-rust`, `build` and `run` share that production path;
- #268 failed only a Clippy dead-code warning after dispatch; fixed on a new SHA;
- head `cc10319f61ef51abe843039671152b6d176c5e29` passed CI #269 / run `34097330115`: **SUCCESS** including the existing Ubuntu performance gates.

## Current CI / staging state

Feature branch currently points at:

- `5fa923227962c89d23a68deee4d990b237e3c325`
- CI #271 / run `34097977337` is the authoritative run for the expanded native/source-map acceptance corpus.

The preceding acceptance head `c0becf2e8d358aa3315816e0990f0a6ca6c708eb` failed CI #270 **only** at `cargo fmt --check`; #270 must never be rerun. `5fa923227...` contains only the required rustfmt corrections.

Staging is intentionally ahead with:

- `be4dc4f09259b93d8ed85705bcecff99b3c5dbb1` — native `repeat` + enum construction + exhaustive match composition regression;
- this documentation update and `PROJECT_STATE.md` may move staging further ahead.

Do **not** fast-forward the feature branch beyond `5fa923227...` until CI #271 has completed.

## Resume sequence

1. Check CI #271 / run `34097977337` for exact SHA `5fa923227962c89d23a68deee4d990b237e3c325`.
2. If it fails, inspect the actual failing job/log and fix only that failure on a new staging SHA. Do not rerun #271.
3. If it succeeds, fast-forward `feature/enums-codegen-v0` to the current staging head with `force=false`.
4. Let that final docs/repeat head receive exactly one fresh CI run. Do not manually trigger duplicates.
5. On final green CI:
   - update #61 checklist only for acceptance items backed by tests/CI;
   - update PR #64 body with final SHA and CI evidence;
   - mark PR ready, confirm mergeability/head SHA, and squash-merge with expected head SHA.
6. Verify the post-merge `main` push CI before closing #61 or updating parent #50.
7. Start #62 only from the actual #64 squash merge SHA after post-merge main CI is green.

## #61 acceptance still being finalized

The expanded acceptance corpus now covers or is intended to prove:

- unit-only enums and every unit variant;
- scalar payloads;
- acyclic record payloads;
- enum parameter/return roundtrip;
- in-arm payload binding use;
- nested `if` + nested `match`;
- `repeat` + enum construction + match (staging `be4dc4f0...`);
- invalid non-exhaustive match fails before rustc and produces no binary;
- source-map lines for enum/variant/constructor/match/arm;
- direct builtin/record/enum Rust payload types;
- no hidden clone/boxing/runtime map.

Do not mark these complete merely because the tests exist. Mark them after the corresponding feature-head CI is green.

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
