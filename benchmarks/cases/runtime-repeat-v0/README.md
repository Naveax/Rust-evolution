# Runtime repeat v0 probe

This case exercises `input_int`, `repeat ... end`, and inferred mutability with a runtime-dependent recurrence:

```text
state = state / 2 + 1
```

The recurrence is intentionally loop-carried and includes integer division so LLVM cannot trivially replace it with the closed-form arithmetic used by a simple increment loop. The reference Rust uses the same `i64` types, stdin parsing, loop structure, arithmetic, and output behavior as generated Evolution Rust.

CI runs this probe only on Ubuntu and always with `--report-only`. The harness keeps byte-exact stdout correctness, so the LF fixture is intentionally not treated as a portable Windows newline fixture.

A timing result is **not** accepted as performance evidence until the emitted LLVM IR is inspected to confirm that the runtime loop remains present and the reference/Evolution normalized IR comparison is equivalent. Noisy measurements remain INCONCLUSIVE under the benchmark harness policy.
