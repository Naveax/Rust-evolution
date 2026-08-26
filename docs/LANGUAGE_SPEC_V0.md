# Rust Evolution Language Sketch v0

Status: **experimental, deliberately small, not stable**.

This document describes the behavior implemented on `main`. Future ideas belong in `docs/LANGUAGE_DESIGN.md` and the tracking issues until code, tests, diagnostics and performance evidence exist for them.

The current implementation establishes a complete source-to-native pipeline while keeping the user-facing surface intentionally small:

`Evolution source -> lexer -> parser -> semantic lowering -> Rust codegen -> rustc -> native binary`

There is no VM and no mandatory standalone runtime.

## Design direction

The long-term surface is inspired by Lua minimalism, Python readability/rapid expression, and Rust safety/performance/systems semantics. It does not mechanically combine those grammars.

A current program can look like:

```text
n = input_int
value = 1
repeat n
    value = value * 3 + 1
    value = value / 2
end
print value
```

The frontend lowers this to ordinary Rust constructs and native code.

## Lexical rules

- Source is UTF-8.
- Identifiers are currently ASCII letters/underscore followed by ASCII letters/digits/underscore.
- Non-ASCII source text is valid UTF-8, but non-ASCII identifier characters are not currently identifier characters.
- `#` starts a comment through the end of the line.
- Newline is a structural statement terminator.
- Integer literals target `i64`.
- String literals use double quotes.
- Supported string escapes are `\n`, `\r`, `\t`, `\"`, and `\\`.
- Operators are `+`, `-`, `*`, `/`, `=`, `(`, and `)`.
- Current keywords are `print`, `repeat`, `end`, and `input_int`.

### Lexical diagnostics

Two lexer APIs exist:

- `lex()` is the fail-fast compatibility API.
- `lex_recovering()` is the user-facing recovery path used by the CLI and benchmark frontend.

On valid source both APIs produce exactly the same token stream. On malformed source the recovery API reports source-ordered lexical errors with a deterministic maximum of 8 errors.

Current recovery boundaries are deliberately conservative:

- an unknown Unicode scalar is consumed as one scalar and lexing continues;
- an overflowing integer literal is consumed as one full numeric literal before continuing;
- an unterminated string at newline leaves the newline available as a structural boundary;
- an unsupported escape synchronizes to a closing quote or newline;
- EOF inside a string/escape terminates recovery cleanly.

Malformed lexical input is not passed to the parser.

## Grammar v0

The implemented grammar can be summarized as:

```text
program          := NEWLINE* (statement (NEWLINE+ | EOF) NEWLINE*)* EOF

statement        := binding
                  | print_statement
                  | repeat_statement

binding          := IDENTIFIER "=" expression
print_statement  := "print" expression
repeat_statement := "repeat" expression NEWLINE+ repeat_body "end"
repeat_body      := (NEWLINE* statement (NEWLINE+ | EOF))* NEWLINE*

expression       := additive
additive         := multiplicative (("+" | "-") multiplicative)*
multiplicative   := unary (("*" | "/") unary)*
unary            := "-" unary | primary
primary          := INTEGER
                  | STRING
                  | IDENTIFIER
                  | "input_int"
                  | "(" expression ")"
```

`repeat` blocks may be nested. A top-level unmatched `end` is an error. Reaching EOF before a required repeat `end` is also an error.

The prose above is authoritative for intent; parser tests are authoritative for exact currently accepted edge cases while the language remains experimental.

## Parser diagnostics and recovery

Two parser APIs exist:

- `parse()` preserves the original fail-fast behavior.
- `parse_recovering()` is used by the CLI and benchmark frontend.

The recovery parser:

- reports multiple independent syntax errors in source order;
- caps diagnostics at 8;
- synchronizes primarily at newline, `end`, and EOF;
- preserves repeat/nested-repeat block boundaries to avoid fake unmatched-`end` cascades;
- never sends an error-bearing partial AST into semantic lowering.

Both parser and lexer recovery paths have deterministic mutation-corpus coverage in normal CI.

## Semantic lowering

Parsing does not decide whether `name = expression` is a declaration or a reassignment. That decision belongs to semantic lowering.

### First definition

The first top-level assignment to a local defines it:

```text
x = 1
```

Current lowering:

```rust
let __evo_x = 1;
```

### Reassignment and inferred mutability

A later assignment to the same local is a reassignment if the value type matches:

```text
x = 1
x = x + 1
```

Current lowering marks the original local mutable automatically:

```rust
let mut __evo_x = 1;
__evo_x = (__evo_x + 1);
```

The user does not write `mut` in this v0 surface. Mutability is inferred only for locals that are actually reassigned.

Current restrictions:

- reading a local before its first definition is rejected by lowering;
- reassigning an existing local with a different current value type is rejected;
- a new local cannot currently be introduced inside a `repeat` block;
- repeat bodies operate on already-defined outer locals;
- lexical shadowing/block-local scope semantics are not implemented yet.

