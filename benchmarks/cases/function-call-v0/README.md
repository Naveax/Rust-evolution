# Function call v0 benchmark

This case validates the first named-function surface under the project runtime contract.

The Evolution program defines a typed `step(x int) int` function, calls it from a 20,000,000-iteration runtime-dependent loop, and accumulates the result.

## Reference policy

This case is specifically a **zero-cost lowering** check. `reference.rs` therefore mirrors the ordinary static Rust shape emitted by the Evolution frontend: the same helper, local/function identifiers, branch form, assignments, and direct function call. Both sides are still compiled independently by the same `rustc` with the same target and optimization flags.

This is not an attempt to make the Rust side artificially slow. It asks a narrower and stronger question: after Evolution syntax has been lowered to normal Rust, did the frontend introduce any extra runtime work at all? If the independently compiled executables are byte-identical after correctness passes, runtime parity is deterministic. If they differ, the strict stable timing rule `T_evolution / T_reference <= 1.00` remains mandatory.

The function call is an ordinary static call. rustc/LLVM is free to inline it. No VM, registry, boxing, vtable, or dynamic dispatch participates in the benchmark.

This workload is deliberately runtime-input dependent so the entire loop cannot be precomputed at compile time.
