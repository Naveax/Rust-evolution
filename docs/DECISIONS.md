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

**Decision:** Ideas from `docs/OMNI_VISION.md` or language research must be classified before implementation as Core, Profile/Capability, Library, Optional Runtime, Backend, or Tooling.

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

## D-019 — Diagnostic spelling help is conservative, namespace-specific and non-semantic

**Decision:** Source-native spelling suggestions may improve existing unknown-symbol diagnostics only as bounded compile-time help. They must not change parsing, semantic resolution, accepted/invalid program classification, ownership rules, generated Rust, or generated-program runtime behavior.

For the accepted v0 policy:

- current ASCII identifiers use a deterministic one-edit matcher;
- supported one-edit shapes are insertion, deletion, substitution and adjacent transposition;
- a suggestion is emitted only when exactly one best candidate is inside the fixed conservative threshold;
- equal-best ties, distant candidates, one-character guesses and empty candidate sets stay silent;
- local candidates are limited to lexically visible bindings;
- functions are suggested only from declared functions;
- record type/constructor/constructor-field/field diagnostics use record-specific candidate sets;
- enum type/constructor/variant/match-variant diagnostics use enum-specific candidate sets;
- no cross-namespace fuzzy global symbol search is introduced;
- the existing primary diagnostic message and span remain authoritative;
- textual `help:` is additional presentation metadata, not semantic correctness state;
- help metadata is keyed/consumed against exact primary `(message, span)` identity so stale state cannot attach to another error;
- textual help and D-018 move-origin related locations may coexist without overloading one representation with two meanings;
- parser autocorrection, source mutation, multi-suggestion ranking, global symbol indexing and LSP/code actions are separate future work, not implied by this decision;
- suggestion metadata remains compiler-only and has generated-program runtime cost **ZERO**.

**Reason:** The compiler already owns precise context-specific candidate sets. Conservative bounded help improves diagnosis/refactoring ergonomics without inventing user intent, widening namespaces, or turning a rejected program into an accepted one.

Accepted evidence: PR #75 final head `1f3fdec69f81c8d6742d3fde47d698a3df3f3543`, CI #319 / run `34197970982`, squash merge `cd6dcd096af20f9f94c4f201e715ae87c848c480`, and post-merge main CI #320 / run `34198404953`.

## D-020 — Build latency optimization follows measured attribution

**Decision:** For the current single-file native build path, optimize the dominant measured compile/link cost before spending effort on the already-small frontend path. The first implementation target after the accepted #76 baseline is verified unchanged-build native artifact reuse. Changed-source incremental compilation remains a separate research problem.

Accepted #76 evidence on the controlled Ubuntu runner:

- `evo check` median: **1.207 ms**;
- `evo emit-rust` median: **1.210 ms**;
- cold `evo build` median: **99.970 ms**;
- unchanged warm `evo build` median: **96.986 ms**;
- edited `evo build` median: **95.515 ms**;
- direct rustc compile/link median: **94.131 ms**;
- cold/warm/edit rustc invocation count: **5/5** in every class.

Policy implications:

- subtraction of separately sampled build and rustc medians is a rough attribution signal only, not causal profiling;
- exact invocation counts plus correctness are stronger evidence than one favorable timing sample;
- verified unchanged-build reuse must still run normal frontend validation/lowering/codegen before any cache hit is accepted;
- a build cache hit must verify exact Evolution source, generated Rust and compiler/configuration identity rather than trusting a lookup hash alone;
- build artifact reuse is tooling state and must not alter generated Rust or generated-program runtime behavior;
- the accepted #67 `run-cache-v0` behavior must not be silently changed merely to share implementation machinery;
- changed-source incremental-rustc/session work is not implied by exact artifact reuse and requires separate design/evidence;
- no daemon, remote executable download, linker replacement, package/dependency redesign or runtime-language change follows from this decision.

**Reason:** #76 shows that direct rustc compile/link consumes essentially the entire unchanged-build latency budget while frontend work is around one millisecond. The evidence therefore justifies native artifact reuse for exact unchanged compilation identity, not speculative frontend micro-optimization. The edited-build result also shows why exact reuse and incremental compilation are distinct problems.

