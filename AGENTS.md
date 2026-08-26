<!-- naveax-ci-execution-policy:v2 -->
# Rust Evolution Agent / Continuation Contract

This policy is mandatory for every AI agent, automation, and coding assistant working in this repository.

Repository: `Naveax/Rust-evolution`

GitHub is the durable project memory. Do **not** depend on prior chat history.

If a user says an equivalent of **“Rust Evolution'a devam et”** or **“continue Rust Evolution”**, recover context from this repository before changing code. Do not ask the user to reconstruct project history when GitHub is accessible.

## Mandatory read order for a new session

1. `AGENTS.md`
2. `docs/PROJECT_STATE.md`
3. `docs/NEXT_ACTION.md`
4. `docs/LANGUAGE_SPEC_V0.md`
5. `docs/PERFORMANCE_CONTRACT.md` and `docs/BENCHMARKING.md`
6. `docs/OMNI_VISION.md`
7. `docs/DECISIONS.md`
8. `docs/ROADMAP.md` and issue #1
9. The active issue/PR named by `NEXT_ACTION.md`
10. Current GitHub Actions for the relevant head SHA

If documents disagree, use this authority order:

`tests + code on main` > `LANGUAGE_SPEC_V0` > current PR/CI evidence > `PROJECT_STATE/NEXT_ACTION` > `DECISIONS` > `ROADMAP` > `OMNI_VISION`.

Vision never overrides implemented semantics.

## Correctness / safety / runtime invariants

### Correctness first

Equivalent Evolution and reference programs must have equivalent observable behavior before performance evidence is valid.

### Zero-cost/native runtime contract

For equivalent semantics under controlled conditions:

`T_evolution <= T_reference_rust`

Stable `ratio > 1.00` is FAIL. Noisy timing is INCONCLUSIVE. Ergonomics does not compensate for a real runtime regression.

Byte-identical independently compiled executables after correctness PASS are deterministic runtime-parity evidence. Raw timing remains visible but cannot turn the same executable into a fake regression.

### Hidden-cost policy

Core/zero-cost features must not silently add allocation, clone, boxing, dynamic dispatch, reference counting, VM/GC/managed runtime dependencies, or hidden domain transfers/synchronization.

### Safety

Do not weaken the intended Rust-level memory-safety or data-race-safety baseline for convenience or benchmark wins. `unsafe` stays explicit and must have documented invariants.

## Omni architecture classification

Every major idea from `docs/OMNI_VISION.md` or language research must be classified before implementation:

- **Core** — general language semantics.
- **Profile / Capability** — domain semantic rules and validation.
- **Library** — reusable APIs without new core semantics.
- **Optional Runtime** — explicit actor/async/managed/distributed runtime behavior.
- **Backend** — target-specific code generation.
- **Tooling** — diagnostics, formatter, LSP, debugger, profiler, build/package/cost UX.

A good idea is not automatically a core keyword.

## Branch / PR discipline

- `main` remains stable.
- Use focused branches: `feature/`, `fix/`, `bench/`, `research/`, `experiment/`, `docs/`.
- One primary purpose per PR.
- Failed experiments are documented rather than erased.
- User-facing syntax updates `docs/LANGUAGE_SPEC_V0.md` only after behavior is proven.
- Temporary source-mutating development workflows must be removed after their one-shot purpose is complete.
- Use `.github/pull_request_template.md` for evidence/handoff expectations.

## CI dispatch invariant

One logical validation target may have at most one active GitHub Actions run.

Before every workflow dispatch, rerun, retry, or CI-triggering operation, inspect existing runs and build a logical key from:
- repository;
- workflow;
- ref / branch;
- HEAD commit SHA;
- normalized workflow inputs.

If an equivalent run is queued, waiting, pending, requested, or in progress, do not create another run. Track/poll the existing run ID instead. Never use reruns or repeated dispatches as a polling mechanism.

If the same logical target already completed successfully, do not rerun it unless code, inputs, environment requirement, or validation objective materially changed.

## Retry budget

For the same commit + workflow + normalized inputs, the automatic dispatch budget is 1. A second execution is allowed only for a concrete infrastructure/runner failure or demonstrated flaky external dependency, and the reason must be recorded. Prefer rerunning only the failed job when possible.

Never create empty/no-op commits or meaningless file changes merely to retrigger CI.

If dispatch frequency rises unexpectedly, stop new dispatches and diagnose the trigger/scheduler/agent loop before continuing.

## Work scheduler

CI is an asynchronous dependency, not the main work loop. Waiting for CI must not cause idling or duplicate dispatches.

Maintain RUNNING, READY, BLOCKED, and DONE work states. When a CI-dependent task becomes BLOCKED, switch to another independent READY task.

Useful work while CI runs includes source inspection, static analysis, tests, documentation, review, dependency analysis, security review, benchmark preparation, log analysis, and preparing the next patch without dispatching another equivalent run.

## CI broker rule

Only the coordinating agent/workstream may authorize GitHub Actions dispatches. Parallel workers report validation needs to the coordinator instead of independently starting Actions runs.

Before a new validation run after failure: collect complete relevant failure evidence, identify root cause, make one coherent patch, then validate the new commit once.

## Workflow authoring

When adding/editing GitHub Actions workflows, preserve existing semantics and add an appropriate top-level concurrency policy when absent. For ordinary branch-scoped validation, prefer a group derived from workflow + ref and keep `cancel-in-progress: false` unless replacement semantics are explicitly intended.

## Continuation / handoff rule

Every meaningful merged slice or incomplete stopping point must leave the repository ready for the next session.

Before ending significant work:

- update `docs/PROJECT_STATE.md` if verified status changed;
- update `docs/NEXT_ACTION.md` to one exact continuation point;
- add durable decisions to `docs/DECISIONS.md`;
- record meaningful CI/benchmark evidence in the issue/PR;
- remove temporary dev/mutation workflows when no longer needed;
- keep vision-only capabilities out of current implementation claims.

For incomplete work, `NEXT_ACTION.md` must include issue, PR, branch, head SHA, run IDs/conclusions, known failure/root cause, first next action, and remaining acceptance gates.

## New-session behavior

On a fresh chat/session:

1. locate this repository;
2. read the mandatory files;
3. verify `main` HEAD/CI and active feature branch HEAD/CI;
4. state briefly what was recovered;
5. continue the active task immediately unless blocked by authorization or an irreversible user decision.

The repository, not the chat transcript, is the durable project memory.