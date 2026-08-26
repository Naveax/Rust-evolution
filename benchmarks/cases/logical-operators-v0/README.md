# Logical operators v0 benchmark

This case exercises runtime-dependent `and` / `or` / `not` inside a 20,000,000-iteration loop.

The Evolution source and reference Rust perform the same stateful recurrence with the same stdin parsing and output. The condition is deliberately non-trivial and depends on loop-carried state:

```text
(x > 1 and not (x == 7)) or x < -10
```

The reference source mirrors the generated Rust shape so exact binary / normalized LLVM parity can prove zero-cost lowering when rustc produces identical code. If binaries differ, the harness falls back to the strict stable timing rule `T_evolution / T_reference <= 1.00`.

This is performance evidence for logical lowering, not a claim that the Evolution syntax makes Rust machine code intrinsically faster.
