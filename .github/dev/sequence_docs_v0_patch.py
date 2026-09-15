from pathlib import Path


def replace_once(path, old, new):
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one docs anchor in {path}, found {count}: {old[:120]!r}")
    p.write_text(text.replace(old, new, 1))


Path("docs/NEXT_ACTION.md").write_text(r'''# Rust Evolution — NEXT ACTION

Last verified update: **2026-09-15**

## Stable gate

Current exact verified `main` before active production issue #140:

`532e88586a252e72d7eb9923148b07e2ab94883e`

This is PR #139 squash merge, completing #132 collection-surface research.

Natural validation on that exact SHA:

- CI #555 / run `34957074327`: **SUCCESS** on Ubuntu 24.04, Windows and macOS;
- Collection surface research #6 / run `34957074432`: **SUCCESS**;
- Explicit shared owner performance #43 / run `34957074499`: **SUCCESS**.

Issue #132 is closed/completed. Its accepted result is **APPEND-ONLY-FIRST** with a contextual bounded sequence type family.

## Active P0 production — #140

`#140 P0 implement append-only sequence v0: contextual seq T, explicit growth, checked lookup`

Branch:

`feature/append-only-sequence-v0`

Validated implementation head before documentation synchronization:

`95d847cfe0b6d6d052b096748360915fe1bbcdc6`

Development validation for the semantic slice:

- Dev sequence semantics v0 #4 / run `34982930824`: **SUCCESS**;
- focused parser sequence tests: **6/6 PASS**;
- focused lowering sequence tests: **9/9 PASS**;
- focused Rust-codegen sequence tests: **6/6 PASS**;
- generated-Rust compile/process tests: **3/3 PASS**;
- exact generated/reference benchmark test: **PASS**;
- full workspace tests: **SUCCESS**;
- workspace Clippy with `-D warnings`: **SUCCESS**;
- append-only sequence differential gate: correctness **PASS**, normalized LLVM IR equal **true**, exact binary equal **true**, stable **true**, ratio **0.996626508**, verdict **PASS** by byte-identical-binary parity.

Implemented production surface:

```text
items = seq Item()
append items, Item(value = 1)

lookup items, 0 as item
    print item.value
else
    print 0
end
```

Locked production semantics:

- `seq`, `append`, `lookup`, and `as` are contextual, not new lexer keywords;
- `seq T` is an owned move-only append-only sequence type; v0 elements are scalar values, nominal records, or explicit `shared Record` owners;
- `seq T()` creates an empty owned sequence and lowers directly to `Vec::<T>::new()`;
- `append owner, value` grows one available sequence in place and lowers directly to `Vec::push` without hidden clone;
- `lookup owner, index as binding ... else ... end` is checked and requires both success and failure branches; negative and out-of-range indices take `else`;
- scalar lookup bindings copy by value; record/shared-owner elements bind as immutable references tied to the sequence owner;
- a live move-only element reference blocks sequence growth, move, and reinitialization; bounded final-use analysis releases the conflict after the final proven use;
- `shared Record` elements remain `&Rc<Record>` on lookup with no hidden `Rc::clone`;
- no removal, slot reuse, generations, nested sequence/reference elements, general generic syntax, mutable references, hidden allocation policy, GC, lock, registry, or unsafe ownership emulation is introduced.

Permanent PR/main regression gate: `.github/workflows/append-only-sequence-performance.yml`.

## Immediate execution order

1. Synchronize `LANGUAGE_SPEC_V0`, `PROJECT_STATE`, `NEXT_ACTION`, and `DECISIONS` on the production branch.
2. Open the #140 production PR from `feature/append-only-sequence-v0`.
3. Require one exact final PR head to pass all naturally triggered gates, including:
   - normal CI on Ubuntu, Windows and macOS;
   - Append-only sequence performance;
   - Explicit shared owner performance when triggered by the touched ownership/codegen surface;
   - Collection surface research when naturally triggered.
4. Review the exact final diff and merge only with expected-head protection.
5. Track natural exact-main postmerge CI and append-only sequence performance; track other naturally triggered ownership/research gates without dispatching duplicates.
6. Close #140 completed only after the required postmerge exact-main gates succeed.
7. Return to the generation-checked arena-handle successor from #126. Removal/reuse/generation semantics remain a separate explicit layer.

## Separate ownership/research lanes

Keep distinct rather than folding them into append-only sequence semantics:

- explicit Weak/cycle edges;
- interior mutability;
- cross-thread shared ownership / synchronization;
- generation-checked removable arena slots;
- mutable references;
- generalized lifetime solving;
- general generic type syntax.

## CI rule

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed/cancelled historical SHAs remain evidence and are not rerun merely for cosmetic green. CI running does not block independent source/docs work.
''')

