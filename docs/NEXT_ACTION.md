# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-10**

## Stable main before active PR #92

- `f122f4011537f2ed73624c95f1809ba122de924c`
- PR #90 / #89 release-optimization research merge
- post-merge CI #375 / run `34457802639`: **SUCCESS** on Ubuntu, Windows and macOS
- post-merge Release optimization research #4 / run `34457802658`: **SUCCESS**
- Rust toolchain: **1.98.0**

## Active completion — #91 / PR #92

Issue **#91 — compile memory baseline v0** has reached a research decision.

PR:

- #92 `research: establish compile memory baseline v0`
- branch: `research/compile-memory-baseline-v0`
- accepted measurement head before documentation sync: `3e2c3e390ff7c90ce61088b6c54540dce9dbf27f`

Accepted validation:

- CI #376 / run `34470282815`: **SUCCESS** on Ubuntu, Windows and macOS;
- Compile memory research #1 / run `34470282845`: **SUCCESS**;
- artifact id `10149215121`;
- digest `sha256:e7fbfff77bed760784cf5ca9c2d57da5a0df206e670e6fc402349c563be7b12a`;
- correctness **PASS**;
- JSON validation **PASS**.

Accepted process peak-RSS medians:

| Case | check | emit-rust | direct rustc |
| --- | ---: | ---: | ---: |
| enums-v0 | 3,932 KiB | 3,964 KiB | 236,816 KiB |
| logical-operators-v0 | 3,916 KiB | 3,912 KiB | 236,748 KiB |

Frontend phases are only about 1.65-1.67% of direct-rustc peak RSS. The pre-registered 64 MiB + 25% Evolution-side hotspot guide is not met.

Decision: **DEFER / NO ACTION** for Evolution-side compile-memory optimization under the current architecture.

Full `evo build --no-cache` whole-tree peak RSS remains intentionally unavailable because GNU time process RSS is not a simultaneous process-tree peak.

Durable report: `docs/COMPILE_MEMORY_RESEARCH.md`.

## Immediate sequence

1. Keep PR #92 on the single documentation-synchronized final head.
2. Track the natural exact-head normal CI and Compile memory research workflows; do not duplicate them.
3. Require normal CI green on Ubuntu, Windows and macOS.
4. Require final-head compile-memory workflow green and artifact retained.
5. Update PR #92 / #91 with final-head provenance.
6. Squash-merge PR #92 using expected-head protection only after both final-head workflows are green.
7. Track natural post-merge `main` CI and compile-memory push workflow.
8. Close #91 completed only after post-merge main CI succeeds.
9. Re-read live main/branch/PR/Actions state.
10. Only then create #93 binary-size research branch from exact verified main.

## Gated successor — #93

Issue **#93 — binary size baseline v0** is open and must not start before PR #92 merge + green post-merge main CI.

First #93 slice should measure the committed seven-case corpus under production-equivalent Rust 1.98 flags, retaining reference/generated binary sizes, byte identity, section sizes where defensible, `DT_NEEDED`, exact output correctness, generated/reference Rust, JSON/CSV/Markdown evidence.

Do not change stripping, LTO, panic strategy, opt-level or linker in the baseline issue.

Dependency-build, proc-macro and workspace-scaling work remains deferred until Evolution has a real user-program package/dependency graph.

## Production contracts unchanged

- Evolution syntax/semantics and ownership/type rules;
- Rust 1.98.0;
- edition 2024;
- opt-level 3;
- codegen-units 1;
- current linker behavior;
- build-cache-v0 / run-cache-v0;
- source mapping and rustc diagnostic remapping;
- #4 runtime parity-or-better contract;
- no hidden runtime allocation/clone/boxing/dynamic dispatch metadata.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Failed SHAs remain evidence and are not rerun merely for color.
