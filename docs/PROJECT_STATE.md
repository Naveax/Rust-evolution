# Rust Evolution — Project State

Last verified update: **2026-09-11**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Stable predecessor merge for active #97: `cfc19e3308a505a45c970e883dffc72a8bf0b81b`
- Rust toolchain: **1.98.0**
- production flags: edition 2024, opt-level 3, codegen-units 1
- measured GNU/Linux linker path: `rustc -> cc -> lld`

## Accepted build / compile sequence

### #76 — build latency baseline

Frontend/check/emit is roughly ~1 ms while uncached single-file native builds are dominated by rustc compile+link. Historical direct rustc median: 94.131 ms.

### #79 — verified unchanged-build reuse

`build-cache-v0` is production-accepted. Exact verified unchanged builds can materialize a native artifact without invoking rustc. Corruption/mismatch/unavailable state fails closed to normal compilation. `--no-cache` bypasses lookup/publication.

### #82 — changed-source rustc incremental research

Decision: **REJECT / DEFER**. Tested persistent rustc incremental configurations did not preserve the project runtime contract across the committed corpus. Durable report: `docs/INCREMENTAL_BUILD_RESEARCH.md`.

### #85 — link-time attribution

Current Rust 1.98 Linux path already uses `cc -> lld`. Linker-child work was a material component of total production-equivalent rustc wall time on controlled cases. Durable report: `docs/LINK_TIME_RESEARCH.md`.

### #87 — linker candidate experiment

Decision: **REJECT / DEFER**. Current lld beat tested GNU ld and pinned mold alternatives on total native build latency. Durable report: `docs/LINKER_CANDIDATE_RESEARCH.md`.

### #89 — release optimization cost

Decision: **REJECT / DEFER opt3 -> opt2**. Production remains opt-level 3. Durable report: `docs/RELEASE_OPTIMIZATION_RESEARCH.md`.

### #91 — compile memory baseline

Decision: **DEFER / NO ACTION** for Evolution-side compile-memory optimization under the current architecture. Durable report: `docs/COMPILE_MEMORY_RESEARCH.md`.

### #93 — binary-size baseline

Decision: **DEFER / NO ACTION** for Evolution-specific binary-size optimization under the current language/corpus. PR #94 merged as `733900781ea61f5684209570a35e9f63df71a7d4`; accepted controlled reference/Evolution binaries were byte-identical across the committed seven-case corpus. Durable report: `docs/BINARY_SIZE_RESEARCH.md`.

Dependency-build, proc-macro cost and workspace scaling remain deferred until Evolution has a real user-program package/dependency graph.

## #95 — borrow inference feasibility completed

Issue #95 established that a narrow, statically local read-only nominal-parameter subset can be inferred as a call-duration shared borrow without generalized lifetime solving or hidden runtime machinery.

Accepted research head:

`b3ef92be7e273bc4a27812d24733b87c12bd81c3`

Accepted research evidence:

- CI #384 / run `34478349183`: **SUCCESS** on Ubuntu, Windows and macOS;
- Borrow inference research #3 / run `34478349203`: **SUCCESS**;
- artifact `evo-borrow-inference-research-ubuntu-24.04`;
- artifact id `10152500994`;
- digest `sha256:3ba6965b0ffac03d9e1b74bd82152c4c0cbcf47a5d502f694910483e7f13b6fd`;
- aggregate verdict: **IMPLEMENT-CANDIDATE**;
- 6 safe local candidates, including 2 cases reproducing current move friction.

PR #96 then squash-merged to `main` as `cfc19e3308a505a45c970e883dffc72a8bf0b81b`. Post-merge CI #386 / run `34481370952` and Borrow inference research #5 / run `34481370964` both succeeded, after which #95 was closed completed.

Durable report: `docs/BORROW_INFERENCE_RESEARCH.md`.

## Active implementation — #97 / PR #99

Issue #97 implements inferred shared-borrow nominal parameters v0.

PR #99: `implement: infer shared-borrow nominal parameters v0`  
Branch: `feature/inferred-shared-borrow-parameters-v0`

Latest accepted implementation/benchmark head before documentation synchronization:

`f33b513188add2c6755263f6c7e1079072ec9d7b`

### Implemented semantic contract

The lowering layer now has an explicit parameter passing contract:

- `Owned`: existing by-value behavior;
- `SharedBorrow`: call-duration immutable borrow for a proven read-only nominal parameter.

A parameter is inferred `SharedBorrow` only when:

1. its declared type is nominal (`Record` or `Enum`);
2. its own body contains at least one direct `Inspect` use;
3. it has zero `Consume` uses under the pre-existing/current body-use rules;
4. it is never reinitialized/assigned;
5. classification does not depend on inferred modes of callees.

Nested calls remain consuming boundaries while classifying the caller, so v0 is deliberately non-transitive and requires no fixpoint or generalized lifetime solver.

