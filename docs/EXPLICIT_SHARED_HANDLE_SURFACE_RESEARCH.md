# Explicit Shared Handle Surface v0 Research

Last verified research evidence: **2026-09-11**

## Status / decision

Decision: **IMPLEMENT-CANDIDATE / CONTEXTUAL-WORDS**.

Accepted source surface candidate for a future bounded one-thread shared-owner implementation:

```text
shared Item
share owner
dup owner
```

Intended mapping:

```text
shared Item -> std::rc::Rc<Item>
share expr  -> Rc::new(expr)
dup expr    -> Rc::clone(&expr)
```

This research introduces no production syntax or shared-owner semantics by itself.

## Parent and baseline

Parent issue: #109 — `P0 research explicit shared handle surface v0: one-thread Rc-like nominal ownership`.

Verified baseline before the research branch:

`0f90cca266c346941ed6453e5a2be065f4f37a07`

Natural baseline CI #467 / run `34617261754` succeeded on Ubuntu, Windows and macOS.

## Exact accepted evidence

Validated research source head:

`0ce2e006b16e5452f460f0c4fbab5c5cb057fc3b`

Validation on that exact head:

- normal CI #471 / run `34618281936`: **SUCCESS** on Ubuntu, Windows and macOS;
- dedicated Explicit shared handle surface research #4 / run `34618282024`: **SUCCESS** on Ubuntu 24.04 / Rust 1.98.0;
- artifact: `evo-explicit-shared-handle-surface-research-ubuntu-24.04`;
- artifact id: `10271057861`;
- artifact digest: `sha256:39b110a20392910ac886e0f83e98d22c5c99daa685aa6dc303649d7be70e0fa8`;
- report `git_sha`: exact `0ce2e006b16e5452f460f0c4fbab5c5cb057fc3b`;
- surface candidates: **4**;
- Rust semantic cases: **15**;
- compile-expectation mismatches: **0**;
- runtime-expectation mismatches: **0**.

Pinned compiler:

```text
rustc 1.98.0 (88d9e12ae 2026-08-18)
host: x86_64-unknown-linux-gnu
LLVM version: 22.1.8
```

An earlier evidence head `567278829bac974fa10306979d7d08653ef52123` passed the dedicated research matrix but failed normal CI only at rustfmt. That SHA remains historical evidence and was not rerun merely for color. A one-shot formatting bootstrap applied Rust 1.98 rustfmt on the research branch, then the bootstrap workflow was removed before the accepted clean head above.

## Surface comparison

| Surface | Classification | Observed frontend cost | Decision |
| --- | --- | --- | --- |
| `Shared<Item>` / `shared(owner)` / `shared_clone(owner)` | `DEFER-GENERIC-INFRASTRUCTURE` | current formatter renders `Shared < Item >`; parser has no general generic type AST | defer |
| `Rc<Item>` / `rc(owner)` / `rc_clone(owner)` | `DEFER-RUST-COUPLED-GENERIC` | same generic parser/formatter cost plus Rust-specific source coupling | defer |
| `shared Item` / `share owner` / `dup owner` | `PREFERRED-CANDIDATE` | existing identifier tokens and spacing preserve the isolated forms; no new lexer token or general generic AST required | implement candidate |
| `@Item` / `@owner` / `@@owner` | `DEFER-NEW-PUNCTUATION` | `@` currently fails lexing and would require new lexer/formatter rules | defer |

The accepted candidate is not a hard-keyword decision. `shared`, `share` and `dup` are ordinary identifier tokens today and should remain contextual. A production parser must recognize the new forms only in grammar positions where the current language does not already assign a valid meaning. Existing ordinary identifier/function usage such as `share(...)` or `dup(...)` must remain source-compatible unless a later separately justified change says otherwise.

## Rust semantic matrix

The 15-case Rust 1.98 corpus established the bounded semantic contract:

