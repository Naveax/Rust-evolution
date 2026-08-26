# Rust Evolution — Project State

Last verified update: **2026-08-26**

This file is the durable project handoff. Fresh sessions should read `AGENTS.md`, this file, and `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Last merged language feature baseline: Functions v0, merge `dc92e892f37353bb38988b9310db1b354f3aefc7` (#37).
- Functions post-merge CI: run **32971472234** / run number **117**, green on Ubuntu/Windows/macOS.
- Omni/continuity/governance documentation is installed on `main` and its final validation run **32974951165** / run number **134** is green on all three operating systems.

## Durable continuation infrastructure

The repository now contains an explicit memory/handoff layer so a new chat can resume without prior conversation history:

- `AGENTS.md` — mandatory agent, CI, authority and continuation contract;
- `docs/PROJECT_STATE.md` — this verified status snapshot;
- `docs/NEXT_ACTION.md` — the single volatile continuation pointer;
- `docs/CONTINUATION_PROTOCOL.md` — fresh-session recovery sequence;
- `docs/HANDOFF_CHECKLIST.md` — end-of-session maintenance checklist;
- `docs/DECISIONS.md` — durable architecture/language decisions;
- `docs/README.md` — documentation map;
- `.github/CONTINUATION.md` — quick GitHub-visible continuation pointer;
- `.github/pull_request_template.md` — semantics/safety/cost/performance/docs/handoff checklist;
- issue **#40** — living `META READ FIRST` project-continuity entry point.

GitHub is the durable source of truth. Chat memory is optional context.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.

## Omni architecture integration

The user-authored Omni/full-stack direction has been integrated without being confused with current implementation:

- `docs/OMNI_VISION.md` — long-term scripting→kernel / CPU→GPU north star;
- `docs/COST_MODEL.md` — ZERO / EXPLICIT / MANAGED cost architecture;
- `docs/PROFILE_MODEL.md` — Core / Profile-Capability / Library / Optional Runtime / Backend / Tooling classification;
- `research/languages/README.md` — systematic external-language/ecosystem idea evaluation matrix.

Long-term architecture direction:

`Small Core + Typed Semantics/HIR + Capability Profiles + Optional Runtimes + Backends + Tooling`

`OMNI_VISION.md` is a vision document. Current language truth remains code/tests + `docs/LANGUAGE_SPEC_V0.md`.

## Current implemented language surface on `main`

Implemented and validated:

- UTF-8 source and source spans;
- integer (`i64`), boolean and current literal/static string values;
- bindings, first-definition vs reassignment, inferred mutability;
- arithmetic and comparisons;
- strict short-circuit `and` / `or` / `not`, no truthiness;
- `print` and `input_int`;
- `repeat ... end`;
- `if / else / end`;
- compact typed named functions with fixed arity, nested/static calls, forward calls and direct recursion;
- function-local semantic scopes and parameter mutability inference;
- source-native lexer/parser/lowering diagnostics;
- bounded lexer/parser recovery;
- deterministic formatter (`evo fmt`, `--check`);
- generated-Rust source mappings;
- rustc diagnostic remapping back to Evolution source;
- native `check`, `emit-rust`, `build`, `run`, `fmt` workflows;
- differential correctness/performance harness;
- normalized LLVM, binary size and exact executable comparison.

Authoritative semantics: `docs/LANGUAGE_SPEC_V0.md`.

## Performance contract

Core/native zero-cost work remains governed by:

`T_evolution <= T_reference_rust`

Correctness is required first. Stable ratio above `1.00` fails unless independently compiled executables are byte-identical, in which case runtime parity is deterministic and raw timing noise remains visible but is not treated as a real regression.

Current Ubuntu CI performance gates cover:

- runtime input / repeat / reassignment;
- control-flow branches;
- logical operators;
- functions v0.

Final Functions v0 PR validation was run **32971103291** / run number **116**, green on all three OSes. The function artifact showed correctness PASS, normalized LLVM equality, exact executable equality and binary size `2,267,040 B / 2,267,040 B`.

## Major completed slices

Completed/merged work includes:

- planning/vision/performance/architecture foundation;
- repository/toolchain/workspace/CI bootstrap;
- executable frontend/CLI vertical slice;
- differential correctness/performance harness;
- runtime input + repeat + inferred mutability;
- runtime-input process corpus;
- source-native diagnostics;
- generated-Rust source-map foundation;
- rustc diagnostic remapping;
- formatter/recovery work;
- booleans/comparisons/if control flow;
- strict logical operators and short-circuit process tests;
- named Functions v0 with static zero-cost lowering and performance gate;
- Omni vision + cost/profile/research architecture;
- durable fresh-chat continuation system.

## Active work

### #38 — Block-local bindings v0

- Issue: **#38**
- Draft PR: **#39** — `feat: add lexical block locals v0`
- Branch: `feature/block-locals-v0`
- Current clean head: `ccf4f860cfb086023e770821442c892ab6f76614`
- PR changed-files list currently contains only `crates/evo-lowering/src/lib.rs`.
- Temporary source-mutating development workflows have been removed.
- Authoritative current validation: CI **32975422570** / run number **139**, **SUCCESS** on Ubuntu/Windows/macOS.
- Ubuntu #139 also passed all existing runtime performance gates.

### #38 cleanup history

The first one-shot refactor workflow committed `c81cd45b4f2767d7689d6955ca7674bfd72477ac` as `github-actions[bot]`. CI #119 and the second refactor run ended `action_required` with zero jobs because GitHub stopped bot-triggered PR workflows at its approval/security layer. This was not a test failure.

After the temporary workflow was removed, normal CI exposed the actual code issue: an unused `name` pattern in `apply_mutability`. That was fixed, the one-shot fix workflow was removed, and final normal CI #139 is green.

Do not treat the old `action_required` state as an unresolved blocker.

## #38 semantic direction

Block locals should lower to ordinary Rust lexical bindings:

- first assignment in a child block creates a local when no visible binding exists;
- child blocks may read visible parent locals;
- block locals disappear at block end;
- assignment to a visible outer binding remains reassignment;
- sibling branch scopes are independent;
- no implicit branch merge / phi / promotion into outer scope;
- zero-iteration repeat cannot leak a local;
- no runtime variable environment/map/object;
- mutability is tracked by declaration identity rather than only source name.

Unit/lowering tests for the lexical-scope refactor are green, but #38 is **not merge-ready yet**. Remaining work is process-level block-local coverage, diagnostics/source-map review, a new runtime-dependent block-local benchmark/gate, spec update only after proof, PR finalization/merge and post-merge main validation.

The exact continuation sequence is in `docs/NEXT_ACTION.md`.

## Handoff invariant

Every significant merge or incomplete stopping point must keep `PROJECT_STATE.md`, `NEXT_ACTION.md`, issue/PR evidence and durable decisions synchronized with GitHub reality.

The repository is the project memory. The chat transcript is not.