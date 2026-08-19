from __future__ import annotations

import csv
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "benchmarks/prep_rhs1/run_local.py"
SCC_SCRIPT = ROOT / "benchmarks/prep_rhs1/analyze_scc.py"


def _module():
    spec = importlib.util.spec_from_file_location("prep_rhs1_local", SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _scc_module():
    spec = importlib.util.spec_from_file_location("prep_rhs1_scc", SCC_SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _write_scc_run(root: Path, role: str, experiment: str) -> Path:
    module = _scc_module()
    directory = root / role / "experiments" / experiment
    directory.mkdir(parents=True)
    commit = "a" * 40 if role == "baseline" else "b" * 40
    bundle = "c" * 64 if role == "baseline" else "d" * 64
    summary = {
        "source_commit": commit,
        "bundle_sha256": bundle,
        **{field: "1" for field in module.SCIENTIFIC_FIELDS},
        **{field: "1" for field in module.STRUCTURAL_FIELDS},
        **{field: "1" for field in module.TIMING_FIELDS},
        **{field: "1" for field in module.RESOURCE_FIELDS},
    }
    with (directory / "summary.csv").open("w", newline="", encoding="utf-8") as file:
        writer = csv.DictWriter(file, fieldnames=summary)
        writer.writeheader()
        writer.writerow(summary)
    validation = {
        "status": "PASS",
        "experiment_id": experiment,
        "source_commit": commit,
        "bundle_sha256": bundle,
        "job_id": "12345",
        "task_sha256": "e" * 64,
        "input_sha256": "f" * 64,
        "qacct": {"hostname": "same.example.edu"},
    }
    (directory / "validation.json").write_text(
        json.dumps(validation) + "\n", encoding="utf-8"
    )
    for name in module.EVIDENCE_FILES:
        path = directory / name
        if not path.exists():
            path.write_text(f"test evidence: {name}\n", encoding="utf-8")
    return directory


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


def test_scc_analyzer_binds_validated_pairs(tmp_path: Path) -> None:
    module = _scc_module()
    _write_scc_run(tmp_path, "baseline", "cell_rep1")
    _write_scc_run(tmp_path, "candidate", "cell_rep1")
    result = module.summarize(
        tmp_path / "baseline", tmp_path / "candidate", 2e-13
    )
    assert result["status"] == "PASS"
    assert result["paired_runs"] == 1
    assert result["same_host_pairs"] == 1
    assert result["structural_comparison"] == "EXACT_PASS"
    assert result["scientific_comparison"] == "ROUNDOFF_PASS"


def test_scc_analyzer_rejects_structural_change(tmp_path: Path) -> None:
    module = _scc_module()
    _write_scc_run(tmp_path, "baseline", "cell_rep1")
    candidate = _write_scc_run(tmp_path, "candidate", "cell_rep1")
    row = next(csv.DictReader((candidate / "summary.csv").open()))
    row["N_retained"] = "2"
    with (candidate / "summary.csv").open("w", newline="", encoding="utf-8") as file:
        writer = csv.DictWriter(file, fieldnames=row)
        writer.writeheader()
        writer.writerow(row)
    try:
        module.summarize(tmp_path / "baseline", tmp_path / "candidate", 2e-13)
    except ValueError as error:
        assert "structural" in str(error)
    else:
        raise AssertionError("structural change was accepted")


def test_scc_analyzer_rejects_scientific_change(tmp_path: Path) -> None:
    module = _scc_module()
    _write_scc_run(tmp_path, "baseline", "cell_rep1")
    candidate = _write_scc_run(tmp_path, "candidate", "cell_rep1")
    row = next(csv.DictReader((candidate / "summary.csv").open()))
    row["corrected_total"] = "1.1"
    with (candidate / "summary.csv").open("w", newline="", encoding="utf-8") as file:
        writer = csv.DictWriter(file, fieldnames=row)
        writer.writeheader()
        writer.writerow(row)
    try:
        module.summarize(tmp_path / "baseline", tmp_path / "candidate", 2e-13)
    except ValueError as error:
        assert "roundoff" in str(error)
    else:
        raise AssertionError("scientific change was accepted")
