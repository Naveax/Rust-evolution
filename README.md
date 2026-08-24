# Rust Evolution

Rust Evolution is an experimental engineering project focused on improving Rust's weakest areas without sacrificing Rust's core strengths.

## Vision

The target is deliberately strict:

- Rust-level correctness and safety.
- Runtime performance equal to or better than equivalent idiomatic Rust.
- A much simpler language/front-end experience inspired by Lua + Python + Rust, aiming to be simpler to write than all three where possible.
- Measurable, benchmark-driven engineering rather than subjective claims.

## Non-negotiable runtime rule

For equivalent semantics, inputs, target, toolchain and optimization conditions:

`T_evolution <= T_reference_rust`

Normalized:

`performance_ratio = T_evolution / T_reference_rust <= 1.00`

Correctness must match before a performance result is valid. Runtime regressions are not accepted merely because syntax is shorter or development is easier.

See issue #4 for the full performance contract and issue #5 for the benchmark/differential validation harness.

## Initial architecture

`Evolution source -> lexer -> parser -> AST -> semantic lowering -> generated Rust -> rustc -> native binary`

The first implementation is intentionally a frontend/transpiler experiment rather than a new VM/runtime. This lets us explore syntax and developer ergonomics while retaining Rust/rustc/LLVM semantics and native code generation.

## Executable v0

The first vertical slice supports a deliberately tiny script syntax:

```text
x = 1
y = 1
print x + y
```

Current commands:

```text
cargo run -p evo-cli -- check examples/basic.evo
cargo run -p evo-cli -- emit-rust examples/basic.evo
cargo run -p evo-cli -- run examples/basic.evo
cargo run -p evo-cli -- build examples/basic.evo
```

The current language sketch and grammar are documented in `docs/LANGUAGE_SPEC_V0.md`. This v0 is experimental and exists to prove the complete source-to-native pipeline before expanding syntax.

## Project roadmap

- Phase 0: repository/toolchain/workspace/CI foundation
- Phase 1: Lua + Python + Rust inspired ergonomic syntax/front-end
- Phase 2: differential correctness + runtime parity benchmark infrastructure
- Phase 3: scripting, diagnostics, ownership ergonomics, build/compile improvements
- Phase 4: async, IDE/LSP, debugging, FFI, cross-platform
- Phase 5: GUI, web, mobile, automation, enterprise ecosystem
- Phase 6: ML/data science, GPU, scientific computing, game development, embedded
- Phase 7: adoption, migration and ecosystem health

## Project tracking

- #1 Master roadmap and execution checklist
- #2 Ergonomic syntax/front-end experiment
- #3 Repository bootstrap
- #4 Runtime performance invariant
- #5 Benchmark and differential validation harness
- #6 Complete Rust weakness map

## Engineering principles

- `main` remains stable.
- Research, experiments, features and fixes use separate branches.
- Claims are backed by tests and benchmarks where applicable.
- Failed experiments are documented rather than erased.
- Safety is not traded for speed.
- Ergonomics is not allowed to hide runtime cost.
- An active GitHub Actions run for the same SHA/workflow/input is not duplicated; its run ID is tracked while independent work continues.

## Status

Early implementation stage. The first executable front-end vertical slice is under active development; performance claims remain blocked until the differential benchmark harness is running under the project contract.
