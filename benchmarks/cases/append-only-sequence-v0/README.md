# append-only-sequence-v0

Production parity gate for the first bounded collection slice. The Evolution program constructs `seq int`, grows it explicitly with `append`, and performs checked indexed lookup with an explicit failure branch. The Rust reference mirrors generated `Vec<i64>`/`push`/`get` code exactly so the benchmark measures abstraction overhead rather than different algorithms.
