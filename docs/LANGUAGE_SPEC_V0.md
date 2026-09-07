# Rust Evolution Language Sketch v0

Status: **experimental, deliberately small, not stable**.

This document describes frontend behavior that has implementation, tests, diagnostics, static Rust codegen evidence, and where applicable accepted differential performance evidence. Future ideas belong in `docs/LANGUAGE_DESIGN.md` and tracking issues until those requirements are met.

The implementation pipeline is:

`Evolution source -> lexer -> parser -> semantic lowering -> Rust codegen -> rustc -> native binary`

There is no VM and no mandatory standalone runtime.

## Design direction

The surface borrows Lua-style low ceremony, Python-style readability, and Rust-style strict semantics/native compilation. It does not mechanically copy any of those grammars.

A current program can combine functions, lexical block locals, nominal records, nominal enums, and exhaustive static matching:

```text
record Point
    x int
    y int
end

enum MaybePoint
    None
    Some Point
end

fn choose(flag bool, point Point) MaybePoint
    if flag
        return MaybePoint.Some(point)
    else
        return MaybePoint.None()
    end
end

point = Point(y = 2, x = 40)
value = choose(true, point)
match value
case MaybePoint.Some(found)
    print found.x + found.y
case MaybePoint.None
    print 0
end
```

Accepted source lowers to ordinary static Rust constructs and native code.

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
- `.` is postfix field access and the qualifier separator for enum variants.
- `,` separates function parameters, call arguments, and named record-constructor fields.
- Current keywords are `print`, `repeat`, `if`, `else`, `end`, `true`, `false`, `input_int`, `and`, `or`, `not`, `fn`, `return`, `record`, `enum`, `match`, `case`, `int`, `bool`, and `string`.
- Keyword matching respects identifier boundaries.
- A lone `!` is not logical negation. `not` is the user-facing operator.

### Lexical diagnostics

Two lexer APIs exist:

- `lex()` is the fail-fast compatibility API.
- `lex_recovering()` is the user-facing recovery path used by the CLI and benchmark frontend.

On valid source both APIs produce the same token stream. On malformed source recovery reports source-ordered errors with a deterministic maximum of 8 diagnostics. Malformed lexical input is not passed to the parser.

## Grammar v0

The following sketch captures the accepted surface. Declaration placement and semantic restrictions are specified after the grammar.

```text
program             := NEWLINE* top_level_item* EOF

top_level_item      := record_definition
                     | enum_definition
                     | function_definition
                     | statement

record_definition   := "record" IDENTIFIER NEWLINE+
                       record_field_list "end"
record_field_list   := (NEWLINE* record_field NEWLINE+)* NEWLINE*
record_field        := IDENTIFIER type_name

enum_definition     := "enum" IDENTIFIER NEWLINE+
                       enum_variant_list "end"
enum_variant_list   := (NEWLINE* enum_variant NEWLINE+)* NEWLINE*
enum_variant        := IDENTIFIER type_name?

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
                     | match_statement

binding             := IDENTIFIER "=" expression
print_statement     := "print" expression
return_statement    := "return" expression
repeat_statement    := "repeat" expression NEWLINE+ block "end"
if_statement        := "if" expression NEWLINE+ block
                       ("else" NEWLINE+ block)?
                       "end"

match_statement     := "match" expression NEWLINE+
                       match_arm+
                       "end"
match_arm           := "case" qualified_variant match_binding? NEWLINE+
                       block
match_binding       := "(" IDENTIFIER ")"
qualified_variant   := IDENTIFIER "." IDENTIFIER

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
                     | enum_constructor
                     | "input_int"
                     | "(" expression ")"

call_or_constructor := IDENTIFIER "(" call_or_named_fields? ")"
call_or_named_fields
                    := arguments | named_fields
arguments           := expression ("," expression)*
named_fields        := named_field ("," named_field)*
named_field         := IDENTIFIER "=" expression

enum_constructor    := qualified_variant "(" arguments? ")"
```

A declared enum variant has either no payload or exactly one typed payload in Enums v0. Multi-payload variants are not part of this version even though ordinary function calls can have multiple arguments.

The parser keeps zero-argument `Name()` call-shaped until semantic resolution. Lowering resolves it as a zero-field record constructor when `Name` is a declared zero-field record; otherwise normal function-call resolution applies.

### Top-level declaration placement

Records and enums are top-level type declarations. Their shared declaration region remains open until the first executable top-level statement. Records, enums, and functions may interleave while that region is open.

After the first executable top-level statement, later `record` or `enum` declarations are rejected source-natively.

