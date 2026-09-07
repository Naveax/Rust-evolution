# Fast edit-run cache v0

Status: implementation-backed tooling behavior for `evo run`; tracked by issue #67 and PR #68.

Fast edit-run v0 reduces repeated single-file development latency without changing Evolution language semantics or generated-program runtime behavior.

The normal path remains:

```text
Evolution source
  -> lexer
  -> parser
  -> semantic lowering
  -> generated Rust
  -> verified native compile cache lookup
  -> rustc on miss
  -> native binary execution
```

Frontend validation and Rust code generation always run before a cache lookup. A cache hit never bypasses lexer, parser, lowering, or codegen validation for the current source.

## CLI behavior

The default run command uses the verified cache automatically:

```text
evo run file.evo
```

For reproducibility, debugging, or an explicitly cold compile:

```text
evo run file.evo --no-cache
```

`--no-cache` always takes the normal uncached compile path. `evo build` behavior is unchanged by this v0 slice.

## Cache identity and verification

A cache key is derived from compilation-relevant inputs, including:

- exact Evolution source bytes;
- exact generated Rust source bytes;
- selected `rustc` command identity;
- `rustc -vV` stdout/stderr;
- Rust edition `2024`;
- `opt-level=3`;
- `codegen-units=1`;
- host OS and architecture;
- platform executable suffix.

The key is only an index. **Hash equality is not sufficient for a cache hit.** Before executing a cached binary, v0 verifies exact cached identity files byte-for-byte against the current Evolution source, generated Rust, and compiler/configuration fingerprint.

A valid entry also requires:

- an exact completion marker;
- a regular cached binary file;
- regular identity files;
- no symlink in place of the verified files.

Missing, corrupt, incomplete, or mismatched entries are cache misses and compile normally. Failed rustc compilations are never published as successful cache entries.

## Publication and concurrency

Compilation for a miss occurs in a staging entry. Completion metadata is written only after successful compilation, and the staging directory is renamed to a final entry for publication.

A partially published or interrupted staging directory is not a hit. V0 permits two concurrent processes to compile the same identity redundantly; correctness is preferred over a locking protocol. They must not execute a partially published artifact.

Staging directories older than 24 hours are eligible for cleanup. The v0 cache retains at most 32 final entries and prunes older entries after successful publication.

## Cache location

`EVO_CACHE_DIR` overrides the per-user cache root. The v0 layout appends `run-cache-v0` to that root.

Default roots are:

- Windows: `%LOCALAPPDATA%/RustEvolution`;
- macOS: `$HOME/Library/Caches/rust-evolution`;
- Linux/other Unix: `$XDG_CACHE_HOME/rust-evolution`, falling back to `$HOME/.cache/rust-evolution`.

Therefore final v0 entries live below the corresponding `run-cache-v0` directory.

If no deterministic per-user cache root can be resolved and no `EVO_CACHE_DIR` override is provided, `evo run` falls back to uncached compilation. It does not silently share a generic system temporary cache between users.

## Cost and safety boundary

The cache is developer tooling state on disk. Generated-program runtime cost remains **ZERO** relative to the same generated Rust/native program:

- generated Rust bytes are unchanged by a cache hit;
- language semantics are unchanged;
- no hidden user-program allocation or clone is introduced;
- no VM, GC, daemon, remote cache, executable download, or runtime dispatch is introduced;
- command execution uses structured `Command` arguments rather than shell command construction.

The cache is not a substitute for normal host account/file-system security. V0 specifically rejects symlinked cache identity/binary files and verifies exact content, but it does not claim to be a sandbox for a hostile user who already controls the same account and cache directory.

## Turnaround evidence

Developer-turnaround timing is separate from the runtime parity contract in `docs/PERFORMANCE_CONTRACT.md`.

CI #287 / run `34123091756`, head `e0d2933690897b60491dcce43cebd869991e339e`, produced and uploaded controlled Ubuntu evidence before this documentation-only follow-up:

- 5 cold samples, each using a fresh empty cache;
- 9 measured warm samples after one untimed priming compile;
- cold median: `68.614 ms`;
- warm median: `14.899 ms`;
- warm speedup: `4.605x`;
- cold rustc compile invocations: `5`;
- measured warm rustc compile invocations: `0`;
- verdict: **PASS**.

The artifact contains `report.md`, `report.json`, and `raw-samples.csv`. Exact numbers are runner-specific evidence, not a universal latency guarantee. The acceptance property is that an unchanged verified warm run performs zero rustc compilations and is measurably below the cold median on the controlled Ubuntu evidence runner.

## Non-goals

Fast edit-run v0 is not:

- a general incremental compiler;
- a rustc query cache;
- a Cargo dependency cache;
- a package manager;
- a long-lived compiler daemon;
- a REPL;
- hot reload;
- a remote/shared cache;
- a way to weaken optimization flags or runtime parity requirements.
