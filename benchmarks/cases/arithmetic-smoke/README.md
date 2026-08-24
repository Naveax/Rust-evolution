# Arithmetic smoke case

This case validates the benchmark harness pipeline only. It intentionally produces no stdout so the fixture remains byte-for-byte platform-neutral across Unix and Windows. The frontend's `print` behavior is tested separately.

Because the workload is tiny and optimizable, this case is **not** performance evidence.
