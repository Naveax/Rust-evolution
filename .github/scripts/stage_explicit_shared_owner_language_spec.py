from pathlib import Path


PATH = Path("docs/LANGUAGE_SPEC_V0.md")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    assert count == 1, f"{label}: expected exactly one match, found {count}"
    return text.replace(old, new, 1)


text = PATH.read_text()

text = replace_once(
    text,
    'record_field        := IDENTIFIER type_name\n\nenum_definition     := "enum" IDENTIFIER NEWLINE+\n                       enum_variant_list "end"\nenum_variant_list   := (NEWLINE* enum_variant NEWLINE+)* NEWLINE*\nenum_variant        := IDENTIFIER type_name?\n\nfunction_definition := "fn" IDENTIFIER "(" parameters? ")" type_name NEWLINE+\n                       function_block "end"\nparameters          := parameter ("," parameter)*\nparameter           := IDENTIFIER type_name\ntype_name           := "int" | "bool" | "string" | IDENTIFIER | "&" IDENTIFIER',
    'record_field        := IDENTIFIER storage_type_name\n\nenum_definition     := "enum" IDENTIFIER NEWLINE+\n                       enum_variant_list "end"\nenum_variant_list   := (NEWLINE* enum_variant NEWLINE+)* NEWLINE*\nenum_variant        := IDENTIFIER storage_type_name?\n\nfunction_definition := "fn" IDENTIFIER "(" parameters? ")" function_type_name NEWLINE+\n                       function_block "end"\nparameters          := parameter ("," parameter)*\nparameter           := IDENTIFIER function_type_name\nstorage_type_name   := "int" | "bool" | "string" | IDENTIFIER\nfunction_type_name  := storage_type_name | "&" IDENTIFIER | "shared" IDENTIFIER',
    "grammar type families",
)

text = replace_once(
    text,
    'unary               := "-" unary | "&" unary | postfix',
    'unary               := "-" unary | "&" unary | contextual_shared_prefix | postfix\ncontextual_shared_prefix\n                    := "share" unary | "dup" unary',
    "shared owner prefix grammar",
)

text = replace_once(
    text,
    '- Current keywords are `print`, `repeat`, `if`, `else`, `end`, `true`, `false`, `input_int`, `and`, `or`, `not`, `fn`, `return`, `record`, `enum`, `match`, `case`, `int`, `bool`, and `string`.\n- Keyword matching respects identifier boundaries.',
    '- Current keywords are `print`, `repeat`, `if`, `else`, `end`, `true`, `false`, `input_int`, `and`, `or`, `not`, `fn`, `return`, `record`, `enum`, `match`, `case`, `int`, `bool`, and `string`.\n- `shared`, `share`, and `dup` remain ordinary identifier tokens. The parser interprets them contextually only in the bounded shared-owner type/prefix positions; calls such as `share(...)` and ordinary bindings/names remain compatible.\n- Keyword matching respects identifier boundaries.',
    "contextual shared words",
)

text = replace_once(
    text,
    '7. unary numeric `-`\n8. unary immutable borrow `&`\n9. postfix qualification/field access\n10. primary/call/constructor/grouping',
    '7. unary numeric `-`\n8. unary immutable borrow `&` and contextual explicit shared-owner `share` / `dup` prefixes\n9. postfix qualification/field access\n10. primary/call/constructor/grouping',
    "precedence list",
)

text = replace_once(
    text,
    '- nominal enum types by declared name;\n- first-class immutable references to nominal record values (`&T`).',
    '- nominal enum types by declared name;\n- first-class immutable references to nominal record values (`&T`);\n- explicit one-thread immutable shared-owner handles to nominal record values (`shared T`).',
    "value type list",
)

