# Rust Evolution Language Sketch v0

Status: **experimental, deliberately small, not stable**.

This document describes frontend behavior that has implementation, tests, diagnostics, codegen evidence, and where applicable performance evidence. Future ideas belong in `docs/LANGUAGE_DESIGN.md` and tracking issues until those requirements are met.

The implementation pipeline is:

`Evolution source -> lexer -> parser -> semantic lowering -> Rust codegen -> rustc -> native binary`

There is no VM and no mandatory standalone runtime.

## Design direction

The surface borrows Lua-style low ceremony, Python-style readability, and Rust-style strict semantics/native compilation. It does not mechanically copy any of those grammars.

A current program can use functions, lexical block locals, strict booleans, and nominal records:

```text
record Point
    x int
    y int
end

fn sum(point Point) int
    return point.x + point.y
end

point = Point(y = 2, x = 40)
print sum(point)
```

The frontend lowers accepted source to ordinary static Rust constructs and native code.

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
- `.` is postfix field access.
- `,` separates function parameters, call arguments, and named constructor fields.
- Current keywords are `print`, `repeat`, `if`, `else`, `end`, `true`, `false`, `input_int`, `and`, `or`, `not`, `fn`, `return`, `record`, `int`, `bool`, and `string`.
- Keyword matching respects identifier boundaries.
- A lone `!` is not a logical-not operator. `not` is the user-facing logical negation keyword.

### Lexical diagnostics

Two lexer APIs exist:

- `lex()` is the fail-fast compatibility API.
- `lex_recovering()` is the user-facing recovery path used by the CLI and benchmark frontend.

On valid source both APIs produce the same token stream. On malformed source recovery reports source-ordered errors with a deterministic maximum of 8 diagnostics. Malformed lexical input is not passed to the parser.

## Grammar v0

The following sketch describes accepted surface shape. Declaration-placement details are specified immediately below because the parser intentionally preserves compatibility that is awkward to express as one compact production.

```text
program             := NEWLINE* top_level_item* EOF

top_level_item      := record_definition
                     | function_definition
                     | statement

record_definition   := "record" IDENTIFIER NEWLINE+
                       record_field_list "end"
record_field_list   := (NEWLINE* record_field NEWLINE+)* NEWLINE*
record_field        := IDENTIFIER record_field_type
record_field_type   := "int" | "bool" | "string" | IDENTIFIER

function_definition := "fn" IDENTIFIER "(" parameters? ")" type_name NEWLINE+
                       function_block "end"
parameters          := parameter ("," parameter)*
parameter           := IDENTIFIER type_name
type_name           := "int" | "bool" | "string" | IDENTIFIER

function_block      := (NEWLINE* function_statement (NEWLINE+ | EOF))* NEWLINE*
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
unary               := "-" unary | postfix
postfix             := primary ("." IDENTIFIER)*
primary             := INTEGER
                     | STRING
                     | "true"
                     | "false"
                     | IDENTIFIER
                     | call_or_constructor
                     | "input_int"
                     | "(" expression ")"
call_or_constructor := IDENTIFIER "(" call_or_named_fields? ")"
call_or_named_fields
                    := arguments | named_fields
arguments           := expression ("," expression)*
named_fields        := named_field ("," named_field)*
named_field         := IDENTIFIER "=" expression
```

The parser keeps a zero-argument `Name()` as a call-shaped AST node. Semantic lowering resolves it as a zero-field record constructor when `Name` is a declared record; otherwise normal function-call resolution applies.

### Top-level declaration placement

Records are top-level type declarations. The record declaration region remains open until the first executable top-level statement. Records and functions may be interleaved while that region is open.

After the first executable statement, a later `record` declaration is rejected with a source-native parser error.

For compatibility, top-level `fn` declarations remain accepted by the current parser even after executable statements. Nested function declarations are rejected. This is implementation behavior, not a recommendation to scatter declarations through a file.

`return` is valid only inside a function body. Top-level `return` is an error.

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
8. postfix field access
9. primary/call/constructor/grouping

`not` is recursive. Comparison precedence remains below arithmetic. Chained comparisons such as `1 < 2 < 3` are explicitly rejected rather than given Python-style semantics.

Calls, constructors, and field access compose with arithmetic, comparison, and logical expressions:

```text
print add(step(4), 2)
print wrapper.point.x + 1
if point.x > 0 and not false
    print point.x
end
```

## Parser diagnostics and recovery

- `parse()` preserves fail-fast behavior.
- `parse_recovering()` is used by user-facing paths.
- Recovery reports independent syntax errors in source order, capped at 8.
- Main synchronization boundaries are newline, `else`, `end`, and EOF.
- Nested `repeat`/`if` boundaries are preserved to avoid fake cascade errors.
- Function parameter lists, record fields/constructors, declaration placement, and required `end` tokens produce source-native parser diagnostics.
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

