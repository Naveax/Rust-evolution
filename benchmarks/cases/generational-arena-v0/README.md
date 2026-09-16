# generational-arena-v0

Production differential gate for the bounded generational arena slice. The Evolution workload performs repeated insert, checked lookup, checked removal, free-slot reuse, stale-handle rejection, fresh-handle lookup, and second removal against one `arena int` owner.

`reference.rs` is an independently written idiomatic safe-Rust generational-slot implementation with the same arena-id, slot-generation, retirement, and free-stack semantics. It intentionally is not a byte-for-byte copy of generated Rust; the gate therefore requires observable correctness first and then applies the repository's normal controlled timing/noise policy.

The fixture uses 2,000,000 iterations so process startup and scheduler noise do not dominate the runtime signal.
