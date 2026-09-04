from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "fevc" / "tools" / "run_match_inference_q0_campaign.py"
SPEC = importlib.util.spec_from_file_location("match_q0_campaign", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)
ORIGINAL_REGISTRATION_IDENTITY = MODULE._registration_identity


@pytest.fixture(autouse=True)
def frozen_identities(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(
        MODULE,
        "_registration_identity",
        lambda _root: {
            "path": MODULE.REGISTRATION_PATH.as_posix(),
            "schema": MODULE.REGISTRATION_SCHEMA,
            "sha256": "c" * 64,
        },
    )
    monkeypatch.setattr(
        MODULE,
        "_git_identity",
        lambda _root: {
            "commit": "a" * 40,
            "dirty": False,
            "status": [],
            "worktree_diff_sha256": "b" * 64,
            "source_file_count": 100,
            "source_manifest_sha256": "d" * 64,
        },
    )


def _fake_binary(
    path: Path,
    *,
    duplicate: bool = False,
    partial: bool = False,
    malformed: bool = False,
    bad_preflight: bool = False,
) -> None:
    cells = json.dumps(MODULE.CELL_BY_NAME, sort_keys=True)
    path.write_text(
        f"""#!/usr/bin/env python3
import json
import sys

cells = json.loads({cells!r})
mode, cell, k, *rest = sys.argv[1:]
k = int(k)
targets = ("worker", "firm", "covariance", "total")
if mode == "design-preflight-v1":
    for target in targets:
        if cell == "one_mode_diagnostic":
            leading = 0.85 if target != "covariance" else 0.49
            remainder = 0.08 if target != "covariance" else 0.90
        elif cell == "multi_mode_diagnostic":
            leading, remainder = 0.85, 0.80
        else:
            leading = 0.18 if target == "covariance" else 0.06
            remainder = 0.20 if target == "covariance" else 0.07
        if {bad_preflight!r} and cell == "diffuse_equal_independent" and target == "worker":
            leading = 0.9
        unequal = cells[cell]["match_mass"] == "highly_unequal"
        print(json.dumps({{
            "schema": "fevc-match-inference-q0-design-preflight-v1",
            "cell": cell,
            "k": k,
            "target": target,
            "target_shape": cells[cell]["target_shape"],
            "independent_matches": k * k,
            "effective_match_count": k * k * (0.85 if unequal else 1.0),
            "largest_match_mass_share": 0.02 if unequal else 1.0 / (k * k),
            "largest_match_leverage": 0.2,
            "smallest_maker_denominator": 0.7,
            "leading_share": leading,
            "remainder_share": remainder,
            "maximum_mode_weight": 0.05,
            "maximum_influence_share": 0.04,
            "nuisance_uncertainty_conditioned_away": True,
        }}, sort_keys=True))
    raise SystemExit(0)

assert mode == "q0-matrix-v1"
start, count = map(int, rest[:2])
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
    seed = splitmix64(0x98d3407af6512cbe ^ splitmix64(label_hash(cell)) ^ splitmix64(k) ^ splitmix64(replication))
    for target in targets:
        if {partial!r} and replication == start and target == "total":
            continue
        row = {{
            "schema": "fevc-match-inference-q0-development-row-v1",
            **cells[cell],
            "k": k,
            "replication": replication,
            "target": target,
            "semantic_seed": seed,
            "status": "success",
            "truth": 0.5,
            "point_estimate": 0.5 + (-0.1 if replication % 2 else 0.1),
            "point_error": -0.1 if replication % 2 else 0.1,
            "estimated_variance": 0.01,
            "estimated_sd": 0.1,
            "covered": True,
            "lower_miss": False,
            "upper_miss": False,
            "independent_matches": k * k,
            "effective_match_count": 0.9 * k * k,
            "largest_match_mass_share": 0.02,
            "largest_match_leverage": 0.2,
            "smallest_maker_denominator": 0.7,
            "maximum_influence_share": 0.04,
            "leading_share": 0.06,
            "remainder_share": 0.07,
            "maximum_mode_weight": 0.05,
            "leading_residual": 1e-8,
            "minimum_active_terms": 12,
            "minimum_training_matches": 200,
            "variance_floor_count": 1,
            "variance_floor_share": 0.0025,
            "boundary_share": 0.01,
            "maximum_boundary_excess": 0.01,
            "maximum_prediction_leverage": 0.2,
            "minimum_fitted_rcond": 0.01,
            "sensitivity_median_log_ratio": 0.02,
            "sensitivity_p90_log_ratio": 0.05,
            "sensitivity_maximum_log_ratio": 0.1,
            "sensitivity_log_variance_correlation": 0.95,
            "maximum_complete_residual": 1e-12,
            "full_residual_tolerance": 1e-10,
            "maximum_trace_mcse": 1e-5,
            "psd_cleanup": 0.0,
            "smallest_covariance_eigenvalue": 1e-6,
            "largest_covariance_eigenvalue": 0.02,
            "point_correction_identity_error": 1e-14,
            "aggregate_variance_mean": 0.2,
            "nuisance_uncertainty_conditioned_away": True,
        }}
        print("{{bad json" if {malformed!r} else json.dumps(row, sort_keys=True))
        if {duplicate!r} and replication == start and target == "worker":
            print(json.dumps(row, sort_keys=True))
""",
        encoding="utf-8",
    )
    path.chmod(0o755)


def _preflight(tmp_path: Path, binary: Path, profile: str = "smoke") -> Path:
    directory = tmp_path / f"{profile}-preflight"
    MODULE.run_preflight(ROOT, profile, directory, binary)
    return directory / "receipt.json"


def _manifest(tmp_path: Path, binary: Path, profile: str = "smoke", shard_size: int | None = None) -> tuple[Path, dict]:
    preflight = _preflight(tmp_path, binary, profile)
    path = tmp_path / f"{profile}-manifest.json"
    return path, MODULE.create_manifest(ROOT, profile, path, preflight, shard_size)


def test_semantic_seed_matches_rust() -> None:
    assert MODULE._semantic_seed("diffuse_equal_independent", 14, 0) == 14_199_910_461_300_949_147


def test_preflight_validates_target_specific_regimes(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    receipt = MODULE.run_preflight(ROOT, "tiny", tmp_path / "preflight", binary)
    assert receipt["status"] == "PASS"
    assert receipt["row_count"] == 4 * len(MODULE.CELLS)


def test_preflight_rejects_named_but_unrealized_diffuse_design(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary, bad_preflight=True)
    with pytest.raises(MODULE.CampaignError, match="leading concentration"):
        MODULE.run_preflight(ROOT, "tiny", tmp_path / "preflight", binary)
    assert not (tmp_path / "preflight" / "receipt.json").exists()


def test_manifest_rejects_tampered_preflight_payload(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    receipt = _preflight(tmp_path, binary)
    payload = receipt.with_name("preflight.jsonl")
    payload.write_bytes(payload.read_bytes() + b"\n")
    with pytest.raises(MODULE.CampaignError, match="does not match its receipt"):
        MODULE.create_manifest(ROOT, "smoke", tmp_path / "manifest.json", receipt, None)


def test_tiny_pipeline_is_complete_and_atomic(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    manifest_path, manifest = _manifest(tmp_path, binary, "tiny")
    assert manifest["task_count"] == len(MODULE.CELLS)
    assert manifest["expected_row_count"] == 4 * len(MODULE.CELLS)
    for task in manifest["tasks"]:
        MODULE.run_task(manifest_path, task["task_id"], tmp_path / "tasks", binary)
    receipt = MODULE.aggregate(manifest_path, tmp_path / "tasks", tmp_path / "aggregate")
    assert receipt["status"] == "COMPLETE"
    assert receipt["row_count"] == manifest["expected_row_count"]
    assert len(receipt["summaries"]) == 4 * len(MODULE.CELLS)


def test_aggregate_is_shard_invariant(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    outputs = []
    for label, shard in (("whole", 2), ("split", 1)):
        root = tmp_path / label
        manifest_path, manifest = _manifest(root, binary, "smoke", shard)
        for task in manifest["tasks"]:
            MODULE.run_task(manifest_path, task["task_id"], root / "tasks", binary)
        MODULE.aggregate(manifest_path, root / "tasks", root / "aggregate")
        outputs.append((root / "aggregate" / "aggregate.jsonl").read_bytes())
    assert outputs[0] == outputs[1]


@pytest.mark.parametrize(
    ("keyword", "expected"),
    (("duplicate", "duplicate key"), ("partial", "inventory mismatch"), ("malformed", "non-JSON")),
)
def test_task_rejects_malformed_duplicate_and_partial_output(tmp_path: Path, keyword: str, expected: str) -> None:
    good = tmp_path / "good.py"
    _fake_binary(good)
    manifest_path, _ = _manifest(tmp_path, good)
    bad = tmp_path / f"{keyword}.py"
    _fake_binary(bad, **{keyword: True})
    with pytest.raises(MODULE.CampaignError, match=expected):
        MODULE.run_task(manifest_path, 1, tmp_path / "tasks", bad)
    assert not (tmp_path / "tasks" / "task-00001.receipt.json").exists()


def test_aggregate_enumerates_all_missing_tasks(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    manifest_path, _ = _manifest(tmp_path, binary, "tiny")
    with pytest.raises(MODULE.CampaignError) as error:
        MODULE.aggregate(manifest_path, tmp_path / "missing", tmp_path / "aggregate")
    message = str(error.value)
    assert "task 1" in message and f"task {len(MODULE.CELLS)}" in message


def test_aggregate_rejects_mixed_source_receipt(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    manifest_path, manifest = _manifest(tmp_path, binary)
    MODULE.run_task(manifest_path, 1, tmp_path / "tasks", binary)
    receipt_path = tmp_path / "tasks" / manifest["tasks"][0]["receipt"]
    receipt = json.loads(receipt_path.read_text())
    receipt["source"]["commit"] = "e" * 40
    receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
    with pytest.raises(MODULE.CampaignError, match="task 1"):
        MODULE.aggregate(manifest_path, tmp_path / "tasks", tmp_path / "aggregate")


def test_cross_platform_task_binary_requires_matching_build_receipt(tmp_path: Path) -> None:
    preflight_binary = tmp_path / "preflight.py"
    _fake_binary(preflight_binary)
    manifest_path, manifest = _manifest(tmp_path, preflight_binary)
    task_binary = tmp_path / "task.py"
    _fake_binary(task_binary)
    task_binary.write_text(task_binary.read_text() + "\n# platform-specific build\n", encoding="utf-8")
    task_binary.chmod(0o755)
    MODULE.run_task(manifest_path, 1, tmp_path / "tasks", task_binary)
    with pytest.raises(MODULE.CampaignError, match="local tasks do not use the preflight binary"):
        MODULE.aggregate(manifest_path, tmp_path / "tasks", tmp_path / "aggregate-local")
    build = tmp_path / "build.json"
    build.write_text(
        json.dumps(
            {
                "schema": MODULE.BUILD_RECEIPT_SCHEMA,
                "status": "success",
                "binary_sha256": MODULE._binary_hash(task_binary),
                "source_commit": manifest["source"]["commit"],
            }
        ),
        encoding="utf-8",
    )
    receipt = MODULE.aggregate(
        manifest_path,
        tmp_path / "tasks",
        tmp_path / "aggregate-scc",
        build,
    )
    assert receipt["status"] == "COMPLETE"
    assert receipt["binary_sha256"] == MODULE._binary_hash(task_binary)


def test_manifest_rejects_overlap(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    manifest_path, manifest = _manifest(tmp_path, binary, "smoke", 1)
    manifest["tasks"][1]["replication_start"] = 0
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    with pytest.raises(MODULE.CampaignError, match="overlapping|partial"):
        MODULE._read_manifest(manifest_path)


def test_smoke_manifest_requires_clean_source(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    preflight = _preflight(tmp_path, binary)
    monkeypatch.setattr(
        MODULE,
        "_git_identity",
        lambda _root: {
            "commit": "a" * 40,
            "dirty": True,
            "status": [" M example"],
            "worktree_diff_sha256": "b" * 64,
            "source_file_count": 100,
            "source_manifest_sha256": "d" * 64,
        },
    )
    with pytest.raises(MODULE.CampaignError, match="clean source"):
        MODULE.create_manifest(ROOT, "smoke", tmp_path / "manifest.json", preflight, None)


def test_named_failures_are_counted_by_type() -> None:
    summary = MODULE._summary(
        [
            {"status": "backend_failure", "error_code": "SINGULAR_INFORMATION", "error_phase": "structured_variance"},
            {"status": "backend_failure", "error_code": "JLA_CONSTRAINT_FAILED", "error_phase": "component_inference_psd"},
        ],
        2,
    )
    assert summary["success_rate"] == 0.0
    assert summary["failure_counts"] == {
        "backend_failure:JLA_CONSTRAINT_FAILED:component_inference_psd": 1,
        "backend_failure:SINGULAR_INFORMATION:structured_variance": 1,
    }


def test_scientific_gate_rejects_independent_failures() -> None:
    summary = {
        "successes": 400,
        "success_rate": 0.9,
        "bias": 0.2,
        "bias_mcse": 0.01,
        "coverage": 0.8,
        "coverage_mcse": 0.01,
        "se_ratio": 1.3,
    }
    assert MODULE._gate_correct("cell", summary) == [
        "cell: success rate",
        "cell: bias",
        "cell: coverage",
        "cell: standard-error ratio",
    ]


def test_registration_rejects_changed_frozen_file(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    root = tmp_path / "root"
    registration_path = root / MODULE.REGISTRATION_PATH
    registration_path.parent.mkdir(parents=True)
    frozen = root / "frozen.txt"
    frozen.write_text("changed", encoding="utf-8")
    registration_path.write_text(
        json.dumps(
            {
                "schema": MODULE.REGISTRATION_SCHEMA,
                "status": "DEVELOPMENT_REGISTERED",
                "source_binding": {"frozen_file_sha256": {"frozen.txt": "0" * 64}},
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(MODULE, "_registration_identity", ORIGINAL_REGISTRATION_IDENTITY)
    with pytest.raises(MODULE.CampaignError, match="frozen file hash"):
        MODULE._registration_identity(root)
