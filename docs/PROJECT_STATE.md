# Rust Evolution — Project State

Last verified update: **2026-09-21**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Exact verified stable main: `b3b3eebe5c1d41b47e26e3bb2bc00db5a34c32c9`
- Rust toolchain: **1.98.0**
- Production flags: edition 2024, opt-level 3, codegen-units 1
- Exact-main CI run `35610602607`: **SUCCESS** on Ubuntu 24.04, Windows and macOS
- Exact-main Dynamic borrow guard surface research `35610602634`: **SUCCESS**
- Exact-main Append-only sequence performance `35610602465`: **SUCCESS**
- Exact-main Explicit shared owner performance `35610602480`: **SUCCESS**
- Exact-main Generational arena performance `35610602782`: **SUCCESS**

`b3b3eebe...` is PR #147 squash merge. It follows generational arena production merge #145 at `0748f767...`.

## Implemented foundations on main

### Immutable references and explicit one-thread shared owners

The bounded `&T` reference model, inferred call-duration shared borrows, and explicit `shared T` / `share` / `dup` owners remain implemented. Shared-owner duplication is explicit; ordinary moves, calls and returns do not insert hidden refcount traffic.

### Append-only sequence v0

Production `seq T`, explicit `append`, and checked `lookup ... else ... end` lower directly to safe `Vec<T>` / `push` / `get`. Move-only element references use the bounded final-use liveness model.

### Generational arena v0 — #144 / PR #145 completed

Production `arena T` / `handle T` supports explicit insert, checked lookup and checked removal. Handles identify runtime `(arena id, slot index, generation)`; stale, wrong-arena, vacant and out-of-range handles fail through explicit branches. Reusable slots advance generation and generation-max slots retire. The generated runtime remains ordinary safe Rust without unsafe identity, registries, GC, `RefCell`, locks or hidden owner duplication.

Natural exact-main evidence on `0748f7672028429c962880a09471f37ada896dc5` was green for CI, arena performance, arena surface research, append-only sequence performance and explicit shared-owner performance.

### Dynamic borrow guard research — #134 / PR #147 completed

Accepted research decision:

- **LEXICAL-GUARDS-FIRST**
- recommended owned-cell surface **EXPLICIT-OWNED-CELL**
- type surface `cell T`
- constructor surface `cell expr`

The accepted bounded research contract keeps `cell`, `borrow`, `borrow_mut`, `try_borrow`, and `try_borrow_mut` contextual. Guards are lexical; panicking and fallible acquisition remain distinct; returned/escaping guard values are not authorized.

Exact-head evidence on `7acb51495799d3b25e2f9dd9f425011584ad22fe`:

- CI `35596923275`: **SUCCESS**
- guard research `35596923400`: **SUCCESS**
- research artifact `10642096339`, digest `sha256:3b9ee0fcabf028ce82c694af84df4dccb054e658546ab97d77f80da25c04e358`
- 21 cases, 0 mismatches, `LEXICAL-GUARDS-FIRST`, `EXPLICIT-OWNED-CELL`

Postmerge exact-main evidence on `b3b3eebe...`:

- CI `35610602607`: **SUCCESS** on all three OSes
- guard research `35610602634`: **SUCCESS**
- research artifact `10643971376`, digest `sha256:007ab949a38a6ca060953e7007ec09d90b8a09c9f671e19e2b1ad9442dcd57a7`
- sequence artifact `10643444016`, digest `sha256:1011215e19c1aca097e07b2845bc4affb0afd864515dba74037990a268cd77d7`
- shared-owner artifact `10643468660`, digest `sha256:de8e8840629f7597edda8f4ab5a8a3746a368e84cd27ef8b09315d3b29f90ee3`
- arena artifact `10643878478`, digest `sha256:96027675e12c7b7afc5f130bce00ac361ba596299d95b0d88b4e2a6906a7a9e7`

Issue #134 is closed completed. This is an accepted **research decision**, not yet production language semantics.

## Active P0 ownership lane — #138 / PR #146

Issue #138 researches an explicit checked result surface for upgrading a one-thread Weak owner edge.

Branch:

`research/weak-upgrade-result-v0`

Current exact base:

`b3b3eebe5c1d41b47e26e3bb2bc00db5a34c32c9`

Current exact head:

`3b07d49d14d46de4f2e45ee228960b3fd1ac1ba0`

Candidate research surface:

~~~text
upgrade edge as owner
    ...
else
    ...
end
~~~

Leading derived verdict remains **SCOPED-UPGRADE-CANDIDATE**: immediate checked branching is sufficient for the bounded first slice, with no first-class Option/result source value required.

Current gate state at this handoff:

- normal CI Windows: **SUCCESS**
- normal CI macOS: **SUCCESS**
- normal CI Ubuntu: queued
- dedicated Weak research: queued
- sequence/shared-owner/arena performance gates: queued

Historical failed heads remain evidence. The latest semantic fix changed the move-after-move compile diagnostic pin from an English fragment to Rust error code `E0382`; the test case and intended semantics are unchanged.

## Hard-gated production successor — #137

Do not create the production Weak branch until #138 / PR #146 merges and natural exact-main CI plus exact-main dedicated Weak research are green.

Prepared first-slice plan:

- contextual `weak T`
- non-consuming `downgrade owner`
- checked `upgrade edge as owner ... else ... end`
- local/function Weak handles first
- success binding is branch-local `shared T`
- direct safe Rust `std::rc::Weak<T>`, `Rc::downgrade(&owner)`, and `Weak::upgrade(&edge)`
- no unwrap, hidden clone, registry, GC, unsafe or generalized Option/result source value
- unsupported Weak storage shapes remain fail-closed in the first slice

## Interior-mutability successor — #148

#148 is now unblocked by #134 completion.

Preparation branch:

`research/exclusive-guard-mutation-surface-v0`

Prepared clean head:

`7fa1a81c122d981c17b28a19a5ea7e69c6e9e4ed`

No PR is open yet. The research-only 21-case matrix compares whole-payload replacement, field/nested/index mutable-place boundaries, an explicit-update wrapper, guard move/escape behavior, Rc+RefCell composition and cross-thread synchronization. The pre-registered candidate is **WHOLE-PAYLOAD-REPLACE-CANDIDATE** with research spelling `replace guard with expr`. Direct field/index mutation remains unauthorized.

## Prepared infrastructure lanes

These are prepared branches, not accepted main changes. Rebase them onto the then-current exact main before opening a PR.

### #149 benchmark provenance v0

Branch `infra/benchmark-provenance-v0`, prepared clean head `a091de92d58fbc5df8583ec45098b86a3d2b7ca7`.

Adds benchmark report schema v3 provenance: exact source SHA, single-source build flags, host/CI/runner identity, target/rustc identity and optional Linux CPU/governor metadata. Global CI and permanent performance workflows validate the report identity against the commit actually executed.

### #150 research workflow provenance v0

Branch `infra/research-workflow-provenance-v0`, prepared clean head `44f0618ccf102b18b4f3a061548c126fcdcfd8b1`.

Makes 14 legacy research workflows checkout and verify the exact SHA they record. It also adds replacement concurrency to the one legacy research workflow that lacked it.

### #151 tooling evidence provenance v0

Stacked branch `infra/tooling-evidence-provenance-v0`, prepared head `dfd3972bf1577a5af156d93854fd1baf8ada6fb1`.

This is a four-line `.github/workflows/ci.yml` delta over #149 so tooling evidence records the global CI commit actually measured.

## Other blocked research

#133 cross-thread shared-ownership research remains blocked. Current exact-main normal CI is green, but a current-main Cross-thread shared ownership research dispatch is still required before opening the successor implementation lane. Historical exhausted/cancelled attempts are not to be cosmetically rerun.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Historical failed SHAs remain evidence. A new SHA gets natural replacement runs; do not repaint an old SHA.

Before merging a PR:

1. verify exact current head;
2. verify exact base/current merge state;
3. require all mandatory exact-head gates;
4. inspect artifacts and provenance;
5. merge with expected-head protection;
6. require natural exact-main postmerge evidence before closing the issue completed.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
