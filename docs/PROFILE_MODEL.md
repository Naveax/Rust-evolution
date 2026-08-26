# Rust Evolution — Profile / Capability Model

Status: **long-term architecture; no user-facing `profile` syntax is implemented today.**

## Purpose

Rust Evolution aims to span many domains without turning the core grammar into unrelated dialects.

A profile is primarily a bundle of:

- semantic capabilities;
- validation rules;
- libraries/prelude selection;
- backend requirements;
- runtime requirements;
- cost-policy defaults;
- tooling integrations.

It is not permission to redefine the meaning of core constructs.

## Candidate profiles

Long-term candidates include:

- `systems`;
- `scripting`;
- `web`;
- `data`;
- `gpu`;
- `distributed`;
- `embedded`;
- `game`;
- `enterprise`;
- `verified`;
- `hardware`.

These names are architectural placeholders until proven by real implementations.

## Shared core rule

Core semantics such as bindings, `fn`, calls, `if`, enums, pattern matching, ownership, types and errors should keep one meaning across profiles.

Profiles may add domain restrictions/capabilities but should not create separate languages wearing one executable name.

## Classification test

Before adding a domain feature, ask:

### Is it general language semantics?

If yes, it may be a Core candidate.

### Is it domain-specific compile-time semantics/validation?

Prefer Profile / Capability.

### Is it reusable behavior without new semantics?

Prefer Library.

### Does it require an execution/failure/runtime model?

Prefer Optional Runtime with explicit cost.

### Is it target-specific code generation?

Prefer Backend.

### Is it mainly developer experience/inspection?

Prefer Tooling.

## Example: GPU

A future GPU profile may provide:

- kernel entry metadata;
- memory-space types;
- workgroup/thread builtins;
- synchronization/resource validation;
- SPIR-V/PTX/Metal/DXIL backend selection;
- transfer/cost diagnostics.

It should not redefine ordinary function calls or booleans throughout the language.

## Example: Verified

A verified profile may add:

- contracts;
- proof/refinement metadata;
- stronger effect/capability checks;
- proof-obligation tooling.

Simple non-verified programs should not inherit theorem-prover ceremony.

## Example: Embedded

An embedded profile may select:

- `no_std` / no-main environment;
- static-allocation constraints;
- HAL/platform capabilities;
- interrupt/MMIO rules;
- bare-metal backend/build defaults.

Core ownership and safety semantics remain shared.

## Progressive disclosure

A beginner should not need to understand profile/backend details to write ordinary code.

Complexity becomes visible only when the chosen domain requires it.

## Acceptance requirements for a new profile

A profile proposal must document:

1. problem/domain;
2. why Core/Library alone is insufficient;
3. semantic capabilities it adds;
4. cost/runtime class;
5. backend/runtime dependencies;
6. safety effects;
7. diagnostics/tooling behavior;
8. interop story;
9. representative corpus;
10. benchmark/validation plan.

No profile should be accepted solely because another language has a keyword for the domain.