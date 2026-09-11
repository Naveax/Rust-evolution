# Benchmarking and Differential Validation

## Goal

Every performance claim should be reproducible by comparing reference Rust and Rust Evolution under the same conditions.

## Pipeline

1. Parse Evolution source.
2. Generate Rust.
3. Compile generated Rust.
4. Compile reference Rust.
5. Run correctness comparison on shared fixtures.
6. If correctness passes, run controlled performance measurements.
7. Calculate normalized ratio.
8. Emit machine-readable and human-readable reports.

## Verdicts

- **PASS:** correctness passes and `performance_ratio <= 1.00`.
- **FAIL:** correctness fails, or a repeatable runtime regression produces `performance_ratio > 1.00`.
- **INCONCLUSIVE:** measurement noise/variance prevents a defensible decision.

## Benchmark metadata

Each case should record:

- benchmark name
- category
- git SHA
- reference source/version
- Evolution source/version
- generated Rust artifact
- rustc version
- target triple
- build flags
- environment/runner identity
- input fixture
- expected output
- warm-up count
- sample count
- timeout

## Output fields

At minimum:

- reference median
- Evolution median
- normalized ratio
- correctness result
- verdict
- notes

Where applicable also record throughput, p95/p99, CPU time, memory, allocations and code-size metrics.

## Differential harness v0

The repository contains an executable `evo-bench` harness. A case directory has this shape:

```text
case-name/
├── case.conf
├── reference.rs
├── evolution.evo
├── expected.stdout
├── expected.stderr   # optional
└── stdin.bin         # optional
```

Run a case with:

```text
cargo run -p evo-bench -- run benchmarks/cases/<case-name>
```

Use `--report-only` for smoke/infrastructure cases where the result must be recorded but is not yet a performance acceptance gate.

### v0 compilation parity

Reference Rust and generated Evolution Rust are compiled by the same `rustc`, with the same crate name and the same baseline optimization settings:

- edition 2024
- `opt-level=3`
- `codegen-units=1`
- thin LTO
- debuginfo disabled

The harness also emits LLVM IR for both sides and records whether normalized IR is equal. IR equality is evidence, not a substitute for runtime validation on meaningful workloads.

### v0 correctness execution

Correctness runs before timing. Both programs receive the same stdin and must match:

- expected stdout
- expected stderr
- successful exit status
- each other’s stdout/stderr/exit status

Correctness execution uses timeout-controlled process polling, but stdout/stderr are captured through temporary files rather than pipes. This prevents a child that produces a large output from blocking because an unread pipe buffer filled.

### v0 timing execution

Timed samples do **not** use timeout polling. Polling and a fixed sleep interval would contaminate short samples. Instead:

- correctness is proven first;
- warmups remain correctness-checked and timeout-controlled;
- timed samples use a blocking process wait;
- stdin is supplied from the same kind of file on both sides;
- stdout/stderr are redirected to the platform null device symmetrically;
- reference/evolution execution order alternates by sample;
- median is the primary ratio metric;
- p95 and relative median absolute deviation (MAD) are recorded;
- excessive relative MAD produces **INCONCLUSIVE**, never PASS.

A workflow-level timeout remains the outer safety boundary for a pathological timed sample. Runtime benchmark cases must be deterministic enough that a program that passed correctness/warmup cannot arbitrarily hang during measurement.

### v0 artifacts

Each run writes:

```text
generated.rs
reference.ll
evolution.ll
reference[.exe]
evolution[.exe]
report.json
report.md
raw-samples.csv
```

The report includes the rustc verbose version, host target, sample configuration, correctness result, binary sizes, normalized LLVM IR equality, sample statistics, performance ratio, stability decision, and final verdict.

### Initial smoke case

`benchmarks/cases/arithmetic-smoke` exists to verify the full harness path across platforms. It is intentionally tiny and therefore **must not be cited as evidence that Evolution is faster than or equal to Rust at runtime**. Process startup and scheduler noise dominate such a case. Meaningful performance acceptance begins once Evolution can express workloads with runtime-dependent input and enough work to measure defensibly.

### Inferred shared-borrow v0 permanent gate

