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

## CI design

Correctness tests run on normal PR CI. Performance workflows should use as controlled an environment as practical and store artifacts. Reference Rust and Evolution must be measured in the same run whenever possible.

Do not start duplicate GitHub Actions for the same SHA/workflow/input. Track the active run ID and continue independent work while it executes.

See issue #5 for the detailed implementation checklist.
