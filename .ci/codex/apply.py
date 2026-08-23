from pathlib import Path

production = Path("varcomp_kss/varcomp_kss.ado")
text = production.read_text(encoding="utf-8")
start = "program define _vckss_rust_generic_planned, eclass sortpreserve\n"
end = "program define _vckss_impl, eclass sortpreserve\n"
if text.count(start) != 1 or text.count(end) != 1:
    raise RuntimeError("planned Rust program boundaries are not unique")
prefix, remainder = text.split(start, 1)
planned_body, suffix = remainder.split(end, 1)
planned = start + planned_body

old = """    local route_result_ok =                                      ///
        `r_req_route'==`route_expected_code' &                   ///
        inlist(`r_sel_route',2,3) &                              ///
        (`route_expected_code'==0 | `r_sel_route'==`route_expected_code') & ///
        // The V6 full-fit route field is frozen as diagonal.  V7
        // plan_route_sel is authoritative for the actual selected route.
        `r_full_route'==2 &                                      ///
"""
new = """    // The V6 full-fit route field is frozen as diagonal.  V7
    // plan_route_sel is authoritative for the actual selected route.
    local route_result_ok =                                      ///
        `r_req_route'==`route_expected_code' &                   ///
        inlist(`r_sel_route',2,3) &                              ///
        (`route_expected_code'==0 | `r_sel_route'==`route_expected_code') & ///
        `r_full_route'==2 &                                      ///
"""
if planned.count(old) != 1:
    raise RuntimeError(
        f"route-result continuation block: expected one match, found {planned.count(old)}"
    )
planned = planned.replace(old, new, 1)
production.write_text(prefix + planned + end + suffix, encoding="utf-8")

static_test = Path("ci/tests/test_stata_planned_route_syntax.py")
if static_test.exists():
    raise RuntimeError(f"refusing to overwrite existing {static_test}")
static_test.write_text(
    '''from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PRODUCTION = ROOT / "varcomp_kss" / "varcomp_kss.ado"
PLANNED_START = "program define _vckss_rust_generic_planned, eclass sortpreserve\\n"
PLANNED_END = "program define _vckss_impl, eclass sortpreserve\\n"


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
            re.search(r"(?m)^\\s*//", block),
            "a // comment inside a continued local expression breaks Stata parsing",
        )

    def test_no_comment_follows_a_continuation_marker(self) -> None:
        self.assertIsNone(
            re.search(r"///[ \\t]*\\n[ \\t]*//", self.planned),
            "a // comment immediately after /// is parsed as part of the command",
        )


if __name__ == "__main__":
    unittest.main()
''',
    encoding="utf-8",
)
print("moved V6/V7 comments outside the continued Stata expression")
