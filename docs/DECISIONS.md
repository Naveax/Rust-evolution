# Rust Evolution — Durable Decisions

This file records decisions that future sessions should not casually re-litigate unless new evidence justifies a change.

## D-001 — GitHub is the durable project memory

**Decision:** Repository state, tests, specs, issues, PRs, CI artifacts and handoff docs are authoritative. Chat history is optional context.

**Reason:** New sessions may not have prior conversation context. Continuity must survive chat boundaries.

## D-002 — Current backend remains Rust/rustc first

**Decision:** Short-term compilation remains `Evolution -> Rust -> rustc -> native`.

**Reason:** This preserves Rust/rustc/LLVM semantics while the frontend and language model are still evolving. Direct LLVM/GPU/etc. backends come only after typed semantics/HIR and backend-neutral IR are justified.

## D-003 — No mandatory VM / GC for core

**Decision:** Core Evolution does not require a VM, mandatory GC, or managed runtime.

**Reason:** The project preserves Rust-class native performance and explicit cost. Optional managed capabilities may exist later, but must be explicit.

## D-004 — Correctness before performance

**Decision:** Performance evidence is invalid unless observable behavior matches the reference baseline first.

## D-005 — Strict zero-cost/native runtime contract

**Decision:** For equivalent semantics under controlled conditions, accepted zero-cost core features target `T_evolution <= T_reference_rust`.

Stable `ratio > 1.00` is FAIL. Unstable timing is INCONCLUSIVE. Ergonomics does not buy a runtime exception.

## D-006 — Exact binary parity is deterministic runtime parity evidence

**Decision:** If independently compiled reference and Evolution outputs are byte-identical after correctness PASS, they are runtime-equivalent by construction. Timing noise remains visible but cannot turn the same executable into a real regression.

## D-007 — User-facing spec is implementation-backed only

**Decision:** `docs/LANGUAGE_SPEC_V0.md` describes only behavior proven on `main`.

**Reason:** Vision and future design must not be mistaken for implemented language capability.

## D-008 — Omni Vision is a north star, not a giant core grammar

**Decision:** Ideas from the Omni/full-stack vision must be classified as Core, Profile/Capability, Library, Optional Runtime, Backend, or Tooling before implementation.

**Reason:** One language must not become a pile of unrelated domain dialects.

## D-009 — Profiles are capabilities, not language forks

**Decision:** Future profiles such as `gpu`, `embedded`, `verified`, `web`, or `distributed` should primarily select semantic capabilities, validation rules, libraries, backend support and runtime/cost requirements. Core constructs keep shared meaning.

## D-010 — Cost classes are semantic first; annotation syntax is not frozen

**Decision:** Long-term cost classes are conceptually ZERO / EXPLICIT / MANAGED. Do not freeze `@zero/@explicit/@managed` syntax until the semantic model and analyzer exist.

## D-011 — Hidden costs are forbidden in core zero-cost features

**Decision:** Do not silently introduce allocation, clone, boxing, dynamic dispatch, reference counting, managed runtimes, or hidden data transfers in features presented as zero-cost core ergonomics.

## D-012 — Unsafe remains explicit

**Decision:** Safety ergonomics may improve, but unsafe operations/boundaries cannot become invisible.

## D-013 — Function v0 surface

**Decision:** Functions v0 uses compact explicit signatures:

```text
fn add(a int, b int) int
    return a + b
end
```

- explicit parameter and return types;
- fixed arity;
- top-level declarations;
- declarations appear before executable top-level statements in v0;
- forward calls and direct recursion allowed through signature pre-pass;
- no unit return in v0;
- no closure/first-class function/dynamic dispatch;
- current `string` function ABI is `&'static str` because string values are currently literal/static; owned-string semantics are deferred.

## D-014 — Strict logical operators

**Decision:** `and/or/not` are boolean-only. There is no truthiness or implicit integer/string-to-bool conversion. `and/or` short-circuit exactly.

## D-015 — Block locals v0 direction

**Decision:** #38 should introduce lexical block-local bindings without implicit branch merging.

- child locals do not escape the block;
- assignment to a visible outer binding is reassignment, not shadowing;
- sibling scopes are independent;
- zero-iteration repeat cannot leak a local;
- generated code uses ordinary Rust lexical bindings only;
- no runtime environment map/object is allowed;
- mutability tracking should use declaration identity, not only variable names.

## D-016 — Temporary mutation workflows are temporary

**Decision:** Development workflows that edit/commit source automatically must be removed once their one-shot purpose is complete. Normal CI remains the authoritative validation path.

## D-017 — `evo run` cache is verified tooling state, not language/runtime semantics

**Decision:** Fast edit-run caching may reuse a previously compiled native artifact only after the current Evolution source has passed frontend validation/codegen and the cache entry's exact compilation identity has been verified.

- a hash/key may locate candidates but is never sufficient proof of identity;
- exact Evolution source, generated Rust, and compiler/configuration identity are verified on hit;
- incomplete, corrupt, missing, or mismatched entries fail closed to recompilation;
- the default cache is per-user, with `EVO_CACHE_DIR` as an explicit override;
- `evo run --no-cache` remains an explicit bypass;
- v0 may compile redundantly under races rather than risk executing a partial artifact;
- no remote cache, daemon, executable download, VM, GC, or incremental-rustc machinery is implied;
- generated Rust bytes and generated-program runtime semantics must remain unchanged by whether a cache hit occurs.

**Reason:** The rapid edit-run weakness is a developer-tooling latency problem. Solving it must not weaken source validation, cache correctness, security boundaries, or the independent runtime parity contract.

## D-018 — Move provenance diagnostics are bounded compile-time metadata

**Decision:** Source-native move diagnostics may retain structured provenance for existing move-only ownership semantics, but that provenance is compiler diagnostic state only.

- the invalid reuse remains the primary Evolution source location;
- v0 renders at most one related Evolution-source location for the move origin;
- provenance records a source span and bounded reason, not generated/runtime metadata;
- continuing-path joins choose provenance deterministically by source order;
- terminal paths remain excluded from continuing-state joins;
- repeat analysis may retain the responsible body move location;
- exact same-type reinitialization clears stale provenance;
- lexical scope exit removes provenance with the binding;
- missing-binding and type-mismatch errors must not inherit unrelated move provenance;
- the public `LowerError { message, span }` shape remains compatible for this bounded slice;
- a matching one-shot compile-time diagnostic sidecar may transport the related location without making rendered text part of semantic correctness;
- accepted generated Rust bytes and generated-program runtime semantics must remain unchanged;
- no persistent runtime metadata, clone, allocation, boxing, managed ownership or dispatch is implied.

**Reason:** The ownership learning/refactoring weakness can be improved substantially by pointing to the actual Evolution move origin. Doing so does not justify changing ownership semantics, forcing a repository-wide error-type migration, or charging generated programs for compiler diagnostics.

Evidence for the accepted decision is PR #71, final head `74f6a5955d46bd9620045bd39e9703383ae30679`, CI #301 / run `34141025715`, squash merge `795461c53f896c2223443cdb022f340a5032a0bd`, and post-merge main CI #302 / run `34158551578`.

## Changing a decision

A future change should record:

1. which decision is being superseded;
2. new evidence or requirement;
3. correctness/safety/cost implications;
4. migration impact;
5. benchmark/CI evidence where applicable.
