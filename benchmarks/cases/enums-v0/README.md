# Enums v0 performance case

This case exercises static enum construction, by-value enum return, exhaustive matching and scalar payload binding in a runtime-dependent hot loop.

Each iteration calls `classify`, which constructs one of two payload variants of `Step` from the current runtime state. The returned enum is consumed by an exhaustive `match`; both arms read the scalar payload, update the accumulated sum and evolve the next runtime value. The recurrence visits both variants and the iteration count comes from stdin, so the workload cannot be reduced to a compile-time constant loop.

The reference Rust mirrors the generated static enum representation and the same algorithm, input/output behavior and payload types. No heap allocation, boxing, cloning, reference counting, dynamic dispatch, runtime variant map or reflection metadata is permitted solely for Enums v0.

The workload uses runtime stdin (`n = 20,000,000`, initial `x = 9`) and the same benchmark harness policy as Records v0: three warmups, thirteen measured samples and a maximum relative MAD of 0.15. Correctness is checked before timing. Scalar replacement or elimination of the temporary enum by LLVM is acceptable and desirable zero-cost evidence when both sides compile to equivalent behavior.