Accepted evidence: PR #78 final head `96b8de7570662aa5cf5886f90ea5f0432a2c14fe`, CI #324 / run `34202560065`, artifact `evo-build-latency-ubuntu-latest` id `10046420977` digest `sha256:d3310910395f9c6be50200c261402bfdbb8c7ef617ed18f71b3aa7486a22a2b0`, squash merge `34f0815d0821543586cd3ae73b9b5b616a1396d3`, and post-merge main CI #325 / run `34205500589`.

## D-021 — `evo build` exact artifact reuse is verified tooling state

**Decision:** `evo build` may reuse a previously compiled native artifact for an exact unchanged compilation identity, but only after the current source has completed the normal frontend/lowering/codegen path and the cache entry has been verified against exact identity files.

For v0:

- build reuse uses a separate `build-cache-v0` layout; accepted `run-cache-v0` behavior is not silently changed;
- normal `evo build <file.evo> [output]` may use verified cache reuse by default;
- `evo build <file.evo> --no-cache` and `evo build <file.evo> <output> --no-cache` explicitly bypass lookup/publish;
- exact Evolution source bytes, exact generated Rust bytes and exact compiler/configuration fingerprint are required on hit;
- the compiler fingerprint includes selected rustc `-vV`, edition, optimization level, codegen units, OS, architecture and executable suffix;
- a cache hash/key is only a locator and is never accepted as identity proof;
- a completion marker and regular non-symlink cached native artifact are required;
- corrupt, incomplete, missing, mismatched or unusable entries fail closed to normal compilation;
- cache storage unavailability falls back to normal compilation;
- a verified hit may materialize the native artifact to a different requested output path and may replace an existing output compatibly;
- publication is best-effort after successful normal compilation; races may compile redundantly rather than consume partial state;
- v0 keeps bounded local pruning and stale staging cleanup;
- the accepted #76 build-latency baseline remains explicitly uncached by invoking `--no-cache`;
- generated Rust bytes and generated-program runtime behavior/cost are independent of cache hits;
- no remote cache, executable download, daemon/compiler service, incremental-rustc session, linker replacement, package/dependency redesign, VM, GC, or language-semantic change is implied.

**Reason:** #76 measured an unchanged warm build at **96.986 ms** with rustc invoked **5/5** times while the frontend was roughly one millisecond. Exact verified artifact reuse removes the dominant redundant compile/link work without weakening frontend validation or changing generated programs.

Accepted final proof:

- PR #81 final head `4288ddcfccf07fcab60d27e9677685b213005ae2`;
- final PR CI #338 / run `34214947823`: **SUCCESS** on Ubuntu, Windows and macOS;
- controlled Ubuntu artifact `evo-build-cache-turnaround-ubuntu-latest`, id `10051449726`, digest `sha256:cf7295bf371695347300bee325e3a5a4b5a96bbf4c8789625904d66bd4c538e9`;
- cold median **128.454 ms**;
- warm cached median **17.177 ms**;
- warm rustc compile count **0**;
- correctness **PASS**;
- **5.646x** speedup versus the accepted #76 warm uncached baseline;
- generated Rust remained 1240 bytes with SHA-256 `61f5f5c99c47196605ae2e461ee589b72a722c4ed5107c6b5fca353795100d83`;
- squash merge `07e85b3a60739f2d1f25caed0fcb622dd8894861`;
- post-merge main CI #339 / run `34215424678`: **SUCCESS** on Ubuntu, Windows and macOS, including all existing Ubuntu turnaround/runtime/performance gates.

Changed-source incremental-rustc/session reuse remains a distinct problem. Issue #82 is the research-first successor and does not alter this accepted exact-artifact cache contract unless later evidence justifies a separately scoped implementation decision.

## D-022 — Append-only sequence v0 is contextual, checked, and direct-Vec

**Decision:** The first production collection is a bounded owned append-only sequence with contextual source forms rather than a general generic container language.

Accepted surface:

```text
items = seq Item()
append items, Item(value = 1)

lookup items, 0 as item
    print item.value
else
    print 0
end
```

For v0:

- `seq`, `append`, `lookup`, and `as` remain ordinary identifier tokens outside their exact contextual positions;
- `seq T` is move-only owned storage; `T` is limited to scalars, nominal records, or explicit `shared Record` owners;
- `seq T()` constructs empty storage directly;
- `append` is explicit exclusive growth and moves move-only payloads without hidden clone;
- lookup is always checked with explicit success and failure branches; negative/out-of-range signed indices select failure;
- scalar elements bind by value while move-only record/shared-owner elements bind through immutable references tied to the sequence owner;
- live move-only element references block sequence growth, move, and reinitialization until bounded last-use analysis proves the reference dead;
- a `shared Record` element lookup does not duplicate its `Rc` handle;
- generated Rust is direct safe `Vec<T>` / `push` / `get` code;
- removal, holes, slot reuse, generations, nested sequence/reference elements, general generic syntax, mutable references, hidden clone/refcount operations, `RefCell`, locks, GC, registries, and unsafe identity machinery are not part of this decision.

