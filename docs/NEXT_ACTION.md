# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-08-27**

## Active P0

Parent milestone: **#50 — Enums v0: nominal sum types + exhaustive static matching**

Semantic umbrella: **#54 — Enums semantics: nominal typing, constructors and exhaustive match**

Completed semantic slices on `main`:

- **PR #55 — nominal declaration validation**
  - squash merge `ca9641d6c7c57ab603cda8b6a4a091f50cfd625d`
  - post-merge main CI #221 / run `33098011627`: **SUCCESS**
- **#56 / PR #58 — resolved variants and constructor typing**
  - squash merge `875b4d8fc255d6699b4f89b5e5c769d8dd34383b`
  - final PR CI #229 / run `33103924169`: **SUCCESS**
  - post-merge main CI #230 / run `33104226016`: **SUCCESS**

Active child / PR:

- **#57 — exhaustive match typing and arm scopes**
- **PR #59 — type-check Enums v0 exhaustive matches**
- PR branch: `feature/enums-match-typing-v0`
- staging branch: `work/enums-match-typing-v0`
- verified code head: `a5270d2a86b1103a431382325c5d77752ccebcf1`
- CI #234 / run `33113246845`: **SUCCESS**

Following children already created:

1. **#60 — Enums v0 ownership + match payload extraction**
2. **#61 — static Rust enum/match codegen + source maps**
3. **#62 — differential performance parity + final language spec sync**

## Verified stable baseline

Current `main` baseline for #57:

`875b4d8fc255d6699b4f89b5e5c769d8dd34383b`

Post-merge main CI #230 is authoritative for #56. Ubuntu preserved every existing runtime/performance gate including Records v0; Windows/macOS preserved the quality/test/release matrix.

## Active #57 implementation

PR #59 now provides the complete semantic match slice while execution remains fail-closed:

- match scrutinee must have a statically known nominal enum type;
- arm enum qualifiers must match the scrutinee enum;
- every arm variant resolves source-natively;
- duplicate variant arms are rejected;
- every declared variant is required exactly once;
- missing variants are reported deterministically in declaration order;
- unit variants reject payload bindings;
- payload variants require one binding under the frozen v0 parser surface;
- payload bindings receive the declared payload type;
- each binding exists only inside its arm body;
- sibling arm scopes are independent;
- bindings cannot leak after the match;
- current no-shadowing policy rejects a payload binding that conflicts with an already-visible local;
- nested `if` / `repeat` / `match` semantic scopes remain deterministic;
- invalid match programs fail at Evolution source spans before rustc.

## Retained match semantic sidecar

The current #57 boundary retains validated match information without enabling runtime lowering:

- structured enum and variant identity;
- match/arm source spans;
- typed payload-binding metadata;
- source-statement indexing without concatenated magic names;
- `all_arms_return`, computed only after structural exhaustiveness succeeds;
- parser ↔ resolved-sidecar invariant checking in the pre-codegen semantic path.

This sidecar is preparation for #60/#61. It is **not** Rust enum/match codegen and does not invent ownership joins.

## CI evidence for PR #59

- structural head `661786a4...`: CI #231 / run `33104721456` — **SUCCESS**
- first typed-match head `a5a8fb8e...`: CI #232 / run `33105128042` — failed only because one new test expected line 9 while the correct payload-binding pattern span was line 10; fmt/Clippy and the remaining new tests were green; the failed SHA was not rerun
- corrected typed-match head `d7d6818937c1d1f353b271938ca14b643b2ac01c`: CI #233 / run `33105515592` — **SUCCESS**
- resolved-sidecar head `a5270d2a86b1103a431382325c5d77752ccebcf1`: CI #234 / run `33113246845` — **SUCCESS**
- Ubuntu #234 passed format, Clippy, workspace tests, benchmark smoke, runtime repeat, control-flow, logical operators, Functions v0, Block Locals v0, Records v0 and release build
- Windows/macOS #234 passed format, Clippy, workspace tests, benchmark smoke and release build

## Resume here

1. Inspect `work/enums-match-typing-v0` and PR #59.
2. The staging branch is allowed to be ahead of verified code head `a5270d2a...` only by handoff/documentation synchronization.
3. Confirm the final staging target SHA has no active Action, then fast-forward `feature/enums-match-typing-v0` exactly once.
4. Follow the single docs-synchronized final CI. Do not rerun #232/#233/#234 and do not create duplicate Actions.
5. Require green format, Clippy, workspace tests, match CLI regressions, all existing Ubuntu runtime/performance gates and release build.
6. Mark PR #59 ready only after the exact final head is green.
7. Squash merge with expected-head protection.
8. Verify the post-merge `main` CI before closing #57 or #54.
9. Update #57 and #54 checklists only from merged-main evidence, then close them completed if their semantic acceptance remains satisfied.
10. Start **#60** from the actual #59 squash-merge SHA, not from staging ancestry.
11. #60 owns enum move/reinitialization and payload extraction ownership. Keep Rust enum/match codegen in #61.
12. #61 owns executable lowered IR, static Rust enum/match emission, source maps and native correctness.
13. #62 owns the dedicated differential performance gate and `LANGUAGE_SPEC_V0.md` synchronization after performance/correctness proof.

## Engineering constraints

Enum and variant identity must remain structured. Do not encode `Enum.Variant` as a concatenated magic name.

Do not infer arbitrary method-call semantics from `object.member(...)`.

Do not add hidden `.clone()`, boxing, GC/RC, runtime variant maps, reflection metadata or dynamic dispatch.

Do not modify Records ownership behavior merely to make enum work convenient. #60 must integrate ownership deliberately and preserve Records v0 regression evidence.

`LANGUAGE_SPEC_V0.md` remains intentionally unsynchronized with Enums executable behavior until #62, because Enums runtime ownership/codegen/performance are not proven yet.

## CI rule

A running CI is work in progress, not a reason to stop. Continue independent work, but never create multiple active Actions for the same SHA/workflow/input.

If a run fails, fix the actual failure on a new SHA. Do not rerun an old failed SHA merely to obtain a different result or timing sample.
