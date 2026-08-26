# Rust Evolution Language Sketch v0

Status: **experimental, deliberately small, not stable**.

This document describes frontend behavior intended on `main` after accepted PRs merge. Future ideas belong in `docs/LANGUAGE_DESIGN.md` and tracking issues until code, tests, diagnostics, and performance evidence exist for them.

The implementation pipeline is:

`Evolution source -> lexer -> parser -> semantic lowering -> Rust codegen -> rustc -> native binary`

There is no VM and no mandatory standalone runtime.

## Design direction

The surface borrows Lua-style low ceremony, Python-style readability, and Rust-style strict semantics/native compilation. It does not copy the three grammars mechanically.

A current program can look like:

```text
fn step(x int) int
    if x > 1 and not (x == 7)
        local = x / 2
        return local
    else
        local = x + 3
        return local
    end
end

n = input_int
x = input_int
sum = 0
repeat n
    temp = step(x)
    x = temp
    sum = sum + temp
end
print sum
```

The frontend lowers this to ordinary static Rust constructs and native code.

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
- `,` separates function parameters and call arguments.
- Current keywords are `print`, `repeat`, `if`, `else`, `end`, `true`, `false`, `input_int`, `and`, `or`, `not`, `fn`, `return`, `int`, `bool`, and `string`.
- Keyword matching respects identifier boundaries. Names such as `android`, `origin`, `notice`, `functionality`, and `integer_value` remain identifiers when they do not exactly match a keyword.
- A lone `!` is not a logical-not operator. `not` is the user-facing logical negation keyword.

### Lexical diagnostics

Two lexer APIs exist:

- `lex()` is the fail-fast compatibility API.
- `lex_recovering()` is the user-facing recovery path used by the CLI and benchmark frontend.

On valid source both APIs produce the same token stream. On malformed source recovery reports source-ordered errors with a deterministic maximum of 8 diagnostics.

Malformed lexical input is not passed to the parser.

## Grammar v0

```text
program             := NEWLINE* function_definition* top_level_statement_list EOF

function_definition := "fn" IDENTIFIER "(" parameters? ")" type_name NEWLINE+
                       function_block "end"
parameters          := parameter ("," parameter)*
parameter           := IDENTIFIER type_name
type_name           := "int" | "bool" | "string"

function_block      := (NEWLINE* function_statement (NEWLINE+ | EOF))* NEWLINE*
top_level_statement_list
                    := (NEWLINE* statement (NEWLINE+ | EOF))* NEWLINE*

function_statement  := statement | return_statement
statement           := binding
                     | print_statement
                     | repeat_statement
                     | if_statement

binding             := IDENTIFIER "=" expression
print_statement     := "print" expression
return_statement    := "return" expression
repeat_statement    := "repeat" expression NEWLINE+ block "end"
if_statement        := "if" expression NEWLINE+ block
                       ("else" NEWLINE+ block)?
                       "end"
block               := (NEWLINE* function_statement (NEWLINE+ | EOF))* NEWLINE*

expression          := logical_or
logical_or          := logical_and ("or" logical_and)*
logical_and         := logical_not ("and" logical_not)*
logical_not         := "not" logical_not | comparison
comparison          := additive (comparison_operator additive)?
comparison_operator := "==" | "!=" | "<" | "<=" | ">" | ">="
additive            := multiplicative (("+" | "-") multiplicative)*
multiplicative      := unary (("*" | "/") unary)*
unary               := "-" unary | primary
primary             := INTEGER
                     | STRING
                     | "true"
                     | "false"
                     | IDENTIFIER
                     | call_expression
                     | "input_int"
                     | "(" expression ")"
call_expression     := IDENTIFIER "(" arguments? ")"
arguments           := expression ("," expression)*
```

### Declaration order

Function declarations are top-level declarations, not executable statements. In v0 all function declarations must appear before the first executable top-level statement.

This is valid:

```text
fn add(a int, b int) int
    return a + b
end

print add(2, 3)
```

A later `fn` after an executable top-level statement is rejected. Nested function declarations are also rejected.

`return` is valid only while parsing a function body. Top-level `return` is an error.

