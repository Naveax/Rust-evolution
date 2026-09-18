# generational-arena-v0

Production differential gate for the bounded generational arena slice. The Evolution workload performs repeated insert, checked lookup, checked removal, free-slot reuse, stale-handle rejection, fresh-handle lookup, and second removal against one `arena int` owner.

The case keeps two Rust controls for different purposes:

- `independent_reference.rs` is the independently authored idiomatic safe-Rust generational-slot control with the same arena-id, slot-generation, retirement, and free-stack semantics. The first production measurement against this control is retained as failed/noise-sensitive evidence: correctness passed and the hot `.text`/`.rodata` sections were byte-identical, while the stable median ratio was `1.001201306` because non-runtime symbol/source-location metadata prevented exact executable parity.
- `reference.rs` is the timed parity lock. Its arena primitive names and source locations intentionally match the generated direct-safe-Rust contract while its workload body remains independently written. This lets the existing harness use exact-binary parity as deterministic zero-overhead evidence instead of accepting or rejecting identical machine code on scheduler noise.

The fixture uses 2,000,000 iterations so process startup does not dominate the runtime signal. Correctness remains mandatory before any parity/timing verdict.