After classification:

- a direct top-level nominal local passed to a `SharedBorrow` parameter is ownership-inspected instead of moved;
- temporaries and subexpressions retain ordinary owned evaluation and are borrowed only for the call duration;
- later owned move/reinitialization remains valid after a completed shared-borrow call;
- executable/lowered IR carries the decided mode before Rust codegen;
- Rust codegen emits direct ordinary `&T` parameters and `&expr` arguments;
- the record-only and enum-integrated pipelines use the same classifier/passing contract.

The following remain `Owned`/fail-closed in v0:

- parameters returned by value;
- a parameter forwarded through a nested call while classifying the caller;
- owned enum match scrutinees and payload extraction;
- parameter reinitialization;
- any direct consume in a branch or after repeat inspection;
- stored or escaping references.

No Evolution `&` syntax, generalized lifetime inference, mutable borrow inference, implicit clone/copy, boxing, RC/GC, runtime ownership map, dynamic dispatch or stored first-class borrow was added.

### Targeted correctness coverage

Committed lowering/codegen coverage proves:

- repeated read-only calls on one nominal local;
- borrow then later owned move;
- branch/repeat/nested field inspection;
- temporary arguments are borrowed only at the call boundary;
- enum-integrated behavior obeys the same rule;
- owned return/reinitialization boundaries stay owned;
- forwarding remains deliberately non-transitive;
- generated benchmark Rust is exactly locked to the committed reference.

CI #404 / run `34576556404` on exact head `f33b513...` passed fmt, Clippy, workspace tests and release builds on Ubuntu, Windows and macOS.

### Permanent differential/runtime evidence

New permanent case: `benchmarks/cases/inferred-shared-borrow-v0`.

The Evolution fixture performs 5,000,000 repeated read-only calls on a nominal value and later reassigns that owned value. Its Rust reference uses explicit ordinary shared borrowing and is guarded by a committed exact-reference test.

Accepted Ubuntu CI #404 evidence:

- correctness: **true**;
- normalized LLVM IR equal: **true**;
- exact binary equal: **true**;
- reference median: **5,305,283 ns**;
- Evolution median: **5,286,876 ns**;
- observed ratio: **0.996530440**;
- stable: **true**;
- timing verdict: **PASS**;
- final verdict: **PASS**;
- verdict basis: `byte-identical-binary-parity`.

Artifact:

- name `evo-bench-inferred-shared-borrow-ubuntu-latest`;
- id `10189953214`;
- digest `sha256:8fcee06fa8df815e0aaf156649d787cf59459f0d829151811f19ff9ae48d7955`.

Every pre-existing Ubuntu runtime gate also passed on the same head. Several byte-identical cases showed tiny wall-clock ratios above 1.0, but final verdict correctly remained PASS on deterministic binary parity under the established #4 policy.

### Failed-SHA history retained

- CI #394 / run `34487001305`: initial enum-integrated implementation failed rustfmt only.
- CI #396 / run `34494690100`: Clippy exposed stale API/dead-code integration issues.
- CI #399 / run `34495384328`: legacy direct-include integration harnesses lacked the new crate-root bridge.
- CI #402 / run `34496493149` on `64b30b6ddf280adfb04b578ed675331df065c658`: all earlier corpus gates passed, but the new benchmark's manually written Rust reference placed the input helper/function in a different source order than generated Rust. Correctness and stability passed, but LLVM/binary identity was false and the measured ratio was `1.006804464`, so the gate correctly failed. That SHA was not rerun.
- The reference was aligned with generated Rust and a permanent exact-reference test was added. Corrected exact head `f33b513...` then produced byte-identical PASS evidence in CI #404.

## Completion gate still open

#97 is implemented and has accepted implementation/benchmark evidence, but PR #99 must not merge until documentation is synchronized and the exact documentation head receives natural CI **SUCCESS** on Ubuntu, Windows and macOS including the new borrow gate and all existing runtime gates.

After merge, natural `main` CI must succeed on the exact merge SHA before #97 is closed completed.

## Implemented language / tooling state

`docs/LANGUAGE_SPEC_V0.md` is the implemented-language source of truth. Current accepted core includes integer/bool/static strings, input/repeat/control flow, functions, lexical block locals, Records v0, Enums v0, by-value ownership/reinitialization, bounded inferred shared-borrow nominal parameters, source-native move provenance diagnostics, source maps, formatter, native check/emit/build/run and verified run/build caches.

## Zero-cost / safety boundary

Core language work must not silently add hidden clone, allocation, boxing, GC/RC, runtime maps, reflection metadata or dynamic dispatch. Shared-borrow inference is call-duration only. It does not invent `'static`, store inferred references, weaken owned-match payload rules, create user-visible reference values, or extend v0 into generalized lifetime semantics.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed SHAs remain evidence and are not rerun merely for a better color.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