`benchmarks/cases/inferred-shared-borrow-v0` is a permanent Ubuntu runtime differential acceptance gate for the bounded shared-borrow parameter feature implemented by #97.

The fixture performs **5,000,000** repeated read-only calls on one nominal value and later reassigns that same owned value. The locked Rust reference uses ordinary explicit `&Item` / `&item` borrowing. `crates/evo-bench/tests/inferred_shared_borrow_reference.rs` requires the committed `reference.rs` to match generated Rust exactly modulo CRLF normalization, so irrelevant helper/function source ordering cannot create artificial code-layout differences between the two sides.

Accepted exact head `f33b513188add2c6755263f6c7e1079072ec9d7b`, CI #404 / run `34576556404`, produced:

- correctness: **true**;
- normalized LLVM IR equal: **true**;
- exact executable bytes equal: **true**;
- reference median: **5,305,283 ns**;
- Evolution median: **5,286,876 ns**;
- observed ratio: **0.996530440**;
- stable measurement: **true**;
- timing verdict: **PASS**;
- final verdict: **PASS**;
- verdict basis: `byte-identical-binary-parity`.

Accepted artifact:

- `evo-bench-inferred-shared-borrow-ubuntu-latest`;
- artifact id `10189953214`;
- digest `sha256:8fcee06fa8df815e0aaf156649d787cf59459f0d829151811f19ff9ae48d7955`.

The earlier benchmark head `64b30b6ddf280adfb04b578ed675331df065c658`, CI #402 / run `34496493149`, is retained as failed evidence rather than rerun. Correctness and stability passed, but its manually written Rust reference placed the input helper/function in a different order than generated Rust. That made normalized IR and binaries differ and produced a timing-only ratio of `1.006804464`, so the gate correctly failed. The corrected permanent reference lock prevents that irrelevant structural mismatch from returning.

The CI benchmark step has an explicit step id and the artifact upload uses `always()` with a non-skipped guard. Therefore a future failed shared-borrow gate still preserves its generated Rust, binaries, IR, reports, and raw samples instead of deleting the evidence precisely when it becomes interesting.

## Developer-turnaround evidence

Developer edit-run latency is a different metric from generated-program runtime parity. Tooling changes such as the verified `evo run` compile cache must not use a faster development loop to excuse a runtime regression, and the `T_evolution <= T_reference_rust` invariant does not claim that compiler/tooling latency equals program runtime.

Fast edit-run v0 therefore has a separate controlled Ubuntu evidence test in `crates/evo-cli/tests/fast_edit_run_turnaround.rs`.

The evidence procedure is:

1. compile a small deterministic Evolution fixture through `evo run` using a real-rustc forwarding wrapper;
2. take multiple cold samples with a fresh empty `EVO_CACHE_DIR` for every sample;
3. prime one separate warm cache with an untimed run;
4. take multiple unchanged warm samples against that verified cache;
5. count real rustc compile invocations independently of wall-clock timing;
6. verify expected program output on every sample;
7. emit `report.md`, `report.json`, and `raw-samples.csv` as a CI artifact.

Fast edit-run v0 passes this tooling gate only when:

- measured warm runs perform zero rustc compilations;
- warm output remains correct;
- the warm median is below the cold median on the controlled Ubuntu runner.

The first accepted evidence is recorded in `docs/FAST_EDIT_RUN_CACHE.md`. The exact speedup is evidence for that runner/run, not a cross-machine performance guarantee.

## Build-latency attribution evidence

Build/compiler latency is also a tooling metric and remains separate from generated-program runtime parity. A faster `evo build` does not buy any exception from the runtime ZERO-cost contract.

Build latency baseline v0 is measured by `crates/evo-cli/tests/build_latency_baseline.rs` on the controlled Ubuntu CI runner.

The evidence procedure uses the committed `benchmarks/cases/enums-v0` fixture and records:

1. repeated `evo check` samples;
2. repeated `evo emit-rust` samples;
3. cold `evo build` samples with fresh output paths;
4. unchanged warm `evo build` samples after a prime build;
5. deterministic edited-source rebuild samples where generated Rust changes but expected program output remains the same;
6. direct rustc compile/link samples for the exact emitted Rust with the same successful-build flags;
7. rustc compile invocation counts for every native compilation class;
8. native output correctness for every measured build;
9. raw CSV, JSON, Markdown, generated Rust, source fixture, stdin, expected stdout and verbose rustc identity as the artifact.