Current scalar/local rules:

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

Sibling branches have independent local scopes. A visible outer name is reassigned rather than shadowed. `repeat` locals are recreated per iteration and disappear when the loop body closes.

Block-local lowering is compile-time semantic scope tracking only. Generated Rust contains ordinary lexical `let`/assignment statements; no runtime scope object, `HashMap` lookup, boxing, reference counting, or dynamic dispatch is introduced for scoping.

## Current value types

The semantic layer recognizes:

- integer (`i64`);
- static/literal string;
- boolean;
- nominal record types by declared name.

Scalar rules:

- unary `-` requires integer;
- arithmetic `+ - * /` requires integer operands;
- repeat counts require integer;
- `if` conditions require boolean;
- `==` / `!=` require operands of the same supported scalar value type;
- ordering `< <= > >=` is integer-only;
- comparisons produce boolean;
- `and` / `or` require boolean operands and produce boolean;
- `not` requires one boolean operand and produces boolean;
- there is no truthiness or implicit scalar-to-boolean conversion;
- there is no hidden dynamic type layer.

Whole-record equality is explicitly unsupported in Records v0 even when both operands have the same nominal record type.

## Records v0

Records v0 is the first user-defined product-data model. It is nominal, statically typed, by-value, and designed as a ZERO-cost frontend feature.

### Declaration and nominal identity

```text
record Point
    x int
    y int
end
```

Each record declaration creates one nominal type. Two records with identical fields remain different types.

Supported field types are:

- `int` -> Rust `i64`;
- `bool` -> Rust `bool`;
- `string` -> Rust `&'static str` under the current static-string model;
- another declared record type.

Forward acyclic record references are accepted. Unknown named field types are rejected. Direct or indirect recursive by-value record layouts are rejected rather than silently boxed.

Record and function names share the Records v0 declaration namespace; collisions are rejected.

Record definitions retain declared field order, field types, and source spans in lowered IR.

### Construction

Named construction is the accepted non-empty form:

```text
point = Point(y = 2, x = 40)
```

Constructor source order does not define layout order. Lowering validates named fields and emits fields in deterministic declaration/schema order.

Construction requires exactly the declared field set:

- missing fields are rejected;
- unknown fields are rejected;
- duplicate constructor fields are rejected;
- each field expression must match the declared field type.

Positional construction of a declared record is rejected.

A declared zero-field record uses `Name()`:

```text
record Marker
end

marker = Marker()
```

`Name()` for a record with required fields is rejected as missing-field construction.

### Field access

Field access is postfix and may chain:

```text
print point.x
print wrapper.point.x
```

Accessing a scalar field does not move the containing record. Chained traversal through record-valued fields is supported when the final value is a reusable scalar.

Field access on a non-record and access to an unknown field are rejected source-natively.

Moving a record-valued field out of a containing record is explicitly unsupported in v0. The compiler does not insert a clone to make that operation appear to work.

### Record move semantics

Records use ordinary by-value move semantics rather than implicit copy/clone semantics.

Reading a record local by value consumes it. Passing a record argument by value and returning a record value use the same rule.

Example rejected by lowering:

```text
record Marker
end

fn bad(value Marker) Marker
    other = value
    return value
end
```

The second use reports a moved-record diagnostic at the Evolution source span before Rust codegen/rustc.

A moved record local may be explicitly reinitialized by assigning a new value of the same nominal type:

```text
point = Point(x = 1)
first = take(point)
point = Point(x = 41)
print first + take(point)
```

There is no implicit `.clone()`, copy insertion, borrow inference, or reference inference.

### Ownership through control flow

`if` branches are analyzed from the same pre-branch ownership state and merged conservatively. A record is available after the `if` only when it is definitely available on every continuing branch.

`repeat` preserves the zero-iteration path and rejects loop-carried record moves that would make a later iteration reuse a moved value unless the value is definitely reinitialized before the next iteration.

Lexical child-scope rules remain the same for record and scalar bindings.

### Explicit Records v0 non-operations

The following are deliberately rejected in v0:

- `print` of a whole record;
- equality/inequality of whole records;
- partial move of a record-valued field;
- recursive by-value record layouts requiring hidden indirection.

These are fail-closed boundaries, not invitations for codegen to insert runtime machinery.

### Static Rust lowering

Records emit ordinary deterministic Rust structs before functions/main:

```rust
struct __EvoRecord_Point {
    __evo_field_x: i64,
    __evo_field_y: i64,
}
```

