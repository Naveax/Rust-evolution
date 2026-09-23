# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-23**

## Stable gate

Current exact verified `main`:

`36022b386782775e449ae6a6eedcbbb8c8d1275f`

This is PR #155 squash merge, completing production Weak edge v0.

Natural exact-main validation:

- CI `35842635637`: **SUCCESS** on Ubuntu 24.04, Windows and macOS
- Generational arena surface research `35842635505`: **SUCCESS**
- Append-only sequence performance `35842635557`: **SUCCESS**
- Explicit shared owner performance `35842635675`: **SUCCESS**
- Generational arena performance `35842635563`: **SUCCESS**

## Immediate active P0 — #148 / PR #156

Research question: bounded payload mutation through an exclusive lexical dynamic-borrow guard.

Branch:

`research/exclusive-guard-mutation-surface-v0`

Current exact head:

`6a223c89352bb380cc6b00ee6379c3fc095f8271`

PR #156 is draft.

Pre-registered verdict:

**WHOLE-PAYLOAD-REPLACE-CANDIDATE**

Candidate research spelling:

~~~text
borrow_mut state as edit
    replace edit with Item(value = 2)
end
~~~

Current rule:

1. follow only the natural replacement run set for `6a223c8...`;
2. if a gate fails, patch only demonstrated evidence;
3. do not authorize field/index mutation or escaping guards through this PR;
4. if exact-head CI + dedicated research + regression/performance gates are green, inspect exact-SHA JSON/CSV/Markdown artifacts;
5. merge with expected-head protection;
6. require natural exact-main CI + dedicated research before closing #148.

## Parallel newly-unblocked P0 — #153

Branch:

`research/weak-storage-surface-v0`

Exact start main:

`36022b386782775e449ae6a6eedcbbb8c8d1275f`

No PR yet.

Research order:

1. measure record-field Weak storage first;
2. separately measure `seq weak T` storage;
3. keep arena `handle T` identity distinct;
4. keep enum Weak payloads fail-closed unless research specifically selects enum integration;
5. choose exactly one bounded verdict among the issue’s registered outcomes.

Do not start production Weak storage implementation in this issue.

## Prepared infrastructure

Current prepared heads:

- #149 benchmark provenance: `1bce806e3d8fb582dab79c4421515fa5ae24a640`
- #150 legacy research workflow provenance: `0a2664a6dc3c073d5033b51f9117e85473a6d667`
- #151 tooling evidence provenance, stacked on #149: `264103cc99e8fa62146dbb7ebdb06dd3da97621b`

Open them only with a clean scope audit and without creating pointless duplicate runner pressure.

## Separate blocked lane — #133

A fresh current-main Cross-thread shared ownership research dispatch is still required. Do not rerun exhausted historical attempts.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Failed historical SHAs remain evidence. Merge only with expected-head protection and require natural exact-main postmerge proof.
