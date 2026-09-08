# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-08**

## Verified merged baseline

Build latency baseline v0 **#76 is completed** and PR **#78 is merged**.

Latest verified code-bearing `main` baseline:

- `34f0815d0821543586cd3ae73b9b5b616a1396d3`
- PR #78 squash merge
- post-merge CI **#325** / run `34205500589`: **SUCCESS** on Ubuntu, Windows and macOS
- Rust toolchain: **1.98.0**

A later docs-only handoff merge may advance live `main`. Before implementation, verify live `main`, open PRs and queued/in-progress Actions rather than assuming this file can predict its own future squash SHA.

## Accepted #76 evidence

Final PR head:

- `96b8de7570662aa5cf5886f90ea5f0432a2c14fe`

Final PR CI:

- CI **#324** / run `34202560065`: **SUCCESS** on Ubuntu, Windows and macOS.

Accepted Ubuntu artifact:

- `evo-build-latency-ubuntu-latest`
- artifact id `10046420977`
- digest `sha256:d3310910395f9c6be50200c261402bfdbb8c7ef617ed18f71b3aa7486a22a2b0`

Controlled medians:

- `evo check`: **1.207 ms**
- `evo emit-rust`: **1.210 ms**
- cold `evo build`: **99.970 ms**
- unchanged warm `evo build`: **96.986 ms**
- edited `evo build`: **95.515 ms**
- direct rustc compile/link: **94.131 ms**

Approximate build-minus-direct-rustc median signal:

- cold: **5.839 ms**
- warm: **2.856 ms**
- edit: **1.384 ms**

Rustc compile counts across five measured samples:

- cold: **5/5**
- unchanged warm: **5/5**
- edit: **5/5**
- direct rustc: **5/5**

Exact retained generated Rust:

- 1240 bytes
- SHA-256 `61f5f5c99c47196605ae2e461ee589b72a722c4ed5107c6b5fca353795100d83`
- matches the accepted Enums v0 baseline.

Retained failed evidence:

- CI #323 / run `34202332386`: rustfmt-only first harness head; not rerun.

## Evidence decision

#76 completed with:

- **Implement:** verified unchanged-build artifact reuse v0;
- **Research separately:** changed-source incremental-rustc/session reuse.

Do not optimize the frontend for this path first. The measured frontend is ~1.2 ms while direct rustc compile/link is ~94.1 ms. Unchanged warm build still invokes rustc every time.

Do not claim exact artifact reuse solves changed-source rebuilds. The deterministic edit path remains ~95.5 ms with rustc invoked 5/5 times.

## Active P0 — #79

Issue **#79 — P0 verified build artifact reuse v0: unchanged evo build without rustc** is open.

Parent: #2

Weakness source: #6 Build / Compile.

Roadmap: #1 Phase 3.4 warm build / incremental build.

### Required design boundary

Keep #79 separate from `run-cache-v0` behavior.

Preferred v0 shape:

- separate `build-cache-v0` layout / adapter;
- same per-user cache-root policy and `EVO_CACHE_DIR` override;
- exact Evolution source + generated Rust + compiler/configuration identity verification;
- completion marker and regular non-symlink native artifact requirement;
- bounded local pruning;
- races may compile redundantly rather than consume partial entries;
- corruption/mismatch/unavailable cache fails closed to normal compilation;
- frontend validation/lowering/codegen always runs before cache reuse;
- hit materializes the verified native artifact to the requested build output path;
- generated Rust and runtime semantics unchanged.

Do **not** silently change the existing #67 `run-cache-v0` layout or behavior to make the implementation look more generic.

### CLI behavior

Required interface:

- normal `evo build <file.evo> [output]` may use verified cache reuse by default;
- `evo build <file.evo> --no-cache` bypasses cache and uses default output;
- `evo build <file.evo> <output> --no-cache` bypasses cache and uses explicit output;
- existing explicit output behavior remains compatible.

## First implementation sequence

Only after this docs-only handoff is merged and its `main` CI is green:

1. Verify live `main`, open PRs and active queued/in-progress Actions.
2. Create `feature/verified-build-cache-v0` from exact verified `main`.
3. Add a dedicated `build_cache.rs` with exact identity verification and tests before wiring CLI behavior.
4. Reuse policy ideas from `run_cache.rs`, but keep layout and externally observable run-cache semantics independent.
5. Add safe artifact materialization to requested output. Parent directories and replacement behavior must remain compatible with current build semantics.
6. Update build argument parsing for optional `--no-cache` without breaking explicit output paths.
7. Wire cache lookup only after `load_program()` has completed successfully.
8. On miss, compile once with the normal existing rustc path/flags, publish a complete verified cache entry, then materialize output.
9. Add process-level tests using the rustc-counting wrapper:
   - cold first build = 1 compile;
   - unchanged warm build = 0 compile;
   - `--no-cache` = compile;
   - changed source/generated/compiler identity = miss;
   - corruption/incomplete/missing/symlink = miss;
   - different output paths reuse the same verified cached artifact;
   - resulting binary output stays correct.
10. Prove `evo run` cache tests and behavior remain unchanged.
11. Extend the #76 Ubuntu build-latency evidence to compare accepted baseline against cached warm builds.
12. Hard acceptance: unchanged warm rustc compile count **0** plus correct output. Timing improvement is supporting evidence, not semantic proof.
13. Keep normal three-OS fmt/Clippy/workspace tests and every existing Ubuntu turnaround/runtime/performance gate green.
14. Merge only from the exact final head after one natural final CI run.

## Explicit non-goals

Do not fold these into #79:

- changed-source incremental rustc sessions;
- compiler daemon/server;
- remote cache or executable download;
- linker replacement;
- Cargo/dependency graph work;
- package system semantics;
- runtime-language changes.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input.

If a run is queued or in progress, track that run ID and continue independent work. Failed SHAs remain evidence; do not rerun them merely to obtain a friendlier color.
