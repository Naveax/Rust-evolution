# Summary

Describe one primary purpose.

## Semantics / behavior

- What changes for Evolution users?
- What remains intentionally unchanged?
- If syntax is new, what is the exact semantic rule?

## Architecture classification

- [ ] Core
- [ ] Profile / Capability
- [ ] Library
- [ ] Optional Runtime
- [ ] Backend
- [ ] Tooling
- [ ] Research-only

## Cost class

- [ ] ZERO
- [ ] EXPLICIT
- [ ] MANAGED
- [ ] Not runtime relevant

Hidden allocation / clone / boxing / dynamic dispatch / runtime dependency notes:

## Correctness / safety

- [ ] unit/integration coverage
- [ ] differential correctness where applicable
- [ ] error/edge cases
- [ ] safety impact documented
- [ ] unsafe invariants documented if any

## Diagnostics / tooling

- [ ] source-native diagnostics covered where relevant
- [ ] formatter behavior covered for user syntax
- [ ] source-map/rustc remap behavior preserved

## Performance / codegen

Reference baseline:

- [ ] generated Rust inspected
- [ ] normalized LLVM/codegen evidence where useful
- [ ] exact executable parity checked where achievable
- [ ] strict timing gate checked when binaries differ
- [ ] raw samples/report artifact retained

Result: PASS / FAIL / INCONCLUSIVE / N/A

## CI evidence

Head SHA:

Run ID(s):

Do not create duplicate runs for the same SHA/workflow/inputs.

## Documentation

- [ ] `LANGUAGE_SPEC_V0.md` updated only if behavior is proven and accepted
- [ ] durable decision added to `docs/DECISIONS.md` if needed
- [ ] long-term-only ideas kept in `OMNI_VISION.md` / research, not current spec

## Continuation / handoff

Before merge or before stopping incomplete work:

- [ ] `docs/PROJECT_STATE.md` updated if verified project state changed
- [ ] `docs/NEXT_ACTION.md` updated to the exact next continuation point
- [ ] temporary dev/mutation workflows removed when no longer needed
- [ ] issue/PR contains meaningful failure history and final evidence

## Known limitations / follow-ups

List explicitly. Do not hide failed experiments or deferred semantics.