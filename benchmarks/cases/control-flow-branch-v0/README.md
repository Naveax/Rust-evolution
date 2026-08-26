# Control-flow branch v0 performance gate

This case exercises runtime-dependent `if`/`else`, comparisons, reassignment and inferred mutability inside `repeat`.

Evolution reads two signed `i64` values from stdin: the iteration count and an initial state. The branch changes that state inside the loop:

```text
if x > 1
    x = x / 2
else
    x = x + 3
end
sum = sum + x
```

The initial state is runtime input rather than a compile-time constant. With fixture seed `1`, the post-update state cycles through `4, 2, 1`, so 20,000,000 iterations produce the deterministic expected sum `46,666,668` while retaining loop-carried control flow that LLVM cannot replace with the trivial alternating-sign closed form used by the first probe draft.

The reference Rust mirrors generated Evolution Rust at the same algorithm, integer type, input parser and output semantics.

CI runs this case as an enforced Ubuntu performance gate, not `--report-only`. Correctness must pass first. If canonical compilation produces byte-identical binaries, the harness records deterministic runtime parity; otherwise the strict stable timing rule remains `T_evolution / T_reference <= 1.00`.

The artifact retains generated Rust, normalized LLVM IR, binary comparison metadata and raw timing samples so optimization behavior can be inspected instead of assumed.