`repeat` and `if` blocks may nest in either direction inside top-level code or function bodies. `if` may omit `else`. Top-level unmatched `end`/`else` and missing required `end` are errors.

### Expression precedence

From lowest to highest:

1. `or`
2. `and`
3. `not`
4. comparisons
5. `+` / `-`
6. `*` / `/`
7. unary numeric `-`
8. primary, calls, and grouping

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

Calls are primary expressions, so they can be nested and composed with existing arithmetic, comparison, and logical expressions:

```text
print add(step(4), 2)
if positive(step(input_int)) and not false
    print 1
end
```

## Parser diagnostics and recovery

- `parse()` preserves fail-fast behavior.
- `parse_recovering()` is used by user-facing paths.
- Recovery reports independent syntax errors in source order, capped at 8.
- Main synchronization boundaries are newline, `else`, `end`, and EOF.
- Nested `repeat`/`if` boundaries are preserved to avoid fake cascade errors.
- Function parameter lists, function-body `end`, and function declaration placement produce source-native parser diagnostics.
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

The user does not write `mut` in v0. Mutability is inferred only for locals or function parameters that are actually reassigned.

Current semantic rules:

- use before first definition is rejected;
- type-changing reassignment is rejected;
- a first assignment creates a binding in the current lexical scope only when no binding with that name is visible in the current or parent scopes;
- assignment to a visible binding remains reassignment, not shadowing;
- `if` then/else bodies and `repeat` bodies create lexical child scopes;
- child scopes may read visible parent bindings while the parent scope is active;
- child-local bindings disappear when their block closes;
- sibling branches are independent scopes and may each define a local with the same source name;
- same-name sibling locals do not merge into an outer binding after `end`;
- a zero-iteration `repeat` never exposes a loop-local outside the loop;
- arbitrary same-name shadowing of an already-visible binding is not a v0 feature.

### Lexical block locals v0

No new user syntax is required. The existing assignment form remains syntax-neutral:

```text
x = 10
if x > 0
    doubled = x * 2
    print doubled
end
```

`doubled` is created in the `if` child scope. It is valid in nested child blocks while that scope is active and is rejected after the matching `end`.

Sibling branches have independent local scopes:

```text
if flag
    temp = 1
    print temp
else
    temp = 2
    print temp
end
```

The two `temp` declarations are independent. Neither exists after `end`, and no implicit branch result, phi value, promotion, or merge is created.

A visible outer name is reassigned rather than shadowed:

```text
x = 1
if true
    x = 2
end
```

Conceptually this lowers to an outer mutable Rust binding plus a normal assignment in the child block.

`repeat` locals use ordinary Rust loop-body scope semantics:

```text
repeat n
    temp = x + 1
    temp = temp + 1
    x = temp
end
```

`temp` is recreated per loop iteration, can become `mut` if reassigned in the body, and is unavailable after the loop. `x` remains an outer reassignment.

Block-local lowering is compile-time semantic scope tracking only. Generated Rust contains ordinary lexical `let`/assignment statements; no runtime scope object, `HashMap` lookup, boxing, reference counting, or dynamic dispatch is introduced for scoping.

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
- there is no dynamic type layer hidden behind logical or function syntax.

Examples rejected by lowering:

```text
print 1 and true
print true or 1
print not 1
print "text" and false
```

## Functions v0

Functions v0 adds reusable named code while keeping calls fully static.

### Declaration syntax

```text
fn add(a int, b int) int
    return a + b
end
```

The v0 signature is deliberately explicit but compact. It avoids Rust-style `:` and `->` punctuation while keeping parameter and return types declaration-local and deterministic.

Supported signature types are:

- `int` -> Rust `i64`;
- `bool` -> Rust `bool`;
- `string` -> Rust `&'static str` in functions v0.

The `string` ABI is intentionally narrow. The current language creates strings only from source literals, so `&'static str` preserves existing zero-allocation behavior. Runtime-produced or owned strings are not silently introduced through cloning or allocation; their ownership model is a later language slice.

### Calls

Calls are expressions:

```text
print add(2, 3)
result = add(step(10), 4)
```

