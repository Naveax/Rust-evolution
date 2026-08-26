# Rust Evolution — Project State

Last verified update: **2026-08-26**

This file is the durable project handoff. New chats/agents should read `AGENTS.md`, this file, then `docs/NEXT_ACTION.md` before changing code.

## Repository

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Rust toolchain: **1.98.0**
- Proven code baseline before the Omni/continuity documentation layer: merge commit `dc92e892f37353bb38988b9310db1b354f3aefc7` (`feat: add functions v0 (#37)`).
- The Omni/continuity/governance files were added afterward as documentation/process updates on `main`.

## Durable continuation infrastructure

The repository now contains an explicit memory/handoff layer so a fresh chat can continue without reconstructing prior conversation history:

- `AGENTS.md` — mandatory agent, CI, authority and continuation contract;
- `docs/PROJECT_STATE.md` — this verified status snapshot;
- `docs/NEXT_ACTION.md` — the single volatile continuation pointer;
- `docs/CONTINUATION_PROTOCOL.md` — new-session recovery sequence;
- `docs/HANDOFF_CHECKLIST.md` — end-of-session maintenance checklist;
- `docs/DECISIONS.md` — durable architecture/language decisions;
- `docs/README.md` — documentation map and authority rules;
- `.github/CONTINUATION.md` — GitHub-visible quick continuation pointer;
- `.github/pull_request_template.md` — correctness/safety/cost/performance/docs/handoff checklist;
- open issue **#40** — living `META READ FIRST` continuation entry point.

Fresh sessions should treat GitHub, not chat memory, as the source of truth.

## Omni architecture integration

The user-authored Omni/full-stack update has been integrated into the project architecture without pretending future domains are already implemented:

- `docs/OMNI_VISION.md` — long-term scripting→kernel / CPU→GPU North Star;
- `docs/COST_MODEL.md` — ZERO / EXPLICIT / MANAGED cost architecture;
- `docs/PROFILE_MODEL.md` — Core / Profile/Capability / Library / Optional Runtime / Backend / Tooling classification;
- `research/languages/README.md` — systematic external-language/ecosystem idea evaluation matrix.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.

## Current implemented language surface on `main`

The authoritative semantics are in `docs/LANGUAGE_SPEC_V0.md`.

Implemented and validated today:

- UTF-8 source with source spans;
- integer (`i64`), boolean, and current literal/static string values;
- bindings with semantic first-definition vs reassignment;
- inferred mutability for reassigned locals/parameters;
- arithmetic `+ - * /`;
- comparisons `== != < <= > >=`;
- strict boolean `and`, `or`, `not` with real short-circuit semantics;
- `print`;
- `input_int` runtime input;
- `repeat ... end`;
- `if / else / end`;
- compact typed named functions: `fn name(a int, b bool) int ... return ... end`;
- fixed-arity static calls, including nested calls;
- forward calls and direct recursion through a function-signature pre-pass;
- function-local semantic scopes and parameter mutability inference;
- source-native lexer/parser/lowering diagnostics;
- bounded lexer/parser recovery paths for user-facing diagnostics;
- deterministic formatter (`evo fmt`, `--check`);
- generated-Rust sidecar source mappings back to Evolution spans;
- rustc compile-diagnostic remapping back to Evolution source;
- native `check`, `emit-rust`, `build`, `run`, and formatter CLI paths;
- differential correctness/performance harness with JSON/Markdown/raw-sample artifacts;
- exact executable-byte and normalized LLVM IR comparison.

## Performance contract status

Core/native zero-cost work remains governed by:

`T_evolution <= T_reference_rust`

Correctness is required first. Stable ratio above `1.00` fails unless independently compiled executables are byte-identical, in which case runtime parity is deterministic and timing noise is reported but not treated as a real regression.

Current CI includes Ubuntu performance gates for:

- runtime input / repeat / reassignment;
- control-flow branches;
- logical operators;
- functions v0.

The final Functions v0 validation on PR #37 was CI run **#32971103291** (run number 116), green on Ubuntu/Windows/macOS. The function benchmark artifact reported correctness PASS, normalized LLVM IR equality, exact executable equality, and identical binary size `2,267,040 B / 2,267,040 B`. A later `main` push CI run **#32971472234** (run number 117) was also green on all three operating systems.

## Major completed slices

Important completed/merged work includes:

- planning/vision/performance/architecture foundation;
- repository/toolchain/workspace/CI bootstrap;
- executable lexer/parser/Rust-codegen/CLI vertical slice;
- differential correctness/performance harness;
- runtime input + repeat + inferred mutability;
- process-level runtime-input corpus;
- source-native diagnostics;
- generated-Rust source map foundation;
- rustc diagnostic remapping;
- formatter and recovery work;
- boolean/comparison/`if` control flow;
- strict logical operators with short-circuit process tests and runtime gate;
- named Functions v0 with static zero-cost call lowering and runtime gate.

Durable details live in the corresponding GitHub issues/PRs and `docs/DECISIONS.md`.

## Long-term direction

`docs/OMNI_VISION.md` is the long-term North Star. It expands the platform goal toward scripting, systems, web, GPU/graphics, data/AI/scientific, distributed, verified, embedded/kernel/driver, and hardware-oriented work.

Important: **Omni Vision is not current implementation.** Current truth remains `LANGUAGE_SPEC_V0.md` + code/tests on `main`.

The long-term architecture is intentionally layered:

`Small Core + Typed Semantics/HIR + Capability Profiles + Optional Runtimes + Backends + Tooling`

## Active work

### P0 #38 — Block-local bindings v0

Issue: **#38**

PR: **#39** — `feat: add lexical block locals v0`

Branch: `feature/block-locals-v0`

Current known head after the temporary refactor workflow committed the first semantic-scope implementation:

`c81cd45b4f2767d7689d6955ca7674bfd72477ac`

Known validation history:

- original head `53eb7aa4e8f218e09743eae54465a3440a5431ff`:
  - normal CI run #118 completed successfully;
  - temporary `block-locals-refactor` run #1 completed successfully and committed the refactor.
- bot-produced head `c81cd45b4f2767d7689d6955ca7674bfd72477ac`:
  - CI run #119 ended `action_required`;
  - temporary refactor workflow run #2 ended `action_required`.

Do **not** guess why `action_required` occurred. The next session must inspect the actual run/PR state and determine whether this is an approval/authentication/workflow-loop issue. The temporary development workflow should be removed once it has served its purpose.

## Current critical design for #38

Block locals should map to ordinary Rust lexical bindings:

- first assignment inside `if`/`else`/`repeat` creates a child-scope local when no visible binding exists;
- child scopes can read visible parent bindings;
- child locals disappear at block end;
- assignment to a visible outer binding remains reassignment;
- sibling branch scopes are independent;
- no implicit branch merge / phi / promotion into outer scope;
- zero-iteration repeat cannot leak a local outward;
- no runtime environment map/object/boxing/dynamic lookup;
- mutability must be tracked by declaration identity, not merely by variable name, so sibling locals with the same source name cannot leak `mut` state into each other.

## Known project-management rule

Every significant merge or unfinished stopping point must update this file and `docs/NEXT_ACTION.md`.

This repository is the durable memory. Chat transcripts are not.