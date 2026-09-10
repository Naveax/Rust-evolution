# Borrow Inference Feasibility v0 Research

Status: **ACCEPTED RESEARCH — IMPLEMENT-CANDIDATE**

Issue: #95  
PR: #96  
Rust toolchain: **1.98.0**

## Decision summary

The current Evolution ownership model has a useful, statically local read-only nominal-parameter subset that can be lowered to ordinary Rust shared borrows without hidden clone/copy, allocation, boxing, reference counting, GC, runtime metadata, unsafe lifetime widening, or generalized lifetime solving.

Accepted research decision:

**IMPLEMENT-CANDIDATE**

The implementation is intentionally bounded to call-duration shared borrowing for nominal function parameters whose bodies qualify under the existing pre-borrow ownership-use rules. Returned/escaping references, mutable borrows, stored borrow values, match-by-reference and transitive/fixpoint inference remain outside v0.

Separate gated implementation issue: #97.

## Accepted evidence

Accepted exact research head:

`b3ef92be7e273bc4a27812d24733b87c12bd81c3`

Validation:

- CI #384 / run `34478349183`: **SUCCESS** on Ubuntu, Windows and macOS;
- Borrow inference research #3 / run `34478349203`: **SUCCESS**;
- artifact `evo-borrow-inference-research-ubuntu-24.04`;
- artifact id `10152500994`;
- digest `sha256:3ba6965b0ffac03d9e1b74bd82152c4c0cbcf47a5d502f694910483e7f13b6fd`;
- report JSON validation: **PASS**;
- aggregate verdict: **IMPLEMENT-CANDIDATE**;
- safe local candidates: **6**;
- demonstrated current move-friction cases: **2**.

The accepted artifact retains:

- `report.md`;
- `report.json`;
- `raw-matrix.csv`.

No production compiler, language, codegen or runtime behavior is changed by the research PR.

## Existing semantic seam

Evolution already distinguishes ownership uses as `Inspect` versus `Consume` in the enum-integrated ownership analysis and carries that distinction into ownership IR / executable IR. Records v0 also has separate move-tracker operations for inspection and consumption.

The missing piece is function-parameter passing semantics: ordinary function arguments currently form a consuming boundary, so passing a nominal local to a read-only function still moves the caller's value.

The research asks whether a parameter can be classified locally from its own body before changing call behavior.

## Research classifier

The committed research test mirrors the existing ownership context rules rather than inventing a new borrow analysis.

For a target parameter:

- an identifier inherits its incoming use mode;
- field-access base is inspected;
- `print` supplies an inspect context;
- `repeat` count is inspected;
- `if` condition is inspected;
- bind/assignment RHS is a consuming context;
- whole-value return is a consuming context;
- ordinary function-call arguments are consuming contexts while classifying the caller;
- record/enum constructor inputs are consuming contexts;
- owned match scrutinee is a consuming context;
- unary/binary operand traversal keeps the current consuming behavior;
- assigning to the parameter marks it reinitialized and disqualifies shared-borrow inference.

A nominal parameter is classified `SAFE-LOCAL-INFERENCE-CANDIDATE` only when:

1. its type is a Record or Enum;
2. it has at least one `Inspect` use;
3. it has zero `Consume` uses;
4. it is never reinitialized.

The classifier deliberately does not use inferred borrow modes of other functions. This prevents recursive propagation or a whole-program fixpoint from entering v0.

## Accepted fixture matrix

| Case | Classification | Inspect | Consume | Reinit | Current lowering |
| --- | --- | ---: | ---: | --- | --- |
| `direct-field-read` | SAFE-LOCAL-INFERENCE-CANDIDATE | 1 | 0 | false | PASS |
| `double-read-current-friction` | SAFE-LOCAL-INFERENCE-CANDIDATE | 1 | 0 | false | REJECTED-CURRENT-MOVE |
| `shared-read-branches` | SAFE-LOCAL-INFERENCE-CANDIDATE | 2 | 0 | false | PASS |
| `nested-record-read` | SAFE-LOCAL-INFERENCE-CANDIDATE | 1 | 0 | false | PASS |
| `repeat-read-only` | SAFE-LOCAL-INFERENCE-CANDIDATE | 1 | 0 | false | PASS |
| `read-then-move-current-friction` | SAFE-LOCAL-INFERENCE-CANDIDATE | 1 | 0 | false | REJECTED-CURRENT-MOVE |
| `inspect-then-owned-return` | KEEP-BY-VALUE | 1 | 1 | false | PASS |
| `forward-through-current-consuming-call` | KEEP-BY-VALUE | 0 | 1 | false | PASS |
| `owned-match-scrutinee` | KEEP-BY-VALUE | 0 | 1 | false | PASS |
| `parameter-reinitialization` | KEEP-BY-VALUE | 1 | 0 | true | PASS |
| `repeat-read-then-owned-return` | KEEP-BY-VALUE | 1 | 1 | false | PASS |

Six representative cases are locally provable candidates. Two of those directly demonstrate existing user-visible move friction:

- calling the same read-only function twice with the same nominal local is currently rejected after the first by-value call;
- calling a read-only function and then moving the original local by value is currently rejected because the call consumed it.

Both are naturally expressible in Rust as a call-duration `&T` borrow followed by continued ownership of the caller's value.

## Explicit boundaries