The accepted #76 baseline on PR #78 / CI #324 recorded medians:

- `evo check`: **1.207 ms**;
- `evo emit-rust`: **1.210 ms**;
- cold `evo build`: **99.970 ms**;
- unchanged warm `evo build`: **96.986 ms**;
- edited `evo build`: **95.515 ms**;
- direct rustc compile/link: **94.131 ms**.

All cold/warm/edit build classes invoked rustc in **5/5** measured samples. The exact retained generated Rust was 1240 bytes with SHA-256 `61f5f5c99c47196605ae2e461ee589b72a722c4ed5107c6b5fca353795100d83`, matching the accepted Enums baseline.

Interpretation rules:

- correctness and invocation counts are hard evidence; wall-clock timing is supporting evidence;
- `evo build median - direct rustc median` is a rough attribution signal only, because separately sampled medians are not causal profiling;
- timing samples with scheduler outliers remain visible rather than being deleted to make the chart friendlier;
- frontend/check/codegen latency and rustc/link latency must be reported separately where possible;
- an unchanged-build cache must prove **zero rustc compile invocations** on verified hits, not merely a lower median;
- changed-source incremental compilation is a distinct problem from exact artifact reuse and requires separate evidence/design;
- build caching, like run caching, is tooling state and must not alter accepted generated Rust or generated-program runtime behavior.

Accepted artifact: `evo-build-latency-ubuntu-latest`, id `10046420977`, digest `sha256:d3310910395f9c6be50200c261402bfdbb8c7ef617ed18f71b3aa7486a22a2b0`.

## Initial benchmark corpus

### Core language
- integer/floating-point arithmetic
- loops and branches
- function calls
- recursion
- structs/enums/pattern matching
- generics
- trait dispatch

### Memory and collections
- stack-heavy workload
- heap-heavy workload
- Vec
- String
- HashMap
- BTreeMap
- iterator chains
- borrow/reference-heavy paths
- clone-sensitive paths

### Parsing/data
- text scanning
- structured parsing
- JSON encode/decode
- serialization/deserialization
- Unicode/string workloads

### I/O
- sequential file read/write
- buffered I/O
- directory traversal

### Async/network
- task scheduling
- channels
- local TCP echo
- local HTTP client/server
- concurrent local requests

### Concurrency
- thread spawn/join
- mutex/RwLock contention
- atomics
- producer/consumer
- CPU-parallel workloads

### Interop/startup
- FFI calls
- callback overhead
- marshaling
- CLI startup time
- binary size

## Benchmark quality rules

- Use meaningful workloads, not optimizer-eliminated toy operations.
- Use `black_box` or equivalent techniques where needed.
- Keep the algorithm equivalent on both sides.
- Do not precompute Evolution results.
- Keep framework overhead symmetric.
- Store raw samples or enough data to reproduce summary statistics.
- Keep failed/regressed results as artifacts rather than deleting them.
- A smoke case is not a performance claim.
- A noisy CI result is not a PASS merely because its median happened to be favorable.
- Developer-turnaround evidence must stay separate from generated-program runtime parity evidence.
- Build-latency evidence must stay separate from generated-program runtime parity evidence.
- When a benchmark intends code-generation parity, lock the Rust reference to generated Rust so irrelevant source ordering cannot masquerade as a runtime difference.

## CI design

Correctness tests run on normal PR CI. Performance workflows should use as controlled an environment as practical and store artifacts. Reference Rust and Evolution must be measured in the same run whenever possible.

Permanent runtime gates currently include runtime-repeat, control-flow branch, logical operators, function calls, block locals, records, inferred shared-borrow parameters, and enums on the controlled Ubuntu PR CI path. Failed performance steps should preserve their artifacts with guarded `always()` uploads when practical.

Do not start duplicate GitHub Actions for the same SHA/workflow/input. Track the active run ID and continue independent work while it executes.

See issue #5 for the detailed implementation checklist.
