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
        for path in sorted((ROOT / "fevc").rglob("*.ado")):
            source = path.read_text(encoding="utf-8")
            for name in RETURN_PATTERN.findall(source):
                if len(name) > 32:
                    offenders.append(
                        f"{path.relative_to(ROOT)}: {name!r} has {len(name)} characters"
                    )
        self.assertEqual([], offenders, "\n".join(offenders))


    def test_planned_program_has_no_comment_after_continuation(self) -> None:
        source = (ROOT / "fevc" / "fevc.ado").read_text(
            encoding="utf-8"
        )
        start = "program define _vckss_rust_generic_planned, eclass sortpreserve\n"
        end = "program define _vckss_impl, eclass sortpreserve\n"
        self.assertEqual(1, source.count(start))
        self.assertEqual(1, source.count(end))
        planned = source.split(start, 1)[1].split(end, 1)[0]
        offenders: list[int] = []
        lines = planned.splitlines()
        for index in range(1, len(lines)):
            if lines[index - 1].rstrip().endswith("///") and lines[index].lstrip().startswith("//"):
                offenders.append(index + 1)
        self.assertEqual(
            [],
            offenders,
            f"full-line comments interrupt continued planned commands at relative lines {offenders}",
        )


if __name__ == "__main__":
    unittest.main()
