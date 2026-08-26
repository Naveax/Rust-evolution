# Rust Evolution — Handoff Maintenance Checklist

Use this at the end of every significant development slice.

## If a PR merged

- [ ] Verify post-merge `main` CI.
- [ ] Record final merge SHA.
- [ ] Record relevant benchmark artifact/verdict.
- [ ] Close the atomic issue only after `main` is green.
- [ ] Update `docs/PROJECT_STATE.md`.
- [ ] Replace `docs/NEXT_ACTION.md` with the next single active task.
- [ ] Add durable semantic/architecture decisions to `docs/DECISIONS.md`.
- [ ] Remove temporary mutation/dev workflows.
- [ ] Ensure `docs/LANGUAGE_SPEC_V0.md` describes the merged behavior, not future intent.

## If work is still in progress

- [ ] Record issue number.
- [ ] Record PR number.
- [ ] Record branch.
- [ ] Record latest head SHA.
- [ ] Record active/failed run IDs and conclusions.
- [ ] Record root cause of known failures when actually established.
- [ ] Record the first concrete next action.
- [ ] Record remaining acceptance gates.
- [ ] Keep `main` status separate from branch status.

## Vision / research changes

- [ ] Keep long-term concepts in `OMNI_VISION.md`, profile/cost docs, roadmap or research files.
- [ ] Do not move them into `LANGUAGE_SPEC_V0.md` until implemented and proven.
- [ ] Classify new ideas as Core / Profile / Library / Optional Runtime / Backend / Tooling.
- [ ] Define cost class and evidence plan before implementation.

## CI discipline

- [ ] Do not dispatch duplicate validation for the same SHA/workflow/inputs.
- [ ] Do not make empty/no-op commits merely to retrigger CI.
- [ ] While CI runs, continue only independent useful work.

## New-chat test

Before declaring the handoff good, ask:

> If a completely fresh assistant only had this repository and the instruction “continue Rust Evolution”, could it identify the exact current task, branch, known failures, engineering invariants and acceptance gates without asking the user to repeat history?

If not, the handoff is incomplete.