Named construction emits an ordinary struct literal in schema order:

```rust
let __evo_point = __EvoRecord_Point {
    __evo_field_x: 40,
    __evo_field_y: 2,
};
```

Field access emits direct Rust field access. Record parameters and returns use the generated nominal Rust struct type by value.

Records v0 adds no hidden:

- heap allocation solely for records;
- `Box`, `Rc`, `Arc`, GC, or managed runtime;
- `.clone()` insertion;
- dynamic dispatch or trait-object object model;
- runtime field map / `HashMap` object representation;
- reflection or runtime record metadata.

## Functions v0

Functions v0 adds reusable named code while keeping calls fully static.

### Declaration syntax

```text
fn add(a int, b int) int
    return a + b
end
```

The signature is explicit but compact. Supported signature types are `int`, `bool`, `string`, and declared nominal record types.

The current `string` ABI is `&'static str`. Runtime-produced or owned strings are not silently introduced through cloning or allocation.

### Calls and signature collection

Calls are expressions. Functions have fixed arity. Call validation rejects unknown function names, wrong argument count, and argument type mismatches.

Lowering collects top-level function signatures before lowering function bodies and executable statements. This allows forward calls and direct recursion under an explicit signature.

The signature pre-pass is compile-time semantic metadata only; it does not create dynamic dispatch.

### Function-local scope

Each function body gets an independent root binding scope.

- parameters enter the function root scope before body lowering;
- function-local first assignments create local bindings;
- nested `if`/`else`/`repeat` bodies use lexical child scopes;
- reassignment uses the existing same-type/inferred-mutability policy;
- mutable parameters are marked `mut` only when reassigned;
- top-level locals are not captured by functions;
- duplicate parameter names and duplicate function names are rejected.

Record parameters additionally participate in the Records v0 move analysis described above.

### Return rules

Functions v0 always declare a non-unit return type. `return expression` must match that declared type.

Every reachable terminal path must return. A terminal `if/else` satisfies this only when both branches return. Loops are not treated as guaranteed-return constructs in v0.

Named functions lower to ordinary static Rust functions with deterministic names prefixed by `__evo_fn_`. There is no function registry, interpreter, VM, vtable, boxing, or dynamic dispatch solely to support named functions.

## Logical operators

`and` and `or` use strict boolean short-circuit semantics and lower directly to Rust `&&` / `||`. `not` negates one boolean value and lowers directly to Rust `!`.

There is no runtime helper for logical operators. Accepted lowering must not add allocations, clones, boxing, dynamic dispatch, reference counting, or eager RHS evaluation.

The process-level short-circuit corpus uses `input_int` as an observable side effect: skipped RHS expressions must not consume stdin.

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

The helper is emitted only when needed, including when only a function body needs it. Invalid input fails through the explicit parse contract.

## `repeat`

`repeat count ... end` lowers directly to a Rust range loop. Zero and negative counts execute zero iterations under current Rust range semantics. A binding first created in the repeat body exists only for that iteration and is unavailable after `end`.

Nested repeats and repeat/if composition are supported. Repeat lowering adds no helper runtime or allocation.

## `if` / `else`

`if condition ... else ... end` is strict boolean control flow. No truthiness is accepted.

Each branch is an independent lexical child scope. A first assignment to a name that is not already visible creates a branch-local binding. Same-name branch locals do not merge or become visible after `end`. Assignments to existing visible outer locals remain reassignment and participate in inferred mutability.

Record ownership availability is merged conservatively as described in Records v0.

## Print semantics

```text
print expression
```

lowers to Rust display output with one newline:

```rust
println!("{}", expression);
```

Integers, strings, and booleans are printable. Whole records are not printable in v0.

## Identifier lowering

- Evolution locals are prefixed with `__evo_`.
- Named functions are prefixed with `__evo_fn_`.
- Record Rust types are prefixed with `__EvoRecord_`.
- Record fields are prefixed with `__evo_field_`.

These deterministic prefixes avoid direct collisions with Rust keywords and support stable source/codegen inspection.

## Formatter

The CLI provides:

```text
evo fmt file.evo
evo fmt file.evo --check
```

Canonical formatting normalizes existing scalar/function/control-flow/block-local syntax and Records v0 syntax, including:

- record declaration indentation;
- field-name/type spacing;
- named constructor comma/assignment spacing;
- nested constructor expressions;
- field access with no whitespace around `.`;
- named record types in function signatures;
- comments and final newline behavior.

Formatting is idempotent. `--check` does not rewrite and fails when source is not canonical.

## Source-native diagnostics

Lexer, parser, and semantic-lowering diagnostics render against the original `.evo` source with message, path, line/column, source line, and caret/range underline.

