# Rust Evolution — Cost Model

Status: **architectural direction; only the existing zero-cost/native core policy is implemented today.**

The purpose of the cost model is to make hidden runtime/ownership/runtime-system costs visible and enforceable.

## ZERO

A ZERO-class feature is intended to lower to equivalent static/native constructs without hidden runtime work.

Forbidden implicit costs include:

- allocation;
- clone/copy beyond the defined value semantics;
- boxing;
- dynamic dispatch;
- reference counting;
- managed runtime/VM/GC dependency;
- hidden host/device transfer;
- hidden synchronization with material runtime cost.

Current core features are developed under this class unless explicitly stated otherwise.

For runtime-relevant ZERO work, the project contract remains:

`T_evolution <= T_reference_rust`

Correctness is required first. Exact independently compiled executable parity is deterministic runtime-parity evidence.

## EXPLICIT

An EXPLICIT cost is allowed only because the programmer intentionally selected the operation/capability and its cost is visible in syntax, types, API, capability metadata, or tooling.

Examples may eventually include:

- heap allocation;
- dynamic dispatch;
- async runtime participation;
- explicit actor/message runtime;
- reflection;
- GPU transfers;
- explicit synchronization;
- FFI marshaling.

## MANAGED

A MANAGED capability uses a special runtime model by design.

Possible future examples:

- optional garbage collection;
- supervision/actor runtime;
- managed plugin/scripting sandbox;
- distributed runtime.

Managed behavior is optional and must never become an invisible dependency of ordinary core programs.

## Annotation syntax is not frozen

The Omni vision discusses `@zero`, `@explicit`, and `@managed`. Those spellings are examples, not accepted language syntax.

First build the semantic representation and analyzer; only then freeze user-facing syntax.

## `evo cost` direction

Long-term tooling should expose costs such as:

```text
Implicit allocations    0
Explicit allocations   14
Implicit clones         0
Boxing                   0
Dynamic dispatch        2
Managed runtime         none
Unsafe regions          1
FFI boundaries          3
GPU kernels             2
GPU transfers           4
```

Potential CI modes:

- `evo cost --check`;
- cost snapshot artifacts;
- before/after cost diffs;
- fail on newly introduced implicit allocation/clone/dispatch;
- per-profile cost policies.

## Feature acceptance

Every future proposal must state:

1. its cost class;
2. its baseline;
3. implicit allocation/clone/boxing/dispatch behavior;
4. runtime dependency;
5. safety implications;
6. how costs are tested/inspected;
7. performance benchmark requirements.

"Convenient" is not a cost model.