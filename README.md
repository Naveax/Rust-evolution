# Rust Evolution

Rust Evolution is an experimental language/platform project focused on making Rust-class safety and native performance available with substantially lower ceremony, while keeping hidden costs forbidden and engineering claims measurable.

## North star

> **Simple things simple. Complex things possible. Hidden costs forbidden.**

Long term, the project aims to scale from scripting and applications down to systems, embedded/kernel/driver work and across to GPU/graphics, data/AI/scientific, distributed and verified domains through a small shared core plus capabilities/profiles, libraries, optional runtimes, backends and tooling.

See `docs/OMNI_VISION.md` for the long-term architecture. It is a vision document, not a claim that all those domains are implemented today.

## Current compiler path

Today the proven pipeline remains:

```text
Evolution source
  -> lexer
  -> parser
  -> semantic lowering
  -> generated Rust
  -> rustc
  -> native binary
```

There is no mandatory VM or standalone managed runtime.

## Current implemented core

The current `main` language includes, among other validated slices:

- integer, boolean and current literal/static string values;
- bindings with first-definition vs reassignment analysis;
- inferred mutability;
- arithmetic and comparisons;
- strict short-circuit `and` / `or` / `not` with no truthiness;
- `input_int`;
- `repeat ... end`;
- `if / else / end`;
- compact typed named functions with direct static calls, forward calls and recursion;
- source-native diagnostics and bounded recovery;
- formatter support;
- generated-Rust source mapping;
- rustc diagnostic remapping to Evolution source;
- native `check`, `emit-rust`, `build`, `run`, and `fmt` workflows;
- differential correctness/performance harness with raw/JSON/Markdown artifacts, LLVM comparison, binary-size comparison and exact executable parity evidence.

The authoritative implemented language semantics are in `docs/LANGUAGE_SPEC_V0.md`.

## Example

```text
fn step(x int) int
    if x > 1 and not (x == 7)
        return x / 2
    else
        return x + 3
    end
end

n = input_int
x = input_int
sum = 0
repeat n
    x = step(x)
    sum = sum + x
end
print sum
```

## Commands

From the workspace:

```text
cargo run -p evo-cli -- check examples/basic.evo
cargo run -p evo-cli -- emit-rust examples/basic.evo
cargo run -p evo-cli -- run examples/basic.evo
cargo run -p evo-cli -- build examples/basic.evo
cargo run -p evo-cli -- fmt examples/basic.evo
```

## Non-negotiable zero-cost runtime rule

For equivalent semantics, inputs, target, toolchain and optimization conditions:

```text
T_evolution <= T_reference_rust
performance_ratio = T_evolution / T_reference_rust <= 1.00
```

Correctness must match first. Stable regressions are not accepted because syntax is shorter.

If independently compiled executables are byte-identical after correctness PASS, runtime parity is deterministic; raw wall-clock timing remains reported but scheduler noise cannot turn the same executable into a real regression.

See `docs/PERFORMANCE_CONTRACT.md` and `docs/BENCHMARKING.md`.

## Hidden-cost policy

Core/zero-cost ergonomics must not silently add:

- heap allocation;
- clone;
- boxing;
- dynamic dispatch;
- reference counting;
- mandatory GC/VM/runtime layers.

The long-term ZERO / EXPLICIT / MANAGED direction is documented in `docs/COST_MODEL.md`.

## Profiles and domains

The long-term platform may expose domain profiles/capabilities for systems, scripting, web, GPU, data, distributed, embedded, game, enterprise, verified and hardware-oriented work.

Profiles are intended to select capabilities, validation, libraries, backends and runtime/cost requirements, **not** to create unrelated dialects with different core semantics.

See `docs/PROFILE_MODEL.md`.

## Continue this project in a fresh chat/session

GitHub is deliberately the durable memory layer.

If a new assistant/session is asked to **“continue Rust Evolution”**, it should not require the entire chat history. Start with:

1. `AGENTS.md`
2. `docs/PROJECT_STATE.md`
3. `docs/NEXT_ACTION.md`
4. the active issue/PR named there
5. current GitHub Actions for that head SHA

Detailed procedure: `docs/CONTINUATION_PROTOCOL.md`.

`docs/NEXT_ACTION.md` is intentionally the single continuation pointer. If it is stale relative to GitHub, correct it during the session.

## Current active work

At the time of the latest handoff, the active P0 is **#38 Block-local bindings v0**, tracked in draft PR **#39** on branch `feature/block-locals-v0`.

Do not trust this README alone for volatile status. Read `docs/PROJECT_STATE.md` and `docs/NEXT_ACTION.md`.

## Research model

Rust Evolution may study ideas from many languages and ecosystems, but syntax is not copied wholesale. Each serious idea should be evaluated through the research matrix in `research/languages/README.md` and classified as Core, Profile/Capability, Library, Optional Runtime, Backend or Tooling.

## Engineering principles

- `main` remains stable.
- Focused feature/research/experiment/benchmark branches are used for real work.
- Correctness comes before benchmark claims.
- Safety is not traded for speed.
- Ergonomics cannot hide runtime cost.
- Failed experiments and regressions are recorded rather than erased.
- User-facing spec updates happen only after behavior is proven.
- Duplicate CI runs for the same logical SHA/workflow/input are forbidden.
- Every significant merge or incomplete stopping point updates the durable handoff files.

## Key project documents

- `AGENTS.md` — mandatory agent/continuation/CI contract
- `docs/PROJECT_STATE.md` — verified current status
- `docs/NEXT_ACTION.md` — exact continuation point
- `docs/LANGUAGE_SPEC_V0.md` — implemented language
- `docs/OMNI_VISION.md` — long-term north star
- `docs/DECISIONS.md` — durable architecture/language decisions
- `docs/COST_MODEL.md` — explicit cost architecture
- `docs/PROFILE_MODEL.md` — profile/capability architecture
- `docs/PERFORMANCE_CONTRACT.md` — runtime invariant
- `docs/BENCHMARKING.md` — differential harness policy
- `docs/ROADMAP.md` + issue #1 — staged roadmap
- `research/languages/README.md` — multi-language idea evaluation matrix

## Project tracking

Core long-lived issues include:

- #1 Master roadmap
- #2 Ergonomic language/frontend program
- #4 Runtime performance invariant
- #5 Benchmark/differential system
- #6 Rust weakness map

Atomic P0 issues/PRs are created as the implementation progresses.