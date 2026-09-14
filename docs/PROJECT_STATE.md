# Rust Evolution — Project State

Last verified update: **2026-09-14**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Current exact verified stable main: `c5ccc21d7bf23d8daec2de36dc635c0f85313605`
- Rust toolchain: **1.98.0**
- Production flags: edition 2024, opt-level 3, codegen-units 1
- Natural exact-main CI #512 / run `34834431873`: **SUCCESS** on Ubuntu 24.04, Windows and macOS
- Natural exact-main Explicit shared owner performance #12 / run `34834431791`: **SUCCESS**
- Performance artifact id `10343513286`
- Performance digest `sha256:fb6714e7f09ca0047c70435b21129128033d8235732ca1d9f82b6fdeab9716bf`

## Build / compile sequence

- #76: build latency baseline established; single-file native builds are rustc-dominated.
- #79 / PR #81: verified unchanged-build cache accepted.
- #82 / PR #84: changed-source rustc incremental research **REJECT / DEFER**.
- #85 / PR #86: link-time attribution established the current `rustc -> cc -> lld` path.
- #87 / PR #88: alternative linker experiment **REJECT / DEFER**.
- #89 / PR #90: opt-level 3 -> 2 candidate **REJECT / DEFER**.
- #91 / PR #92: compile-memory baseline **DEFER / NO ACTION**.
- #93 / PR #94: binary-size baseline **DEFER / NO ACTION**; controlled Evolution/reference binaries were byte-identical across the accepted corpus.

Dependency-build, proc-macro cost and workspace scaling remain deferred until Evolution has a real package/dependency graph.

## Ownership ergonomics implemented

### Inferred shared-borrow parameters

`Owned` and call-duration `SharedBorrow` parameter modes are distinct. SharedBorrow is non-owning and does not create a stored or escaping reference.

### First-class immutable references

`&T` / `&expr` are implemented for the bounded nominal slice with deterministic provenance, stored reference locals, source-owner move/reinitialization conflicts, bounded final-use liveness and direct safe Rust reference lowering.

### Explicit one-thread shared owners — #112 / PR #117 completed

Production source surface:

```text
shared Item
share expr
dup owner
```

Direct generated Rust mapping:

```text
shared Item -> std::rc::Rc<__EvoRecord_Item>
share expr  -> std::rc::Rc::new(expr)
dup expr    -> std::rc::Rc::clone(&expr)
```

Implemented contracts:

- `SharedOwner` is distinct from owned records, first-class `SharedRef`, and inferred `SharedBorrow`;
- `shared Item` is accepted in the bounded function parameter/return surface;
- `share expr` explicitly allocates the first shared owner and requires an owned nominal record operand;
- `dup expr` explicitly creates another available owner handle;
- ordinary assignment, by-value parameter passing and returns move handles with no implicit strong-count increment;
- moved shared-owner locals may be explicitly reinitialized with the same `shared T` type;
- `dup` of a moved handle rejects rather than reviving it;
- scalar payload fields are readable through shared owners;
- move-only nominal payload extraction through shared ownership rejects instead of cloning;
- no implicit conversion exists among owned `T`, `shared T`, and `&T`;
- payload references derived from a shared owner remain ordinary immutable references tied to that specific source handle;
- source-handle move/reinitialization rejects while such a reference may still be live;
- a different duplicated handle may move independently;
- bounded final-use analysis releases the source handle after the final proven dependent-reference use;
- branch/repeat ownership joins retain existing conservative move rules;
- `shared`, `share`, and `dup` remain contextual identifiers rather than global keywords, preserving existing ordinary calls/names;
- record fields and enum payloads containing shared owners are outside v0;
- enum-bearing unsupported reference/shared-owner combinations fail closed before executable enum IR/codegen.

Generated code contains no Evolution ownership wrapper, global runtime ownership registry, hidden payload clone, `Arc`, `RefCell`, lock, synchronization, GC, unsafe block or invented lifetime widening.

### Permanent equivalent-Rc performance gate

