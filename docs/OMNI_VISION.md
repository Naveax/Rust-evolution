# Rust Evolution — Omni Vision

Status: **long-term architectural north star, not current language spec**.

This document integrates the project's working Rust Evolution core with the broader Omni / full-stack direction. It is intentionally separated from `docs/LANGUAGE_SPEC_V0.md`, which must describe only behavior that is actually implemented and validated on `main`.

## Core idea

Rust Evolution is not intended to become a pile of copied syntax from many languages. The long-term goal is to extract proven ideas from programming languages and domains, then re-design them under one coherent platform without weakening the project's existing correctness, safety, native-performance, diagnostics, or hidden-cost rules.

North-star rules:

> **Simple things simple. Complex things possible. Hidden costs forbidden.**

> **One language, progressive capabilities, explicit costs.**

The eventual system should be able to span scripting, CLI, backend, frontend/web, desktop, mobile, games, graphics/Vulkan, GPU compute, AI/ML, data/scientific computing, embedded, kernel/driver work, networking/distributed systems, enterprise/automation, databases, verification, and hardware-oriented development.

That ambition does **not** mean every domain feature belongs in the core grammar.

## Preserve the current proven foundation

The current implementation model remains the short-term foundation:

```text
Evolution source
  -> lexer
  -> parser
  -> semantic lowering
  -> generated Rust
  -> rustc
  -> native binary
```

This path is already valuable because it allows the language surface to evolve while reusing Rust/rustc/LLVM safety and native code generation.

Long-term expansion is staged, not a rewrite:

```text
Evolution Source
  -> Lexer
  -> Parser
  -> AST
  -> Typed AST / HIR
  -> Type + Ownership + Effect Analysis
  -> Evolution IR
  -> Optimization / Specialization
  -> Backends
```

Possible future backends include Rust, direct LLVM, WASM, SPIR-V, PTX/CUDA, Metal, DXIL/DirectX, JIT, and bare-metal/embedded targets. Rust remains the primary backend until another backend is justified by concrete semantic or engineering limits.

## Small core, broad platform

The core should contain only general semantic building blocks such as:

- bindings and lexical scopes;
- functions;
- types;
- records/structs;
- enums/sum types;
- traits/interfaces;
- generics;
- pattern matching;
- ownership and borrowing;
- error handling;
- modules;
- effects/capabilities where proven;
- async primitives where proven;
- explicit unsafe boundaries;
- compile-time metadata.

Domain-specific power should normally enter through one of five locations:

1. **Core** — general language semantics used across domains.
2. **Profiles / capabilities** — compile-time semantic capabilities and validation for a domain.
3. **Libraries** — reusable APIs that do not require new core semantics.
4. **Optional runtimes** — explicit actor/async/managed/distributed behavior.
5. **Backends / tooling** — target-specific code generation, validation, analysis, IDE/debugger/profiler features.

This classification is mandatory for future proposals. A good idea does not automatically become a keyword.

## Progressive complexity

The language should allow beginners to write very low-ceremony code while preserving an escape path down to explicit systems-level control.

Simple code should stay simple:

```text
name = "Naveax"
print name
```

Advanced code should be possible when justified, without making the advanced syntax mandatory for ordinary programs.

High-level convenience must never block low-level control. A safe graphics/GPU profile may provide strong abstractions, while raw Vulkan/C/assembly-style access remains available only through explicit unsafe or low-level capability boundaries.

## Profiles are capabilities, not language forks

Future profiles may include:

```text
systems
scripting
web
data
gpu
distributed
embedded
game
enterprise
verified
hardware
```

A profile should primarily select capabilities, libraries, validation rules, backend support, and cost/runtime requirements. It should **not** turn Evolution into unrelated dialects with different meanings for core constructs such as `fn`, `if`, `match`, ownership, or types.

Different modules may eventually opt into different profiles, but the core semantic model remains shared.

## Cost model

The Omni direction strengthens, rather than weakens, the current hidden-cost policy.

### ZERO

Zero-cost features must not secretly add work relative to their defined baseline:

- no hidden allocation;
- no hidden clone;
- no hidden boxing;
- no hidden dynamic dispatch;
- no mandatory managed runtime.