| Case | Contract | Result |
| --- | --- | --- |
| create-and-inspect | initial shared allocation is explicit | compile/run match |
| explicit-handle-duplication | owner duplication visibly increments the strong count | compile/run match |
| ordinary-assignment-moves-handle | assignment moves the handle without refcount increment | compile/run match |
| drop-one-retain-other | another owner keeps the payload alive | compile/run match |
| function-by-value-move | by-value forwarding moves the handle | compile/run match |
| reuse-after-function-move-rejected | moved handle cannot be reused | expected rejection matched |
| duplicate-then-forward-return | explicit duplicate preserves another owner during forwarding | compile/run match |
| repeat-duplicate-drop | repeated refcount work remains explicit | compile/run match |
| payload-borrow | borrow through shared owner is still ordinary non-owning `&T` | compile/run match |
| source-handle-move-with-live-borrow-rejected | moving the specific handle that sources a live borrow is invalid | expected rejection matched |
| different-handle-move-with-live-borrow-allowed | another duplicated handle may move while original-sourced borrow lives | compile/run match |
| borrow-final-use-then-source-move | bounded final-use liveness allows later source-handle move | compile/run match |
| handle-duplicate-vs-deep-clone | handle duplication differs from payload clone/second allocation | compile/run match |
| immutable-shared-mutation-rejected | shared owner does not imply interior mutability | expected rejection matched |
| cross-thread-transfer-rejected | one-thread owner is not silently `Arc`/`Send` | expected rejection matched |

## Accepted implementation contract

A production successor may implement only the bounded one-thread nominal shared-owner model proven here:

- `shared T` is a distinct shared-owner value category, not `SharedRef` and not `SharedBorrow`;
- `share expr` performs one explicit shared allocation equivalent to `Rc::new`;
- `dup expr` creates another owner handle equivalent to `Rc::clone` and is observably different from payload deep clone;
- ordinary assignment, by-value parameter passing and by-value return move the handle; they must not insert hidden refcount increments;
- payload inspection/field access must preserve the distinction between the owner handle and the payload;
- payload references derived from a handle remain ordinary non-owning references and reuse the existing provenance/final-use model where possible;
- moving/reinitializing the particular handle that is still the source of a possibly-live payload reference must fail closed;
- moving another duplicated handle is independent of a reference derived from the original handle;
- codegen must map directly to idiomatic safe Rust `Rc<T>`, `Rc::new`, `Rc::clone`, ordinary moves, borrows and drops;
- no Evolution runtime ownership table or unsafe emulation is permitted.

## Frontend compatibility rule

The contextual words are accepted precisely because they avoid both generic-type infrastructure and mandatory keyword reservation.

Production parsing must therefore preserve existing identifier behavior. Contextual recognition should be syntactically narrow, for example:

- in type position, identifier `shared` followed by a nominal type name may form `shared Item`;
- in prefix-expression position, identifier `share` followed by the bounded operand grammar may form explicit allocation;
- in prefix-expression position, identifier `dup` followed by the bounded operand grammar may form explicit handle duplication;
- normal call syntax `share(...)`, `dup(...)`, bindings named `share`/`dup`/`shared`, and other currently valid identifier usages must not be globally reclassified into keywords.

Formatter rules must remain idempotent and preserve the accepted spellings exactly.

## Explicit non-goals

This decision does **not** approve:

- `Arc` / cross-thread shared ownership;
- `Send`/`Sync`-equivalent inference;
- interior mutability (`RefCell` or equivalent);
- locks or synchronized shared mutation;
- `Weak` production semantics or cycle solving;
- arena/index ownership;
- mutable references;
- generalized/user-written lifetime solving;
- implicit allocation;
- implicit owner duplication;
- hidden payload deep clone;
- implicit conversion between owned `T`, `shared T`, and `&T`;
- shared-owner record/enum fields unless the production successor explicitly proves and scopes them;
- GC/runtime ownership tables;
- unsafe lifetime or aliasing emulation.

## Production impact

None yet. Existing owned `T`, immutable `&T`, inferred call-duration `SharedBorrow`, `SharedRef`, move tracking and reference provenance/liveness remain unchanged until a separately reviewed production implementation lands.