The permanent benchmark compares Evolution against idiomatic Rust performing the same ownership work: one `Rc` allocation, explicit owner duplication, equivalent forwarding/field reads/drops.

Final exact-main evidence:

- Explicit shared owner performance #12 / run `34834431791`: **SUCCESS**;
- artifact id `10343513286`;
- digest `sha256:fb6714e7f09ca0047c70435b21129128033d8235732ca1d9f82b6fdeab9716bf`;
- artifact head SHA exactly `c5ccc21d7bf23d8daec2de36dc635c0f85313605`.

Accepted component evidence also established normalized LLVM IR equality and byte-identical Evolution/reference executables for the equivalent-Rc corpus.

## Active research lanes

The following are deliberately separate cost/safety models and are not production behavior merely because the Rc-like owner is implemented.

### #118 — cyclic / graph ownership boundaries

PR #120 is parked closed while its five persistent research-only files are reconstructed onto current verified main.

Dedicated evidence on historical exact research head `4c6a7e4f1d24388d3081e28ca88c73f142f88a68`:

- Cyclic graph ownership research #12 / run `34832221728`: **SUCCESS**;
- artifact id `10342831245`;
- digest `sha256:5ebf114519378c431f6cee33181de569ef5d4cd6e54e58f09cf3e502b43612ba`;
- verdict **SPLIT-RESEARCH**;
- 15 cases, zero compile/runtime expectation mismatches;
- explicit Weak-edge and arena/index candidates remain separate;
- strong-cycle retention, weak-edge destruction, stale-index identity risk, generation-checked handle behavior, interior-mutability/concurrency/self-reference boundaries are separately demonstrated.

Gated successors:

- #125 — explicit Weak-edge surface v0;
- #126 — arena/generational graph handles v0.

Neither successor may start until #118 merges and natural exact-main normal CI plus dedicated cyclic-graph research both pass.

### #121 — interior mutability ergonomics

PR #122 is parked closed with a clean research-only branch containing:

- pinned Rust 1.98 `RefCell<T>` / `Rc<RefCell<T>>` semantics matrix;
- runtime conflict/panic and fallible borrow cases;
- direct guard-state lifetime controls;
- dedicated research workflow;
- durable report.

Ownership and dynamic borrow-state cost remain separate. No hidden `RefCell`, hidden Rc, synchronization or production mutation syntax is approved.

### #123 — cross-thread shared ownership

PR #124 is parked closed with a clean research-only branch containing:

- `Arc<T>` ownership and atomic refcount controls;
- `Rc` cross-thread rejection;
- thread-capability boundary cases;
- `Mutex`, `RwLock`, atomic-payload synchronization contrasts;
- Weak/deep-clone/single-thread cost contrasts;
- dedicated research workflow and durable report.

No automatic `Rc -> Arc` upgrade, hidden synchronization or production concurrency surface is approved.

## Current explicit non-goals

Not yet production-approved:

- cross-thread `Arc` ownership / Send-Sync-like capability rules;
- interior mutability;
- locks / synchronized shared mutation;
- Weak/cycle-edge production semantics;
- arena/index/generational graph handles;
- mutable references;
- generalized/user-written lifetime solving;
- general generic type syntax;
- implicit shared ownership inference;
- implicit allocation or owner duplication;
- hidden payload deep clone;
- shared-owner record fields / enum payloads;
- generalized shared-owner type algebra;
- GC/global runtime ownership tables;
- unsafe ownership emulation.

## Next operational sequence

1. Merge the small post-#112 docs handoff and verify its natural exact-main normal CI.
2. Reconstruct PR #120's five research-only files onto that verified main and reopen #120.
3. Require exact-head normal CI plus dedicated cyclic research before merge; then require both natural postmerge gates before #118 closes.
4. While #120 Actions run, reconstruct #122 and #124 onto the same verified main while keeping them closed to avoid redundant runner pressure.
5. Unlock #125/#126 only after #118 postmerge validation.
6. Validate #121 and #123 independently and create narrower production successors only when the evidence supports them.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed/cancelled historical SHAs remain evidence and are not rerun merely for color. CI running does not block independent work.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