Recovered lexer/parser errors are displayed in source order. Parser errors prevent lowering/rustc.

Record-specific source-native diagnostics cover declaration/type errors, exact constructor validation, invalid field access, recursive by-value layouts, and moved-record reuse. Unsupported Records v0 operations fail during lowering rather than being delegated to generated Rust.

## Generated Rust source mapping

Codegen returns optional sidecar generated-line to Evolution `Span` metadata.

Current policy:

- record struct opening/closing lines map to the owning record declaration span;
- generated record field lines map to their field declaration spans;
- `let`, reassignment, `print`, and `return` lines map to their statement spans;
- a constructor or field access rendered inside one of those statements therefore maps to the owning statement line under the current line-level policy;
- repeat/if structural generated lines map to the owning statement span;
- sibling block-local declarations map independently even when they use the same source identifier;
- function signature and closing lines map to the owning function span;
- nested function-body statements retain their own spans;
- helper/wrapper lines remain intentionally unmapped.

Source-map metadata does not alter generated Rust bytes. Column-level generated-subexpression mapping is not implemented.

## rustc diagnostic remapping

`evo build` and `evo run` map rustc errors from generated lines back to Evolution statement/function/record spans when mapping exists. Unmapped helper/wrapper/internal failures preserve raw rustc stderr rather than dropping detail.

The frontend catches known Records v0 ownership/type errors before codegen, so moved-record reuse and invalid constructors/fields remain Evolution-native diagnostics.

## Native compilation and performance contract

Accepted programs compile through rustc to native binaries.

The hard timing rule remains:

```text
T_evolution <= T_reference_rust
```

Correctness must match first. Exact byte-identical executable equality after correctness PASS is stronger deterministic runtime parity evidence; raw timing is still retained and reported rather than hidden.

See `docs/PERFORMANCE_CONTRACT.md`, `docs/BENCHMARKING.md`, issue #4, and issue #5.

Runtime-dependent Ubuntu CI gates include:

- `runtime-repeat-v0` for input/repeat/reassignment;
- `control-flow-branch-v0` for comparisons/branches/mutability;
- `logical-operators-v0` for strict logical operators;
- `function-call-v0` for typed named static calls;
- `block-locals-v0` for lexical child scopes;
- `records-v0` for named record construction and direct scalar field access in a hot runtime-dependent loop.

The harness compares correctness, raw timing, normalized LLVM IR, binary size, and exact executable bytes.

### Accepted function-call parity evidence

For `function-call-v0`:

- differential correctness: PASS;
- normalized LLVM IR equality: true;
- exact executable equality: true;
- binary size: 2,267,040 bytes on both sides;
- observed median ratio: 1.000021041;
- final verdict: PASS;
- verdict basis: `byte-identical-binary-parity`.

### Accepted block-locals parity evidence

For `block-locals-v0`:

- differential correctness: PASS;
- normalized LLVM IR equality: true;
- exact executable equality: true;
- binary size: 2,267,072 bytes on both sides;
- observed median ratio: 1.001008999;
- timing-only verdict: FAIL;
- final verdict: PASS;
- verdict basis: `byte-identical-binary-parity`.

### Accepted Records v0 parity evidence

For `records-v0`, the reference Rust mirrors the static Evolution record layout/algorithm and both sides are independently compiled with the same release path. CI #190 / run `33071967025` produced:

- differential correctness: PASS;
- normalized LLVM IR equality: true;
- exact executable equality: true;
- binary size: 2,267,104 bytes on both sides;
- observed median reference time: 18,806,589 ns;
- observed median Evolution time: 18,810,846 ns;
- observed median ratio: 1.000226357;
- timing-only verdict: FAIL;
- final verdict: PASS;
- verdict basis: `byte-identical-binary-parity`.

The timing-only value remains visible even when identical executables make runtime parity deterministic; scheduler noise is not promoted into a fictitious codegen regression.

## Current explicit non-features

Not implemented:

- closures/lambdas and first-class function values;
- inferred function parameter or return types;
- unit-returning functions;
- nested function declarations;
- function overloading/default/named/variadic arguments;
- truthiness or implicit boolean coercion;
- chained-comparison semantics;
- general explicit local type annotations;
- runtime-produced/owned string semantics beyond the current literal/static string model;
- whole-record display/equality semantics;
- partial move of record-valued fields;
- implicit clone/copy/borrow/reference inference for records;
- methods / impl blocks;
- recursive heap/self-referential records requiring indirection;
- enums/sum types;
- pattern matching;
- generics/traits;
- user-facing ownership/borrow syntax;
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

These omissions are deliberate. Unsupported behavior must fail closed rather than silently acquiring a runtime cost model.

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
