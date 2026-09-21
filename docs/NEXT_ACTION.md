# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-21**

## Stable gate

Current exact verified `main`:

`b3b3eebe5c1d41b47e26e3bb2bc00db5a34c32c9`

This is PR #147 squash merge, accepting the bounded dynamic-borrow guard research decision after #145 completed generational arena v0.

Natural exact-main validation on `b3b3eebe...`:

- CI `35610602607`: **SUCCESS** on Ubuntu 24.04, Windows and macOS
- Dynamic borrow guard research `35610602634`: **SUCCESS**
- Append-only sequence performance `35610602465`: **SUCCESS**
- Explicit shared owner performance `35610602480`: **SUCCESS**
- Generational arena performance `35610602782`: **SUCCESS**

## Immediate P0 — finish #138 / PR #146

Current branch:

`research/weak-upgrade-result-v0`

Exact base:

`b3b3eebe5c1d41b47e26e3bb2bc00db5a34c32c9`

Exact head:

`3b07d49d14d46de4f2e45ee228960b3fd1ac1ba0`

Current evidence:

- Windows normal CI: **SUCCESS**
- macOS normal CI: **SUCCESS**
- Ubuntu normal CI: queued
- Checked Weak upgrade result research: queued
- sequence/shared-owner/arena performance: queued

Execution order:

1. do not change the head unless an exact-head gate produces a concrete failure;
2. if a gate fails, patch only the demonstrated cause and let the new SHA trigger naturally;
3. when all exact-head gates are green, inspect the Weak research artifact for exact SHA, Rust 1.98.0, zero mismatches and `SCOPED-UPGRADE-CANDIDATE`;
4. merge PR #146 with expected-head protection;
5. require natural exact-main normal CI plus natural exact-main Checked Weak upgrade result research;
6. close #138 only after those postmerge gates are green.

## Next production gate — #137

#137 stays blocked until #138 is completed on exact main.

First production slice is already mapped across parser, lowering, formatter and Rust codegen:

- contextual `weak T`;
- non-consuming `downgrade owner`;
- checked `upgrade edge as owner ... else ... end`;
- branch-local success binding of `shared T`;
- direct `std::rc::Weak<T>`, `Rc::downgrade`, and `Weak::upgrade`;
- local/function Weak handles first;
- unsupported record/enum/sequence/arena/nested Weak storage remains fail-closed unless separately proven;
- whole-program enum-bearing parity is mandatory.

Do not create the production branch before #138 exact-main completion.

## Parallel READY research — #148

#148 is unblocked by completed #134.

Prepared branch:

`research/exclusive-guard-mutation-surface-v0`

Prepared clean head:

`7fa1a81c122d981c17b28a19a5ea7e69c6e9e4ed`

The branch contains exactly three research files and no production semantics. Pre-registered verdict: **WHOLE-PAYLOAD-REPLACE-CANDIDATE** with bounded research surface `replace guard with expr`.

Open its PR only when doing so will not unnecessarily amplify an active hosted-runner backlog.

## Prepared infrastructure

Before opening these PRs, rebase onto the current exact main and re-audit the resulting diff:

- #149 `infra/benchmark-provenance-v0` — prepared head `a091de92d58fbc5df8583ec45098b86a3d2b7ca7`
- #150 `infra/research-workflow-provenance-v0` — prepared head `44f0618ccf102b18b4f3a061548c126fcdcfd8b1`
- #151 `infra/tooling-evidence-provenance-v0` — stacked prepared head `dfd3972bf1577a5af156d93854fd1baf8ada6fb1`

#150 naturally fans out into many research workflows, so do not open it during a runner-capacity bottleneck merely to create more queued rectangles on a web page.

## Separate blocked lane — #133

Current-main normal CI is green. A new current-main Cross-thread shared ownership research dispatch is still required. Do not rerun the exhausted historical cancelled attempt.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Keep one exact head per active PR. Historical failures remain evidence. Merge only with expected-head protection and require natural exact-main postmerge proof.
