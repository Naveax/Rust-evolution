# New-Session Continuation Protocol

Goal: a fresh chat or agent should be able to continue Rust Evolution without relying on previous conversation history.

## Trigger

When the user says any equivalent of:

- `Rust Evolution'a devam et`
- `Continue Rust Evolution`
- `Projeye kaldığın yerden devam et`

and GitHub access is available, do not ask the user to reconstruct project history.

## Recovery sequence

1. Locate repository `Naveax/Rust-evolution`.
2. Read root `AGENTS.md`.
3. Read `docs/PROJECT_STATE.md`.
4. Read `docs/NEXT_ACTION.md`.
5. Verify `main` HEAD and current CI.
6. Open the active issue/PR/branch listed in `NEXT_ACTION.md`.
7. Verify the active branch HEAD and existing Actions runs.
8. Read the relevant implementation files and current `docs/LANGUAGE_SPEC_V0.md`.
9. Briefly report what was recovered and continue the exact active task.

## Authority hierarchy

When sources conflict:

1. tests + code on `main`;
2. `docs/LANGUAGE_SPEC_V0.md`;
3. current PR/issue evidence and CI artifacts;
4. `docs/PROJECT_STATE.md` / `docs/NEXT_ACTION.md`;
5. `docs/DECISIONS.md`;
6. `docs/ROADMAP.md` / issue #1;
7. `docs/OMNI_VISION.md`.

Vision never overrides implemented semantics.

## Staleness rule

`PROJECT_STATE.md` and `NEXT_ACTION.md` are handoff files, not magic. A new session must compare their recorded SHAs/run IDs with GitHub. If they are stale, correct them during the session.

## No duplicate CI

Before dispatch/rerun/retry, inspect existing runs for the same SHA/workflow/inputs. Track the existing run instead of using new runs as polling.

## End-of-session handoff

Before a significant session ends:

- update `PROJECT_STATE.md` if verified status changed;
- update `NEXT_ACTION.md` to one exact continuation point;
- record durable decisions in `DECISIONS.md`;
- leave issue/PR comments containing meaningful CI/benchmark evidence;
- remove temporary mutation/dev workflows once no longer required;
- do not describe vision-only work as implemented.

## Minimum handoff data for incomplete work

Always record:

- issue number;
- PR number;
- branch;
- latest head SHA;
- relevant run IDs and conclusions;
- known failure/root cause if any;
- the first concrete next action;
- acceptance gates still missing.

The repository is the durable memory layer. The chat transcript is optional context.