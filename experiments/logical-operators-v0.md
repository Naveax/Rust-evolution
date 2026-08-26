# Logical operators v0 experiment

Status: implementation contract for #34. This file is intentionally narrower than the stable-ish language sketch: it defines the behavior the branch must prove before merge.

## Surface

```text
value = input_int
limit = input_int

if value > 0 and limit > 0
    print value
end

if not (value == 0) or limit > 10
    print limit
end
```

Reserved keywords: `and`, `or`, `not`.

Identifiers such as `android`, `origin`, and `notice` remain ordinary identifiers.

## Type semantics

Logical operators are strict boolean operators.

- `bool and bool -> bool`
- `bool or bool -> bool`
- `not bool -> bool`
- integer operands are rejected
- string operands are rejected
- there is no truthiness
- there is no implicit conversion to boolean

This deliberately follows the project's existing strict `if` condition model rather than copying Lua/Python truthiness.

## Evaluation semantics

`and` and `or` must short-circuit.

- `false and rhs` does not evaluate `rhs`
- `true or rhs` does not evaluate `rhs`
- `true and rhs` evaluates `rhs`
- `false or rhs` evaluates `rhs`

`input_int` is used by the process-level corpus as an observable side effect so a fake eager implementation cannot pass unnoticed.

## Precedence

From lowest to highest:

1. `or`
2. `and`
3. `not`
4. comparison: `== != < <= > >=`
5. additive: `+ -`
6. multiplicative: `* /`
7. unary numeric `-`
8. primary/grouping

Therefore:

```text
not value > 0
```

means:

```text
not (value > 0)
```

and:

```text
a > 0 and b > 0 or c > 0
```

means:

```text
((a > 0) and (b > 0)) or (c > 0)
```

## AST / lowering direction

Parser AST should represent logical operations explicitly rather than encoding them as magic identifiers.

Suggested shape:

```text
ExprKind::LogicalNot(expr)
BinaryOp::And
BinaryOp::Or
```

Exact names may differ if a cleaner representation appears during implementation, but the semantic boundary stays parser -> lowering -> Rust codegen.

Lowering owns boolean operand validation.

## Rust codegen

Zero-cost lowering target:

- `and` -> `&&`
- `or` -> `||`
- `not` -> `!`

No runtime helper, allocation, clone, boxing, dynamic dispatch, or reference counting may be added for these operators.

Because expression codegen stays inside the owning statement line, current statement-level generated-Rust source mappings remain valid.

## Required correctness corpus

### Precedence

- `true or false and false` => `true`
- `not 1 > 0` => `false`
- `not not true` => `true`
- grouped forms preserve explicit parentheses

### Type rejection

- `1 and true` => lowering error
- `true or 1` => lowering error
- `not 1` => lowering error
- string logical operands => lowering error

### Short circuit

Use a generated program that consumes stdin with `input_int`:

```text
if false and input_int > 0
    print 1
end
print 2
```

With empty stdin this must still succeed and print `2`, proving RHS was not evaluated.

Likewise `true or input_int > 0` must not consume input. Positive controls verify `true and ...` and `false or ...` do evaluate RHS.

## Formatter

Canonical source keeps keyword operators human-readable:

```text
if value > 0 and not (limit == 0)
```

Spacing around `and`/`or` is one space on each side. `not` is followed by one space unless formatter grouping rules make parentheses immediately follow after that space.

Formatting must remain idempotent.

## Performance gate

The accepted implementation must retain all existing runtime gates and add logical-operator evidence.

Preferred evidence order:

1. differential correctness PASS;
2. generated Rust inspection confirms direct `&&`, `||`, `!`;
3. normalized LLVM comparison;
4. exact binary comparison where applicable;
5. if binaries differ, stable timing must satisfy `T_evolution / T_reference <= 1.00`.

Scheduler noise is not permission to weaken #4. Exact byte-identical executables remain deterministic parity evidence under the existing harness policy.

## Merge gate

This experiment is merge-ready only when lexer, parser, lowering, codegen, formatter, subprocess short-circuit corpus, language spec, three-OS CI, and the Ubuntu performance gate are all green.