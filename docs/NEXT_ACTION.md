# Rust Evolution — NEXT ACTION

This file is intentionally short and operational. It tells a new chat/agent exactly where to continue.

Last verified update: **2026-08-26**

## Active P0

**Issue #38 — Block-local bindings v0**

PR: **#39** — `feat: add lexical block locals v0`

Branch: `feature/block-locals-v0`

Latest known branch head:

`c81cd45b4f2767d7689d6955ca7674bfd72477ac`

## Resume here

Do **not** start a new language feature before resolving #38.

First actions for the next session:

1. Read `AGENTS.md`, `docs/PROJECT_STATE.md`, issue #38 and PR #39.
2. Inspect current PR #39 head and compare it with the SHA above; if it moved, use the newer head and update this file before ending the session.
3. Inspect GitHub Actions for the current head.
4. Known last state on `c81cd45...`:
   - CI run #119: `action_required`;
   - temporary `block-locals-refactor` run #2: `action_required`.
5. Determine the real cause from GitHub metadata/logs/approval state. Do not guess.
6. The temporary `.github/workflows/block-locals-refactor.yml` development workflow should be removed once the refactor commit exists and no further one-shot mutation is needed.
7. Verify the bot commit `c81cd45...` contains the intended lexical-scope lowering refactor and no unrelated changes.
8. Get a normal CI run green on the final branch head without relying on the temporary mutation workflow.
9. Add/verify correctness corpus for:
   - `if`-local usable inside branch and rejected after `end`;
   - independent `then` / `else` locals;
   - repeat-local usable/reassignable inside loop and rejected after loop;
   - nested block reading a parent block-local;
   - nested child local rejected after child closes;
   - outer reassignment from child still valid and marks outer binding mutable;
   - sibling same-name locals do not leak mutability/scope state;
   - function-local block scopes compose correctly;
   - top-level capture inside functions remains forbidden.
10. Add process-level CLI coverage for block locals.
11. Add a runtime-dependent `block-locals-v0` differential performance case and Ubuntu gate.
12. Require correctness first; prefer normalized LLVM/exact binary parity; otherwise enforce stable `T_evolution / T_reference <= 1.00`.
13. Only after all gates are green, update `docs/LANGUAGE_SPEC_V0.md`, finalize PR #39, merge, verify post-merge `main` CI, close #38, then update `PROJECT_STATE.md` and this file to the next P0.

## #38 semantic target

No new syntax is required. Existing `name = expression` remains syntax-neutral.

Rules:

- first assignment in a child control-flow scope creates a block-local when no visible binding exists;
- child scopes may read visible parent locals;
- block-local bindings disappear at block end;
- assignment to a visible outer local is reassignment, not shadowing;
- no implicit promotion/phi/merge after `if`;
- zero-iteration `repeat` cannot make a local visible outside;
- generated code uses ordinary Rust lexical `let` bindings only;
- no runtime environment object, `HashMap` lookup, boxing, or dynamic dispatch;
- mutability is tracked by declaration identity so sibling scopes using the same source name cannot contaminate each other.

## After #38

Do not pre-commit to the next feature in code. Re-evaluate issue #2 + `docs/OMNI_VISION.md` + weakness map after #38 closes. Likely Core-stabilization candidates include records/structs, enums/pattern matching, error handling, modules, collections, and ownership ergonomics.

The next feature must still go through the normal problem → semantics → cost class → correctness → diagnostics/tooling → benchmark → accept/reject pipeline.