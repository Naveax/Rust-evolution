# Rust Evolution Language Sketch v0

Status: **experimental, deliberately small, not stable**.

This document describes the frontend behavior intended on `main` after accepted PRs merge. Future ideas belong in `docs/LANGUAGE_DESIGN.md` and tracking issues until code, tests, diagnostics, and performance evidence exist for them.

The implementation pipeline is:

`Evolution source -> lexer -> parser -> semantic lowering -> Rust codegen -> rustc -> native binary`

There is no VM and no mandatory standalone runtime.

## Design direction

The surface borrows Lua-style low ceremony, Python-style readability, and Rust-style strict semantics/native compilation. It does not copy the three grammars mechanically.

A current program can look like:

```text
n = input_int
x = input_int
sum = 0
repeat n
    if x > 1 and not (x == 7) or x < -10
        x = x / 2
    else
        x = x + 3
    end
    sum = sum + x
end
print sum
```

The frontend lowers this to ordinary Rust constructs and native code.

## Lexical rules

- Source is UTF-8.
- Identifiers are ASCII letters/underscore followed by ASCII letters/digits/underscore.
- `#` starts a comment through end of line.
- Newline is a structural statement terminator.
- Integer literals target `i64`.
- String literals use double quotes.
- Supported string escapes are `\n`, `\r`, `\t`, `\"`, and `\\`.
- Arithmetic/assignment/grouping operators are `+`, `-`, `*`, `/`, `=`, `(`, and `)`.
- Comparison operators are `==`, `!=`, `<`, `<=`, `>`, and `>=`.
- Logical operators are keyword operators `and`, `or`, and `not`.
- Current keywords are `print`, `repeat`, `if`, `else`, `end`, `true`, `false`, `input_int`, `and`, `or`, and `not`.
- Keyword matching respects identifier boundaries. Names such as `android`, `origin`, and `notice` remain identifiers.
- A lone `!` is not a logical-not operator. `not` is the user-facing logical negation keyword.

### Lexical diagnostics

Two lexer APIs exist:

- `lex()` is the fail-fast compatibility API.
- `lex_recovering()` is the user-facing recovery path used by the CLI and benchmark frontend.

On valid source both APIs produce the same token stream. On malformed source recovery reports source-ordered errors with a deterministic maximum of 8 diagnostics.

Malformed lexical input is not passed to the parser.

## Grammar v0

```text
program          := NEWLINE* (statement (NEWLINE+ | EOF) NEWLINE*)* EOF

statement        := binding
                  | print_statement
                  | repeat_statement
                  | if_statement

binding          := IDENTIFIER "=" expression
print_statement  := "print" expression
repeat_statement := "repeat" expression NEWLINE+ block "end"
if_statement     := "if" expression NEWLINE+ block
                    ("else" NEWLINE+ block)?
                    "end"
block            := (NEWLINE* statement (NEWLINE+ | EOF))* NEWLINE*

expression       := logical_or
logical_or       := logical_and ("or" logical_and)*
logical_and      := logical_not ("and" logical_not)*
logical_not      := "not" logical_not | comparison
comparison       := additive (comparison_operator additive)?
comparison_operator := "==" | "!=" | "<" | "<=" | ">" | ">="
additive         := multiplicative (("+" | "-") multiplicative)*
multiplicative   := unary (("*" | "/") unary)*
unary            := "-" unary | primary
primary          := INTEGER
                  | STRING
                  | "true"
                  | "false"
                  | IDENTIFIER
                  | "input_int"
                  | "(" expression ")"
```

`repeat` and `if` blocks may nest in either direction. `if` may omit `else`. Top-level unmatched `end`/`else` and missing required `end` are errors.

### Expression precedence

From lowest to highest:

1. `or`
2. `and`
3. `not`
4. comparisons
5. `+` / `-`
6. `*` / `/`
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

`not` is recursive, so `not not true` is valid.

Comparison precedence remains below arithmetic. Chained comparisons such as `1 < 2 < 3` are explicitly rejected rather than given Python-style semantics.

