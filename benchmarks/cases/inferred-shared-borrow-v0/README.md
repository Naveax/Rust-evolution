# Inferred shared-borrow v0 benchmark

This permanent differential case exercises the bounded #97 `SharedBorrow` parameter rule under sustained runtime work.

The Evolution program passes the same owned `Item` local to a read-only nominal function on every loop iteration, then reassigns the owned value after the call. The reference Rust expresses the same contract explicitly with `&Item` / `&item`.

The case verifies:

- exact stdout parity;
- ordinary call-duration shared borrowing with no stored reference;
- the owned value remains available for later reassignment after the borrow;
- the existing `evo-bench` LLVM/binary parity and runtime parity-or-better policy.

Inputs are chosen at runtime so the repeated read cannot be reduced to a compile-time constant workload.
