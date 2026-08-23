from __future__ import annotations

import re
from pathlib import Path

root = Path.cwd()
ado = root / "varcomp_kss" / "varcomp_kss_rust.ado"
text = ado.read_text(encoding="utf-8")
old = "return scalar leverage_batch_resolution_deferred ="
new = "return scalar leverage_batch_deferred ="
if text.count(old) != 1:
    raise RuntimeError(
        f"expected exactly one overlength planned capability return, found {text.count(old)}"
    )
text = text.replace(old, new, 1)
ado.write_text(text, encoding="utf-8")

return_pattern = re.compile(
    r"^\s*return\s+(?:scalar|local|matrix)\s+([A-Za-z_][A-Za-z0-9_]*)\b",
    re.MULTILINE,
)
overlength: list[str] = []
for path in sorted((root / "varcomp_kss").rglob("*.ado")):
    source = path.read_text(encoding="utf-8")
    for name in return_pattern.findall(source):
        if len(name) > 32:
            overlength.append(f"{path.relative_to(root)}:{name}:{len(name)}")
if overlength:
    raise RuntimeError("overlength Stata return names remain: " + ", ".join(overlength))

test_path = root / "ci" / "tests" / "test_stata_return_names.py"
if test_path.exists():
    raise RuntimeError(f"refusing to overwrite existing {test_path}")
test_path.write_text(
    '''from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
RETURN_PATTERN = re.compile(
    r"^\\s*return\\s+(?:scalar|local|matrix)\\s+([A-Za-z_][A-Za-z0-9_]*)\\b",
    re.MULTILINE,
)


class StataReturnNameTests(unittest.TestCase):
    def test_static_return_names_fit_stata_identifier_limit(self) -> None:
        offenders: list[str] = []
        for path in sorted((ROOT / "varcomp_kss").rglob("*.ado")):
            source = path.read_text(encoding="utf-8")
            for name in RETURN_PATTERN.findall(source):
                if len(name) > 32:
                    offenders.append(
                        f"{path.relative_to(ROOT)}: {name!r} has {len(name)} characters"
                    )
        self.assertEqual([], offenders, "\\n".join(offenders))


if __name__ == "__main__":
    unittest.main()
''',
    encoding="utf-8",
)
print("shortened the planned capability return and added a Stata name-limit fence")
