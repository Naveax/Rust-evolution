# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-08-27**

## Active P0

**Parent #41 — Records v0: typed product data**

PR #48 is merged. Issue #46 is closed. Only PR #49 remains before Records v0 can be closed on `main`.

## Current repository evidence

### `main`

- head: `9f55b3ac44f17c52cbda5f5d6a5a626075e9a6e8`
- merge: PR #48, production typed lowering / ownership / static Rust codegen
- post-merge CI #195 / run `33073491890`: **SUCCESS**
- Ubuntu / Windows / macOS: format, Clippy, workspace tests, benchmark smoke, release build green
- Ubuntu: all pre-Records runtime performance gates green
- #46: closed as completed

### PR #49

- branch: `feature/records-parity-v0`
- desired base: `main`
- pre-clean validated head: `007655f3056616df26c01941e21a93ecce2e8f1b`
- CI #194 / run `33073047724`: **SUCCESS**
- PR remains draft until ancestry is normalized, base is `main`, and the resulting CI is green

Dedicated Records parity evidence from CI #190 / run `33071967025`:

- correctness PASS
- normalized LLVM equal
- executable bytes equal
- binary size `2,267,104 B / 2,267,104 B`
- timing ratio `1.000226357`
- timing-only verdict FAIL
- final verdict PASS
- verdict basis `byte-identical-binary-parity`

## Important squash-merge ancestry detail

PR #48 was squash-merged. Therefore the old PR #49 branch history and `main` diverge at commit ancestry even though the PR #48 base tree is already present on `main`.

A naive retarget can make GitHub show the already-merged #48 history again. Normalize PR #49 first:

1. preserve the validated PR #49 final tree contents;
2. create a clean PR #49 commit whose parent is `9f55b3ac44f17c52cbda5f5d6a5a626075e9a6e8`;
3. include updated `PROJECT_STATE.md` / `NEXT_ACTION.md` reflecting the completed #48 merge;
4. move `feature/records-parity-v0` to that clean commit only after confirming no active CI remains for the old state;
5. use force only because the rewrite intentionally replaces obsolete stacked ancestry, never to overwrite unknown concurrent work.

## Resume here

### 1. Clean and retarget PR #49

1. Confirm PR #49 has not moved unexpectedly.
2. Confirm the clean commit preserves the accepted parity tree except for intentional handoff-document updates.
3. Move `feature/records-parity-v0` to the clean `main`-parented commit.
4. Retarget PR #49 base to `main`.
5. Inspect the new PR diff. It must not reintroduce the already-merged #48 implementation history.
6. Record the single CI run ID for the new head; do not dispatch or rerun manually.

### 2. Validate PR #49

Require:

- format green on Ubuntu, Windows, macOS;
- Clippy green;
- workspace tests green;
- benchmark smoke green;
- all older Ubuntu runtime gates green;
- Ubuntu `Records v0 performance gate` green;
- release build green;
- no duplicate workflow run for the same SHA/workflow/input.

If CI fails, fix only the actual failing evidence and create a new SHA. Do not rerun a failed old SHA merely to obtain a different scheduler sample.

### 3. Merge PR #49

After the cleaned PR is mergeable and green:

1. mark PR #49 ready for review;
2. squash-merge through the normal authorized repository path using the verified head SHA;
3. track the resulting `main` push CI by run ID;
4. do not manually dispatch another run.

### 4. Close Records v0 P0

Only after final post-merge `main` CI is green, including the Records gate:

1. close #47 as completed;
2. update #41 with the final merge SHA and post-merge CI run;
3. close #41 as completed;
4. update `docs/PROJECT_STATE.md` and this file on `main` to remove Records merge-stack instructions;
5. re-read current issue #1 / roadmap / weakness map and select the next atomic P0 from repository truth.

## Accepted Records v0 boundaries

Preserve these deliberate v0 limitations unless a new tracked feature explicitly changes them:

- no whole-record `print`;
- no whole-record equality;
- no record-valued partial field move;
- no implicit clone/copy insertion;
- no implicit borrow/reference inference;
- no recursive by-value layout that would require hidden boxing;
- no runtime object dictionary/reflection layer.

## CI rule

A running CI is work in progress, not a reason to stop. Work on independent tasks while it runs, but do not retrigger it.

Never create multiple active Actions for the same SHA/workflow/input.

## Performance rule

For ZERO/native core work:

`T_evolution <= T_reference_rust`

Evidence priority:

1. correctness PASS;
2. generated Rust inspection;
3. normalized LLVM equality;
4. exact executable equality;
5. otherwise stable hard runtime ratio gate.

Records v0 currently reaches level 4: exact executable parity.