## Parser diagnostics and recovery

- `parse()` preserves fail-fast behavior.
- `parse_recovering()` is used by user-facing paths.
- Recovery reports independent syntax errors in source order, capped at 8.
- Main synchronization boundaries are newline, `else`, `end`, and EOF.
- Nested `repeat`/`if` boundaries are preserved to avoid fake cascade errors.
- An error-bearing partial AST is never sent to lowering.

## Semantic lowering

Parsing keeps `name = expression` syntax-neutral. Lowering decides first definition versus reassignment.

### First definition and reassignment

```text
x = 1
x = x + 1
```

lowers conceptually to:

```rust
let mut __evo_x = 1;
__evo_x = (__evo_x + 1);
```

The user does not write `mut` in v0. Mutability is inferred only for locals that are actually reassigned.

Current restrictions:

- use before first definition is rejected;
- type-changing reassignment is rejected;
- new locals cannot currently be introduced inside `repeat` or `if` blocks;
- control-flow bodies operate on already-defined outer locals;
- lexical shadowing/block-local scope semantics are not implemented yet.

## Current value types

The semantic layer recognizes:

- integer (`i64`);
- string;
- boolean.

Rules:

- unary `-` requires integer;
- arithmetic `+ - * /` requires integer operands;
- repeat counts require integer;
- `if` conditions require boolean;
- `==` / `!=` require operands of the same current value type;
- ordering `< <= > >=` is integer-only;
- comparisons produce boolean;
- `and` / `or` require boolean operands and produce boolean;
- `not` requires one boolean operand and produces boolean;
- there is no truthiness;
- there is no implicit integer/string-to-boolean conversion;
- there is no dynamic type layer hidden behind logical syntax.

Examples rejected by lowering:

```text
print 1 and true
print true or 1
print not 1
print "text" and false
```

## Logical operators

### `and`

```text
left and right
```

uses strict boolean short-circuit semantics. If `left` is false, `right` is not evaluated.

Direct Rust target:

```rust
left && right
```

### `or`

```text
left or right
```

uses strict boolean short-circuit semantics. If `left` is true, `right` is not evaluated.

Direct Rust target:

```rust
left || right
```

### `not`

```text
not value
```

negates one boolean value and lowers directly to Rust `!`.

There is no runtime helper for `and`, `or`, or `not`. Accepted lowering must not add allocations, clones, boxing, dynamic dispatch, reference counting, or eager RHS evaluation.

The process-level short-circuit corpus uses `input_int` as an observable side effect: expressions such as `false and input_int > 0` and `true or input_int > 0` must not consume stdin, while `true and ...` and `false or ...` must evaluate the RHS.

## `input_int`

`input_int` reads one line from standard input and parses signed `i64`.

Generated Rust helper shape:

```rust
fn __evo_input_int() -> i64 {
    let mut __evo_input = String::new();
    std::io::stdin()
        .read_line(&mut __evo_input)
        .expect("failed to read integer input");
    __evo_input
        .trim()
        .parse::<i64>()
        .expect("expected signed integer input")
}
```

The helper is emitted only when needed. Invalid input fails through the explicit parse contract; subprocess tests cover valid and invalid cases.

## `repeat`

`repeat count ... end` lowers directly to a Rust range loop:

```text
repeat n
    x = x + 1
end
```

becomes the equivalent shape:

```rust
for _ in 0..__evo_n {
    __evo_x = (__evo_x + 1);
}
```

Zero and negative counts execute zero iterations under current Rust range semantics. Nested repeats and repeat/if composition are supported. Repeat lowering adds no helper runtime or allocation.

## `if` / `else`

`if condition ... else ... end` is strict boolean control flow. No truthiness is accepted.

Example:

```text
value = input_int
if value > 0 and not (value == 7)
    print value
else
    print -value
end
```

Assignments to existing outer locals are allowed and participate in inferred mutability. New branch-local locals are rejected in v0.