The artifact records three important cases outside the local v0 model:

### Returned / escaping borrow

Classification: **REQUIRES-LIFETIME-MODEL**.

Current Evolution syntax has no returned-reference surface. Inferring a reference that escapes a function would create a caller-visible lifetime relationship and therefore cannot be justified by the local call-duration rule.

### Mutable borrow inference

Classification: **REQUIRES-EXPLICIT-BORROW-SYNTAX**.

Mutation requires exclusivity and conflict rules. Silent shared-borrow inference does not provide that contract.

### Stored/overlapping borrow versus move

Classification: **UNSAFE/AMBIGUOUS**.

The v0 candidate never stores a borrow and never lets it outlive the call. A stored or escaping inferred reference would require a real borrow/lifetime model and is therefore rejected from this slice.

## Why the candidate is locally safe

For a qualifying nominal parameter, the function body only inspects the value and never consumes or replaces it. The proposed implementation therefore needs no owned access to that parameter.

At a call site, the borrow can begin immediately before the call and end when the call returns. Ordinary Rust can represent exactly that scope using `&T` and `&expr`. No reference needs to be stored in Evolution user state and no lifetime needs to be named or inferred across statements.

A later by-value move of the caller's original local is valid after the call because the inferred shared borrow has ended. Actual owned moves continue to use the existing move-state diagnostics.

## Non-transitive v0 rule

The research intentionally treats nested calls as consuming boundaries while classifying a function's own parameters.

Therefore a forwarding function such as one whose parameter is only passed to another function remains `Owned` in v0, even if the nested callee independently qualifies for `SharedBorrow`.

After classifications are fixed, a call to a known `SharedBorrow` parameter may inspect the caller's local rather than move it. That does not retroactively reclassify the caller function. No iterative fixpoint is permitted in the first implementation slice.

This rule keeps the semantics deterministic, local and directly supported by the research corpus.

## Required implementation architecture

The research supports an explicit semantic passing mode, not a codegen-only text rewrite.

Recommended implementation boundary for #97:

1. classify function parameters as `Owned` or `SharedBorrow` before executable lowering;
2. preserve the classification in function signature / lowered IR metadata;
3. when lowering a call, use the already-decided callee parameter mode to inspect a top-level nominal local for `SharedBorrow` instead of consuming it;
4. preserve ordinary owned evaluation of temporaries and subexpressions;
5. carry the passing mode through the record-only and enum-integrated executable paths;
6. render `SharedBorrow` parameters as Rust `&T`;
7. render corresponding call arguments as Rust `&expr`;
8. keep codegen free of body-level ownership inference.

Current codegen already has narrow mechanical seams for this:

- function signatures render parameter types in one place in both record-only and enum codegen;
- function calls render argument expressions in one place in both codegen paths;
- enum codegen view already exposes parameter and local ownership metadata;
- record move tracking already distinguishes inspection from consumption.

## Generated Rust expectation

A qualifying read-only parameter should eventually lower in the shape:

```rust
fn __evo_fn_read_value(__evo_item: &__EvoRecord_Item) -> i64 {
    (__evo_item).__evo_field_value
}

let first = __evo_fn_read_value(&__evo_item);
let second = __evo_fn_read_value(&__evo_item);
```

Exact names and formatting remain governed by existing codegen conventions.

This is a lowering sketch, not behavior introduced by PR #96.

## Zero-cost / safety contract

The implementation candidate must not introduce:

- `.clone()` or implicit `Clone`;
- implicit `Copy` for nominal values;
- `Rc`, `Arc`, `Box` or GC;
- heap allocation helpers;
- runtime ownership maps;
- reflection metadata;
- dynamic dispatch;
- unsafe code;
- invented `'static` lifetimes;
- stored or escaping inferred references.

The target lowering is ordinary Rust shared borrowing with no additional runtime ownership mechanism.

## Performance / correctness gate for implementation

#97 must add representative differential evidence for repeated read-only nominal calls and preserve the existing #4 runtime parity-or-better contract.

At minimum the implementation must prove:

- exact output parity against idiomatic Rust explicit shared borrowing;
- generated Rust contains the expected `&T` / `&expr` shape;
- current negative ownership cases remain by-value/fail-closed;
- existing benchmark and three-OS CI remain green;
- no hidden runtime mechanism appears.

## Failed-SHA evidence retained

Initial research head:

`5b3a3ee2b5041bba6010c66ea7264fa3522fd9e9`

- CI #382 / run `34475611657` failed only `cargo fmt --check` on one research-test formatting diff;
- Borrow inference research #1 / run `34475611799` successfully executed the semantic test and printed `IMPLEMENT-CANDIDATE`, 6 safe candidates and 2 friction cases;
- the dedicated workflow then failed because report output was relative to the package test working directory while JSON validation/upload searched workspace-root `target/`.

The failed SHA was not rerun.

Formatting was corrected on a new SHA. The artifact path was then made absolute under `${{ github.workspace }}` on another new SHA. The accepted head `b3ef92be...` completed the full semantic-test + JSON-validation + artifact-upload chain successfully.

## Final research decision

**IMPLEMENT-CANDIDATE** for a deliberately narrow, non-transitive, call-duration inferred shared-borrow parameter mode.

Implementation is tracked separately in #97 and is gated on PR #96 merge plus successful natural post-merge `main` CI and Borrow inference research push workflow.