Path("docs/PROJECT_STATE.md").write_text(r'''# Rust Evolution — Project State

Last verified update: **2026-09-15**

This is the durable project handoff. Always re-read live GitHub issue/PR/Actions state before changing code.

## Repository / toolchain

- Repository: `Naveax/Rust-evolution`
- Stable branch: `main`
- Exact verified stable main before active production #140: `532e88586a252e72d7eb9923148b07e2ab94883e`
- Rust toolchain: **1.98.0**
- Production flags: edition 2024, opt-level 3, codegen-units 1
- Natural exact-main CI #555 / run `34957074327`: **SUCCESS** on Ubuntu 24.04, Windows and macOS
- Natural exact-main Collection surface research #6 / run `34957074432`: **SUCCESS**
- Natural exact-main Explicit shared owner performance #43 / run `34957074499`: **SUCCESS**

`532e885...` is PR #139 squash merge. Issue #132 collection-surface research is closed/completed with the accepted **APPEND-ONLY-FIRST** result.

## Build / compile sequence

- #76: build latency baseline established; single-file native builds are rustc-dominated.
- #79 / PR #81: verified unchanged-build cache accepted.
- #82 / PR #84: changed-source rustc incremental research **REJECT / DEFER**.
- #85 / PR #86: link-time attribution established the current `rustc -> cc -> lld` path.
- #87 / PR #88: alternative linker experiment **REJECT / DEFER**.
- #89 / PR #90: opt-level 3 -> 2 candidate **REJECT / DEFER**.
- #91 / PR #92: compile-memory baseline **DEFER / NO ACTION**.
- #93 / PR #94: binary-size baseline **DEFER / NO ACTION**; controlled Evolution/reference binaries were byte-identical across the accepted corpus.

Dependency-build, proc-macro cost and workspace scaling remain deferred until Evolution has a real package/dependency graph.

## Ownership ergonomics implemented

### Inferred shared-borrow parameters

`Owned` and call-duration `SharedBorrow` parameter modes are distinct. SharedBorrow is non-owning and does not create a stored or escaping reference.

### First-class immutable references

`&T` / `&expr` are implemented for the bounded nominal slice with deterministic provenance, stored reference locals, source-owner move/reinitialization conflicts, bounded final-use liveness and direct safe Rust reference lowering.

### Explicit one-thread shared owners

Production source surface:

```text
shared Item
share expr
dup owner
```

Direct generated Rust mapping remains ordinary safe `Rc<T>`:

```text
shared Item -> std::rc::Rc<Item>
share expr  -> Rc::new(expr)
dup expr    -> Rc::clone(&expr)
```

Locked invariants:

- shared owners are distinct from owned values, first-class immutable references, and inferred SharedBorrow parameters;
- allocation and owner duplication are explicit only;
- ordinary assignment, by-value calls and returns move handles without hidden count increments;
- payload references remain tied to the specific source handle;
- moving/reinitializing that source handle rejects while its dependent reference may still be live;
- bounded final-use analysis releases the source after the final proven reference use;
- move-only payload extraction through shared ownership rejects instead of cloning;
- no hidden `Arc`, `RefCell`, synchronization, GC, global ownership registry or unsafe ownership emulation.

The permanent Explicit shared owner performance workflow remains a regression gate for changes that touch the relevant language/compiler surface.

## Cyclic / graph ownership research sequence

### Arena/generational handles — #126 / PR #131 completed

Durable report: `docs/ARENA_GENERATIONAL_HANDLES_RESEARCH.md`.

Accepted result:

- aggregate gate: **REQUIRES-COLLECTION-SURFACE**;
- graph-identity recommendation: **GENERATIONAL-HANDLE-CANDIDATE**;
- plain numeric indices are acceptable only while removal/reuse cannot silently rebind identity;
- reusable/removable slots require generation checking;
- independent node lifetime remains a shared-owner problem rather than an arena problem.

### Collection surface — #132 / PR #139 completed

Durable report: `docs/COLLECTION_SURFACE_RESEARCH.md`.

Accepted result:

- verdict: **APPEND-ONLY-FIRST**;
- recommended surface: **CONTEXTUAL-SEQUENCE-TYPE-CANDIDATE**;
- append/growth plus checked lookup maps directly to ordinary safe Rust storage;
- append-only numeric indices remain logically stable across physical `Vec` reallocation;
- shifting removal can silently rebind identity, while hole-preserving removal introduces an explicit occupancy model;
- live element references must block conflicting container growth/move until bounded final-use release proves them dead;
- removal, holes, generations and reusable arena slots remain later explicit layers.

## Active production — #140 append-only sequence v0

Branch:

`feature/append-only-sequence-v0`

Validated implementation head before documentation synchronization:

`95d847cfe0b6d6d052b096748360915fe1bbcdc6`

Development evidence:

- Dev sequence semantics v0 #4 / run `34982930824`: **SUCCESS**;
- focused parser/lowering/codegen/generated-Rust tests: **PASS**;
- workspace tests: **SUCCESS**;
- workspace Clippy `-D warnings`: **SUCCESS**;
- append-only differential benchmark: correctness **true**, normalized LLVM IR equal **true**, exact binary equal **true**, stable **true**, ratio **0.996626508**, verdict **PASS** by byte-identical-binary parity.

Production source surface:

```text
items = seq Item()
append items, Item(value = 1)

lookup items, 0 as item
    print item.value
else
    print 0
end
```

Implemented invariants:

- `seq`, `append`, `lookup`, and `as` remain contextual identifiers;
- `seq T` is an owned move-only append-only sequence;
- supported v0 element types are scalars, declared nominal records, and explicit `shared Record`; nested sequence/reference element types remain rejected;
- empty construction lowers to `Vec::<T>::new()`;
- append lowers to direct `Vec::push` and moves move-only payloads without hidden clone;
- lookup converts the signed index with `usize::try_from`, uses direct `Vec::get`, and requires an explicit success/failure branch;
- negative and out-of-range indices take the `else` branch;
- scalar elements bind by value; move-only record/shared-owner elements bind through an immutable element reference;
- live move-only element references block sequence growth, move, and reinitialization source-natively;
- existing bounded last-use analysis releases that conflict after the final proven reference use;
- shared-owner element lookup keeps `&Rc<T>` behavior without hidden `Rc::clone`;
- sequence parameters become mutable in generated Rust only when grown;
- unused move-only lookup bindings do not artificially pin the sequence;
- generated Rust remains ordinary safe `Vec<T>` / `push` / `get` with no `unsafe`, `RefCell`, lock, GC, registry, or custom runtime.

Permanent regression workflow: `.github/workflows/append-only-sequence-performance.yml`.

## Separate / deferred research tracks

Do not silently fold these into append-only sequence semantics:

- explicit Weak/cycle-edge surface;
- interior mutability;
- cross-thread shared ownership / `Arc` / synchronization;
- mutable references;
- generalized/user-written lifetime solving;
- general generic type syntax;
- sequence removal/pop/delete;
- hole reuse and generation-checked reusable arena slots;
- shared-owner record fields / enum payloads;
- hidden allocation policy, owner duplication, deep clone, GC or runtime ownership maps.

## Current operational sequence

1. Synchronize implementation-backed language and handoff docs for #140.
2. Open the #140 production PR from `feature/append-only-sequence-v0`.
3. Require one exact final PR head to pass normal CI plus the Append-only sequence performance gate and all other naturally triggered ownership/research regressions.
4. Review the final PR diff and merge only with expected-head protection.
5. Require natural exact-main postmerge CI and append-only sequence performance before closing #140 completed.
6. Return to the generation-checked arena-handle candidate from #126; removal/reuse/generation remains a separate explicit layer.

## CI / handoff invariant

Never create duplicate active Actions for the same SHA/workflow/input. Track the existing run. Failed/cancelled historical SHAs remain evidence and are not rerun merely for color. CI running does not block independent source/docs work.

Authority hierarchy:

`tests + main code > LANGUAGE_SPEC_V0 > current PR/CI evidence > PROJECT_STATE/NEXT_ACTION > DECISIONS > ROADMAP > OMNI_VISION`.
''')

