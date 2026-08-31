from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PRODUCTION = ROOT / "fevc" / "fevc.ado"
MAX_STATA_NAME = 32
PLANNED_START = "program define _vckss_rust_generic_planned, eclass sortpreserve\n"
PLANNED_END = "program define _vckss_impl, eclass sortpreserve\n"
NAME = r"[A-Za-z_][A-Za-z0-9_]*"


class StataNameLengthTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        source = PRODUCTION.read_text(encoding="utf-8")
        if source.count(PLANNED_START) != 1:
            raise AssertionError("planned Rust program start is not unique")
        if source.count(PLANNED_END) != 1:
            raise AssertionError("planned Rust program end is not unique")
        remainder = source.split(PLANNED_START, 1)[1]
        cls.planned = PLANNED_START + remainder.split(PLANNED_END, 1)[0]
        cls.logical = re.sub(r"///\s*\n", " ", cls.planned)

    def assert_names_fit(self, label: str, names: list[str]) -> None:
        too_long = sorted({name for name in names if len(name) > MAX_STATA_NAME})
        self.assertEqual(
            too_long,
            [],
            f"{label} exceed Stata's {MAX_STATA_NAME}-character name limit: "
            + ", ".join(f"{name} ({len(name)})" for name in too_long),
        )

    def test_planned_literal_program_local_and_return_names_fit(self) -> None:
        names: list[str] = []
        patterns = (
            rf"(?im)^\s*program\s+define\s+({NAME})\b",
            rf"(?im)^\s*(?:local|global)\s+({NAME})\b",
            rf"(?im)^\s*(?:ereturn|return)\s+"
            rf"(?:scalar|matrix|local)\s+({NAME})\b",
        )
        for pattern in patterns:
            names.extend(re.findall(pattern, self.logical))

        for match in re.finditer(
            r"(?im)^\s*temp(?:name|var)\s+([^\n]+)$", self.logical
        ):
            names.extend(re.findall(NAME, match.group(1)))

        self.assert_names_fit("literal planned-route names", names)

    def test_dynamic_capability_aliases_fit(self) -> None:
        loops = re.findall(
            r"foreach\s+name\s+in\s+(.*?)\{\s*"
            r"local\s+cap_`name'\s*=\s*r\(`name'\)",
            self.planned,
            flags=re.S,
        )
        self.assertGreaterEqual(len(loops), 1, "capability capture loop not found")
        generated: list[str] = []
        for fields in loops:
            generated.extend(f"cap_{field}" for field in re.findall(NAME, fields))
        self.assert_names_fit("generated capability aliases", generated)

    def test_public_ereturn_names_fit(self) -> None:
        names = re.findall(
            rf"(?im)^\s*ereturn\s+(?:scalar|matrix|local)\s+({NAME})\b",
            self.logical,
        )
        self.assertIn("rust_plan_solve_peak_bytes", names)
        self.assert_names_fit("planned public e() names", names)


if __name__ == "__main__":
    unittest.main()
