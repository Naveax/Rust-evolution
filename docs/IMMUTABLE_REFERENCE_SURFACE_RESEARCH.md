# Immutable Reference Surface v0 Research

Status: **ACCEPTED RESEARCH — SURFACE-CANDIDATE / PUNCTUATION-AMPERSAND**

Issue: #102  
PR: #103  
Rust toolchain: **1.98.0**

## Decision summary

The first caller-visible immutable reference surface should use punctuation syntax equivalent to Rust shared references:

- type surface: `&T`;
- borrow expression: `&expr`.

The accepted research verdict is **SURFACE-CANDIDATE**, with **PUNCTUATION-AMPERSAND** as the recommended surface.

This result is deliberately narrower than a production feature. It establishes a syntax family, a bounded semantic model and a fail-closed boundary for a future implementation. It does **not** change production Evolution syntax, parser behavior, ownership semantics, lowering, generated Rust, runtime behavior or accepted programs.

## Accepted evidence

Accepted research code/evidence head before documentation synchronization:

`7664d28dc0865ef4442063ed702875302271087c`

Validation on that exact head:

- Immutable reference surface research #3 / run `34598646494`: **SUCCESS** on Ubuntu 24.04 / Rust 1.98.0;
- artifact `evo-immutable-reference-surface-research-ubuntu-24.04`;
- artifact id `10263292508`;
- digest `sha256:59535a36276e7c5903d37c1b271128bf54de9e0af894d61ecb7052f52428d1f8`;
- verdict: **SURFACE-CANDIDATE**;
- recommended surface: **PUNCTUATION-AMPERSAND**;
- semantic cases: **19**;
- compile-expectation mismatches: **0**;
- normal CI #413 / run `34598646488`: **SUCCESS** on Ubuntu, Windows and macOS, including fmt, Clippy, workspace tests and release build.

## Surface comparison

The research locks representative signature/borrow examples and compares compatibility cost rather than choosing syntax by taste.

| Candidate | Signature / borrow shape | Added lexer tokens | Reserved identifiers | Locked chars | Direct Rust lowering | Compatibility |
| --- | --- | ---: | ---: | ---: | --- | --- |
| `PUNCTUATION-AMPERSAND` | `fn borrow_item(item &Item) &Item` / `r = &item` | 1 | 0 | 42 | yes | occupies currently-invalid source character |
| `KEYWORD-REF-BORROW` | `fn borrow_item(item ref Item) ref Item` / `r = borrow item` | 2 | 2 | 54 | yes | would reserve current `ref` / `borrow` identifiers |
| `GENERIC-LIKE-REF` | `fn borrow_item(item Ref(Item)) Ref(Item)` / `r = borrow(item)` | 0 | 1 | 57 | no | collides with current nominal/call-shaped identifier space |

Current lexer probes on the accepted head establish:

- `&` is currently rejected, so adding it does not steal valid existing source syntax;
- `ref` and `borrow` are currently ordinary identifiers, so keyword syntax would create avoidable source incompatibility;
- `Ref` is currently ordinary identifier/nominal space, so special-casing `Ref(...)` would collide with existing language space.

The punctuation candidate is therefore the smallest compatibility expansion and maps directly to safe Rust `&T` / `&expr` without a translation-specific wrapper type or runtime representation.

## Semantic matrix

The research matrix is intentionally broader than the syntax comparison. It tests the ownership/liveness rules that must accompany any first-class immutable reference surface.