For compatibility, top-level `fn` declarations remain accepted by the current parser even after executable statements. Nested record, enum, and function declarations are rejected.

`return` is valid only inside a function body. Top-level `return` is an error.

`repeat`, `if`, and `match` may nest inside top-level code or function bodies. `if` may omit `else`. Unmatched `case`, `end`, or `else`, missing required `end`, and malformed match cases are parser errors.

### Expression precedence

From lowest to highest:

1. `or`
2. `and`
3. `not`
4. comparisons
5. `+` / `-`
6. `*` / `/`
7. unary numeric `-`
8. postfix qualification/field access
9. primary/call/constructor/grouping

`not` is recursive. Comparison precedence remains below arithmetic. Chained comparisons such as `1 < 2 < 3` are explicitly rejected rather than given Python-style semantics.

## Parser diagnostics and recovery

- `parse()` preserves fail-fast behavior.
- `parse_recovering()` is used by user-facing paths.
- Recovery reports independent syntax errors in source order, capped at 8.
- Main synchronization boundaries are newline, `else`, `case`, `end`, and EOF.
- Nested `repeat` / `if` / `match` boundaries are preserved to avoid fake cascade errors.
- Function parameter lists, record fields/constructors, enum declarations/constructors, match cases, declaration placement, and required `end` tokens produce source-native parser diagnostics.
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

Current binding rules:

- use before first definition is rejected;
- type-changing reassignment is rejected;
- a first assignment creates a binding in the current lexical scope only when no binding with that name is visible in the current or parent scopes;
- assignment to a visible binding remains reassignment, not shadowing;
- `if` then/else bodies, `repeat` bodies, and individual `match` arms create lexical child scopes;
- child scopes may read visible parent bindings while that scope is active;
- child-local bindings disappear when their block closes;
- sibling branches and sibling match arms are independent scopes;
- same-name sibling locals do not merge into an outer binding;
- a zero-iteration `repeat` never exposes a loop-local outside the loop;
- arbitrary same-name shadowing of an already-visible binding is not a v0 feature.

### Current value types

The semantic layer recognizes:

- integer (`i64`);
- static/literal string;
- boolean;
- nominal record types by declared name;
- nominal enum types by declared name.

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

Whole nominal record/enum display and equality are not v0 operations.

## Records v0

Records v0 is the first user-defined product-data model. It is nominal, statically typed, by-value, and ZERO-cost-class.

### Declaration and nominal identity

```text
record Point
    x int
    y int
end
```

Each record declaration creates one nominal type. Two records with identical fields remain different types.

Supported field types are `int`, `bool`, `string`, declared record types, and declared enum types when the resulting by-value nominal layout is acyclic.

Forward acyclic nominal references are accepted. Unknown named field types are rejected. Direct or indirect recursive by-value layouts are rejected rather than silently boxed.

Record, enum, and function declarations follow the v0 namespace collision rules; a name collision is rejected source-natively.

### Construction

Named construction is the accepted non-empty form:

```text
point = Point(y = 2, x = 40)
```

Lowering validates exact fields and emits them in deterministic declaration/schema order.

Construction requires exactly the declared field set:

- missing fields are rejected;
- unknown fields are rejected;
- duplicate constructor fields are rejected;
- every field expression must match its declared type.

Positional construction of a declared record is rejected.

A declared zero-field record uses `Name()`.

### Field access and ownership

Field access is postfix and may chain:

```text
print point.x
print wrapper.point.x
```

Accessing a scalar field does not move the containing record. Chained traversal through record-valued fields is supported when the final value is reusable.

Moving a record-valued or otherwise move-only nominal field out of a containing record is deliberately rejected in v0 rather than implemented through an implicit clone.

Records use ordinary by-value move semantics. Reading a record local by value consumes it. Passing or returning a record by value uses the same rule.

A moved record local may be explicitly reinitialized by assigning a new value of the exact same nominal type.

There is no implicit `.clone()`, copy insertion, borrow inference, or reference inference.

### Ownership through control flow

`if` branches are analyzed from the same pre-branch ownership state and merged conservatively. A move-only value is available after the `if` only when it is definitely available on every continuing branch.

`repeat` preserves the zero-iteration path and rejects loop-carried moves that would make a later iteration reuse a moved value unless the value is definitely reinitialized before the next iteration.

Terminal branches do not poison the ownership state of continuing branches.

### Static Rust lowering

Records emit ordinary deterministic Rust structs before functions/main. Named construction emits ordinary struct literals, and field access emits direct Rust field access.

