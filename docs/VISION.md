# Rust Evolution Vision

## Mission

Rust Evolution exists to improve Rust's weakest areas without surrendering the properties that make Rust valuable.

The project is driven by four simultaneous goals:

1. **Correctness:** equivalent programs must produce equivalent correct results.
2. **Safety:** Rust's intended memory-safety and data-race-safety properties must not be silently weakened.
3. **Runtime performance:** equivalent Evolution programs must be equal to or faster than equivalent idiomatic Rust programs under controlled conditions.
4. **Ergonomics:** the user-facing language and tooling should aim for a writing experience inspired by Lua's minimalism and Python's readability while retaining Rust's systems-level control.

## North-star target

`Rust Evolution = Rust safety/performance + simpler-than-Lua/Python-oriented ergonomics + measurable engineering discipline`

This is intentionally ambitious. If an ergonomic idea makes runtime slower, hides allocations, weakens safety, or changes semantics, it has failed the core contract even if the syntax looks pleasant.

## Non-negotiable runtime invariant

For equivalent semantics, inputs, target, toolchain and optimization conditions:

`T_evolution <= T_reference_rust`

Normalized:

`performance_ratio = T_evolution / T_reference_rust <= 1.00`

A repeatable ratio above `1.00` is a failure. An inconclusive/noisy result is not a pass.

## What we want to improve

- Rapid prototyping and scripting
- Error diagnostics
- Ownership/borrowing/lifetime ergonomics
- Build and incremental compile performance
- Async/concurrency ergonomics
- IDE/LSP/debugging/profiling
- FFI and cross-platform integration
- GUI/web/mobile/automation/enterprise ecosystem gaps
- ML/data science/GPU/scientific/game/embedded domain gaps

The exhaustive starting backlog is tracked in GitHub issue #6.

## What this project is not

- A Python clone with Rust branding
- A plan to remove ownership
- A plan to add a mandatory garbage collector
- A plan to hide `unsafe`
- A plan to win benchmarks by comparing against intentionally poor Rust
- A syntax-only project
- A rewrite of the entire Rust compiler on day one

## Initial implementation strategy

The first language experiment is a front-end that lowers to Rust:

`Evolution source -> lexer -> parser -> AST -> semantic lowering -> generated Rust -> rustc -> native binary`

This lets us experiment with syntax and developer experience while preserving Rust/rustc/LLVM as the native compilation path. A deeper compiler integration is only considered after the front-end model proves correctness, safety and zero-runtime-regression.

## Decision gates

Every major feature must pass:

- Correctness gate
- Safety gate
- Runtime performance gate
- Engineering quality gate
- Documentation gate

Ergonomics is optimized after the first three constraints are satisfied, not instead of them.

## Culture of evidence

Claims should be tied to tests, benchmarks, generated-code inspection or reproducible experiments. Failed experiments remain documented. We want a repository that can explain not only what works, but why discarded approaches failed.