Functions have fixed arity. Call validation rejects:

- unknown function names;
- wrong argument count;
- argument type mismatches.

### Signature pre-pass, forward calls, and recursion

Lowering collects all top-level function signatures before lowering any function body or executable top-level statement. This allows calls between functions independent of declaration order within the declaration section and allows direct recursion under an explicit signature.

The signature pre-pass does not create dynamic dispatch. It is compile-time semantic metadata only.

### Function-local scope

Each function body gets an independent root binding scope.

- parameters enter the function root scope before body lowering;
- function-local first assignments create local bindings;
- nested `if`/`else`/`repeat` bodies use the same lexical child-scope model described above;
- reassignment uses the existing same-type/inferred-mutability policy;
- mutable parameters are marked `mut` only when reassigned;
- top-level locals are not captured by functions;
- duplicate parameter names are rejected;
- duplicate function names are rejected.

### Return rules

Functions v0 always declare a non-unit return type. `return expression` must match that declared type.

Every reachable terminal path must return. A terminal `if/else` satisfies this only when both branches return. Loops are not treated as guaranteed-return constructs in v0.

Examples:

```text
fn choose(flag bool) int
    if flag
        result = 1
        return result
    else
        result = 2
        return result
    end
end
```

is valid, while a function that can fall through without `return` is rejected.

### Rust lowering

Named functions become ordinary static Rust functions with deterministic generated names:

```text
fn add(a int, b int) int
    return a + b
end

print add(2, 3)
```

lowers to the equivalent shape:

```rust
fn __evo_fn_add(__evo_a: i64, __evo_b: i64) -> i64 {
    return (__evo_a + __evo_b);
}

fn main() {
    println!("{}", __evo_fn_add(2, 3));
}
```

There is no function registry, interpreter, VM, vtable, boxing, or dynamic dispatch solely to support named functions.

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

The helper is emitted only when needed, including when only a function body needs it. Invalid input fails through the explicit parse contract; subprocess tests cover valid and invalid cases.

## `repeat`

`repeat count ... end` lowers directly to a Rust range loop:

```text
repeat n
    temp = x + 1
    x = temp
end
```

becomes the equivalent shape:

```rust
for _ in 0..__evo_n {
    let __evo_temp = (__evo_x + 1);
    __evo_x = __evo_temp;
}
```

Zero and negative counts execute zero iterations under current Rust range semantics. A binding first created in the repeat body exists only for that iteration and is unavailable after `end`. Nested repeats and repeat/if composition are supported. Repeat lowering adds no helper runtime or allocation.

## `if` / `else`

`if condition ... else ... end` is strict boolean control flow. No truthiness is accepted.

Example:

```text
value = input_int
if value > 0 and not (value == 7)
    shown = value
    print shown
else
    shown = -value
    print shown
end
```

Each branch is an independent lexical child scope. A first assignment to a name that is not already visible creates a branch-local binding. Same-name branch locals do not merge or become visible after `end`. Assignments to existing visible outer locals remain reassignment and participate in inferred mutability.

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

Evolution locals are prefixed with `__evo_` in generated Rust. Named functions are prefixed with `__evo_fn_`. These deterministic prefixes avoid direct collisions with Rust keywords and support stable source/codegen inspection.

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
- `repeat` and `if`/`else` indentation, including block-local bindings;
- nested block-local indentation;
- function signature spacing;
- function parameter and call-argument comma spacing;
- function body indentation;
- `return` spacing;
- unary-minus spacing;
- comments and raw string spelling;
- final newline behavior.

Function/block-local example:

```text
fn add(a int, b int) int
    if a > 0
        result = a + b
        return result
    else
        result = b
        return result
    end
end

print add(1, 2)
```

Formatting is idempotent. `--check` does not rewrite and fails when source is not canonical.

## Source-native diagnostics

Lexer, parser, and semantic-lowering diagnostics render against the original `.evo` source with message, path, line/column, source line, and caret/range underline.

The renderer is deterministic and ANSI-free. Recovered lexer/parser errors are displayed in source order. Parser errors prevent lowering/rustc.

