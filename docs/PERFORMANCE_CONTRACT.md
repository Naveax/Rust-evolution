# Performance Contract

## Hard rule

For equivalent semantics and controlled build/runtime conditions:

`T_evolution <= T_reference_rust`

`performance_ratio = T_evolution / T_reference_rust <= 1.00`

Correctness must match first:

`output_evolution == output_reference_rust`

A repeatable ratio above `1.00` is a failure. A noisy or inconclusive result is not a pass.

## Comparable conditions

- Same inputs
- Same expected output
- Same algorithmic task
- Same target triple
- Same Rust toolchain where applicable
- Same optimization level
- Same CPU feature set
- Same linker/LTO/codegen configuration
- Same machine/runner
- Release/benchmark profile for runtime comparisons

## Anti-cheating rules

- Reference Rust must be idiomatic and reasonably optimized.
- Evolution may not do less work.
- Constant-folded/precomputed fake workloads are not accepted.
- Dead-code elimination must be prevented in microbenchmarks.
- Hidden allocation, cloning, boxing or dynamic dispatch must be inspected.
- Safety regressions cannot be counted as performance wins.

## Measurement policy

- Warm up before measurement.
- Collect multiple samples.
- Use median as the primary latency comparison.
- Report variance/confidence information where practical.
- Report p95/p99 for latency-sensitive workloads where meaningful.
- Mark unstable results INCONCLUSIVE.
- Repeat or investigate regressions before deciding.

## Secondary metrics

Where applicable:

- Throughput
- CPU time
- Peak RSS
- Allocation count and allocated bytes
- Syscalls
- Context switches
- Instruction count
- Branch/cache miss counters
- Binary size

These secondary metrics help explain behavior. The main runtime gate remains parity-or-better against reference Rust.

## Correctness gate before performance

- Unit tests
- Integration tests
- Golden/output tests
- Differential tests
- Property tests where useful
- Fuzzing for parsers/unsafe/FFI boundaries
- Error/panic semantics
- Boundary and Unicode cases
- Concurrency correctness

If correctness fails, the performance result is invalid.

## Codegen investigation path

When Evolution differs from reference Rust:

1. Inspect generated Rust.
2. Check hidden clones/allocations/boxing/dispatch.
3. Compare MIR where useful.
4. Compare LLVM IR where useful.
5. Compare hot-path assembly.
6. Re-run controlled benchmark.

## Merge/release gate

A feature that causes a repeatable runtime regression is not accepted as a successful default feature merely because it improves syntax or developer productivity. It may remain an experiment with documented results.

See GitHub issue #4 for the living checklist and issue #5 for the executable benchmark harness plan.
