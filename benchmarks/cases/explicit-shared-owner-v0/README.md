# Explicit shared owner v0 benchmark

This case compares Evolution's bounded one-thread explicit shared-owner surface against equivalent idiomatic Rust `Rc<T>` work.

The Evolution workload performs one `share` allocation before the loop and one explicit `dup` per iteration, then moves the duplicated handle through a by-value function. The Rust reference performs the same `Rc::new`, `Rc::clone`, by-value move, field reads, and drops.

The case is intentionally not compared with a cheaper owned or borrowed program, because that would measure different ownership work rather than codegen overhead. No `Arc`, lock, interior mutability, hidden payload clone, or runtime ownership table belongs in this gate.
