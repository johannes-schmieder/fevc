from __future__ import annotations

import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "benchmarks/prep_rhs1/run_local.py"


def _module():
    spec = importlib.util.spec_from_file_location("prep_rhs1_local", SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_harness_is_archive_isolated_and_fail_closed() -> None:
    source = SCRIPT.read_text(encoding="utf-8")
    assert "git\", \"archive" in source
    assert "TemporaryDirectory" in source
    assert "KSS_NUMOPT2_LOCAL_PASS" in source
    assert "_compare_science" in source
    assert "candidate HEAD must have a clean worktree" in source
    assert "targets_are_advisory" in source
    assert "small_safe_counter_gains_are_retained" in source


def test_warm_medians_exclude_cold_row() -> None:
    module = _module()
    rows = []
    for run, value in enumerate((100, 3, 1, 2), start=1):
        row = {field: str(value) for field in module.TIMING_FIELDS}
        row["run"] = str(run)
        rows.append(row)
    medians = module._warm_medians(rows)
    assert set(medians) == set(module.TIMING_FIELDS)
    assert all(value == 2 for value in medians.values())


def test_scientific_comparison_rejects_one_changed_cell() -> None:
    module = _module()
    baseline = []
    candidate = []
    for _ in range(4):
        left = {field: "1" for field in module.EXACT_FIELDS}
        left["max_residual"] = "1e-12"
        baseline.append(left)
        candidate.append(dict(left))
    module._compare_science(baseline, candidate)
    candidate[2]["r34"] = "2"
    try:
        module._compare_science(baseline, candidate)
    except RuntimeError as error:
        assert "r34" in str(error)
    else:
        raise AssertionError("changed scientific cell was accepted")
