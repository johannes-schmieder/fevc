from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PRODUCTION = ROOT / "fevc" / "fevc.ado"
PLANNED_START = "program define _vckss_rust_generic_planned, eclass sortpreserve\n"
PLANNED_END = "program define _vckss_impl, eclass sortpreserve\n"


class PlannedRouteSyntaxTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        source = PRODUCTION.read_text(encoding="utf-8")
        if source.count(PLANNED_START) != 1 or source.count(PLANNED_END) != 1:
            raise AssertionError("planned Rust program boundaries are not unique")
        remainder = source.split(PLANNED_START, 1)[1]
        cls.planned = PLANNED_START + remainder.split(PLANNED_END, 1)[0]

    def test_route_predicate_has_no_continuation_comment(self) -> None:
        marker = "local route_result_ok ="
        next_marker = "local batch_result_ok ="
        self.assertEqual(self.planned.count(marker), 1)
        self.assertEqual(self.planned.count(next_marker), 1)
        block = self.planned.split(marker, 1)[1].split(next_marker, 1)[0]
        self.assertIsNone(
            re.search(r"(?m)^[ \t]*//(?!/)", block),
            "a standalone // comment inside a continued local expression breaks Stata parsing",
        )

    def test_no_comment_follows_a_continuation_marker(self) -> None:
        self.assertIsNone(
            re.search(r"///[ \t]*\n[ \t]*//(?!/)", self.planned),
            "a standalone // comment immediately after /// is parsed as part of the command",
        )


if __name__ == "__main__":
    unittest.main()
