from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "fevc" / "tools" / "run_structured_inference_campaign.py"
SPEC = importlib.util.spec_from_file_location("structured_campaign", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def _fake_binary(path: Path) -> None:
    path.write_text(
        """#!/usr/bin/env python3
import json
import sys

_, mode, k, start, count = sys.argv
assert mode == "diagnostic-q1-v3"
for cell in ("dominant_leverage", "dominant_common_t8"):
    for replication in range(int(start), int(start) + int(count)):
        for source in ("oracle", "fitted"):
            row = {
                "schema": "fevc-q1-diagnostic-v3",
                "cell": cell,
                "k": int(k),
                "replication": replication,
                "variance_source": source,
                "status": "success",
                "point_error": (-1.0 if replication % 2 else 1.0) * 0.1,
                "estimated_sd": 0.1,
                "covered": True,
                "legacy_covered": True,
                "lower_miss": False,
                "upper_miss": False,
                "leading_variance": 1.0,
                "leading_variance_correction": 0.9,
                "remainder_identity_error": 1.0e-14,
                "remainder_variance": 0.2,
                "leading_remainder_covariance": 0.0,
                "curvature": 0.2,
                "reference_critical": 2.04,
                "interval_width": 0.4,
                "score_error": 0.0,
                "remainder_error": 0.0,
                "leading_share": 0.8,
                "remainder_share": 0.1,
                "maximum_mode_share": 0.05,
                "remainder_influence_concentration": 0.02,
                "floor_share": 0.0,
                "boundary_share": 0.01,
            }
            print(json.dumps(row, sort_keys=True))
""",
        encoding="utf-8",
    )
    path.chmod(0o755)


def test_smoke_manifest_task_and_aggregate_are_complete(tmp_path: Path) -> None:
    manifest_path = tmp_path / "manifest.json"
    manifest = MODULE.create_manifest(ROOT, "smoke", manifest_path, None)
    assert manifest["task_count"] == 1
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    tasks = tmp_path / "tasks"
    MODULE.run_task(manifest_path, 1, tasks, binary)
    receipt = MODULE.aggregate(manifest_path, tasks, tmp_path / "aggregate")
    assert receipt["status"] == "COMPLETE"
    assert receipt["row_count"] == 8
    assert len(receipt["summaries"]) == 4


def test_aggregate_is_shard_invariant(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    aggregates = []
    for label, shard_size in (("whole", 2), ("split", 1)):
        root = tmp_path / label
        manifest_path = root / "manifest.json"
        manifest = MODULE.create_manifest(ROOT, "smoke", manifest_path, shard_size)
        for task in manifest["tasks"]:
            MODULE.run_task(manifest_path, task["task_id"], root / "tasks", binary)
        MODULE.aggregate(manifest_path, root / "tasks", root / "aggregate")
        aggregates.append((root / "aggregate" / "aggregate.jsonl").read_bytes())
    assert aggregates[0] == aggregates[1]


def test_task_rejects_duplicate_semantic_key(tmp_path: Path) -> None:
    manifest_path = tmp_path / "manifest.json"
    manifest = MODULE.create_manifest(ROOT, "smoke", manifest_path, None)
    task = manifest["tasks"][0]
    row = {
        "schema": MODULE.ROW_SCHEMA,
        "cell": "dominant_leverage",
        "k": 8,
        "replication": 0,
        "variance_source": "oracle",
        "status": "q1_failed",
    }
    output = tmp_path / "duplicate.jsonl"
    output.write_text(json.dumps(row) + "\n" + json.dumps(row) + "\n")
    with pytest.raises(MODULE.CampaignError, match="duplicate key"):
        MODULE._task_rows(output, task)


def test_aggregate_reports_every_missing_task(tmp_path: Path) -> None:
    manifest_path = tmp_path / "manifest.json"
    MODULE.create_manifest(ROOT, "smoke", manifest_path, None)
    with pytest.raises(MODULE.CampaignError, match="task 1"):
        MODULE.aggregate(manifest_path, tmp_path / "missing", tmp_path / "aggregate")


def test_confirmation_manifest_requires_clean_source(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(
        MODULE,
        "_git_identity",
        lambda _root: {
            "commit": "a" * 40,
            "dirty": True,
            "status": [" M example"],
            "worktree_diff_sha256": "b" * 64,
        },
    )
    with pytest.raises(MODULE.CampaignError, match="clean source"):
        MODULE.create_manifest(ROOT, "confirmation", tmp_path / "manifest.json", 100)


def test_scientific_gate_enumerates_independent_failures() -> None:
    summary = {
        "successes": 100,
        "success_rate": 0.8,
        "bias": 0.2,
        "bias_mcse": 0.01,
        "coverage": 0.8,
        "coverage_mcse": 0.01,
        "se_ratio": 1.2,
    }
    failures = MODULE._gate("cell", summary)
    assert failures == [
        "cell: success rate",
        "cell: bias",
        "cell: coverage",
        "cell: standard-error ratio",
    ]


def test_scientific_gate_rejects_zero_successes_without_crashing() -> None:
    summary = MODULE._summary(
        [{"status": "q1_failed"}, {"status": "variance_fit_failed"}], 2
    )
    assert summary["successes"] == 0
    assert summary["bias"] is None
    assert summary["empirical_sd"] is None
    assert summary["failure_counts"] == {"q1_failed": 1, "variance_fit_failed": 1}
    assert MODULE._gate("cell", summary) == [
        "cell: success rate",
        "cell: insufficient successful replications",
    ]