| Case | Classification | Rust result | Required v0 meaning |
| --- | --- | --- | --- |
| owned identity remains owned | `OWNED-CONTROL` | compiles | existing `T -> T` remains an ownership transfer |
| borrowed input -> scalar result | `NO-ESCAPING-RELATION` | compiles | copied scalar carries no escaping reference |
| single-source whole reference | `SINGLE-SOURCE-REFERENCE` | compiles | one owner source can determine returned reference relation |
| single-source nested reference | `SINGLE-SOURCE-REFERENCE` | compiles | nested nominal reference remains tied to sole owner |
| immediate reference inspection | `SINGLE-SOURCE-REFERENCE` | compiles | reference contract remains visible even if immediately read |
| reference stored in local | `SINGLE-SOURCE-REFERENCE` | compiles | first-class immutable reference local is representable |
| owner move while reference live | `SINGLE-SOURCE-REFERENCE` | rejected | must fail before codegen |
| owner reinit while reference live | `SINGLE-SOURCE-REFERENCE` | rejected | must fail before codegen |
| owner move after final reference use | `BOUNDED-LAST-USE` | compiles | borrow may end at proven final use |
| move nominal field through shared reference | `SINGLE-SOURCE-REFERENCE` | rejected | move-only nominal cannot move through `&T` |
| read scalar field through shared reference | `NO-ESCAPING-RELATION` | compiles | reusable scalar inspection remains allowed |
| single-source forwarding | `SINGLE-SOURCE-REFERENCE` | compiles | sole owner provenance can propagate through a call |
| single-source recursion | `SINGLE-SOURCE-REFERENCE` | compiles | deterministic sole-owner relation remains representable |
| same-owner branch | `SINGLE-SOURCE-REFERENCE` | compiles | multiple control paths are valid when owner source is identical |
| two-owner ambiguous return | `REQUIRES-LIFETIME-RELATION` | rejected | v0 must fail closed rather than invent a relation |
| different-owner branch | `REQUIRES-LIFETIME-RELATION` | rejected | v0 must fail closed when branch changes owner source |
| reference to local owner | `INVALID-OWNER` | rejected | returned reference cannot outlive local owner |
| reference passed to shared-borrow parameter | `SINGLE-SOURCE-REFERENCE` | compiles | first-class reference interoperates with read-only call boundary |
| owned local borrowed only for call | `NO-ESCAPING-RELATION` | compiles | existing call-duration `SharedBorrow` stays a separate non-escaping mechanism |

All 19 pre-registered compile expectations match on the accepted head.

## Bounded last-use result

The important ownership result is not merely that live references block owner mutation. The matrix also proves the positive boundary: moving an owner **after the final reference use** compiles.

A production successor should therefore attempt bounded local last-use liveness instead of treating every stored reference as live until the end of the function or lexical scope.

This does not require generalized lifetime solving for v0. The implementation can remain conservative:

- track first-class immutable-reference locals and their owner provenance;
- determine the final local use where control flow is statically bounded;
- end the borrow after that proven final use;
- keep the borrow live across paths/loops that the bounded analysis cannot prove safe;
- reject a move/reinitialization while any possibly-live reference still points at that owner.

Ambiguous cases should lose ergonomics, not safety.

## Recommended v0 static model

This section is a production-successor design recommendation derived from the accepted evidence and the current compiler structure. It is not yet implemented behavior.

### Syntax AST

Current parser `TypeName` represents only owned primitive/nominal types, and `ExprKind` has no borrow node. The successor should add explicit variants rather than reinterpret an existing owned node:

```text
TypeName::SharedRef(Box<TypeName>)
ExprKind::SharedBorrow(Box<Expr>)
```

Surface forms:

```text
&Item
&item
```

Function parameter and return contracts must preserve that distinction in the syntax tree. An existing `Item` return must continue to mean owned `Item`.

### Semantic type

Current semantic types likewise represent owned values only. The successor should preserve reference-ness explicitly, for example:

```text
SemanticType::SharedRef(Box<SemanticType>)
```

The ownership relation should not be encoded as a user-visible lifetime name. Reference provenance should be separate semantic metadata attached to reference-valued expressions/bindings, such as a single owner/source identity for v0.

This separation keeps two facts distinct:

1. the value's type is an immutable reference to `T`;
2. the particular reference value is derived from owner/source `X`.

### Single-source relation rule