immutable_exclusions = "Explicit v0 exclusions: mutable references, nested `&&T`/`&&expr`, primitive reference types such as `&int`, reference fields in records/enums, generalized or user-written lifetime parameters, multi-owner lifetime solving, hidden clone/copy, allocation, RC/GC, runtime borrow tables, unsafe lifetime widening, and invented `'static` lifetimes."
shared_section = r'''Explicit v0 exclusions: mutable references, nested `&&T`/`&&expr`, primitive reference types such as `&int`, reference fields in records/enums, generalized or user-written lifetime parameters, multi-owner lifetime solving, hidden clone/copy, allocation, RC/GC, runtime borrow tables, unsafe lifetime widening, and invented `'static` lifetimes.

### Explicit shared-owner handles v0

Evolution also has a bounded one-thread immutable shared-owner value category for nominal records:

```text
record Item
    value int
end

fn forward(item shared Item) shared Item
    return item
end

owner = share Item(value = 7)
alias = dup owner
moved = forward(alias)
print owner.value + moved.value
```

The three source words stay contextual rather than becoming lexer keywords:

- `shared Item` is accepted only in function parameter/return type positions in this slice;
- `share expr` explicitly creates the first shared owner and is accepted only when `expr` produces an owned nominal record;
- `dup expr` explicitly duplicates one available shared-owner handle;
- normal calls/names such as `share(...)`, `dup(...)`, or an identifier named `shared` retain ordinary identifier behavior outside those contextual positions.

Shared-owner handles are move-only Evolution values. Ordinary assignment, a by-value function argument, and return move the handle. None of those operations inserts a reference-count increment. A caller that wants to retain another owner must write `dup` explicitly. Reinitialization is allowed only with the exact same `shared T` type under the existing move/reinitialization rules.

`share` and `dup` are deliberately different operations. `share` maps to one allocation; `dup` maps to one owner-handle duplication. `dup` is not a payload deep clone and there is no generic implicit clone operation.

Read-only payload access uses the underlying nominal record through ordinary `Rc` dereference behavior. Scalar payload fields are reusable. Moving a move-only nominal payload field out through shared ownership is rejected rather than cloned. Field assignment/mutation through an immutable shared owner is not part of v0.

An explicit payload borrow such as `r = &owner` produces an ordinary non-owning `&T`, not another owner. Its provenance remains tied to that particular source handle. Moving or reinitializing that source handle while the reference may still be live is rejected even when another duplicate owner exists. Moving a different duplicate is allowed, and bounded final-use analysis permits moving/reinitializing the source handle after the final proven reference use.

There is no implicit conversion among owned `T`, `shared T`, and `&T`; source code must use the operation that matches the intended ownership category.

Rust codegen is direct and safe:

```text
shared Item -> std::rc::Rc<__EvoRecord_Item>
share expr  -> std::rc::Rc::new(expr)
dup expr    -> std::rc::Rc::clone(&expr)
```

Payload references through a shared owner lower to an ordinary reference to the payload, using stable safe `Rc` dereference/as-ref behavior. Evolution adds no wrapper object, runtime ownership table, hidden deep clone, `Arc`, `RefCell`, lock, GC, unsafe code, or invented `'static` lifetime.

Shared-owner record fields and enum payloads, nested/general `shared` type algebra, `Weak`, cycle solving, cross-thread ownership, interior mutability, synchronization, mutable references, and generalized lifetime/generic machinery remain outside this slice.'''
text = replace_once(text, immutable_exclusions, shared_section, "shared owner semantic section")

text = replace_once(
    text,
    'Supported signature types are `int`, `bool`, `string`, and declared nominal record/enum types.',
    'Supported signature types are `int`, `bool`, `string`, declared nominal record/enum types, bounded immutable record references `&T`, and bounded explicit shared-owner record handles `shared T`. Shared-owner storage in record fields/enum payloads remains excluded from v0.',
    "function signature types",
)

text = replace_once(
    text,
    'Evolution source has no `&` parameter syntax. Lowering internally decides one of two passing modes for each function parameter:',
    'For an ordinary nominal source parameter declared as `T`, lowering may internally decide one of two passing modes. This inference is separate from explicit `&T` reference contracts and explicit `shared T` shared-owner contracts:',
    "shared borrow wording",
)

text = replace_once(
    text,
    '- function signatures;\n- comments and final newline behavior.',
    '- function signatures;\n- immutable-reference spelling such as `&Item` / `&item`;\n- contextual explicit shared-owner spelling `shared Item`, `share owner`, and `dup owner`;\n- ordinary call/name compatibility for contextual words;\n- comments and final newline behavior.',
    "formatter shared owner bullets",
)

text = replace_once(
    text,
    'Known record/enum errors are rejected before Rust codegen, including declaration/type errors, constructor errors, invalid match semantics, ownership reuse-after-move, invalid payload-binding scope, and unsupported partial-move cases.',
    'Known record/enum/reference/shared-owner errors are rejected before Rust codegen, including declaration/type errors, constructor errors, invalid match semantics, ownership reuse-after-move, invalid `share`/`dup` operands, shared-owner/reference category mismatches, live payload-reference conflicts with source-handle move/reinitialization, invalid payload-binding scope, and unsupported partial-move cases.',
    "diagnostic coverage",
)

text = replace_once(
    text,
    '- `inferred-shared-borrow-v0`;\n- `enums-v0`.',
    '- `inferred-shared-borrow-v0`;\n- `enums-v0`;\n- `explicit-shared-owner-v0` (matching idiomatic Rust `Rc<T>` ownership work).',
    "performance case list",
)

text = replace_once(
    text,
    '- implicit clone/copy insertion or general reference inference beyond the bounded shared-borrow parameter rule;',
    '- implicit clone/copy insertion or generalized ownership/reference inference beyond the implemented bounded rules;',
    "non-feature clone wording",
)

for stale in [
    '- user-facing ownership/borrow syntax;\n',
    '- returned/escaping references and generalized lifetime syntax/inference;\n',
    '- stored first-class borrow/reference values;\n',
]:
    assert stale in text, f"stale non-feature entry missing: {stale!r}"
    text = text.replace(stale, "", 1)

text = replace_once(
    text,
    '- mutable borrow inference;\n- collections and collection literals;',
    '- mutable references or mutable borrow inference;\n- `Arc`/cross-thread shared ownership, `Weak`, interior mutability, locks, or synchronization;\n- generalized shared-owner type algebra or implicit owner duplication;\n- generalized/user-written lifetime syntax or multi-owner lifetime solving;\n- collections and collection literals;',
    "current non-features shared owner boundaries",
)

PATH.write_text(text)
