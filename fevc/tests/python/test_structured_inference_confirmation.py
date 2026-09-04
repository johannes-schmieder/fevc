from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "fevc" / "tools" / "run_structured_inference_confirmation.py"
SPEC = importlib.util.spec_from_file_location("structured_confirmation", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


@pytest.fixture(autouse=True)
def registration(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(
        MODULE,
        "_registration_identity",
        lambda _root: {
            "path": MODULE.REGISTRATION_PATH.as_posix(),
            "schema": MODULE.REGISTRATION_SCHEMA,
            "sha256": "c" * 64,
        },
    )


def _fake_binary(path: Path, *, duplicate: bool = False, partial: bool = False, malformed: bool = False) -> None:
    path.write_text(
        f"""#!/usr/bin/env python3
import json
import sys

_, mode, cell, k, start, count = sys.argv
assert mode == "confirmation-matrix-v5"
k = int(k)
start = int(start)
count = int(count)
dominant = cell.startswith("dominant")
reference = "Q0" if not dominant or cell.endswith("_q0") else "Q1"
gate = "mild" if "mild" in cell else "descriptive" if "severe" in cell else "diagnostic" if "null" in cell else "correct"
model = "structured_leverage" if "leverage" in cell else "structured_common"
dgp = "SevereOmitted" if "severe" in cell else "MildOmitted" if "mild_omitted" in cell else "MildFunctional" if "mild_functional" in cell else "Leverage" if "leverage" in cell else "Homoskedastic" if "homoskedastic" in cell else "Common"
error = "StudentT8" if cell.endswith("_t8") else "Gaussian"
beta = "zero" if "null" in cell else "nonzero"
targets = ("worker", "firm", "covariance", "total")
mask = (1 << 64) - 1
def splitmix64(value):
    value = (value + 0x9e3779b97f4a7c15) & mask
    value = ((value ^ (value >> 30)) * 0xbf58476d1ce4e5b9) & mask
    value = ((value ^ (value >> 27)) * 0x94d049bb133111eb) & mask
    return (value ^ (value >> 31)) & mask
def label_hash(value):
    state = 0xcbf29ce484222325
    for byte in value.encode():
        state = ((state ^ byte) * 0x1000000001b3) & mask
    return state
for replication in range(start, start + count):
    semantic_seed = splitmix64(0x7a93d4c152e86b0f ^ splitmix64(label_hash(cell)) ^ splitmix64(k) ^ splitmix64(replication))
    for target in targets:
        if {partial!r} and replication == start and target == "total":
            continue
        if dominant and target in ("worker", "firm"):
            leading, remainder = 0.80, 0.10
        elif dominant and target == "covariance":
            leading, remainder = 0.98, 0.60
        elif target in ("worker", "firm"):
            leading, remainder = (0.09 if k == 12 else 0.07), 0.10
        elif target == "covariance":
            leading, remainder = 0.15, 0.20
        else:
            leading, remainder = (0.05 if k == 12 else 0.04), 0.05
        row = {{
            "schema": "fevc-structured-inference-confirmation-v5",
            "cell": cell,
            "gate": gate,
            "k": k,
            "controls": "controls" in cell,
            "dominant": dominant,
            "variance_model": model,
            "variance_dgp": dgp,
            "error_dgp": error,
            "reference": reference,
            "beta": beta,
            "replication": replication,
            "target": target,
            "semantic_seed": semantic_seed,
            "status": "success",
            "point_error": -0.1 if replication % 2 else 0.1,
            "estimated_sd": 0.1,
            "covered": True,
            "lower_miss": False,
            "upper_miss": False,
            "interval_width": 0.4,
            "leading_variance": 1.0,
            "remainder_variance": 0.2,
            "leading_remainder_covariance": 0.0,
            "remainder_identity_error": 1.0e-14,
            "critical": 2.0,
            "leading_share": leading,
            "remainder_share": remainder,
            "floor_share": 0.0,
            "boundary_share": 0.01,
            "maximum_boundary_excess": 0.0,
            "maximum_prediction_leverage": 0.2,
            "minimum_fitted_rcond": 0.01,
        }}
        print("{{bad json" if {malformed!r} else json.dumps(row, sort_keys=True))
        if {duplicate!r} and replication == start and target == "worker":
            print(json.dumps(row, sort_keys=True))
""",
        encoding="utf-8",
    )
    path.chmod(0o755)


def _manifest(tmp_path: Path, profile: str = "smoke", shard_size: int | None = None) -> tuple[Path, dict]:
    path = tmp_path / "manifest.json"
    return path, MODULE.create_manifest(ROOT, profile, path, shard_size)


def test_tiny_manifest_covers_the_complete_80_row_matrix(tmp_path: Path) -> None:
    path, manifest = _manifest(tmp_path, "tiny")
    assert path.is_file()
    assert manifest["task_count"] == 20
    assert manifest["expected_row_count"] == 160
    assert len(manifest["scientific_contract"]["cells"]) == 20


def test_semantic_seed_vectors_match_rust() -> None:
    assert MODULE._semantic_seed("diffuse_common", 12, 0) == 6_180_164_651_401_935_216
    assert MODULE._semantic_seed("dominant_common_t8", 16, 2_499) == 2_983_714_797_850_544_688


def test_smoke_pipeline_is_complete_and_atomic(tmp_path: Path) -> None:
    manifest_path, manifest = _manifest(tmp_path)
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    for task in manifest["tasks"]:
        MODULE.run_task(manifest_path, task["task_id"], tmp_path / "tasks", binary)
    receipt = MODULE.aggregate(manifest_path, tmp_path / "tasks", tmp_path / "aggregate")
    assert receipt["status"] == "COMPLETE"
    assert receipt["row_count"] == manifest["expected_row_count"]
    assert len(receipt["summaries"]) == 4 * len(manifest["scientific_contract"]["cells"])
    assert (tmp_path / "aggregate" / "summaries.csv").is_file()


def test_aggregate_is_shard_invariant(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    outputs = []
    for label, shard in (("whole", 4), ("split", 2)):
        root = tmp_path / label
        manifest_path = root / "manifest.json"
        manifest = MODULE.create_manifest(ROOT, "smoke", manifest_path, shard)
        for task in manifest["tasks"]:
            MODULE.run_task(manifest_path, task["task_id"], root / "tasks", binary)
        MODULE.aggregate(manifest_path, root / "tasks", root / "aggregate")
        outputs.append((root / "aggregate" / "aggregate.jsonl").read_bytes())
    assert outputs[0] == outputs[1]


@pytest.mark.parametrize(
    ("keyword", "expected"),
    (("duplicate", "duplicate key"), ("partial", "inventory mismatch"), ("malformed", "non-JSON")),
)
def test_task_rejects_bad_output(tmp_path: Path, keyword: str, expected: str) -> None:
    manifest_path, _ = _manifest(tmp_path)
    binary = tmp_path / "fake.py"
    _fake_binary(binary, **{keyword: True})
    with pytest.raises(MODULE.ConfirmationError, match=expected):
        MODULE.run_task(manifest_path, 1, tmp_path / "tasks", binary)
    assert not (tmp_path / "tasks" / "task-00001.receipt.json").exists()


def test_aggregate_enumerates_every_missing_task(tmp_path: Path) -> None:
    manifest_path, _ = _manifest(tmp_path)
    with pytest.raises(MODULE.ConfirmationError) as error:
        MODULE.aggregate(manifest_path, tmp_path / "missing", tmp_path / "aggregate")
    message = str(error.value)
    assert "task 1" in message and "task 7" in message


def test_aggregate_rejects_mixed_source_receipt(tmp_path: Path) -> None:
    manifest_path, manifest = _manifest(tmp_path)
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    for task in manifest["tasks"]:
        MODULE.run_task(manifest_path, task["task_id"], tmp_path / "tasks", binary)
    receipt_path = tmp_path / "tasks" / manifest["tasks"][0]["receipt"]
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    receipt["source"]["commit"] = "d" * 40
    receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
    with pytest.raises(MODULE.ConfirmationError, match="task 1"):
        MODULE.aggregate(manifest_path, tmp_path / "tasks", tmp_path / "aggregate")


def test_manifest_rejects_overlapping_ranges(tmp_path: Path) -> None:
    manifest_path, manifest = _manifest(tmp_path, "smoke", 2)
    manifest["tasks"][1]["replication_start"] = 0
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    with pytest.raises(MODULE.ConfirmationError, match="overlapping|partial"):
        MODULE._read_manifest(manifest_path)


def test_confirmation_manifest_requires_clean_source(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(
        MODULE,
        "_git_identity",
        lambda _root: {
            "commit": "a" * 40,
            "dirty": True,
            "status": [" M example"],
            "worktree_diff_sha256": "b" * 64,
            "source_file_count": 1,
            "source_manifest_sha256": "e" * 64,
        },
    )
    with pytest.raises(MODULE.ConfirmationError, match="clean source"):
        MODULE.create_manifest(ROOT, "confirmation", tmp_path / "manifest.json", None)


def test_named_failures_are_counted_and_fail_common_gate() -> None:
    summary = MODULE._summary(
        [{"status": "variance_fit_failed"}, {"status": "q1_failed"}], 2
    )
    assert summary["failure_counts"] == {"variance_fit_failed": 1, "q1_failed": 1}
    assert MODULE._gate_correct("cell", summary) == [
        "cell: success rate",
        "cell: insufficient successful replications",
    ]


def test_registration_rejects_changed_frozen_file(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.undo()
    root = tmp_path / "root"
    registration_path = root / MODULE.REGISTRATION_PATH
    registration_path.parent.mkdir(parents=True)
    frozen = root / "frozen.txt"
    frozen.write_text("changed", encoding="utf-8")
    registration_path.write_text(
        json.dumps(
            {
                "schema": MODULE.REGISTRATION_SCHEMA,
                "status": "CONFIRMATION_REGISTERED",
                "source_binding": {"frozen_file_sha256": {"frozen.txt": "0" * 64}},
            }
        ),
        encoding="utf-8",
    )
    with pytest.raises(MODULE.ConfirmationError, match="frozen file hash"):
        MODULE._registration_identity(root)