## Print semantics

```text
print expression
```

lowers to Rust display output with one newline:

```rust
println!("{}", expression);
```

Integers, strings, and booleans are currently printable.

## Identifier lowering

Evolution locals are prefixed with `__evo_` in generated Rust. This avoids direct collisions with Rust keywords.

## Formatter

The CLI provides:

```text
evo fmt file.evo
evo fmt file.evo --check
```

Canonical formatting currently normalizes:

- assignment/arithmetic/comparison spacing;
- `and` / `or` spacing;
- `not` keyword spacing;
- `repeat` and `if`/`else` indentation;
- unary-minus spacing;
- comments and raw string spelling;
- final newline behavior.

Logical example:

```text
if value > 0 and not (limit == 0) or false
```

Formatting is idempotent. `--check` does not rewrite and fails when source is not canonical.

## Source-native diagnostics

Lexer, parser, and semantic-lowering diagnostics render against the original `.evo` source with message, path, line/column, source line, and caret/range underline.

The renderer is deterministic and ANSI-free. Recovered lexer/parser errors are displayed in source order. Parser errors prevent lowering/rustc.

## Generated Rust source mapping

Codegen returns optional sidecar generated-line to Evolution `Span` metadata.

- `let`, reassignment, and `print` lines map to their statement spans;
- repeat/if structural generated lines map to the owning statement span;
- nested body statements retain their own spans;
- helper/wrapper lines remain intentionally unmapped;
- logical expressions remain on the owning statement line and therefore do not create synthetic source-map lines.

Source-map metadata does not alter generated Rust bytes.

## rustc diagnostic remapping

`evo build` and `evo run` map rustc errors from generated lines back to Evolution statement spans when mapping exists. Unmapped helper/wrapper/internal failures preserve raw rustc stderr rather than dropping detail.

Column-level generated-subexpression mapping is not implemented yet.

## Native compilation and performance contract

Accepted programs compile through rustc to native binaries.

The hard rule remains:

```text
T_evolution <= T_reference_rust
```

Correctness must match first. See `docs/PERFORMANCE_CONTRACT.md`, `docs/BENCHMARKING.md`, issue #4, and issue #5.

Runtime-dependent Ubuntu CI gates include:

- `runtime-repeat-v0` for input/repeat/reassignment;
- `control-flow-branch-v0` for comparisons/branches/mutability;
- `logical-operators-v0` for `and`/`or`/`not` inside runtime-dependent control flow.

The harness compares correctness, raw timing, normalized LLVM IR, binary size, and exact executable bytes. Exact byte-identical binaries after correctness PASS are deterministic runtime parity evidence. Non-identical binaries remain subject to the strict stable timing gate `T_evolution / T_reference <= 1.00`; unstable timing is INCONCLUSIVE.

## Current explicit non-features

Not implemented yet:

- functions and closures;
- truthiness or implicit boolean coercion;
- chained-comparison semantics;
- explicit type annotations;
- user-defined records/structs;
- enums/sum types;
- pattern matching;
- generics/traits;
- ownership/borrow syntax;
- references/lifetimes;
- collections and collection literals;
- general ranges/iteration syntax outside `repeat`;
- `Result` / `Option` sugar;
- async/concurrency syntax;
- FFI syntax;
- modules/packages;
- user-defined block-local bindings;
- definite-initialization/branch-merge semantics for new locals;
- a stable language specification.

These omissions are deliberate. A small language with measured semantics beats a large syntax brochure whose costs exist mostly in imagination.

## Acceptance rule for future syntax

Every new construct must define:

1. exact semantics;
2. equivalent idiomatic Rust;
3. generated Rust expectations;
4. correctness tests;
5. safety implications;
6. hidden allocation/clone/boxing/dispatch impact;
7. source diagnostic behavior;
8. formatter behavior when syntax is user-facing;
9. runtime comparison under the project performance contract when applicable.

A repeatable runtime regression against equivalent Rust is not accepted merely because the syntax is shorter.
