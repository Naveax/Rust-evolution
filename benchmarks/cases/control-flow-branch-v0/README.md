# Control-flow branch v0 performance gate

This case exercises runtime-dependent `if`/`else`, comparisons, reassignment and inferred mutability inside `repeat`.

Evolution reads two signed `i64` values from stdin: the iteration count and an initial sign carrier. The sign carrier is runtime data rather than a compile-time constant, then flips sign each iteration:

```text
if x > 0
    sum = sum + x
else
    sum = sum - x
end
x = -x
```

With the fixture seed `1`, each iteration contributes `1`, so the expected result equals the runtime iteration count. The reference Rust intentionally mirrors generated Evolution Rust at the same algorithm, integer type, input parser and output semantics.

CI runs this case as an enforced Ubuntu performance gate, not `--report-only`. Correctness must pass first. If canonical compilation produces byte-identical binaries, the harness records deterministic runtime parity; otherwise the strict stable timing rule remains `T_evolution / T_reference <= 1.00`.

The artifact retains generated Rust, normalized LLVM IR, binary comparison metadata and raw timing samples so optimization of the branch can be inspected rather than guessed.
