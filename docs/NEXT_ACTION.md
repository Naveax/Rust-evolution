# Rust Evolution — NEXT ACTION

This file is intentionally operational. A fresh chat/agent should be able to resume from here without prior conversation history.

Last verified update: **2026-09-08**

## Verified merged feature baseline

Diagnostic suggestions v0 **#74 is completed** and PR **#75 is merged**.

Latest verified code-bearing `main` baseline:

- `cd6dcd096af20f9f94c4f201e715ae87c848c480`
- PR #75 squash merge
- post-merge CI **#320** / run `34198404953`: **SUCCESS** on Ubuntu, Windows and macOS
- Rust toolchain: **1.98.0**

A later docs-only handoff merge may advance live `main`. Before implementation, verify live `main`, open PRs and active Actions rather than assuming this document can contain its own future merge SHA.

## Final diagnostic-suggestions evidence

Final PR head:

- `1f3fdec69f81c8d6742d3fde47d698a3df3f3543`

Final PR CI:

- CI **#319** / run `34197970982`: **SUCCESS** on Ubuntu, Windows and macOS.

Accepted behavior:

- deterministic conservative one-edit matcher for current ASCII identifiers;
- insertion/deletion/substitution/adjacent-transposition support;
- exactly one close unique-best candidate is required;
- tied, distant, one-character and empty-candidate cases remain silent;
- local candidate lookup respects lexical visibility;
- function, record and enum suggestions remain in their correct semantic namespaces;
- primary diagnostic text/span remain stable and suggestion is bounded extra `help:` text;
- textual help and #69 move-origin related source locations can coexist;
- stale suggestion state cannot leak to an unrelated diagnostic;
- parser autocorrect, automatic source edits and LSP/code actions remain out of scope;
- ownership semantics and accepted generated-program behavior remain unchanged.

Retained failed/intermediate evidence was not rerun merely for color:

- #314 / `34191629058`: rustfmt-only failure;
- #316 / `34192026716`: rustfmt-only process-test failure;
- #317 / `34197292048`: acceptance-test failure demonstrating `MabyInt -> MaybeInt` exceeds the fixed one-edit boundary. The test was corrected to `MaybInt -> MaybeInt`; the matcher threshold was not loosened.

Generated-output preservation evidence:

- baseline and feature `generated.rs` SHA-256: `61f5f5c99c47196605ae2e461ee589b72a722c4ed5107c6b5fca353795100d83`;
- baseline and feature native executable SHA-256: `d2b172767e1ca9267173322ca2d2eb3bc9941361c93f3f64056d6c81d05d0431`;
- baseline and feature LLVM IR SHA-256: `0c82e4c394ab5edc7f32d21a9f48800ada2ed169c695c04b8d66c677eecc533f`.

## Active P0

Issue **#76 — P0 build latency baseline v0: cold, warm and edit compile attribution** is open.

Parent: #2

Weakness source: #6 Build / Compile.

Roadmap: #1 Phase 3.4 — cold build baseline, warm build baseline, incremental build.

### Verified root cause / feasibility

Current `evo build` uses the normal frontend/load path and then calls `compile_rust()` directly.

`compile_rust_with_rustc()`:

- creates a fresh temporary compile directory;
- writes generated `main.rs`;
- invokes selected rustc with the existing edition/optimization/codegen-unit flags;
- writes the native binary to the requested output;
- removes the temporary directory.

The persistent verified compile cache from #67 exists only in `run_generated()` for `evo run`. `evo build` does not consult it.

Thus unchanged `evo build` currently follows a full rustc compile/link control path. #76 must still prove actual invocation counts and timings under controlled evidence instead of treating static inspection as a latency measurement.

Existing `crates/evo-cli/tests/fast_edit_run_turnaround.rs` already contains useful bounded test infrastructure:

- rustc wrapper construction;
- compile-invocation counting;
- `Instant` timing;
- median computation;
- raw sample JSON/CSV/Markdown reporting;
- controlled Ubuntu ignored-test CI pattern.

Reuse or factor this infrastructure where doing so keeps the patch smaller and clearer. Do not add a dependency merely to avoid a few obvious standard-library helpers.

## #76 first implementation slice

After this docs-only handoff is merged and its `main` CI is green:

1. Verify live `main`, open PRs and queued/in-progress Actions.
2. Create `bench/build-latency-baseline-v0` from exact verified `main`.
3. Do **not** change `evo build` caching or semantics.
4. Add a controlled ignored integration test/harness under `evo-cli` that can:
   - time `evo check`;
   - time `evo emit-rust`;
   - time cold `evo build`;
   - time unchanged repeated `evo build`;
   - make one deterministic small source edit and time rebuilt output;
   - compile the exact emitted Rust directly with rustc using the same flags;
   - count rustc compile invocations for each build class;
   - verify resulting binary output.
5. Use one representative accepted fixture with functions/control flow/nominal data. Prefer reusing an existing stable Enums-v0-like representative source rather than inventing a new language surface.
6. Record toolchain, target, flags, git SHA, raw samples, median, min/max and p95/max as defined in #76.
7. Emit machine-readable JSON/CSV and human-readable Markdown into a stable artifact directory.
8. Add one Ubuntu-only CI evidence step and artifact upload only after the harness is coherent.
9. Keep normal three-OS workspace quality/tests and all existing Ubuntu runtime/performance gates unchanged.
10. From the resulting evidence classify the next build intervention as **Implement / Research / No-action**.

## Acceptance boundary

#76 is measurement/attribution only.

Do not fold in:

- `evo build` cache reuse;
- rustc incremental-session integration;
- a daemon/compiler server;
- remote cache;
- linker replacement;
- package/dependency semantics;
- proc-macro/workspace-scale optimization;
- runtime-language changes.

No generated Rust semantic change is expected. Generated-program runtime cost remains **ZERO**.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input.

If a run is queued or in progress, track that run ID and continue independent work. Failed SHAs remain evidence; do not rerun them merely to obtain a friendlier color.
