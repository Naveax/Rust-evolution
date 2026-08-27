# Records v0 design experiment

Issue: #41

Status: implementation candidate, not language spec until accepted by tests/performance.

## Goal

Introduce nominal, statically typed product data that lowers to ordinary Rust structs without runtime object machinery.

## Chosen v0 surface candidate

### Declaration

```text
record Point
    x int
    y int
end
```

### Named construction

```text
p = Point(x = 2, y = 3)
```

### Field access

```text
print p.x
```

### Function use

```text
record Point
    x int
    y int
end

fn sum(p Point) int
    return p.x + p.y
end

p = Point(x = 2, y = 3)
print sum(p)
```

## Why named construction

Candidates considered:

1. `Point(2, 3)`
2. `Point(x = 2, y = 3)`
3. `Point { x = 2, y = 3 }`

v0 selects **`Point(x = 2, y = 3)`**.

Reasons:

- field names remain visible at the call site;
- reordering fields does not silently change meaning;
- missing/duplicate/unknown field diagnostics can point to named initializer spans;
- no new brace-based block family is introduced;
- the existing `=` visual language is reused rather than adding `:` solely for construction;
- it leaves future default/named-field evolution open without making positional order part of the public ABI.

Function calls remain positional in v0. A call-like expression containing named `field = expression` arguments is therefore syntactically distinguishable as a record-construction candidate and semantic resolution verifies that the callee name is a record type.

## Declaration region

`record` and `fn` are both top-level declarations and may be interleaved **before the first executable top-level statement**.

Example:

```text
record A
    value int
end

fn read(a A) int
    return a.value
end

record B
    inner A
end

b = B(inner = A(value = 3))
print read(b.inner)
```

The parser remains syntax-only. Semantic pre-passes collect all record declarations before resolving field types and function signatures, so declaration order within the declaration region does not determine type visibility.

A `record` or `fn` after executable top-level code is rejected.

## Type model

Records are **nominal** types.

Two declarations with the same fields are not interchangeable:

```text
record A
    value int
end

record B
    value int
end
```

`A` and `B` are different types.

The semantic type representation should become structured enough to represent `Record(name)` directly. Do not encode a user record by pretending it is a string scalar.

Initial field/signature types:

- `int`
- `bool`
- `string` (current literal/static string model)
- named record types

## Recursive layout policy

By-value recursive record cycles are rejected in v0.

Invalid:

```text
record Node
    next Node
end
```

Also invalid through an indirect cycle:

```text
record A
    b B
end

record B
    a A
end
```

Rust requires indirection for such layouts. Records v0 will not silently insert `Box`, `Rc`, handles, or another heap/runtime representation.

## Value / ownership semantics

**No implicit Copy or Clone is added by Records v0.**

Generated Rust structs do not receive an automatic `Copy`/`Clone` derive merely because their fields could support it.

Therefore record assignment, function argument passing and return use ordinary Rust move semantics.

Example:

```text
record Point
    x int
end

fn read(p Point) int
    return p.x
end

p = Point(x = 1)
print read(p)
print p.x
```

The final use may be rejected by rustc as use-after-move. The existing generated-line source map / rustc diagnostic remapping must report the error against Evolution source rather than silently cloning `p`.

Why this conservative bootstrap policy:

- hidden clone is forbidden;
- silently deriving Copy would create a new language semantic rule before ownership analysis exists;
- ordinary Rust move behavior is already safety-correct and native;
- later work may add an explicit or provably safe Copy policy, but that must be a separate accepted decision.

## Field mutation

Direct field assignment such as:

```text
p.x = 4
```

is **not part of Records v0**.

Field reads are supported first. Record rebinding still uses ordinary local assignment semantics. Field mutation requires a deliberate mutability/ownership design and should not sneak in because a dot token exists.

## Parser direction

Add:

- `record` keyword;
- `.` token;
- record declaration AST with field declaration spans;
- named-construction AST or generic call-like named initializer representation;
- postfix field-access expressions.

Postfix access has higher precedence than unary/binary operators and may chain:

```text
outer.inner.value
```

## Semantic passes

Suggested order:

1. collect record names / field syntax;
2. resolve record field types and reject unknown type names;
3. detect direct/indirect by-value record cycles;
4. collect function signatures using resolved type model;
5. lower function bodies and top-level statements;
6. validate record constructors and field access.

## Rust codegen target

Conceptual output:

```rust
struct __evo_record_Point {
    __evo_field_x: i64,
    __evo_field_y: i64,
}

fn __evo_fn_sum(__evo_p: __evo_record_Point) -> i64 {
    return (__evo_p.__evo_field_x + __evo_p.__evo_field_y);
}
```

No object map, reflection table, vtable, allocation, boxing, GC, Rc, RefCell, or managed runtime is needed.

## Benchmark direction

The first performance case should repeatedly construct a small record from runtime-dependent scalar values and consume fields in a recurrence.

Reference Rust should mirror ordinary generated Rust shape and be regression-locked, as with Functions v0 and Block locals v0.

Acceptance prefers:

- correctness PASS;
- no hidden allocation/clone/boxing/dispatch;
- normalized LLVM parity;
- exact executable parity;
- otherwise strict stable timing ratio <= 1.00.

## Explicit non-goals

- methods;
- field mutation;
- inheritance/classes;
- reflection;
- defaults;
- positional record construction;
- automatic Copy/Clone;
- recursive heap records;
- enums/pattern matching;
- traits/generics.
