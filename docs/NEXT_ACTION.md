# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume the project from here without prior conversation history.

Last verified update: **2026-08-26**

## Active P0

**Issue #38 — Block-local bindings v0**

PR: **#39** — `feat: add lexical block locals v0`

Branch: `feature/block-locals-v0`

Current clean head:

`ccf4f860cfb086023e770821442c892ab6f76614`

Authoritative validation:

- CI run ID: **32975422570**
- run number: **139**
- conclusion: **SUCCESS**
- Ubuntu / Windows / macOS: fmt, Clippy, workspace tests, benchmark smoke, release build all green
- Ubuntu: existing runtime-repeat, control-flow, logical-operators and functions performance gates all green

PR #39 currently changes only:

`crates/evo-lowering/src/lib.rs`

Temporary source-mutating workflows used during the lexical-scope refactor have been removed.

## Important resolved history

An earlier bot-authored head (`c81cd45b4f2767d7689d6955ca7674bfd72477ac`) produced `action_required` with zero jobs. This was GitHub Actions approval/security behavior for workflows triggered by a `github-actions[bot]` commit, not a test failure.

After removing the temporary workflow, normal CI exposed one real issue: an unused `name` pattern binding in `apply_mutability`. It was fixed, the second one-shot fix workflow was also removed, and final normal CI #139 is green.

Do not re-investigate the old `action_required` state unless new evidence appears.

## Resume here

Do **not** start a new language feature before finishing #38.

Next implementation work:

1. Read issue #38, PR #39, current lowering diff and `docs/DECISIONS.md` D-015.
2. Review the lexical-scope implementation for semantic correctness beyond unit tests.
3. Add process-level CLI coverage for block locals through the real Evolution -> build/run -> native path.
4. Required process/correctness corpus:
   - `if` local usable inside branch;
   - `if` local rejected after `end`;
   - `else` local usable only inside else;
   - sibling branch locals do not leak or merge;
   - repeat local usable and reassignable inside loop;
   - repeat local rejected after loop;
   - nested child reads a parent block-local;
   - nested child local is rejected after child closes;
   - outer local reassignment inside `if` remains reassignment and marks outer binding mutable;
   - outer local reassignment inside `repeat` remains valid;
   - sibling same-name locals keep independent declaration/mutability identity;
   - function-local block scopes compose correctly;
   - functions still cannot silently capture top-level locals.
5. Add/verify generated Rust snapshots proving plain lexical `let`/assignment codegen with no runtime scope structure.
6. Add source-map/diagnostic coverage for block-local declaration, reassignment and use-after-scope.
7. Add a runtime-dependent `block-locals-v0` differential benchmark and Ubuntu CI gate.
8. Require correctness first; prefer normalized LLVM/exact executable parity; otherwise enforce stable `T_evolution / T_reference <= 1.00`.
9. Keep all previous runtime gates green.
10. Only after behavior/performance is proven, update `docs/LANGUAGE_SPEC_V0.md` to remove the old “new locals cannot be introduced inside control-flow blocks” restriction and specify lexical block scope precisely.
11. Finalize PR #39, merge only when all gates are green, verify post-merge `main` CI, close #38.
12. Then update `docs/PROJECT_STATE.md`, this file and any durable decision records to the next P0.

## #38 semantic target

No new user syntax is required. Existing `name = expression` remains syntax-neutral.

Rules:

- first assignment in a child control-flow scope creates a block-local when no visible binding exists;
- child scopes may read visible parent locals;
- block locals disappear at block end;
- assignment to a visible outer local is reassignment, not shadowing;
- sibling scopes are independent;
- no implicit promotion / phi / merge into outer scope;
- zero-iteration `repeat` cannot make a local visible outside;
- generated code uses ordinary Rust lexical bindings;
- no runtime environment object, variable `HashMap`, boxing, or dynamic lookup;
- mutability is tracked by declaration identity, not just source name.

## After #38

Re-evaluate issue #2, `docs/OMNI_VISION.md`, `docs/ROADMAP.md` and the weakness map before choosing the next atomic P0.

Likely Core-stabilization candidates include records/structs, enums/pattern matching, error handling, modules, collections and ownership ergonomics. The next slice still requires exact semantics, architecture classification, cost class, diagnostics/tooling support and an evidence plan before implementation.