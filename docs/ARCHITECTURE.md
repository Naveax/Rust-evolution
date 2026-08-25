# Initial Architecture

## Front-end pipeline

`Evolution source -> lexer -> parser -> AST -> semantic lowering -> Rust codegen -> rustc -> native binary`

The first architecture intentionally keeps Rust/rustc/LLVM as the compilation backend. This reduces risk and gives the project a direct path to zero-runtime-regression experiments.

## Planned components

- `evo-cli`: user-facing build/run/check/test/inspect commands
- `evo-lexer`: tokenization
- `evo-parser`: grammar, parser, error recovery
- `evo-ast`: syntax tree and source spans
- `evo-lowering`: semantic lowering into Rust-compatible constructs
- `evo-codegen-rust`: deterministic Rust source generation
- `evo-diagnostics`: source-mapped diagnostics
- `evo-bench`: differential correctness/performance harness

These crates should be created only when needed; the architecture is a boundary plan, not an excuse to manufacture empty crates.

## Core boundaries

### Parser vs semantics

Parsing should not silently decide ownership or performance policy. Syntax becomes AST first; semantics/lowering remains a separate inspectable stage.

### Mutability inference v0

The current lowering pass infers `mut` only from same-type reassignment in the v0 scalar/string value model. This is not ownership or borrow analysis. New locals inside `repeat` are rejected in v0 so zero-iteration loops cannot create maybe-uninitialized bindings observable afterward.

When owned or move-sensitive types arrive, this analysis must grow to track definite initialization, moves, borrows and control-flow joins before inferring mutation. Ergonomic syntax must not weaken Rust ownership or permit use-after-move or maybe-uninitialized states.

### Source spans

Every important AST node should retain source location information so rustc/generated-code errors can be mapped back to Evolution source.

### Code generation

Generated Rust should be deterministic, inspectable, and suitable for snapshot tests. A developer command should eventually expose generated Rust for debugging.

### Runtime library

A runtime library is not assumed. If one becomes necessary, every feature added to it must justify its cost and satisfy the runtime contract. Syntax sugar should prefer lowering to zero-cost Rust constructs.

## Safety

- `unsafe` generated code is avoided by default.
- Any unavoidable `unsafe` path gets a documented invariant and tests.
- Parser/codegen must treat input as untrusted.
- Generated identifiers/strings/paths need escaping/injection tests.
- FFI boundaries receive dedicated correctness and fuzzing work.

## Tooling direction

The long-term developer workflow may expose:

- `evo check`
- `evo build`
- `evo run`
- `evo test`
- `evo fmt`
- generated-Rust inspection
- LSP integration
- REPL/script mode if they can be implemented without violating project invariants

## Evolution path

Only after the front-end demonstrates useful ergonomics, correctness, safety and runtime parity should deeper integrations be considered, such as direct rustc front-end integration, alternative intermediate representations, or compiler changes.