Named lifetime syntax remains unnecessary for the accepted v0 subset when a returned reference has exactly one deterministic owner source.

Allowed examples include:

- one immutable-reference parameter -> whole reference result;
- one immutable-reference parameter -> nested nominal reference;
- forwarding/recursion that preserves that same sole source;
- branches where every returned reference derives from the same source.

A returned reference with two possible input owner sources must fail closed in v0. No hidden tie-breaking, invented `'static`, implicit clone, allocation or runtime ownership object is permitted.

### Local reference bindings

A local may store a first-class immutable reference. Its semantic type can be inferred from the producing expression, but the reference nature must remain explicit in semantic IR and ownership analysis.

The owner must remain valid while that local reference is live. Moving or reinitializing the owner before the final possible reference use must produce an Evolution-native diagnostic before Rust codegen.

### Existing `SharedBorrow` interoperability

The already-implemented inferred `SharedBorrow` parameter mode is a **call-duration passing mode**, not a first-class reference value. The new surface must not collapse those concepts.

Required behavior:

- owned local -> existing `SharedBorrow` parameter: borrow for the call only;
- first-class `&T` local -> `SharedBorrow` parameter: pass/read through the existing reference without consuming the owner;
- neither case creates an escaping reference unless the callee signature explicitly returns `&T`.

### Code generation

The accepted punctuation surface should lower directly to safe ordinary Rust:

```text
&T    -> &T
&expr -> &expr
```

No `unsafe`, lifetime extension, heap allocation, wrapper object, `Rc`, `Arc`, GC, hidden clone or runtime borrow table is justified by this research.

## Required source-native diagnostics

A production successor should reject at Evolution semantic/lowering time, with owner/reference provenance in the diagnostic, at least:

- owner move while a later reference use remains;
- owner reinitialization while a later reference use remains;
- moving a move-only nominal field through an immutable reference;
- returning a reference derived from a local/temporary owner;
- returned-reference provenance with multiple possible owners under v0;
- any unsupported mutable-reference syntax or operation.

Generated Rust errors are not the language's ownership UI.

## Explicit non-goals

This research does not approve:

- mutable references;
- generalized or user-written lifetime parameters;
- multi-owner lifetime relation solving;
- self-referential structures;
- reference fields in records/enums unless separately researched;
- escaping closure captures;
- unsafe lifetime widening;
- invented `'static` references;
- hidden clone/copy;
- boxing, `Rc`, `Arc`, GC or runtime ownership maps;
- changing existing owned `T -> T` APIs into borrowed contracts.

## Failed-SHA evidence retained

Initial research head:

`6f9f9f10c61f2b01ca44ac02a46106c8af5c604c`

- dedicated immutable-reference research succeeded;
- normal CI #411 / run `34583343742` failed only rustfmt;
- that SHA was not rerun.

Format-only successor:

`733f7efe154422ac0e5c84ce500204b62cf84f51`

- Immutable reference surface research #2 / run `34583596906`: **SUCCESS**;
- artifact id `10192626647`;
- digest `sha256:2208bfaf76475e53e5e9581d4d4cc16afdb06057651d348047a53042fea02d`;
- normal CI #412 / run `34583596924` failed because the research-only `write_reports` helper exceeded Clippy's argument-count lint under `-D warnings` on all three platforms;
- the semantic matrix and dedicated research gate were not the failure.

The lint was fixed narrowly on new head `7664d28dc0865ef4442063ed702875302271087c` with a justified function-local Clippy allowance. Workspace lint policy, CI policy and research semantics were not weakened. That head then passed dedicated research #3 and normal three-OS CI #413.

## Final research decision

**SURFACE-CANDIDATE / PUNCTUATION-AMPERSAND**.

The evidence supports a bounded production successor with explicit immutable-reference types/values, deterministic single-source provenance, local last-use liveness and fail-closed multi-owner behavior. The successor should implement this narrow contract before any generalized lifetime or mutable-reference work.