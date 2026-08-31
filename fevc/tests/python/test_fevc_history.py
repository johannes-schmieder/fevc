from __future__ import annotations

import importlib.util
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "fevc/tools/check_fevc_history.py"
SPEC = importlib.util.spec_from_file_location("fevc_history", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def test_vckss_evidence_and_changelog_history_are_byte_identical() -> None:
    count, errors = MODULE.audit()
    assert count > 2_000
    assert errors == []


def test_active_sources_are_not_misclassified_as_history() -> None:
    assert MODULE.is_historical("vckss/qualification/example/receipt.json")
    assert MODULE.is_historical("vckss/benchmarks/demo/evidence/receipt.json")
    assert not MODULE.is_historical("vckss/vckss.ado")
    assert not MODULE.is_historical("vckss/benchmarks/demo/run.py")