Records v0 adds no hidden heap allocation solely for records, `Box`, `Rc`, `Arc`, GC, managed runtime, `.clone()` insertion, dynamic dispatch, runtime field maps, or reflection metadata.

## Enums v0

Enums v0 is the first user-defined nominal sum-data model. It is closed, statically typed, by-value, exhaustively matched, and ZERO-cost-class.

### Declaration and nominal identity

```text
enum MaybeInt
    None
    Some int
end
```

Each enum declaration creates one nominal type. Variants belong to that enum and are resolved by structured enum/variant identity.

A variant is either:

- unit: `None`
- one payload: `Some int`

Payload types may be `int`, `bool`, `string`, declared records, or declared enums. Unknown payload types are rejected.

The same variant name may appear in different enums. Duplicate variant names inside one enum are rejected.

Direct or indirect recursive by-value nominal layouts, including record-enum cycles, are rejected instead of silently introducing boxing or indirection.

### Qualified construction

Variant construction is always explicitly qualified:

```text
empty = MaybeInt.None()
value = MaybeInt.Some(41)
```

Semantic validation resolves exactly one declared enum and variant.

- unit variants require zero arguments;
- payload variants require exactly one argument;
- the payload expression must have exactly the declared type;
- nominal payload equality is by declared type identity, not structural shape.

Constructors produce the nominal enum type.

### Exhaustive statement-only matching

Enums v0 `match` is a statement construct:

```text
match value
case MaybeInt.Some(x)
    print x
case MaybeInt.None
    print 0
end
```

Rules:

- the scrutinee must have a statically known enum type;
- every arm is explicitly qualified as `Enum.Variant`;
- every arm must name a variant of the scrutinee enum;
- duplicate variant arms are rejected;
- every declared variant must appear exactly once;
- there is no wildcard arm in v0;
- there are no guards, or-patterns, arbitrary nested destructuring, or match expressions returning values;
- a payload variant requires one lexical payload binding;
- a unit variant rejects a payload binding;
- the payload binding exists only inside that arm;
- sibling arm scopes are independent;
- a payload binding may not silently shadow a still-visible outer local.

An exhaustive match in a function can satisfy terminal return analysis only when all validated arms return.

### Enum ownership

Enums are move-only nominal values in v0 even when every payload is scalar.

Reading an enum local by value consumes it. Passing an enum argument by value, returning an enum, or performing an owned exhaustive match uses the same by-value ownership model.

A moved enum local may be explicitly reinitialized by assigning a fresh value of the exact same enum type. CI contains a full native process regression proving reinitialization after a consuming call restores availability.

Payload bindings follow their payload type:

- scalar payload bindings are reusable under scalar rules;
- record/enum payload bindings are move-only;
- unsupported partial-move complexity fails closed rather than triggering clone insertion.

Ownership joins for `if`, `repeat`, and exhaustive `match` are conservative across continuing paths. Terminal branches/arms are excluded from continuing-state merges. `repeat` retains zero-iteration safety.

### Static Rust lowering

Enums lower to ordinary deterministic Rust enum definitions, direct constructors, and direct exhaustive Rust `match`.

Conceptually:

```rust
enum __EvoEnum_MaybeInt {
    __EvoVariant_None,
    __EvoVariant_Some(i64),
}
```

A payload constructor lowers directly:

```rust
__EvoEnum_MaybeInt::__EvoVariant_Some(41)
```

A match lowers directly to Rust pattern matching without a runtime variant map or interpreter layer.

Enums v0 adds no hidden:

- heap allocation solely for enum representation;
- `Box`, `Rc`, `Arc`, GC, or managed runtime;
- `.clone()` insertion;
- dynamic dispatch or trait-object representation;
- runtime variant dictionaries;
- reflection/type metadata beyond ordinary Rust enum layout;
- interpreter/VM machinery.

## Functions v0

Functions are reusable named static code.

### Declarations and calls

```text
fn add(a int, b int) int
    return a + b
end
```

Supported signature types are `int`, `bool`, `string`, and declared nominal record/enum types.

Calls are expressions with fixed arity. Lowering rejects unknown functions, wrong argument counts, and argument type mismatches.

Function signatures are collected before bodies and executable statements so forward calls and direct recursion work under explicit signatures. This pre-pass is compile-time metadata only.

### Function-local scope and returns

Each function body gets an independent root binding scope. Parameters enter that scope before body lowering. Nested `if`, `repeat`, and `match` bodies use lexical child scopes.

Top-level locals are not captured. Duplicate parameter and function names are rejected.

