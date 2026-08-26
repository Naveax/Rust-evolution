# Language / Ecosystem Research Matrix

Rust Evolution studies proven ideas from other languages and domains. The goal is not to copy syntax; the goal is to extract the underlying problem/semantics/cost model and decide whether Evolution can adopt or improve the idea coherently.

Create one file per serious research subject when work begins, for example:

```text
research/languages/python.md
research/languages/ruby.md
research/languages/zig.md
research/languages/elixir.md
research/languages/julia.md
research/languages/lean.md
research/languages/cuda-wgsl.md
```

## Required template

```text
# <Language / Ecosystem>

## Best feature / idea

## Problem solved

## Why the idea works

## Semantics

## Runtime cost

## Compile-time cost

## Safety impact

## Developer-experience impact

## Failure modes / trade-offs

## Existing Rust equivalent

## Can Evolution do better?

## Classification
Core / Profile / Library / Optional Runtime / Backend / Tooling

## Cost class
ZERO / EXPLICIT / MANAGED / Research-only

## Benchmark / comparison baseline

## Minimal Evolution experiment

## Acceptance gates

## Decision
Accept / Iterate / Research / Reject / Ecosystem-only / Not actionable
```

## Initial idea sources

The Omni vision identifies useful ideas to investigate from ecosystems including:

- Python — readability, scripting, REPL/prototyping;
- Ruby — expressive APIs/DSL ergonomics;
- Rust — ownership/safety/zero-cost abstractions;
- C — ABI/raw systems/freestanding control;
- C++ — specialization/generic/HPC/SIMD/ecosystem interop;
- Zig — explicit allocation/C interop/cross-compilation UX;
- Go — tooling/deployment/concurrency simplicity;
- TypeScript — IDE/type-inference/diagnostics/tooling-first approach;
- Kotlin/Swift/C# — null safety/application/enterprise ergonomics;
- Erlang/Elixir — actors/supervision/fault tolerance;
- Haskell/OCaml/F# — ADTs/purity/effects/functional composition;
- SQL — declarative query/planning/optimization;
- Julia/R/APL/q — numerical/array/vectorized/columnar ideas;
- CUDA/GLSL/HLSL/WGSL — GPU kernels/shaders/memory spaces/synchronization;
- Prolog — logic/constraint solving;
- Lean/Coq/Idris/Agda/SPARK — contracts/proofs/refined constraints;
- Lisp/Scheme/Racket — hygienic metaprogramming/DSL ideas;
- Lua — minimal syntax/embedding/plugin model;
- Nix — reproducibility/immutable dependency graphs;
- Verilog/VHDL/SystemVerilog — hardware/timing/synthesis ideas.

## Rule

No research file creates an implementation commitment by itself.

A proposal becomes code only after a focused GitHub issue defines exact Evolution semantics, safety/cost implications, tooling/diagnostics requirements, and an evidence plan.