spec = "docs/LANGUAGE_SPEC_V0.md"
replace_once(
    spec,
    "- `shared`, `share`, and `dup` remain ordinary identifier tokens. The parser interprets them contextually only in the bounded shared-owner type/prefix positions; calls such as `share(...)` and ordinary bindings/names remain compatible.\n",
    "- `shared`, `share`, and `dup` remain ordinary identifier tokens. The parser interprets them contextually only in the bounded shared-owner type/prefix positions; calls such as `share(...)` and ordinary bindings/names remain compatible.\n- `seq`, `append`, `lookup`, and `as` also remain ordinary identifier tokens. The parser interprets them contextually only for the bounded append-only sequence type/constructor and statement forms; ordinary bindings, names, and calls remain compatible outside those exact positions.\n",
)
replace_once(
    spec,
    '''storage_type_name   := "int" | "bool" | "string" | IDENTIFIER
function_type_name  := storage_type_name | "&" IDENTIFIER | "shared" IDENTIFIER
''',
    '''storage_type_name   := "int" | "bool" | "string" | IDENTIFIER
sequence_element_type
                    := "int" | "bool" | "string" | IDENTIFIER | "shared" IDENTIFIER
function_type_name  := storage_type_name
                     | "&" IDENTIFIER
                     | "shared" IDENTIFIER
                     | "seq" sequence_element_type
''',
)
replace_once(
    spec,
    '''statement           := binding
                     | print_statement
                     | repeat_statement
                     | if_statement
                     | match_statement

binding             := IDENTIFIER "=" expression
print_statement     := "print" expression
return_statement    := "return" expression
repeat_statement    := "repeat" expression NEWLINE+ block "end"
''',
    '''statement           := binding
                     | print_statement
                     | repeat_statement
                     | if_statement
                     | match_statement
                     | sequence_append
                     | sequence_lookup

binding             := IDENTIFIER "=" expression
print_statement     := "print" expression
return_statement    := "return" expression
sequence_append     := "append" IDENTIFIER "," expression
sequence_lookup     := "lookup" IDENTIFIER "," expression "as" IDENTIFIER NEWLINE+
                       block "else" NEWLINE+ block "end"
repeat_statement    := "repeat" expression NEWLINE+ block "end"
''',
)
replace_once(
    spec,
    '''primary             := INTEGER
                     | STRING
                     | "true"
                     | "false"
                     | IDENTIFIER
                     | call_or_constructor
                     | enum_constructor
                     | "input_int"
                     | "(" expression ")"

call_or_constructor := IDENTIFIER "(" call_or_named_fields? ")"
''',
    '''primary             := INTEGER
                     | STRING
                     | "true"
                     | "false"
                     | sequence_constructor
                     | IDENTIFIER
                     | call_or_constructor
                     | enum_constructor
                     | "input_int"
                     | "(" expression ")"

sequence_constructor
                    := "seq" sequence_element_type "(" ")"
call_or_constructor := IDENTIFIER "(" call_or_named_fields? ")"
''',
)
replace_once(
    spec,
    "`repeat`, `if`, and `match` may nest inside top-level code or function bodies. `if` may omit `else`. Unmatched `case`, `end`, or `else`, missing required `end`, and malformed match cases are parser errors.\n",
    "`repeat`, `if`, `match`, and checked `lookup` may nest inside top-level code or function bodies. `if` may omit `else`; `lookup` always requires an explicit `else`. Unmatched `case`, `end`, or `else`, missing required `end`, and malformed match/lookup forms are parser errors.\n",
)
replace_once(
    spec,
    "- explicit one-thread immutable shared-owner handles to nominal record values (`shared T`).\n",
    "- explicit one-thread immutable shared-owner handles to nominal record values (`shared T`);\n- owned append-only sequences (`seq T`) for the bounded v0 element set.\n",
)
sequence_section = r'''### Append-only sequences v0

Evolution's first production collection is deliberately narrower than a general generic container system:

```text
record Item
    value int
end

items = seq Item()
append items, Item(value = 1)

lookup items, 0 as item
    print item.value
else
    print 0
end
```

`seq`, `append`, `lookup`, and `as` are contextual parser words only. They remain ordinary identifier tokens outside these exact forms.

The v0 type and construction rules are:

- `seq T` is an owned, move-only sequence type;
- `T` may be `int`, `bool`, `string`, a declared nominal record, or `shared Record`;
- nested `seq` elements and reference element types are rejected;
- `seq T()` constructs an empty sequence;
- there is no general user-facing generic type syntax introduced by this feature.

Growth is explicit:

```text
append items, value
```

The owner must be an available `seq T`, and the appended value must have exactly type `T`. Move-only payloads move into the sequence. Append does not clone the payload, duplicate a shared owner, or replace exclusive mutation with interior mutability. A sequence local or parameter is marked mutable in generated Rust only when growth requires it.

Lookup is checked and statement-only:

```text
lookup items, index as value
    # success branch
else
    # failure branch
end
```

The index must be an integer. Lowering converts it with `usize::try_from` and then performs `Vec::get`; therefore negative and out-of-range Evolution indices enter the explicit `else` branch. The success binding exists only inside the success branch and cannot be reassigned in this v0 slice.

Binding ownership follows the element category:

- scalar elements bind by value, matching the copy-like scalar model;
- nominal record elements bind as immutable references tied to the source sequence;
- `shared Record` elements bind as references to the stored `Rc<Record>` handle, so payload field reads use ordinary Rust dereference behavior without `Rc::clone`;
- an unused move-only lookup binding does not artificially extend a borrow.

A live move-only element reference prevents any operation that may invalidate its source: sequence growth, sequence move, and exact-type sequence reinitialization are rejected source-natively while that reference may still be live. The existing bounded last-use analysis releases the conflict after the final proven reference use, allowing later growth/move/reinitialization when safe.

Sequences themselves use ordinary by-value ownership. Passing or returning `seq T` by value moves the sequence unless ordinary function semantics say otherwise; there is no implicit sequence clone.

Rust codegen is direct and safe:

```text
seq T                  -> Vec<T>
seq T()                -> Vec::<T>::new()
append owner, value    -> owner.push(value)
checked lookup         -> usize::try_from(index).ok().and_then(|i| owner.get(i))
```

The accepted differential case proves correctness, normalized LLVM IR parity and exact binary parity against the direct Rust `Vec<i64>` reference workload.

Explicit v0 exclusions: removal/pop/delete, slot holes, slot reuse, generation counters, stable arena identity, iterators/ranges/algorithms, mutable references, nested sequence/reference element types, general generic syntax, hidden clone/copy of move-only values, hidden `Rc`/`Arc` duplication, `RefCell`, locks, GC, runtime ownership registries, and unsafe pointer tables.

'''
replace_once(spec, "Scalar rules:\n", sequence_section + "Scalar rules:\n")
replace_once(
    spec,
    "Supported signature types are `int`, `bool`, `string`, declared nominal record/enum types, bounded immutable record references `&T`, and bounded explicit shared-owner record handles `shared T`. Shared-owner storage in record fields/enum payloads remains excluded from v0.\n",
    "Supported signature types are `int`, `bool`, `string`, declared nominal record/enum types, bounded immutable record references `&T`, bounded explicit shared-owner record handles `shared T`, and bounded append-only sequence types `seq T`. Shared-owner storage in record fields/enum payloads remains excluded from v0, and sequence elements remain limited to the production v0 element set above.\n",
)
replace_once(
    spec,
    "Functions v0 always declare a non-unit return type. Every reachable terminal path must return. A terminal `if/else` satisfies this only when both branches return; an exhaustive match satisfies it only when all arms return. Loops are not considered guaranteed-return constructs.\n",
    "Functions v0 always declare a non-unit return type. Every reachable terminal path must return. A terminal `if/else` satisfies this only when both branches return; an exhaustive match satisfies it only when all arms return; a checked sequence lookup satisfies it only when both its success and failure branches return. Loops are not considered guaranteed-return constructs.\n",
)

