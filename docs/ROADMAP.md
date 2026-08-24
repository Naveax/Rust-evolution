# Rust Evolution Roadmap

The authoritative living checklist is GitHub issue #1. This file keeps the repository-level phase structure.

## Phase 0 — Foundation

- [ ] Initialize `main`
- [ ] Pin Rust toolchain
- [ ] Create Cargo workspace
- [ ] Add fmt/clippy/test/build quality gates
- [ ] Add Linux/Windows/macOS CI baseline
- [ ] Define test conventions
- [ ] Define branch/PR conventions
- [ ] Add architecture, performance and benchmarking docs
- [ ] Create benchmark harness skeleton
- [ ] Create language front-end skeleton

Exit: clean checkout builds, CI is green, core documentation and skeletons exist.

## Phase 1 — Ergonomic language front-end

- [ ] Language sketch v0
- [ ] Grammar v0
- [ ] Lexer
- [ ] Parser
- [ ] AST
- [ ] Source spans
- [ ] Diagnostics/error recovery
- [ ] Semantic lowering
- [ ] Rust code generation
- [ ] Source mapping
- [ ] Formatter foundation
- [ ] Differential correctness tests
- [ ] Parser fuzzing
- [ ] At least 10 representative programs

Exit: representative Evolution programs lower to Rust, compile natively, match reference correctness, and satisfy runtime parity-or-better for accepted core features.

## Phase 2 — Correctness and performance infrastructure

- [ ] Reference Rust benchmark suite
- [ ] Evolution benchmark suite
- [ ] Shared inputs and expected outputs
- [ ] Automated stdout/stderr/exit-code comparison
- [ ] Warm-up and repeated samples
- [ ] Median/p95/p99 where meaningful
- [ ] CPU/memory/allocation metrics
- [ ] Generated Rust/MIR/LLVM/assembly inspection path
- [ ] Automatic ratio calculation
- [ ] PASS/FAIL/INCONCLUSIVE verdict
- [ ] CI artifacts and Markdown reports

Exit: every performance claim can be reproduced through one harness.

## Phase 3 — Core developer experience

- [ ] Script mode
- [ ] Single-file execution
- [ ] REPL feasibility
- [ ] Hot-reload feasibility
- [ ] Better parser/type/trait/generic diagnostics
- [ ] Borrow/lifetime diagnostics and inference research
- [ ] Ownership ergonomics
- [ ] Cold/warm/incremental build benchmarks
- [ ] Dependency/proc-macro/link-time analysis
- [ ] Binary-size and compile-memory analysis

## Phase 4 — Async, tooling, debug, FFI, platform

- [ ] Async syntax and diagnostics
- [ ] Pin/Unpin user-surface reduction research
- [ ] Send/Sync diagnostics
- [ ] Structured concurrency/cancellation research
- [ ] LSP completion/navigation/refactoring
- [ ] Debugger source mapping
- [ ] Profiling/coverage/sanitizer integration
- [ ] C/C++/Python/Java/Kotlin/Swift/JS/.NET interoperability
- [ ] FFI overhead benchmarks
- [ ] Cross-compilation UX

## Phase 5 — Application ecosystem

- [ ] GUI/Desktop
- [ ] Web/WASM
- [ ] Mobile
- [ ] Automation/scripting ecosystem
- [ ] Enterprise/serverless/vendor SDK gaps

Each domain must be decomposed into atomic measurable problems rather than one vague mega-task.

## Phase 6 — Specialized domains

- [ ] Machine learning/data science
- [ ] GPU/GPGPU/CUDA
- [ ] Scientific computing
- [ ] Game development
- [ ] Embedded/no_std/vendor SDK edge cases

## Phase 7 — Adoption and migration

- [ ] C/C++ migration and interoperability
- [ ] Learning/onboarding path
- [ ] Compatibility/versioning policy
- [ ] Package ecosystem strategy
- [ ] Security/advisory process
- [ ] Stable language specification path

## Immediate execution queue

1. [ ] Complete repository bootstrap (#3)
2. [ ] Put performance contract (#4) into executable benchmark infrastructure
3. [ ] Implement benchmark harness skeleton (#5)
4. [ ] Create first reference Rust benchmark
5. [ ] Create first differential correctness runner
6. [ ] Create lexer/front-end crates (#2)
7. [ ] Write language sketch and grammar v0
8. [ ] Implement lexer
9. [ ] Implement parser and AST
10. [ ] Implement Rust code generation
11. [ ] Compile first Evolution program to native code
12. [ ] Compare correctness against reference Rust
13. [ ] Compare runtime against reference Rust
14. [ ] Require `performance_ratio <= 1.00`
15. [ ] Publish benchmark result as CI artifact/report
16. [ ] Select next highest-value weakness from #6 and repeat the evidence cycle
