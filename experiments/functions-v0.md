# Functions v0 design decision

## Goal

Choose the smallest function surface that preserves static, zero-cost semantics and remains materially simpler than Rust syntax.

## Evaluated surfaces

### A — Rust-like explicit syntax

```text
fn add(a: int, b: int) -> int
    return a + b
end
```

Pros: familiar to Rust users and mechanically precise.

Cons: keeps much of Rust's punctuation ceremony (`:`, `->`) and therefore does not materially advance the project ergonomics goal.

### B — compact explicit types (selected)

```text
fn add(a int, b int) int
    return a + b
end
```

Pros:
- parameter and return types are explicit at the declaration site;
- no call-site-dependent inference;
- fewer punctuation tokens than Rust;
- one-pass function signature collection is straightforward;
- forward calls can be validated before body lowering;
- generated calls can lower directly to ordinary Rust static calls.

Cons:
- still more explicit than Python/Lua;
- type inference is deferred rather than solved.

### C — inferred parameters / return

```text
fn add(a, b)
    return a + b
end
```

Pros: shortest surface.

Cons:
- parameter types become call-site dependent unless a larger inference system is added;
- forward-reference and recursion typing becomes substantially more complex;
- diagnostics become harder before we have a general inference engine;
- risks trading syntax brevity for compiler complexity and surprising semantics.

## Decision

Functions v0 uses **B: compact explicit types**.

```text
fn add(a int, b int) int
    return a + b
end

print add(2, 3)
```

Current type names for signatures are:

- `int` -> Rust `i64`;
- `bool` -> Rust `bool`;
- `string` -> Rust `&'static str` in functions v0.

The current language can only construct string values from source literals, so `&'static str` preserves existing zero-allocation string behavior. This is intentionally a bootstrap ABI, not the long-term ownership model for strings. Once runtime-produced/owned strings exist, string parameter/return ownership must be revisited explicitly rather than silently adding clones or allocations.

## Semantic policy

- functions are top-level only;
- function names and parameter names use existing identifier rules;
- functions have fixed arity;
- parameters and return type are explicit;
- `return expression` is required on every reachable terminal path in v0;
- functions do not return unit in the first slice;
- top-level locals are not captured;
- function-local first assignment defines a local;
- reassignment keeps existing same-type / inferred-mutability rules;
- type-changing reassignment remains rejected;
- calls are expressions;
- function signatures are collected before body lowering, so forward calls are allowed;
- direct recursion is allowed if its explicit signature is valid;
- duplicate function names and duplicate parameter names are rejected;
- no overloading, closures, first-class functions, defaults, variadics, generics, methods, async, or dynamic dispatch.

## Grammar direction

```text
function_definition := "fn" IDENTIFIER "(" parameters? ")" type_name NEWLINE+ function_block "end"
parameters          := parameter ("," parameter)*
parameter           := IDENTIFIER type_name
type_name           := "int" | "bool" | "string"
return_statement    := "return" expression
call_expression     := IDENTIFIER "(" arguments? ")"
arguments           := expression ("," expression)*
```

Function definitions are declarations, not executable top-level statements. Calls remain ordinary expressions.

## Zero-cost target

The accepted implementation should lower to the equivalent Rust shape:

```rust
fn __evo_fn_add(__evo_a: i64, __evo_b: i64) -> i64 {
    return (__evo_a + __evo_b);
}
```

Calls become direct static Rust calls. No heap allocation, boxing, vtable, registry, interpreter, or runtime dispatch is permitted solely to support named functions.

## Acceptance evidence

Before merge:

- parser/lowering/codegen tests cover definitions, calls, arity/types, returns, forward calls and recursion policy;
- source-native diagnostics cover malformed signatures and semantic call errors;
- formatter is deterministic/idempotent;
- one runtime-dependent function benchmark compares against equivalent idiomatic Rust;
- exact binary/LLVM parity is preferred; otherwise the strict stable timing gate from #4/#5 applies.

Tracks #36.