# Function call v0 benchmark

This case validates the first named-function surface under the project runtime contract.

The Evolution program defines a typed `step(x int) int` function, calls it from a 20,000,000-iteration runtime-dependent loop, and accumulates the result. The reference Rust program uses the same input, algorithm, integer type, branch condition, output, and rustc optimization configuration.

The function call is an ordinary static call on both sides. rustc/LLVM is free to inline it. If codegen becomes byte-identical after optimization, that is deterministic zero-cost parity evidence. If the binaries differ, the benchmark remains subject to the strict stable timing rule `T_evolution / T_reference <= 1.00`.

This workload is deliberately runtime-input dependent so the entire loop cannot be precomputed at compile time.
