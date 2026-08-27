# Records v0 performance case

This case exercises static record construction and direct scalar field access in a runtime-dependent hot loop.

Each iteration constructs `Pair` with named fields intentionally written out of declaration order. Lowering must validate the exact field set and emit deterministic schema-order Rust struct literals. The loop then reads both scalar fields directly and updates runtime state through a branch.

The reference Rust mirrors the generated static representation and algorithm. No heap allocation, boxing, cloning, dynamic dispatch, runtime object map or reflection metadata is permitted solely for the record feature.

The workload uses runtime stdin (`n = 20,000,000`, initial `x = 9`) and a recurrence that visits both branches, so the loop survives compilation. Scalar replacement of the aggregate is acceptable and desirable: Records v0 is a zero-cost frontend feature, so LLVM eliminating the temporary struct is valid parity evidence.
