from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "fevc" / "tools" / "run_q1_reference_diagnostic.py"
SPEC = importlib.util.spec_from_file_location("q1_reference_diagnostic", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def _fake_binary(path: Path, mode: str = "valid") -> None:
    path.write_text(
        f"""#!/usr/bin/env python3
import json
import sys

MODE = {mode!r}
MASK = (1 << 64) - 1
OUTCOME = {{"calibration": 0x6F9A31C2074D85E1, "evaluation": 0xBD427619A05E3CF8}}
REFERENCE = {{"calibration": 0x293E8CB754A1F602, "evaluation": 0xE51740AD9B6328C4}}

def splitmix(value):
    value = (value + 0x9E3779B97F4A7C15) & MASK
    value = ((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9) & MASK
    value = ((value ^ (value >> 27)) * 0x94D049BB133111EB) & MASK
    return (value ^ (value >> 31)) & MASK

def label_hash(value):
    state = 0xCBF29CE484222325
    for byte in value.encode():
        state = ((state ^ byte) * 0x100000001B3) & MASK
    return state

def semantic(master, label, k, replication):
    return splitmix(master ^ splitmix(label_hash(label)) ^ splitmix(k) ^ splitmix(replication))

_, command, sample, k, start, count = sys.argv
assert command == "diagnostic-q1-v4"
k = int(k)
start = int(start)
count = int(count)
for replication in range(start, start + count):
    for error_dgp in ("gaussian", "student_t8"):
        seed = semantic(OUTCOME[sample], "dominant_common_v4", k, replication)
        row = {{
            "schema": "fevc-q1-reference-diagnostic-v4",
            "kind": "outcome",
            "sample": sample,
            "error_dgp": error_dgp,
            "k": k,
            "replication": replication,
            "semantic_seed": seed,
            "status": "success",
            "truth": 0.0,
            "point_error": (-1.0 if replication % 2 else 1.0) * 0.1,
            "score_error": (-1.0 if replication % 2 else 1.0) * 0.2,
            "remainder_error": (-1.0 if replication % 2 else 1.0) * 0.1,
            "leading_variance_estimated": 1.0,
            "leading_variance_population": 1.0,
            "remainder_variance_estimated": 0.4,
            "remainder_variance_population": 0.4,
            "cross_covariance_estimated": 0.0,
            "cross_covariance_population": 0.0,
            "full_variance_estimated": 0.5,
            "full_variance_population": 0.5,
            "leading_variance_correction": 0.9,
            "remainder_identity_error": 1e-14,
            "leading_share": 0.8,
            "remainder_share": 0.1,
            "maximum_mode_share": 0.05,
            "maximum_full_influence_share": 0.03,
            "maximum_remainder_influence_share": 0.02,
            "required_radius_population": 1.0,
        }}
        if sample == "evaluation":
            for variant in {MODULE.VARIANTS!r}:
                row.update({{
                    f"{{variant}}_lower": -0.3,
                    f"{{variant}}_upper": 0.3,
                    f"{{variant}}_width": 0.6,
                    f"{{variant}}_critical": 2.0,
                    f"{{variant}}_covered": True,
                    f"{{variant}}_lower_miss": False,
                    f"{{variant}}_upper_miss": False,
                }})
        if MODE == "mixed_seed" and error_dgp == "student_t8" and replication == start:
            row["semantic_seed"] += 1
        if MODE == "malformed" and error_dgp == "gaussian" and replication == start:
            row["point_error"] = None
        if MODE == "failure" and error_dgp == "student_t8":
            row = {{key: row[key] for key in ("schema", "kind", "sample", "error_dgp", "k", "replication", "semantic_seed")}}
            row["status"] = "q1_failed"
        print(json.dumps(row, sort_keys=True))
        if MODE == "duplicate" and error_dgp == "gaussian" and replication == start:
            print(json.dumps(row, sort_keys=True))
    if MODE != "missing" or replication != start:
        seed = semantic(REFERENCE[sample], "q1_reference_v4", k, replication)
        row = {{
            "schema": "fevc-q1-reference-diagnostic-v4",
            "kind": "reference",
            "sample": sample,
            "error_dgp": "gaussian_reference",
            "k": k,
            "replication": replication,
            "semantic_seed": seed,
            "status": "success",
            "truth": 0.0,
            "score_error": (-1.0 if replication % 2 else 1.0) * 0.2,
            "remainder_error": (-1.0 if replication % 2 else 1.0) * 0.1,
            "leading_variance_population": 1.0,
            "remainder_variance_population": 0.4,
            "cross_covariance_population": 0.0,
            "curvature_population": 0.2,
            "theoretical_critical": 2.0,
            "required_radius_population": 1.0,
        }}
        if sample == "evaluation":
            row.update({{
                "reference_q1_lower": -0.3,
                "reference_q1_upper": 0.3,
                "reference_q1_width": 0.6,
                "reference_q1_critical": 2.0,
                "reference_q1_covered": True,
                "reference_q1_lower_miss": False,
                "reference_q1_upper_miss": False,
            }})
        print(json.dumps(row, sort_keys=True))
""",
        encoding="utf-8",
    )
    path.chmod(0o755)


@pytest.fixture(autouse=True)
def registration_identity(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(
        MODULE,
        "_registration_identity",
        lambda _root: {"path": str(MODULE.REGISTRATION), "sha256": "a" * 64},
    )


def _run_tiny(tmp_path: Path, binary: Path, shard_size: int = 2) -> tuple[Path, dict]:
    manifest_path = tmp_path / "manifest.json"
    manifest = MODULE.create_manifest(ROOT, "tiny", manifest_path, shard_size)
    tasks = tmp_path / "tasks"
    for task in manifest["tasks"]:
        MODULE.run_task(manifest_path, task["task_id"], tasks, binary)
    return manifest_path, manifest


def test_tiny_generator_task_validator_aggregate_and_receipt(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    manifest_path, manifest = _run_tiny(tmp_path, binary)
    assert manifest["task_count"] == 2
    receipt = MODULE.aggregate(manifest_path, tmp_path / "tasks", tmp_path / "aggregate")
    assert receipt["status"] == "COMPLETE"
    assert receipt["row_count"] == 12
    assert set(receipt["aggregate_output_sha256"]) == {
        "diagnostics.json",
        "coverage.csv",
        "distribution.csv",
        "required_radius_cdf.csv",
        "required_radius_cdf.svg",
        "leading_qq.svg",
        "remainder_qq.svg",
        "coverage_by_variant.svg",
    }


def test_aggregate_is_shard_invariant(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    outputs = []
    for label, shard in (("whole", 2), ("split", 1)):
        root = tmp_path / label
        manifest_path, _ = _run_tiny(root, binary, shard)
        MODULE.aggregate(manifest_path, root / "tasks", root / "aggregate")
        outputs.append((root / "aggregate" / "coverage.csv").read_bytes())
    assert outputs[0] == outputs[1]


@pytest.mark.parametrize(
    ("mode", "message"),
    [
        ("duplicate", "duplicate key"),
        ("missing", "inventory mismatch"),
        ("mixed_seed", "mixed or incorrect semantic seed"),
        ("malformed", "nonfinite or missing point_error"),
    ],
)
def test_task_rejects_malformed_or_incomplete_output(
    tmp_path: Path, mode: str, message: str
) -> None:
    binary = tmp_path / f"{mode}.py"
    _fake_binary(binary, mode)
    manifest_path = tmp_path / "manifest.json"
    MODULE.create_manifest(ROOT, "tiny", manifest_path, 2)
    with pytest.raises(MODULE.DiagnosticError, match=message):
        MODULE.run_task(manifest_path, 1, tmp_path / "tasks", binary)


def test_aggregate_rejects_partial_task_inventory(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    manifest_path = tmp_path / "manifest.json"
    MODULE.create_manifest(ROOT, "tiny", manifest_path, 2)
    MODULE.run_task(manifest_path, 1, tmp_path / "tasks", binary)
    with pytest.raises(MODULE.DiagnosticError, match="task 2"):
        MODULE.aggregate(manifest_path, tmp_path / "tasks", tmp_path / "aggregate")


def test_aggregate_rejects_deliberate_scientific_failures(tmp_path: Path) -> None:
    binary = tmp_path / "failure.py"
    _fake_binary(binary, "failure")
    manifest_path, _ = _run_tiny(tmp_path, binary)
    with pytest.raises(MODULE.DiagnosticError, match="success rate below V4 minimum"):
        MODULE.aggregate(manifest_path, tmp_path / "tasks", tmp_path / "aggregate")


def test_calibration_evaluation_and_reference_rng_domains_are_distinct() -> None:
    outcome_calibration = MODULE._semantic_seed(
        MODULE.OUTCOME_SEEDS["calibration"], "dominant_common_v4", 64, 7
    )
    outcome_evaluation = MODULE._semantic_seed(
        MODULE.OUTCOME_SEEDS["evaluation"], "dominant_common_v4", 64, 7
    )
    reference_evaluation = MODULE._semantic_seed(
        MODULE.REFERENCE_SEEDS["evaluation"], "q1_reference_v4", 64, 7
    )
    assert len({outcome_calibration, outcome_evaluation, reference_evaluation}) == 3


def test_manifest_rejects_dirty_scc_source(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(
        MODULE,
        "_git_identity",
        lambda _root: {
            "commit": "b" * 40,
            "dirty": True,
            "status": [" M example"],
            "worktree_diff_sha256": "c" * 64,
            "source_file_count": 1,
            "source_manifest_sha256": "d" * 64,
        },
    )
    with pytest.raises(MODULE.DiagnosticError, match="clean committed source"):
        MODULE.create_manifest(ROOT, "smoke", tmp_path / "manifest.json", None)
