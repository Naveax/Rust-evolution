# Rust Evolution — NEXT ACTION

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