Functions v0 always declare a non-unit return type. Every reachable terminal path must return. A terminal `if/else` satisfies this only when both branches return; an exhaustive match satisfies it only when all arms return. Loops are not considered guaranteed-return constructs.

Nominal record/enum parameters and returns participate in the same by-value ownership analysis as other uses.

Named functions lower to ordinary static Rust functions prefixed by `__evo_fn_`. There is no function registry, VM, vtable, boxing, or dynamic dispatch solely for named functions.

## Logical operators

`and` and `or` use strict boolean short-circuit semantics and lower directly to Rust `&&` / `||`. `not` lowers directly to Rust `!`.

There is no runtime helper for logical operators and no eager RHS evaluation. The process-level short-circuit corpus uses `input_int` as an observable side effect.

## `input_int`

`input_int` reads one line from standard input and parses signed `i64`.

The generated helper uses `std::io::stdin().read_line`, `trim`, and `parse::<i64>()`. It is emitted only when needed, including when use appears only inside a function.

## `repeat`

`repeat count ... end` lowers directly to a Rust range loop. Zero and negative counts execute zero iterations under current range semantics.

A binding first created in the repeat body is lexical to that body. Repeat lowering adds no helper runtime or allocation.

## `if` / `else`

`if condition ... else ... end` is strict boolean control flow. There is no truthiness.

Each branch is an independent lexical child scope. Assignment to an already-visible outer binding remains reassignment and participates in inferred mutability.

Move-only record/enum ownership availability is merged conservatively across continuing branches.

## Print semantics

```text
print expression
```

lowers to Rust display output with one newline:

```rust
println!("{}", expression);
```

Integers, static strings, and booleans are printable. Whole records and whole enums are not printable in v0.

## Identifier lowering

- Evolution locals are prefixed with `__evo_`.
- Named functions are prefixed with `__evo_fn_`.
- Record Rust types are prefixed with `__EvoRecord_`.
- Record fields are prefixed with `__evo_field_`.
- Enum Rust types are prefixed with `__EvoEnum_`.
- Enum variants are prefixed with `__EvoVariant_`.

These deterministic prefixes avoid direct collisions with Rust keywords and make generated-code inspection stable.

## Formatter

The CLI provides:

```text
evo fmt file.evo
evo fmt file.evo --check
```

Canonical formatting is idempotent and covers current scalar/function/control-flow/block-local syntax plus Records v0 and Enums v0 syntax, including:

- record and enum declaration indentation;
- field/variant type spacing;
- named record-constructor spacing;
- qualified enum constructor spacing;
- `match` / `case` indentation;
- payload binding formatting;
- field/variant qualification with no whitespace around `.`;
- function signatures;
- comments and final newline behavior.

`--check` never rewrites and fails when source is not canonical.

## Source-native diagnostics

Lexer, parser, and semantic diagnostics render against the original `.evo` source with path, message, line/column, source line, and caret/range underline.

Recovered lexer/parser errors are displayed in source order. Parser errors prevent lowering/rustc.

Known record/enum errors are rejected before Rust codegen, including declaration/type errors, constructor errors, invalid match semantics, ownership reuse-after-move, invalid payload-binding scope, and unsupported partial-move cases.

For move-only record/enum reuse diagnostics:

- the invalid reuse remains the primary Evolution source location;
- when deterministic move provenance is available, the diagnostic renders at most one related Evolution-source location identifying the move origin;
- direct consumption is source-native, and function-argument, return, owned-match, continuing-control-flow, and repeat-body causes are retained where the existing ownership analysis exposes that context;
- Records v0 generic expression consumption may use the direct move wording rather than inventing a context the Records lowering path does not structurally expose;
- when multiple continuing paths make the same binding unavailable, provenance selection is deterministic by source order;
- terminal paths are excluded from continuing-state ownership joins;
- exact same-type reinitialization clears prior move provenance;
- provenance disappears with a binding when its lexical scope ends;
- missing-binding, type-mismatch, parser, and unrelated semantic diagnostics do not inherit a move-origin note;
- move-provenance metadata is compile-time diagnostic state only and does not change accepted generated Rust bytes or generated-program runtime behavior.

## Generated Rust source mapping

Codegen returns generated-line to Evolution `Span` sidecar metadata.

Current policy includes:

- record struct opening/closing lines -> owning record span;
- record field lines -> field declaration spans;
- enum opening/closing lines -> owning enum span;
- enum variant lines -> variant declaration spans;
- `let`, reassignment, `print`, and `return` -> statement spans;
- constructors rendered inside statements -> owning statement line under the line-level policy;
- repeat/if/match structural lines -> owning statement span;
- match arm pattern/closing lines -> arm spans;
- function signature/closing lines -> owning function span;
- nested statements retain their own spans;
- helper/wrapper lines remain intentionally unmapped.

