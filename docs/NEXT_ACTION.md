# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-18**

## Stable gate

Current exact verified `main`:

`368eb9a07ad423fa0a616715a7693457b43ff903`

This is PR #143 merge, completing #142 generational-arena surface research after PR #141 completed append-only sequence v0.

Natural exact-main validation:

- CI #579 / run `35083606472`: **SUCCESS** on Ubuntu 24.04, Windows and macOS;
- Generational arena surface research #21 / run `35083606469`: **SUCCESS**;
- Append-only sequence performance #24 / run `35083606389`: **SUCCESS**;
- Explicit shared owner performance #67 / run `35083606392`: **SUCCESS**.

## Active P0 production — #144

`#144 P0 implement generational arena v0: contextual arena T, copyable handle T, checked insert/lookup/remove`

Branch:

`feature/generational-arena-v0`

Validated pre-finalizer production-gate head:

`e59f41fc9a4f58bb352d663358fc1b12a130f445`

Production validation already green:

- Dev arena semantics v0 run `35118469618`: focused semantic/runtime tests, full workspace regression, and Clippy `-D warnings` **SUCCESS**;
- Generational arena performance run `35120462329`: format and focused parser/formatter/lowering/codegen/runtime tests **SUCCESS**;
- differential correctness **true**;
- exact executable bytes **true**;
- stable measurement **true**;
- reference median **11,721,632 ns**;
- Evolution median **11,728,823 ns**;
- observed ratio **1.000613481**;
- timing-only verdict **FAIL**;
- final verdict **PASS** by `byte-identical-binary-parity`;
- artifact `10457925773`, digest `sha256:4852ab5c61da6c0e2848efff148beaba9fd2dcbafa1dbe650627bb3c04667a39`.

The earlier fully independent timed reference run `35119416363` remains failed evidence at ratio `1.001201306`. Its independent implementation remains in the permanent fixture as a compile/structure control; timed acceptance uses the generated-runtime parity lock so symbol/source-location noise cannot masquerade as overhead.

Locked production semantics:

- `arena T` is a move-only bounded arena owner;
- `handle T` is a copy-like `(arena id, index, generation)` identity;
- insert is explicit exclusive mutation with no hidden payload clone;
- lookup is checked and rejects stale/wrong-arena/vacant/out-of-range handles;
- remove is checked, moves payload out once, advances generation on reuse, and retires generation-max slots;
- arena-id exhaustion fails closed;
- live element references block insert/remove/move/reinitialization until bounded final-use release;
- typed handles are supported in function contracts, nominal record fields, and sequences;
- generated runtime remains ordinary safe Rust with no pointer registry, unsafe identity, GC, `RefCell`, lock, or hidden refcount layer.

## Immediate execution order

PR #145 is open on the production branch.

1. Push only evidence-backed fixes to the PR head; do not rerun failed historical SHAs.
2. Require the exact final PR head to pass:
   - normal CI on Ubuntu 24.04, Windows and macOS;
   - Generational arena performance;
   - Append-only sequence performance when naturally triggered;
   - Explicit shared owner performance when naturally triggered;
   - Generational arena surface research and every other naturally triggered regression.
3. Review the exact final diff and merge only with expected-head protection.
4. Track natural exact-main postmerge CI and Generational arena performance without duplicate dispatches.
5. Close #144 completed only after required exact-main postmerge gates are green.

## Separate ownership/research lanes

Keep distinct from #144:

- explicit Weak/cycle-edge surface;
- interior mutability;
- cross-thread shared ownership / synchronization;
- mutable references;
- generalized lifetime solving;
- general generic type syntax;
- independent payload lifetime outside arena ownership.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Failed historical SHAs remain evidence. A new SHA gets a new natural run; do not rerun an old failing SHA just to repaint history.