**Reason:** #132 proved that append-only indexed storage preserves the useful direct-`Vec` cost model without prematurely choosing removal/reuse identity semantics. #140 implements that smallest production slice while reusing the existing move and bounded immutable-reference liveness model.

Pre-PR implementation evidence: Dev sequence semantics v0 #4 / run `34982930824` passed focused parser/lowering/codegen/native tests, the full workspace suite, Clippy `-D warnings`, and the differential benchmark. The benchmark reported correctness PASS, normalized LLVM IR equality, exact binary equality, stable timing, ratio `0.996626508`, and PASS by byte-identical-binary parity.

## D-023 — Generational arena v0 uses runtime arena identity plus generation-checked reusable slots

**Decision:** Production arena storage uses a bounded contextual `arena T` / `handle T` surface with explicit insert, checked lookup, and checked removal. Handles identify exactly `(arena id, slot index, generation)`; arenas own reusable slot storage and remain move-only.

For v0:

- `arena`, `handle`, `insert`, `lookup`, `remove`, and `as` remain contextual outside their exact forms;
- arena payloads are limited to scalars, nominal records, and explicit `shared Record` owners;
- `handle T` is copy-like and performs no allocation, refcount traffic, registry lookup, or payload operation;
- function parameter/return contracts may use typed handles, record fields may use `handle T` for the full supported arena payload set, and sequences may store typed handles;
- runtime arena identity separates same-payload arenas even when slot index and generation collide;
- checked lookup/removal reject stale, wrong-arena, vacant, and out-of-range handles through the explicit failure branch;
- successful removal moves the payload out once, increments generation before reuse, and retires a slot permanently at `u64::MAX`;
- arena-id exhaustion fails closed;
- live arena-derived element references block insert, remove, move, and reinitialization until bounded final-use analysis proves them dead;
- generated support code is ordinary safe Rust using `Vec<Slot<T>>`, `Vec<usize>`, `Option<T>`, a one-thread-v0 checked `Cell<u64>` identity source, and typed `PhantomData`;
- there is no unsafe pointer identity, global handle registry, GC, `RefCell`, lock, hidden payload clone, hidden `Rc` duplication, or general source-level generic syntax.

**Reason:** #142/#143 established that index+generation without arena identity is insufficient for same-type cross-arena safety, while runtime arena identity plus generation-checked slot reuse preserves O(1)-class operations and a compact fixed-size handle. #144 implements that research result without merging arena ownership with the append-only `seq T` identity model.

Production validation before the final documentation/PR commit:

- semantic/runtime implementation head `280e391f53359ae0d7ad056773f70489081a8374` passed focused arena tests, the full workspace suite, and Clippy `-D warnings` in Dev arena semantics v0 run `35118469618`;
- permanent production gate head `e59f41fc9a4f58bb352d663358fc1b12a130f445`, run `35120462329`: correctness **true**, exact executable bytes **true**, stable **true**, final verdict **PASS** with basis `byte-identical-binary-parity`;
- that run recorded reference median **11,721,632 ns**, Evolution median **11,728,823 ns**, observed timing ratio **1.000613481**, normalized LLVM IR equality **false**, and a timing-only verdict of FAIL; byte-identical executable parity is the deterministic acceptance evidence rather than the noisy median ordering;
- artifact `10457925773`, digest `sha256:4852ab5c61da6c0e2848efff148beaba9fd2dcbafa1dbe650627bb3c04667a39`;
- the earlier independent-reference run `35119416363` is retained as failed evidence at ratio `1.001201306`; artifact inspection showed identical hot executable code despite symbol/source-location identity differences. The permanent fixture therefore keeps the independently authored implementation as a compile/structure control while using a direct-runtime parity lock for timed acceptance.

## Changing a decision

A future change should record:

1. which decision is being superseded;
2. new evidence or requirement;
3. correctness/safety/cost implications;
4. migration impact;
5. benchmark/CI evidence where applicable.
