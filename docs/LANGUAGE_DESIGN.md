# Language Design Direction

## Objective

Create a Rust-compatible front-end whose user-facing syntax is inspired by:

- Lua: minimal syntax and low ceremony
- Python: readability and rapid expression of intent
- Rust: ownership, pattern matching, type system, native performance and systems control

The aim is not to blend three grammars mechanically. The aim is to take the useful properties and design a coherent language surface that is simpler to write while lowering to efficient Rust.

## Hard constraints

- Correct semantics
- Memory/data-race safety goals preserved
- No hidden mandatory GC
- No silent weakening of ownership guarantees
- No hidden expensive runtime layer
- No hidden allocation/clone/boxing/dynamic dispatch for ordinary sugar
- Runtime parity-or-better against equivalent Rust

## Areas to design

### Bindings
- immutable-by-default behavior
- concise mutable declaration
- optional explicit types
- destructuring

### Functions
- concise declaration
- expression bodies
- return type inference boundaries
- closures
- generics/trait bounds

### Data types
- records/structs
- enums/sum types
- methods
- traits/interfaces
- defaults
- pattern matching

### Collections
- Vec/list literal
- map/set literal
- ranges/slices
- iteration
- zero-cost comprehension-like forms if feasible

### Errors
- concise `Result` propagation
- clear `Option` handling
- explicit panic/error distinction
- no hidden stack/context overhead

### Ownership
- reduce visible borrow syntax where inference is safe
- keep moves understandable
- keep clones explicit or policy-controlled
- investigate lifetime-elision opportunities
- improve shared ownership ergonomics without turning everything into reference counting

### Async/concurrency
- concise async/await
- task/spawn ergonomics
- channels
- structured-concurrency research
- improve Send/Sync/Pin-related diagnostics and user surface

## Example corpus before freezing syntax

No grammar decision should be trusted based on `hello world` alone. The v0 design should be exercised against:

- CLI arguments
- loops and iteration
- collections
- file I/O
- JSON transformation
- structs/enums
- pattern matching
- error handling
- generics/traits
- async local networking
- multithreading/shared state
- FFI

Each important example should have reference Rust and Evolution versions; Python/Lua examples may be included for ergonomics comparison where useful.

## Ergonomics metrics

Separate from runtime performance, record:

- token count
- character count
- LOC
- punctuation density
- explicit type annotations
- explicit lifetime annotations
- boilerplate lines

These metrics help compare writing surface, but they never override correctness/safety/runtime gates.

## Feature acceptance process

For every proposed syntax feature:

1. Define semantics precisely.
2. Define equivalent reference Rust.
3. Implement/lower it.
4. Snapshot generated Rust.
5. Check for hidden allocations/clones/dispatch.
6. Run correctness comparison.
7. Run performance comparison.
8. Accept only if required gates pass.

The living implementation checklist is GitHub issue #2.