Source-map metadata does not alter generated Rust bytes. Column-level generated-subexpression mapping is not implemented.

## rustc diagnostic remapping

`evo build` and `evo run` map rustc errors from generated lines back to Evolution spans when a mapping exists. Unmapped helper/wrapper/internal failures preserve raw rustc stderr.

Known Records/Enums v0 type, match, and ownership errors are intended to remain Evolution-native before rustc.

## Native compilation and performance contract

Accepted programs compile through rustc to native binaries.

The hard timing rule remains:

```text
T_evolution <= T_reference_rust
```

Correctness must match first. When Evolution and the locked equivalent Rust reference compile to byte-identical executables, that exact binary identity is stronger deterministic runtime parity evidence; raw wall-clock samples are still retained and reported rather than hidden.

See `docs/PERFORMANCE_CONTRACT.md`, `docs/BENCHMARKING.md`, issue #4, and issue #5.

Runtime-dependent Ubuntu CI gates include:

- `runtime-repeat-v0`;
- `control-flow-branch-v0`;
- `logical-operators-v0`;
- `function-call-v0`;
- `block-locals-v0`;
- `records-v0`;
- `enums-v0`.

The harness compares correctness, raw timing, normalized LLVM IR, binary size, and exact executable bytes.

### Accepted function-call parity evidence

For `function-call-v0`:

- differential correctness: PASS;
- normalized LLVM IR equality: true;
- exact executable equality: true;
- binary size: 2,267,040 bytes on both sides;
- final verdict: PASS;
- verdict basis: `byte-identical-binary-parity`.

### Accepted block-locals parity evidence

For `block-locals-v0`:

- differential correctness: PASS;
- normalized LLVM IR equality: true;
- exact executable equality: true;
- binary size: 2,267,072 bytes on both sides;
- final verdict: PASS;
- verdict basis: `byte-identical-binary-parity`.

### Accepted Records v0 parity evidence

Records v0 has accepted differential evidence with correctness PASS, normalized LLVM equality, byte-identical executables, equal binary size, and final `byte-identical-binary-parity` PASS. Historical raw timing remains retained in benchmark evidence rather than being promoted over identical executable bytes.

### Accepted Enums v0 parity evidence

Corrected Enums v0 feature head `69bc2d1b15db1bd841b85e8a508c156dc689550d`, CI #276 / run `34108814832`, produced:

- exact benchmark-reference/generated-Rust lock: PASS;
- differential stdout/stderr/exit correctness: PASS;
- normalized LLVM IR equality: true;
- exact executable equality: true;
- reference binary size: 2,267,072 bytes;
- Evolution binary size: 2,267,072 bytes;
- reference median: 16,506,786 ns;
- Evolution median: 16,520,050 ns;
- p95: 16,596,414 ns reference / 16,619,046 ns Evolution;
- relative MAD: 0.001764426 reference / 0.002294908 Evolution;
- stable measurement: true;
- observed median ratio: 1.000803548;
- timing-only verdict: FAIL;
- final verdict: PASS;
- verdict basis: `byte-identical-binary-parity`.

Because both accepted sides compile to the same executable bytes after correctness PASS, scheduler-level wall-clock jitter cannot represent a generated-code runtime regression. The timing-only result remains visible as evidence rather than being erased.

## Current explicit non-features

Not implemented in v0:

- closures/lambdas and first-class function values;
- inferred function parameter or return types;
- unit-returning functions;
- nested record/enum/function declarations;
- function overloading/default/named/variadic arguments;
- truthiness or implicit boolean coercion;
- chained-comparison semantics;
- general explicit local type annotations;
- runtime-produced/owned string semantics beyond the current literal/static string model;
- whole-record or whole-enum display/equality semantics;
- partial move of move-only nominal fields;
- implicit clone/copy/borrow/reference inference;
- methods / impl blocks;
- recursive heap/self-referential nominal layouts requiring indirection;
- generic enums or generic records;
- match expressions returning values;
- match guards;
- wildcard patterns;
- or-patterns;
- arbitrary nested destructuring;
- slice/range/reference patterns;
- numeric enum discriminant control;
- C-layout/FFI enum guarantees;
- open/extensible variants;
- runtime reflection;
- traits/generics generally;
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

Unsupported behavior must fail closed rather than silently acquiring a runtime cost model.

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