The no-new-local-inside-repeat rule avoids exposing a local after a zero-iteration loop without definite-initialization semantics. It is a current limitation, not a long-term language goal.

## Current value types

The semantic layer currently recognizes two value categories:

- integer (`i64`);
- string.

Rules implemented today:

- unary `-` requires an integer operand;
- `+`, `-`, `*`, and `/` require integer operands;
- repeat counts must be integer expressions;
- no implicit numeric widening or coercion exists;
- no dynamic type system is introduced;
- type-changing reassignment is rejected.

String concatenation or other string operators are not currently implemented.

## `input_int`

`input_int` is a built-in primary expression that reads one line from standard input and parses a signed `i64`.

Conceptually, generated Rust uses:

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

The helper is emitted only when a program uses `input_int`.

Current error semantics are explicit but intentionally simple: read failure or invalid integer input causes the generated program to fail through the shown `expect` contract. The subprocess corpus verifies valid zero/one/positive/negative input and invalid-input failure behavior.

## `repeat`

`repeat count ... end` evaluates an integer repeat count and lowers directly to a Rust range loop:

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

Current semantics:

- the count must be an integer;
- zero executes the body zero times;
- one executes it once;
- a negative count executes the body zero times because the generated `0..negative` range is empty;
- nested repeats are supported;
- repeat lowering itself adds no heap allocation, boxing, or dynamic dispatch.

## Print semantics

```text
print expression
```

currently lowers to:

```rust
println!("{}", expression);
```

This uses Rust display formatting and writes one trailing newline. It is bootstrap I/O behavior, not a final general-purpose I/O design.

## Identifier lowering

Evolution locals lower to generated Rust identifiers prefixed with `__evo_`.

Example:

```text
type = 7
print type
```

can lower safely even though `type` has special meaning in Rust, because the generated local is `__evo_type`.

## Source-native diagnostics

Lexer, parser, and semantic-lowering diagnostics are rendered against the original `.evo` source with:

- error message;
- source path;
- one-based line/column;
- source line;
- caret/range underline.

The renderer is deterministic and ANSI-free so CI output and snapshots remain stable. It handles UTF-8 byte offsets without treating bytes as display columns.

## Generated Rust source mapping

Rust code generation can return sidecar metadata mapping generated Rust lines back to Evolution `Span` values.

Current mapping policy:

- generated `let`, reassignment, and `print` lines map to their Evolution statement span;
- repeat opening and closing lines map to the repeat statement span;
- nested repeat body lines retain their own statement spans;
- generated `input_int` helper lines are intentionally unmapped;
- generated `fn main` wrapper lines are intentionally unmapped;
- unknown generated lines return no mapping.

The source map is metadata only. It does not inject comments or directives into generated Rust, so mapped and plain code generation produce the same Rust source bytes.

## rustc diagnostic remapping

`evo build` and `evo run` compile generated Rust with rustc. When rustc reports an error on a generated line that has source-map metadata, the CLI presents the primary error on the corresponding Evolution source statement rather than making the temporary generated `main.rs` the main user-facing location.

Unmapped helper/wrapper/internal compiler errors retain raw rustc stderr as a fallback instead of discarding diagnostic detail.

The current source map is line/statement-granular. Generated-column to Evolution-subexpression mapping is not implemented yet.

## Native compilation and performance contract

Accepted programs compile through rustc to native binaries.

The project performance rule remains:

```text
T_evolution <= T_reference_rust
```

Correctness must match before performance evidence is valid. See:

- `docs/PERFORMANCE_CONTRACT.md`;
- `docs/BENCHMARKING.md`;
- GitHub issue #4 for the living invariant;
- GitHub issue #5 for benchmark infrastructure.

The `runtime-repeat-v0` case is an enforced Ubuntu CI gate. The harness compares correctness, timing, normalized LLVM IR, binary size, and exact executable bytes. Byte-identical executables after correctness PASS are treated as deterministic runtime parity evidence; non-identical binaries remain subject to the strict timing gate.

## Current explicit non-features

The following are not implemented yet:

- functions and closures;
- booleans;
- comparison/equality operators;
- `if` / `else` conditionals;
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
- a stable language specification.

These omissions are deliberate. A smaller language with tested semantics is preferable to a larger syntax surface whose costs and safety rules are imaginary.

## Acceptance rule for future syntax

Every new construct must define:

1. exact semantics;
2. equivalent idiomatic Rust;
3. generated Rust snapshot expectations;
4. correctness tests;
5. safety implications;
6. hidden allocation/clone/boxing/dispatch impact;
7. source diagnostic behavior;
8. runtime comparison under the project performance contract when applicable.

A repeatable runtime regression against equivalent Rust is not accepted as a successful default feature merely because the syntax is shorter.