The current `T_evolution <= T_reference_rust` contract continues to govern accepted zero-cost core features, with correctness required first and deterministic codegen parity used where applicable.

### EXPLICIT

Some capabilities have real costs but are user-selected and visible, such as heap allocation, dynamic dispatch, async runtime usage, actor runtime usage, reflection, or expensive data transfers.

### MANAGED

Managed profiles/runtimes may provide optional GC, supervision systems, managed scripting sandboxes, distributed runtimes, or similar facilities. Their runtime dependency must be visible and cannot be smuggled into ordinary core code.

The exact user-facing annotation syntax (`@zero`, `@explicit`, `@managed`, or another form) is **not frozen**. First implement the semantic cost model and tooling, then freeze syntax.

## Cost analyzer

A long-term first-class feature is:

```text
evo cost app.evo
```

The analyzer should eventually expose information such as:

- explicit and implicit allocations;
- implicit clones;
- boxing;
- static vs dynamic dispatch;
- managed runtime dependencies;
- unsafe regions;
- FFI boundaries;
- GPU kernels/transfers;
- selected SIMD/target features;
- effect/capability usage.

Useful future CI modes include cost snapshots, before/after diffs, and hard failure on forbidden implicit costs.

## Language research matrix

The project should systematically study ideas from many ecosystems without copying their syntax wholesale.

Representative idea sources include Python, Ruby, Rust, C, C++, Zig, Go, TypeScript, Kotlin, Swift, C#, Erlang/Elixir, Haskell/OCaml/F#, SQL, Julia/R/APL/q, CUDA/GLSL/HLSL/WGSL, Prolog, Lean/Coq/Idris/Agda/SPARK, Lisp/Scheme/Racket, Lua, Nix, and Verilog/VHDL/SystemVerilog.

For every language/domain idea, research should record:

```text
Language / source domain:
Best feature:
Problem solved:
Why it works:
Semantics:
Runtime cost:
Compile-time cost:
Safety impact:
Can Evolution do better?
Core / Profile / Library / Runtime / Backend / Tooling?
Benchmark baseline:
Decision: Accept / Iterate / Research / Reject
```

The purpose is evidence-driven idea extraction, not feature collection.

## Domain direction

The following are long-term domains, not current implementation claims.

### Systems / embedded / kernel / drivers

Future work may require `no_std`, `no_main`, custom allocators, MMIO, interrupts, DMA, atomics/memory ordering, ABI control, inline assembly, kernel APIs, and safe-wrapper generation. Unsafe remains explicit and should carry documented invariants.

### Graphics / Vulkan / GPU

The platform should eventually support both safe high-level graphics/GPU abstractions and raw low-level access. GPU/shader code should be capable of targeting SPIR-V, PTX/CUDA, Metal, DXIL or WGSL-like destinations without requiring separate unrelated languages where practical.

GPU semantics may need explicit memory spaces, workgroups, synchronization, vector/matrix types, resource lifetimes, and transfer/cost analysis. These should be profile/backend capabilities unless they prove general enough for core.

### Web / backend / frontend / desktop / mobile / game

The ergonomic target is rapid application development without abandoning static semantics or native performance. Framework/library work belongs outside core unless a recurring semantic primitive genuinely requires language support.

### Data / scientific / AI

Arrays, tensors, broadcasting, dataframe/query semantics, autograd, scientific numerics, notebook workflows, and GPU acceleration are long-term platform capabilities. Declarative query planning and dataflow may eventually benefit from HIR/Evolution IR rather than being forced through naïve Rust source lowering.

### Distributed systems

Actors, supervision, message passing, cancellation, backpressure and structured concurrency may be supported as explicit capabilities/runtimes. Their costs and failure model must be visible.

### Verification

A verified profile may eventually add contracts, refinement-like types, effects/capabilities, proof obligations, or theorem-prover integration. Verification should be optional and composable rather than forcing theorem proving into simple scripts.

### Hardware-oriented development

FPGA/synthesis/timing-aware constructs are research territory and must not be forced into Evolution Core. A future hardware profile/backend can be evaluated independently.