Function-specific semantic errors such as duplicate names, unknown calls, arity/type mismatches, illegal capture, and missing return paths are reported against Evolution source spans.

Block-scope semantic errors such as reading a local after its defining block closes are also reported against the original Evolution source. Type-changing reassignment to a visible outer or block-local binding remains a semantic error under the same source-native diagnostic path.

## Generated Rust source mapping

Codegen returns optional sidecar generated-line to Evolution `Span` metadata.

- `let`, reassignment, `print`, and `return` lines map to their statement spans;
- block-local declarations and reassignments keep their own source statement spans;
- repeat/if structural generated lines map to the owning statement span;
- sibling block-local declarations map independently even when they use the same source identifier;
- function signature and closing lines map to the owning function span;
- nested function-body statements retain their own spans;
- function call expressions stay on the owning generated statement line;
- helper/wrapper lines remain intentionally unmapped;
- logical expressions remain on the owning statement line and therefore do not create synthetic source-map lines.

Source-map metadata does not alter generated Rust bytes.

## rustc diagnostic remapping

`evo build` and `evo run` map rustc errors from generated lines back to Evolution statement/function spans when mapping exists. Unmapped helper/wrapper/internal failures preserve raw rustc stderr rather than dropping detail.

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
- `logical-operators-v0` for `and`/`or`/`not` inside runtime-dependent control flow;
- `function-call-v0` for typed named functions and direct static calls inside a runtime-dependent loop;
- `block-locals-v0` for sibling branch-local declarations, block-local reads, and outer reassignment inside a runtime-dependent loop.

The harness compares correctness, raw timing, normalized LLVM IR, binary size, and exact executable bytes. Exact byte-identical binaries after correctness PASS are deterministic runtime parity evidence. Non-identical binaries remain subject to the strict stable timing gate `T_evolution / T_reference <= 1.00`; unstable timing is INCONCLUSIVE.

For `function-call-v0`, the Rust reference is locked by regression test to the frontend's ordinary generated static Rust source shape, modulo platform newline normalization. Both sides are then independently compiled with the same rustc/target/optimization settings. The accepted evidence showed:

- differential correctness: PASS;
- normalized LLVM IR equality: true;
- exact executable equality: true;
- binary size: 2,267,040 bytes on both sides;
- observed median ratio: 1.000021041;
- final verdict: PASS;
- verdict basis: `byte-identical-binary-parity`.

For `block-locals-v0`, the Rust reference is likewise locked to ordinary generated Rust. The accepted Ubuntu evidence showed:

- differential correctness: PASS;
- stable measurement: true;
- normalized LLVM IR equality: true;
- exact executable equality: true;
- binary size: 2,267,072 bytes on both sides;
- observed median ratio: 1.001008999;
- timing-only verdict: FAIL;
- final verdict: PASS;
- verdict basis: `byte-identical-binary-parity`.

The timing-only value remains visible even when identical binaries make runtime parity deterministic; scheduler noise is not promoted into a fictitious codegen regression.

## Current explicit non-features

Not implemented yet:

- closures/lambdas and first-class function values;
- inferred function parameter or return types;
- unit-returning functions;
- nested function declarations;
- function overloading/default/named/variadic arguments;
- truthiness or implicit boolean coercion;
- chained-comparison semantics;
- general explicit local type annotations;
- runtime-produced/owned string semantics beyond the current literal/static string model;
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
- branch-result values or automatic promotion/merge of block locals into an outer scope;
- arbitrary same-name shadowing of an already-visible local;
- definite-initialization/phi semantics for conditionally created outer values;
- a stable language specification.

These omissions are deliberate. A small language with measured semantics beats a large syntax brochure whose costs exist mostly in imagination.

## Acceptance rule for future syntax

Every new construct must define:

1. exact semantics;
2. equivalent Rust behavior;
3. generated Rust expectations;
4. correctness tests;
5. safety implications;
6. hidden allocation/clone/boxing/dispatch impact;
7. source diagnostic behavior;
8. formatter behavior when syntax is user-facing;
9. runtime comparison under the project performance contract when applicable.

A repeatable runtime regression against equivalent Rust is not accepted merely because the syntax is shorter.
