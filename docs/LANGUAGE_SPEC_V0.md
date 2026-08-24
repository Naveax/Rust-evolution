# Rust Evolution Language Sketch v0

Status: **experimental, deliberately tiny, not stable**.

The purpose of v0 is to establish a complete source-to-native vertical slice before expanding the language surface. Syntax will only grow when semantics, safety, code generation and performance implications can be inspected.

## Design direction

The long-term language surface is inspired by Lua minimalism, Python readability/rapid expression, and Rust safety/performance/systems semantics. It is not intended to copy any of those grammars mechanically.

Current v0 intentionally removes ceremony:

```text
x = 1
y = 1
print x + y
```

Equivalent generated Rust:

```rust
fn main() {
    let __evo_x = 1;
    let __evo_y = 1;
    println!("{}", (__evo_x + __evo_y));
}
```

There is no VM or mandatory runtime layer in this path.

## Lexical rules

- Source is UTF-8.
- v0 identifiers are ASCII letters/underscore followed by ASCII letters/digits/underscore.
- `#` starts a comment through the end of the line.
- Newline terminates a statement.
- Integer literals currently target `i64`.
- String literals use double quotes.
- Supported string escapes: `\n`, `\r`, `\t`, `\"`, `\\`.
- Operators: `+`, `-`, `*`, `/`, `=`, `(`, `)`.
- `print` is currently a keyword.

## Grammar v0

```text
program       := NEWLINE* statement (NEWLINE+ statement)* NEWLINE* EOF
statement     := IDENTIFIER "=" expression
               | "print" expression
expression    := additive
additive      := multiplicative (("+" | "-") multiplicative)*
multiplicative := unary (("*" | "/") unary)*
unary         := "-" unary | primary
primary       := INTEGER | STRING | IDENTIFIER | "(" expression ")"
```

## Binding semantics

`name = expression` currently lowers to an immutable Rust `let` binding. Reusing a name therefore follows Rust shadowing semantics rather than mutation semantics.

Mutation syntax is intentionally not invented yet. It will be designed only after ownership/move/borrow behavior is specified clearly.

## Expression semantics

- Arithmetic precedence matches the generated Rust expression.
- Parentheses are preserved semantically.
- Unary minus is supported.
- No implicit numeric widening/coercion policy has been added.
- No dynamic type system is introduced.

## Print semantics

`print expression` currently lowers to Rust's display formatting via `println!("{}", ...)`.

This is a bootstrap feature, not the final I/O design.

## Identifier lowering

Evolution bindings currently lower to generated identifiers prefixed with `__evo_`. This prevents ordinary Evolution identifiers from colliding with Rust keywords in generated source.

## Explicit non-features in v0

Not yet supported:

- functions
- explicit types
- mutable bindings
- structs/records
- enums/sum types
- pattern matching
- generics/traits
- ownership/borrow syntax
- references/lifetimes
- collections
- Result/Option sugar
- async/concurrency
- FFI
- modules/packages

These omissions are deliberate. Adding syntax before semantics are testable would merely make the language larger, which is the opposite of the project goal.

## Acceptance rule for future syntax

Every new construct must define:

1. exact semantics;
2. equivalent idiomatic Rust;
3. generated Rust snapshot expectations;
4. correctness tests;
5. safety implications;
6. hidden allocation/clone/boxing/dispatch impact;
7. runtime comparison under the project performance contract.

A repeatable `T_evolution > T_reference_rust` is not accepted as a successful default feature.
