# Rust Evolution — Project State

Last verified update: **2026-09-18**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Exact verified stable main: `368eb9a07ad423fa0a616715a7693457b43ff903`
- Rust toolchain: **1.98.0**
- Production flags: edition 2024, opt-level 3, codegen-units 1
- Natural exact-main CI #579 / run `35083606472`: **SUCCESS** on Ubuntu 24.04, Windows and macOS
- Natural exact-main Generational arena surface research #21 / run `35083606469`: **SUCCESS**
- Natural exact-main Append-only sequence performance #24 / run `35083606389`: **SUCCESS**
- Natural exact-main Explicit shared owner performance #67 / run `35083606392`: **SUCCESS**

`368eb9a...` is PR #143 merge, completing #142 generational-arena surface research. PR #141 previously merged append-only sequence v0 and closed #140.

## Ownership / collection foundations already on main

### Immutable references and explicit shared owners

The bounded `&T` reference model, inferred call-duration shared borrows, and explicit one-thread `shared T` / `share` / `dup` owners remain implemented. Shared-owner duplication is explicit; ordinary moves/calls/returns do not insert hidden refcount traffic.

### Append-only sequences — #140 / PR #141 completed

Production `seq T`, explicit `append`, and checked `lookup ... else ... end` lower directly to safe `Vec<T>` / `push` / `get`. Move-only element references participate in the existing bounded final-use liveness model. Sequence removal/reuse remains separate from arena semantics.

### Generational arena research — #142 / PR #143 completed

Accepted result: **RUNTIME-ARENA-ID-GENERATIONAL-CANDIDATE / CONTEXTUAL-ARENA-HANDLE-CANDIDATE**.

The accepted identity is `(arena id, slot index, generation)`; reusable slots advance generation, generation exhaustion retires a slot, and arena-id exhaustion must fail closed.

## Active P0 production — #144 generational arena v0

Branch:

`feature/generational-arena-v0`

Validated production-gate head before the final documentation/PR commit:

`e59f41fc9a4f58bb352d663358fc1b12a130f445`

Implemented source surface:

```text
items = arena Item()
insert items, Item(value = 1) as h

lookup items, h as item
    print item.value
else
    print 0
end

remove items, h as removed
    print removed.value
else
    print 0
end
```

Type forms:

```text
arena Item
handle Item
```

Implemented invariants:

- `arena`, `handle`, `insert`, and `remove` remain contextual identifiers outside exact forms;
- `arena T` is move-only owned storage for the bounded scalar/record/explicit-shared-owner payload set;
- `handle T` is copy-like fixed-size identity carrying arena id, slot index, and generation;
- same-payload arenas are runtime-distinct; stale, wrong-arena, vacant, and out-of-range handles take checked failure branches;
- insert consumes move-only payloads without hidden clone and returns a fresh handle;
- removal moves payload ownership out once, invalidates the old handle, increments generation before reuse, and retires a generation-max slot;
- arena-id exhaustion fails closed;
- live move-only element references block insert, remove, arena move, and reinitialization until bounded final-use release;
- `handle T` works in function contracts, `handle Record` works in record fields without recursive-layout classification, and `seq handle T` supports adjacency-list-class storage;
- shared-owner lookup/removal does not insert hidden `Rc::clone`;
- generated support code is ordinary safe Rust with `Vec`, `Option`, `Cell`, and `PhantomData`; no unsafe pointer identity, registry, GC, `RefCell`, lock, or hidden refcount layer.

Validation evidence:

- Dev arena semantics v0 run `35118469618`: focused semantic/runtime tests, workspace regression, and Clippy `-D warnings` **SUCCESS**;
- Generational arena performance run `35120462329` on head `e59f41fc...`: focused parser/formatter/lowering/codegen/runtime tests **SUCCESS**;
- differential correctness **true**, exact binary **true**, stable **true**, final verdict **PASS** by `byte-identical-binary-parity`;
- observed timing ratio **1.000613481** is retained as timing-only FAIL evidence rather than rounded away;
- artifact `10457925773`, digest `sha256:4852ab5c61da6c0e2848efff148beaba9fd2dcbafa1dbe650627bb3c04667a39`;
- the earlier independent timed run `35119416363` remains retained failed evidence at ratio `1.001201306`.

Permanent regression workflow: `.github/workflows/generational-arena-performance.yml`.

## Explicit exclusions

Do not silently fold these into #144:

- general Evolution generic syntax;
- general-purpose vectors/maps/sets or algorithms;
- mutable references;
- cross-thread `Arc`/synchronization;
- static per-runtime-arena-instance type provenance;
- independent payload lifetime outside arena ownership;
- hidden clone/refcount/GC;
- process-global handle registries/tables;
- unsafe pointer identity;
- arena nesting/arbitrary generic payload composition;
- unrelated allocator tuning.

## Current operational sequence

PR #145 is open from `feature/generational-arena-v0` to exact main `368eb9a...`.

1. Keep one exact PR head and fix only evidence-backed gate failures; historical failed heads remain evidence.
2. Require normal Ubuntu/Windows/macOS CI, Generational arena performance, and all naturally triggered append-only/shared-owner/research regressions on that exact head.
3. Review the exact final diff and merge only with expected-head protection.
4. Require natural exact-main postmerge CI and Generational arena performance before closing #144 completed.
5. Choose the next graph/ownership successor only after #144 is durably closed; do not smuggle Weak, interior mutability, cross-thread ownership, or generalized generics into this slice.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Historical failed SHAs remain evidence and are not rerun merely for cosmetic green. CI running does not block independent source/docs work.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
