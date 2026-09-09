# Verified `evo build` cache v0

This document records the implementation-backed tooling contract for verified unchanged-build native artifact reuse.

It is **not** a language-semantics feature. It does not change accepted Evolution programs, generated Rust semantics, or generated-program runtime behavior/cost.

## Scope

The v0 cache solves one measured problem: repeated **unchanged** `evo build` invocations previously re-ran rustc even when Evolution source, generated Rust and compiler configuration were identical.

It intentionally does not solve changed-source incremental compilation.

## CLI contract

Normal build may reuse the verified cache:

```text
evo build <file.evo>
evo build <file.evo> <output>
```

Explicit bypass:

```text
evo build <file.evo> --no-cache
evo build <file.evo> <output> --no-cache
```

With `--no-cache`, lookup and publication are bypassed and the normal rustc path is used.

## Frontend invariant

Cache reuse never skips the language frontend.

Every invocation still performs:

```text
Evolution source
  -> lex
  -> parse
  -> semantic lowering
  -> generated Rust
  -> cache identity lookup
  -> rustc only on miss / bypass
```

Invalid current source therefore cannot succeed by finding an older native artifact.

## Separate layout from `evo run`

Build reuse uses:

```text
build-cache-v0
```

The accepted `run-cache-v0` behavior is intentionally independent. PR #81 did not modify `crates/evo-cli/src/run_cache.rs`.

Sharing a cache root does not make the two layouts semantically interchangeable.

## Cache root

`EVO_CACHE_DIR` is the explicit root override. The build cache lives under:

```text
$EVO_CACHE_DIR/build-cache-v0
```

Without the override, the platform per-user cache policy is:

- Windows: `%LOCALAPPDATA%/RustEvolution/build-cache-v0`
- macOS: `$HOME/Library/Caches/rust-evolution/build-cache-v0`
- Linux/other Unix: `$XDG_CACHE_HOME/rust-evolution/build-cache-v0` when `XDG_CACHE_HOME` exists, otherwise `$HOME/.cache/rust-evolution/build-cache-v0`

If no usable cache root is available, build falls back to the ordinary uncached path.

## Exact identity

A lookup key locates candidate entries but is not proof of identity.

A hit requires exact verification of:

1. Evolution source bytes;
2. generated Rust bytes;
3. compiler/configuration fingerprint;
4. completion marker;
5. regular non-symlink native artifact.

The compiler/configuration fingerprint contains the selected rustc command identity and `rustc -vV` output plus the build configuration relevant to v0:

- Rust edition `2024`;
- optimization level `3`;
- codegen units `1`;
- OS;
- architecture;
- executable suffix.

Any mismatch is a cache miss.

## Entry shape

A completed entry contains identity/evidence files plus the native program:

```text
entry-<key>-<generation>/
  source.evo
  generated.rs
  compiler.txt
  complete
  program[.exe]
```

The completion marker is exact `complete-v0\n`.

Staging directories use a separate `staging-...` name and are never accepted as completed entries.

## Hit behavior

On a verified hit:

1. the cached binary is rechecked as a regular non-symlink file;
2. the requested output parent directory is created if needed;
3. the cached native artifact is copied to the requested output path;
4. rustc compilation is not invoked.

The requested output path is not part of compilation identity, so the same verified artifact may be materialized to different output locations.

Existing output replacement remains compatible with ordinary build behavior.

## Miss and publication behavior

On a miss:

1. compile through the existing rustc path and flags to the requested output;
2. prepare a separate cache staging directory with exact identity files;
3. copy the successful native output into staging;
4. write the completion marker;
5. rename staging to a completed entry;
6. prune old entries best-effort.

Cache publication is best-effort. A successful requested build remains successful even if cache publication cannot be completed.

Races may compile redundantly rather than accept partial cache state.

## Failure-closed behavior

The following do not produce a cache hit:

