# Rust Evolution — Project State

Last verified update: **2026-09-23**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Exact verified stable main: `36022b386782775e449ae6a6eedcbbb8c8d1275f`
- Rust toolchain: **1.98.0**
- Exact-main CI `35842635637`: **SUCCESS** on Ubuntu 24.04, Windows and macOS
- Exact-main Generational arena surface research `35842635505`: **SUCCESS**
- Exact-main Append-only sequence performance `35842635557`: **SUCCESS**
- Exact-main Explicit shared owner performance `35842635675`: **SUCCESS**
- Exact-main Generational arena performance `35842635563`: **SUCCESS**

## Accepted production foundations

### Explicit shared ownership

`shared T`, `share expr`, and `dup owner` remain explicit one-thread `Rc<T>`-class ownership. Ordinary assignment, call, and return move handles; only `dup` performs explicit owner duplication.

### Append-only sequence v0

`seq T`, `append`, and checked `lookup ... else ... end` lower directly to safe Rust `Vec<T>` / `push` / `get`. Move-only element references use bounded final-use liveness.

### Generational arena v0

`arena T` / `handle T` support explicit insert, checked lookup, and checked removal. Handles identify runtime arena id + slot + generation. Stale, wrong-arena, vacant, and out-of-range handles fail through explicit branches. Slot reuse advances generation; generation-max slots retire.

### Dynamic borrow guard research — #134 / PR #147 completed

Accepted research decision:

- **LEXICAL-GUARDS-FIRST**
- owned cell surface **EXPLICIT-OWNED-CELL**
- `cell T` / `cell expr`
- lexical `borrow`, `borrow_mut`, `try_borrow`, `try_borrow_mut`
- returned/escaping guards remain outside the bounded surface
- payload mutation syntax is intentionally separated into #148

Exact-head research artifact: `10642096339`, digest `sha256:3b9ee0fcabf028ce82c694af84df4dccb054e658546ab97d77f80da25c04e358`.

Postmerge exact-main research artifact on `b3b3eebe5c1d41b47e26e3bb2bc00db5a34c32c9`: `10643971376`, digest `sha256:007ab949a38a6ca060953e7007ec09d90b8a09c9f671e19e2b1ad9442dcd57a7`.

### Weak edge v0 — #138 research + #137 production completed

Accepted production surface:

~~~text
fn forward(edge weak Item) weak Item
    return edge
end

owner = share Item(value = 1)
edge = downgrade owner

upgrade edge as live
    print live.value
else
    print 0
end
~~~

Locked invariants:

- `weak T`, `downgrade`, and `upgrade` remain contextual;
- `weak T` is bounded to nominal record local/function contracts in v0;
- ordinary Weak values are move-only;
- `downgrade owner` inspects a `shared T` non-consumingly and lowers to `Rc::downgrade(&owner)`;
- `upgrade edge as owner ... else ... end` inspects the Weak non-consumingly and binds lexical `shared T` only in the success branch;
- repeated upgrade checks do not consume the Weak handle;
- no first-class Option/result owner value is added;
- record fields, enum payloads, sequence elements, arena payloads, and enum-bearing Weak execution remain fail-closed in this first slice;
- generated Rust uses direct safe `std::rc::Weak<T>`, `Rc::downgrade`, and `Weak::upgrade`;
- no hidden clone, implicit strong-count increment, registry, GC, unsafe, or ownership runtime wrapper.

Research acceptance:

- #138 / PR #146 exact-head artifact `10651252644`, digest `sha256:1401fbcd09db781b8ee2d23d602ec60397d176dbbaa57443737956d30cff1f4f`;
- verdict **SCOPED-UPGRADE-CANDIDATE**, 15 cases, 0 mismatches;
- research squash merge `bef3e1b7def9869fe4a8947b846ac6f3f481bd3b`.

Production acceptance:

- PR #155 final head `5cac4516d0bf035ef53706832812aa881e877dde`;
- exact-head CI `35745511907`: **SUCCESS** on all three OSes;
- squash merge / current main `36022b386782775e449ae6a6eedcbbb8c8d1275f`;
- exact-main CI `35842635637`: **SUCCESS** on all three OSes;
- exact-main arena research artifact `10741608107`, digest `sha256:1e0c527c56448e2cbcd3a9f29ba873759b4d343d7eaf85688963e339e09529f2`;
- exact-main sequence artifact `10742151734`, digest `sha256:d1245947dc7015d757ccae7e3e9d4d8f4f48424e3d95fd56c8368b7e03b8834d`;
- exact-main shared-owner artifact `10742411398`, digest `sha256:4ff6f462ba295f589122d3f955d3237d79ccfefeecdc39d43852d8a2d929621d`;
- exact-main arena artifact `10742505232`, digest `sha256:8e6ab407c6d6717fa2b5a705a7a04dd4a85c942e6dc16c5c3586e3cff0195246`.

## Active P0 research — #148 / PR #156

Issue #148 researches the smallest mutation surface for an exclusive lexical dynamic-borrow guard without silently adding general mutable references or mutable-place syntax.

Branch: `research/exclusive-guard-mutation-surface-v0`

Current exact head: `6a223c89352bb380cc6b00ee6379c3fc095f8271`

PR #156 is draft. Scope is exactly:

- dedicated workflow;
- 21-case executable Rust matrix;
- research document.

Pre-registered candidate: **WHOLE-PAYLOAD-REPLACE-CANDIDATE**, source spelling `replace guard with expr`.

Direct field/index mutation and escaping guards remain unauthorized. The first PR head failed only rustfmt; current replacement head carries the exact formatter output and natural replacement runs.

## Newly unblocked P0 research — #153

#153 Weak storage surface research is now unblocked because #137 is complete.

Branch: `research/weak-storage-surface-v0`

Start/base main: `36022b386782775e449ae6a6eedcbbb8c8d1275f`

No research files or PR have been created yet at this handoff.

Primary decision families:

- **WEAK-RECORD-FIELD-CANDIDATE**
- **WEAK-RECORD-AND-SEQUENCE-CANDIDATE**
- **ENUM-INTEGRATION-FIRST**
- **NESTED-TYPE-INFRASTRUCTURE-FIRST**
- **DEFER**

Research must distinguish fixed-size Weak handle storage from recursive by-value payload layout and must not smuggle in hidden strong ownership, GC, general generics, or enum integration.

## Prepared infrastructure lanes

Prepared branches have no open PR unless live GitHub says otherwise.

- #149 benchmark provenance: `infra/benchmark-provenance-v0`, head `1bce806e3d8fb582dab79c4421515fa5ae24a640`
- #150 research workflow provenance: `infra/research-workflow-provenance-v0`, head `0a2664a6dc3c073d5033b51f9117e85473a6d667`
- #151 tooling evidence provenance: `infra/tooling-evidence-provenance-v0`, stacked head `264103cc99e8fa62146dbb7ebdb06dd3da97621b`

## Separate blocked lane

#133 cross-thread shared-ownership research still needs a fresh exact-current-main dedicated research dispatch. Historical exhausted/cancelled attempts are not to be cosmetically rerun.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Historical failed SHAs remain evidence. A new SHA gets natural replacement runs.

Before merging a PR:

1. verify exact current head and base;
2. require all mandatory exact-head gates;
3. inspect artifacts/provenance;
4. merge with expected-head protection;
5. require natural exact-main postmerge evidence before closing the issue completed.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
