# Block locals v0 performance case

This case exercises lexical block-local bindings in a runtime-dependent hot loop.

Each iteration enters an `if`/`else` branch. Both sibling branches declare a local named `temp`; the declarations are semantically independent and remain scoped to their branch. The branch-local value is then used to update visible outer locals.

The reference Rust intentionally mirrors ordinary Evolution Rust lowering. This makes the case a direct zero-cost frontend test: if lowering remains structurally equivalent, independently compiled executables should be byte-identical under the benchmark harness's canonical source identity.

The workload uses runtime stdin (`n = 20,000,000`, initial `x = 9`) and a recurrence that visits both branches, so the branch/local work survives compilation rather than reducing to a compile-time constant.
