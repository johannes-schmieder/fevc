from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PRODUCTION = ROOT / "fevc" / "fevc.ado"
IMPL_START = "program define _vckss_impl, eclass sortpreserve\n"
PREDICATE_START = "        local rust_auto_engine_generic ="
PREDICATE_END = "        local rust_planned_generic_supported ="
EXPECTED_PARSED_CONTROLS = "strtrim(`\"`controls'\"')!=\"\""


class EngineAutoPhaseTests(unittest.TestCase):
    def test_early_router_uses_parsed_controls_not_materialized_controlvars(self) -> None:
        source = PRODUCTION.read_text(encoding="utf-8")
        self.assertEqual(source.count(IMPL_START), 1)
        impl = source.split(IMPL_START, 1)[1]
        self.assertEqual(impl.count(PREDICATE_START), 1)
        self.assertEqual(impl.count(PREDICATE_END), 1)
        predicate = impl.split(PREDICATE_START, 1)[1].split(PREDICATE_END, 1)[0]
        self.assertIn(EXPECTED_PARSED_CONTROLS, predicate)
        self.assertNotIn("controlvars", predicate)


if __name__ == "__main__":
    unittest.main()