- incomplete entry;
- corrupt source identity;
- corrupt generated-Rust identity;
- corrupt compiler identity;
- missing cached binary;
- symlink cached binary where symlink testing is supported;
- compiler fingerprint change;
- Evolution source change;
- generated Rust change;
- unusable/materialization-failed cached artifact.

Cache-root creation failure or unavailable cache storage falls back to normal compilation.

The cache never downloads or executes a remote artifact.

## Bounds and cleanup

v0 keeps at most **32** completed build-cache entries on a best-effort oldest-first basis after publication.

Staging directories older than **24 hours** are eligible for best-effort cleanup.

The policy favors redundant compilation over consuming a possibly partial artifact.

## #76 baseline preservation

Before build caching, #76 measured the controlled Enums v0 fixture with these accepted medians on Ubuntu:

- cold `evo build`: **99.970 ms**;
- unchanged warm `evo build`: **96.986 ms**;
- deterministic edited build: **95.515 ms**;
- direct rustc compile/link: **94.131 ms**.

Cold/warm/edit each invoked rustc **5/5** times.

After verified cache reuse became the normal `evo build` path, the #76 harness was changed to call `evo build ... --no-cache`. This preserves its original uncached attribution semantics rather than silently turning the historical baseline into a cache benchmark.

## Accepted final evidence

PR #81 final head:

```text
4288ddcfccf07fcab60d27e9677685b213005ae2
```

Final PR CI:

- CI #338 / run `34214947823`
- Ubuntu: **SUCCESS**
- Windows: **SUCCESS**
- macOS: **SUCCESS**

Controlled Ubuntu final-head artifact:

- name: `evo-build-cache-turnaround-ubuntu-latest`
- id: `10051449726`
- digest: `sha256:cf7295bf371695347300bee325e3a5a4b5a96bbf4c8789625904d66bd4c538e9`
- platform: `linux-x86_64`
- fixture: `benchmarks/cases/enums-v0/evolution.evo`

Measured evidence:

- cold median: **128.454 ms** across 5 samples
- warm cached median: **17.177 ms** across 9 samples
- cold-to-warm speedup: **7.478x**
- accepted #76 uncached warm baseline: **96.986 ms**
- accepted-baseline-to-cached speedup: **5.646x**
- cold rustc compile count: **5**
- warm rustc compile count: **0**
- correctness: **PASS**

Hard acceptance is exact warm rustc compile count **0** plus correct native output. Timing improvement is supporting evidence only.

PR #81 squash merge:

```text
07e85b3a60739f2d1f25caed0fcb622dd8894861
```

Post-merge main CI **#339** / run `34215424678` is **SUCCESS** on Ubuntu, Windows and macOS. Ubuntu repeated the build-cache evidence step, preserved uncached baseline, benchmark smoke, all existing runtime/performance gates and release build successfully on the merge SHA.

## Generated Rust identity

The controlled final-head artifact retained generated Rust of exactly **1240 bytes** with SHA-256:

```text
61f5f5c99c47196605ae2e461ee589b72a722c4ed5107c6b5fca353795100d83
```

This matches the accepted Enums v0 and #76 baseline.

## Cost and safety boundary

The build cache is compiler/CLI tooling state.

It must not add to generated programs:

- heap allocation;
- clone;
- boxing;
- reference counting;
- dynamic dispatch;
- GC/VM/runtime layers;
- cache lookup code;
- reflection or cache metadata.

No daemon, compiler service, remote cache, executable download, linker replacement, package/dependency redesign, or rustc incremental-session integration is part of v0.

## Changed-source successor

The #76 deterministic edit path remained **95.515 ms** with rustc invoked **5/5** times. Exact artifact reuse cannot help when compilation identity changes.

Issue **#82 — `P0 research changed-source incremental build v0: measure rustc session reuse`** is therefore the bounded research successor.

#82 is measurement/research first. It must determine whether persistent rustc incremental/session state produces a stable, material edited-source gain while preserving correctness, diagnostics/source mapping, bounded state/invalidation rules and existing cache/runtime contracts. A production incremental-build implementation is not implied unless that evidence succeeds.
