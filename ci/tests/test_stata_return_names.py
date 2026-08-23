from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
RETURN_PATTERN = re.compile(
    r"^\s*return\s+(?:scalar|local|matrix)\s+([A-Za-z_][A-Za-z0-9_]*)\b",
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
        self.assertEqual([], offenders, "\n".join(offenders))


if __name__ == "__main__":
    unittest.main()
