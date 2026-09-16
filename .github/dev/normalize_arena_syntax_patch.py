from pathlib import Path

path = Path('.github/dev/arena_syntax_v0_patch.py')
text = path.read_text()
old = '''def replace_once(path: str, old: str, new: str) -> None:\n    p = Path(path)\n    text = p.read_text()\n    count = text.count(old)\n    if count != 1:\n        raise SystemExit(f"{path}: expected one anchor, found {count}: {old[:120]!r}")\n    p.write_text(text.replace(old, new, 1))\n'''
new = '''def replace_once(path: str, old: str, new: str) -> None:\n    p = Path(path)\n    text = p.read_text()\n    count = text.count(old)\n    equality_aggregate = "ValueType::Record(_) | ValueType::SharedOwner(_) | ValueType::Sequence(_)"\n    if equality_aggregate in old:\n        if count == 0:\n            return\n        p.write_text(text.replace(old, new))\n        return\n    if count != 1:\n        raise SystemExit(f"{path}: expected one anchor, found {count}: {old[:120]!r}")\n    p.write_text(text.replace(old, new, 1))\n'''
if text.count(old) != 1:
    raise SystemExit(f'expected one replace_once helper, found {text.count(old)}')
path.write_text(text.replace(old, new, 1))
