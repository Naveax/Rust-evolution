# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-14**

## Stable gate

Current exact verified `main`:

`42fc00323a1359824d0a0adf6df6c28410556b22`

Natural validation on that exact SHA:

- CI #475 / run `34816316570`: **SUCCESS** on Ubuntu, Windows and macOS;
- Explicit shared handle surface research #8 / run `34816316546`: **SUCCESS**;
- artifact id `10337270060`;
- digest `sha256:2894d8b1c76a9ad064d18dd632d35e4e3b56a2bc159c14d2235767348c76e335`;
- accepted verdict: **IMPLEMENT-CANDIDATE / CONTEXTUAL-WORDS**.

## Active implementation — #112

Production branch: `feature/explicit-shared-handle-v0`.

The bounded one-thread immutable shared-owner implementation is present and now in final integration/validation. It uses contextual `shared Item`, `share expr`, and `dup expr`, direct safe `Rc<T>` codegen, move-only handles and existing bounded immutable-reference provenance/final-use rules.

## Accepted component performance evidence

Exact performance component head `c83d42dbe0e346021f1f524cf9d65f67fdbc66d3` passed Explicit shared owner performance #7 / run `34824865453`. Artifact `evo-bench-explicit-shared-owner-ubuntu-24.04`, id `10339932088`, digest `sha256:a490d5a0bc5d2cfe15c4da01b89cb45eb9e4d8aa51d8309d865a926dc60d6721`. Correctness passed, normalized LLVM IR matched, executable bytes were identical at 2,267,304 bytes each, the stable observed median ratio was `0.992969173`, and the final verdict was PASS via `byte-identical-binary-parity`.

This evidence validates the component benchmark/reference lock. The final combined feature head must still pass the same permanent performance workflow after all remaining gates/docs are integrated.

## Remaining completion sequence

1. Integrate the permanent no-implicit-conversion boundary tests.
2. Integrate the source-native shared-owner diagnostic assertions.
3. Require the equivalent-`Rc` benchmark reference to match generated Rust exactly and pass the permanent differential performance gate.
4. Rebase/synchronize the language spec and living handoff onto the final feature head.
5. Require final exact feature-head normal CI on Ubuntu, Windows and macOS plus the permanent explicit-shared-owner performance workflow.
6. Open/update the final `feature/explicit-shared-handle-v0 -> main` PR with exact evidence and review the complete diff against verified main.
7. Squash-merge only with expected-head protection.
8. Track natural post-merge exact-main CI and explicit-shared-owner performance runs; do not create duplicate runs.
9. Close #112 only after those natural main gates succeed.

## Contracts that must remain true

- `shared T` is distinct from owned `T`, immutable `&T`, and inferred call-duration `SharedBorrow`.
- `share` is the only initial shared allocation operation.
- `dup` is the only shared-owner duplication operation in this slice.
- ordinary assignment/parameter/return moves a shared handle without hidden refcount increments.
- payload borrowing remains non-owning and tied to the specific source handle.
- direct codegen uses safe `std::rc::Rc`; there is no `Arc`, `RefCell`, synchronization, wrapper ownership runtime, hidden deep clone or unsafe emulation.
- record/enum storage of shared owners, cross-thread ownership, interior mutability, `Weak`, cycle solving, general generics and generalized lifetime machinery remain excluded.

## Performance contract

Compare against equivalent idiomatic Rust `Rc<T>` doing exactly the same ownership work. Correctness must match first. The committed Rust reference must mirror generated static Rust exactly so scheduler noise cannot masquerade as a generated-code regression when executable bytes are identical.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. If one gate is queued/running, advance independent work and return to that gate later. Failed SHAs remain evidence and are not rerun merely for a better color.