## FFI and ecosystem strategy

Evolution should interoperate with existing ecosystems while its native ecosystem grows. Long-term FFI targets include Rust, C, C++, Python, JVM/Kotlin, Swift, JavaScript/WASM, .NET, CUDA, Vulkan, and vendor SDKs.

FFI ergonomics must not hide marshaling, allocation, ownership transfer, lifetime, or unsafe costs.

## Safety direction

Rust-level memory safety and data-race safety remain baseline goals. Future capabilities may extend this with null safety, effects/capabilities, taint analysis, resource-lifecycle checking, stronger FFI boundaries, contracts, and bounded unsafe regions.

High-level ergonomics may reduce ceremony but may not silently disable safety checks.

## Diagnostics and tooling

A single Evolution-facing diagnostic system should progressively cover syntax, types, ownership, traits, effects, GPU validation, FFI, build/link/package, and domain-specific failures. Backend-internal details should be mapped back to Evolution source whenever reliable mappings exist.

Tooling is a first-class product surface:

```text
evo new
evo run
evo build
evo test
evo check
evo fmt
evo lint
evo bench
evo cost
evo inspect
evo doc
evo package
evo publish
```

Not all commands exist today. `docs/PROJECT_STATE.md` is authoritative for current implementation status.

## Long-term roadmap

### A — Core stabilization

Functions, lexical block locals, records/structs, enums, pattern matching, traits, generics, collections, errors, modules, and ownership ergonomics.

### B — Typed semantics / HIR

Typed AST/HIR, richer inference, move/borrow analysis, effect metadata, and stronger source mapping.

### C — Evolution IR

Stable semantic IR, optimization boundaries, backend API, and IR-level diagnostics. Do not freeze this before the semantic model is mature enough to justify it.

### D — Systems / FFI

C/C++/Rust interop, raw memory boundaries, embedded/no_std, platform APIs.

### E — GPU / graphics

Vulkan, shader/compute semantics, SPIR-V and other GPU backends, resource-safety analysis.

### F — Application profiles

Web, desktop, mobile, game, enterprise.

### G — Data / AI / scientific

Arrays/tensors/dataframes/autograd/notebooks/GPU compute.

### H — Distributed

Actors, supervision, remote messaging, structured concurrency.

### I — Verified

Contracts, effects, refined constraints and optional verification integration.

### J — Hardware

FPGA/synthesis/hardware-software co-design research.

## Reality boundaries

Rust Evolution must not claim that it will automatically beat every language, every handwritten assembly routine, every vendor compiler, or solve every safety/algorithm problem. The defensible target is to approach or achieve the best practical performance class in each supported domain while preserving unusually strong ergonomics, safety, explicit cost, and low-level control.

## Final design rules

1. Core stays small.
2. Safety is the default.
3. Unsafe stays explicit.
4. Hidden cost is forbidden.
5. High-level use never blocks low-level control.
6. Simple work should be exceptionally low ceremony.
7. Accepted native zero-cost features remain in Rust/C/C++ performance class under defined baselines.
8. GPU/domain code may use specialized backends rather than pretending every domain is ordinary CPU Rust.
9. FFI is first-class.
10. Diagnostics return to Evolution source whenever possible.
11. We take proven ideas, not whole foreign grammars.
12. Features pass correctness, safety, diagnostics/tooling, and performance/cost gates appropriate to their class.
13. Tooling is as important as syntax.
14. Ecosystem strategy is as important as compiler architecture.
15. Complexity is progressively disclosed.

## Source and governance

This vision was integrated from the user-authored `RUST_EVOLUTION_OMNI_UPDATE_SPEC.md` on 2026-08-26. It is a vision document. Current truth remains split deliberately:

- `docs/LANGUAGE_SPEC_V0.md` — implemented language semantics;
- `docs/PROJECT_STATE.md` — current verified project status;
- `docs/NEXT_ACTION.md` — active continuation point;
- `docs/ROADMAP.md` and issue #1 — staged project roadmap;
- this file — long-term Omni direction.

If this file and implemented behavior conflict, implemented tests/spec win until a new feature passes the project acceptance pipeline.