# Keep lexical scope wording aligned with lookup's success binding.
replace_once(
    spec,
    "- `if` then/else bodies, `repeat` bodies, and individual `match` arms create lexical child scopes;\n",
    "- `if` then/else bodies, `repeat` bodies, individual `match` arms, and checked-lookup branches create lexical child scopes;\n",
)

# Durable implementation decision.
decisions = "docs/DECISIONS.md"
decision = r'''## D-022 — Append-only sequence v0 is contextual, checked, and direct-Vec

**Decision:** The first production collection is a bounded owned append-only sequence with contextual source forms rather than a general generic container language.

Accepted surface:

```text
items = seq Item()
append items, Item(value = 1)

lookup items, 0 as item
    print item.value
else
    print 0
end
```

For v0:

- `seq`, `append`, `lookup`, and `as` remain ordinary identifier tokens outside their exact contextual positions;
- `seq T` is move-only owned storage; `T` is limited to scalars, nominal records, or explicit `shared Record` owners;
- `seq T()` constructs empty storage directly;
- `append` is explicit exclusive growth and moves move-only payloads without hidden clone;
- lookup is always checked with explicit success and failure branches; negative/out-of-range signed indices select failure;
- scalar elements bind by value while move-only record/shared-owner elements bind through immutable references tied to the sequence owner;
- live move-only element references block sequence growth, move, and reinitialization until bounded last-use analysis proves the reference dead;
- a `shared Record` element lookup does not duplicate its `Rc` handle;
- generated Rust is direct safe `Vec<T>` / `push` / `get` code;
- removal, holes, slot reuse, generations, nested sequence/reference elements, general generic syntax, mutable references, hidden clone/refcount operations, `RefCell`, locks, GC, registries, and unsafe identity machinery are not part of this decision.

**Reason:** #132 proved that append-only indexed storage preserves the useful direct-`Vec` cost model without prematurely choosing removal/reuse identity semantics. #140 implements that smallest production slice while reusing the existing move and bounded immutable-reference liveness model.

Pre-PR implementation evidence: Dev sequence semantics v0 #4 / run `34982930824` passed focused parser/lowering/codegen/native tests, the full workspace suite, Clippy `-D warnings`, and the differential benchmark. The benchmark reported correctness PASS, normalized LLVM IR equality, exact binary equality, stable timing, ratio `0.996626508`, and PASS by byte-identical-binary parity.

'''
replace_once(decisions, "## Changing a decision\n", decision + "## Changing a decision\n")
