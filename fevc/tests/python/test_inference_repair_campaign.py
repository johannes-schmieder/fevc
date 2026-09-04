from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "fevc" / "tools" / "run_inference_repair_campaign.py"
SPEC = importlib.util.spec_from_file_location("repair_match_campaign", SCRIPT)
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
    bad_preflight: str | None = None,
    bad_q1_covariance: bool = False,
    fail_cell: str | None = None,
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
    spec = cells[cell]
    for target in targets:
        if spec["target_shape"] == "one_mode":
            leading = 0.90 if target != "covariance" else 0.50
            remainder = 0.08 if target != "covariance" else 0.80
        elif spec["target_shape"] == "multi_mode":
            leading, remainder = 0.72, 0.70
        else:
            leading = 0.18 if target == "covariance" else 0.06
            remainder = 0.20 if target == "covariance" else 0.07
        if {bad_preflight!r} == "one_mode" and cell == "one_mode_equal_independent" and target == "worker":
            remainder = 0.9
        if {bad_preflight!r} == "multi_mode" and cell == "multi_mode_diagnostic" and target == "total":
            remainder = 0.1
        unequal = spec["match_mass"] == "highly_unequal"
        print(json.dumps({{
            "schema": "fevc-match-inference-q1-design-preflight-v1",
            "cell": cell,
            "k": k,
            "target": target,
            "target_shape": spec["target_shape"],
            "reference_distribution": spec["reference_distribution"],
            "coverage_eligible": target in spec["coverage_targets"],
            "independent_matches": k * k,
            "effective_match_count": k * k * (0.85 if unequal else 1.0),
            "largest_match_mass_share": 0.02 if unequal else 1.0 / (k * k),
            "largest_match_leverage": 0.2,
            "smallest_maker_denominator": 0.7,
            "leading_eigenvalue": 0.01,
            "second_eigenvalue": 0.001,
            "leading_share": leading,
            "remainder_share": remainder,
            "maximum_mode_weight": 0.05,
            "maximum_influence_share": 0.04,
            "leading_residual": 1e-8,
            "second_residual": 2e-8,
            "nuisance_uncertainty_conditioned_away": True,
        }}, sort_keys=True))
    raise SystemExit(0)

assert mode in ("repair-development-v1", "repair-confirmation-v1")
start, count = map(int, rest[:2])
critical_simulations = int(rest[-1])
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
    seed = splitmix64((0xb73490d26a18ce55 if mode == "repair-confirmation-v1" else 0x4d617463685131a1) ^ splitmix64(label_hash(cell)) ^ splitmix64(k) ^ splitmix64(replication))
    for target in targets:
        if {partial!r} and replication == start and target == "total":
            continue
        spec = cells[cell]
        metadata = {{name: value for name, value in spec.items() if name != "coverage_targets"}}
        if {fail_cell!r} == cell:
            row = {{
                "schema": "fevc-inference-repair-match-row-v1",
                **metadata,
                "k": k,
                "replication": replication,
                "target": target,
                "semantic_seed": seed,
                "coverage_eligible": target in spec["coverage_targets"],
                "status": "backend_failure",
                "error_code": "JLA_CONSTRAINT_FAILED",
                "error_phase": "component_inference_q1",
            }}
        else:
            is_q1 = spec["reference_distribution"] == "q1"
            row = {{
                "schema": "fevc-inference-repair-match-row-v1",
                **metadata,
                "k": k,
                "replication": replication,
                "target": target,
                "semantic_seed": seed,
                "coverage_eligible": target in spec["coverage_targets"],
                "status": "success",
                "truth": 0.5,
                "point_estimate": 0.5 + (-0.1 if replication % 2 else 0.1),
                "point_error": -0.1 if replication % 2 else 0.1,
                "estimated_variance": 0.01,
                "estimated_sd": 0.1,
                "confidence_lower": 0.3,
                "confidence_upper": 0.7,
                "covered": True,
                "lower_miss": False,
                "upper_miss": False,
                "independent_matches": k * k,
                "effective_match_count": 0.9 * k * k,
                "largest_match_mass_share": 0.02,
                "largest_match_leverage": 0.2,
                "smallest_maker_denominator": 0.7,
                "maximum_influence_share": 0.04,
                "leading_eigenvalue": 0.01,
                "second_eigenvalue": -0.001 if target == "covariance" else 0.001,
                "leading_share": 0.9 if is_q1 else 0.06,
                "remainder_share": 0.08 if target != "covariance" else 0.8,
                "maximum_mode_weight": 0.05,
                "leading_residual": 1e-8,
                "second_residual": 2e-8,
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
                "q1_diagnostics_applicable": is_q1,
                "critical_simulations": critical_simulations if is_q1 else 0,
                "leading_score": 0.4 if is_q1 else None,
                "raw_leading_variance_correction": 0.02 if is_q1 else None,
                "leading_variance": (-0.02 if {bad_q1_covariance!r} else 0.02) if is_q1 else None,
                "leading_recentered_component": 0.14 if is_q1 else None,
                "remainder_estimate": 0.46 if is_q1 else None,
                "remainder_identity_error": 1e-14 if is_q1 else None,
                "leading_remainder_covariance": 0.001 if is_q1 else None,
                "remainder_variance": 0.03 if is_q1 else None,
                "remainder_trace_mcse": 1e-5 if is_q1 else None,
                "curvature": 2 * 0.01 * 0.02 / (0.03 * (1 - 0.001**2 / (0.02 * 0.03)))**0.5 if is_q1 else None,
                "critical_value": 2.2 if is_q1 else None,
                "leading_f_statistic": 8.0 if is_q1 else None,
                "remainder_influence_concentration": 0.03 if is_q1 else None,
                "q1_point_identity_error": 1e-14 if is_q1 else None,
                "q1_status": 0 if is_q1 else None,
                "quadrature_critical": is_q1,
                "production_critical_value": 2.2 if is_q1 else None,
                "standardized_determinant": 1 - 0.001**2 / (0.02 * 0.03) if is_q1 else None,
                "remainder_influence_variance": 0.04 if is_q1 else None,
                "remainder_trace_variance": 0.01 if is_q1 else None,
                "outer_fold_fingerprint": 12345,
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
    assert MODULE._semantic_seed("one_mode_equal_independent", 20, 0) == 4_156_217_649_202_129_127


def test_preflight_validates_target_specific_regimes(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    receipt = MODULE.run_preflight(ROOT, "tiny", tmp_path / "preflight", binary)
    assert receipt["status"] == "PASS"
    assert receipt["row_count"] == 4 * len(MODULE.CELLS)
    assert receipt["preflight_spectrum_probes"] == 4_096


@pytest.mark.parametrize("regime", ("one_mode", "multi_mode"))
def test_preflight_rejects_named_but_unrealized_regime(tmp_path: Path, regime: str) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary, bad_preflight=regime)
    with pytest.raises(MODULE.CampaignError, match="regime|concentrated remainder"):
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


def test_tiny_pipeline_is_complete_atomic_and_target_specific(tmp_path: Path) -> None:
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
    rows = [json.loads(line) for line in (tmp_path / "aggregate" / "aggregate.jsonl").read_text().splitlines()]
    covariance = next(row for row in rows if row["cell"] == "one_mode_equal_independent" and row["target"] == "covariance")
    comparator = next(row for row in rows if row["cell"] == "diffuse_q0_comparator")
    assert not covariance["coverage_eligible"]
    assert covariance["q1_diagnostics_applicable"]
    assert comparator["reference_distribution"] == "q0"
    assert not comparator["q1_diagnostics_applicable"]


def test_aggregate_is_shard_invariant(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    outputs = []
    for label, shard in (("whole", 2), ("split", 1)):
        directory = tmp_path / label
        manifest_path, manifest = _manifest(directory, binary, "smoke", shard)
        for task in manifest["tasks"]:
            MODULE.run_task(manifest_path, task["task_id"], directory / "tasks", binary)
        MODULE.aggregate(manifest_path, directory / "tasks", directory / "aggregate")
        outputs.append((directory / "aggregate" / "aggregate.jsonl").read_bytes())
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


def test_task_rejects_invalid_q1_covariance(tmp_path: Path) -> None:
    good = tmp_path / "good.py"
    _fake_binary(good)
    manifest_path, _ = _manifest(tmp_path, good)
    bad = tmp_path / "bad.py"
    _fake_binary(bad, bad_q1_covariance=True)
    with pytest.raises(MODULE.CampaignError, match="q1 covariance"):
        MODULE.run_task(manifest_path, 1, tmp_path / "tasks", bad)


def test_aggregate_enumerates_every_missing_task(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    manifest_path, manifest = _manifest(tmp_path, binary, "tiny")
    with pytest.raises(MODULE.CampaignError) as error:
        MODULE.aggregate(manifest_path, tmp_path / "tasks", tmp_path / "aggregate")
    message = str(error.value)
    assert "task 1:" in message
    assert f"task {manifest['task_count']}:" in message


def test_aggregate_rejects_mixed_source_receipt(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    manifest_path, _ = _manifest(tmp_path, binary, "smoke")
    MODULE.run_task(manifest_path, 1, tmp_path / "tasks", binary)
    receipt_path = tmp_path / "tasks" / "task-00001.receipt.json"
    receipt = json.loads(receipt_path.read_text())
    receipt["source"]["commit"] = "e" * 40
    receipt_path.write_text(json.dumps(receipt))
    with pytest.raises(MODULE.CampaignError, match="does not bind q1 task output"):
        MODULE.aggregate(manifest_path, tmp_path / "tasks", tmp_path / "aggregate")


def test_local_aggregate_rejects_binary_different_from_preflight(tmp_path: Path) -> None:
    preflight_binary = tmp_path / "preflight.py"
    _fake_binary(preflight_binary)
    manifest_path, _ = _manifest(tmp_path, preflight_binary, "smoke")
    task_binary = tmp_path / "task.py"
    _fake_binary(task_binary)
    task_binary.write_text(task_binary.read_text() + "\n")
    task_binary.chmod(0o755)
    MODULE.run_task(manifest_path, 1, tmp_path / "tasks", task_binary)
    with pytest.raises(MODULE.CampaignError, match="preflight binary"):
        MODULE.aggregate(manifest_path, tmp_path / "tasks", tmp_path / "aggregate")


def test_aggregate_rejects_build_receipt_for_another_source(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    manifest_path, manifest = _manifest(tmp_path, binary, "smoke")
    MODULE.run_task(manifest_path, 1, tmp_path / "tasks", binary)
    build = tmp_path / "build.json"
    build.write_text(json.dumps({
        "schema": MODULE.BUILD_RECEIPT_SCHEMA,
        "status": "success",
        "binary_sha256": MODULE._binary_hash(binary),
        "source_commit": "f" * 40,
    }))
    assert manifest["source"]["commit"] != "f" * 40
    with pytest.raises(MODULE.CampaignError, match="does not bind manifest source"):
        MODULE.aggregate(manifest_path, tmp_path / "tasks", tmp_path / "aggregate", build)


def test_typed_failures_are_counted_without_success_conditioning(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary, fail_cell="controls_varying_fixedoffset")
    manifest_path, manifest = _manifest(tmp_path, binary, "smoke")
    MODULE.run_task(manifest_path, 1, tmp_path / "tasks", binary)
    receipt = MODULE.aggregate(manifest_path, tmp_path / "tasks", tmp_path / "aggregate")
    assert receipt["row_count"] == manifest["expected_row_count"] == 8
    assert all(row["attempts"] == 2 and row["successes"] == 0 for row in receipt["summaries"])
    assert all(sum(row["failure_counts"].values()) == 2 for row in receipt["summaries"])


def test_smoke_manifest_rejects_dirty_source(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    preflight = _preflight(tmp_path, binary)
    identity = MODULE._git_identity(ROOT)
    identity["dirty"] = True
    monkeypatch.setattr(MODULE, "_git_identity", lambda _root: identity)
    with pytest.raises(MODULE.CampaignError, match="clean source tree"):
        MODULE.create_manifest(ROOT, "smoke", tmp_path / "manifest.json", preflight, None)


def test_manifest_rejects_overlapping_task_ranges(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    _, manifest = _manifest(tmp_path, binary, "smoke", 1)
    manifest["tasks"][1]["replication_start"] = 0
    with pytest.raises(MODULE.CampaignError, match="overlapping"):
        MODULE._validate_manifest(manifest)


def _good_development_summaries() -> list[dict]:
    rows = []
    for spec in MODULE.CELLS:
        for target in MODULE.TARGETS:
            rows.append({
                **spec,
                "target": target,
                "attempts": 400,
                "successes": 400,
                "success_rate": 1.0,
                "bias": 0.0,
                "bias_mcse": 0.01,
                "coverage": 0.95,
                "coverage_mcse": 0.011,
                "se_ratio": 1.0,
            })
    for row in rows:
        if row["cell"] == "one_mode_severe_omitted" and row["target"] == "worker":
            row["coverage"] = 0.8
            row["se_ratio"] = 0.7
    return rows


def test_development_gate_excludes_multimode_covariance_but_not_eligible_targets() -> None:
    rows = _good_development_summaries()
    covariance = next(row for row in rows if row["cell"] == "one_mode_equal_independent" and row["target"] == "covariance")
    covariance["coverage"] = 0.0
    covariance["se_ratio"] = 9.0
    failures, _ = MODULE._scientific_failures(rows, "development")
    assert not any("one_mode_equal_independent/covariance" in failure for failure in failures)
    worker = next(row for row in rows if row["cell"] == "one_mode_equal_independent" and row["target"] == "worker")
    worker["coverage"] = 0.0
    failures, _ = MODULE._scientific_failures(rows, "development")
    assert any("one_mode_equal_independent/worker: coverage" == failure for failure in failures)


def test_repository_registration_is_bound(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(MODULE, "_registration_identity", ORIGINAL_REGISTRATION_IDENTITY)
    identity = MODULE._registration_identity(ROOT)
    assert identity["schema"] == MODULE.REGISTRATION_SCHEMA
    assert len(identity["sha256"]) == 64


def test_partial_target_is_counted_without_discarding_points_or_other_targets(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    manifest_path, manifest = _manifest(tmp_path, binary, "smoke")
    MODULE.run_task(manifest_path, 1, tmp_path / "tasks", binary)
    rows = [json.loads(line) for line in (tmp_path / "tasks" / "task-00001.jsonl").read_text().splitlines()]
    row = next(row for row in rows if row["target"] == "covariance")
    row.update(status="target_unavailable", q1_status=1, remainder_variance=-0.01,
               remainder_influence_variance=0.0, critical_simulations=0,
               confidence_lower=None, confidence_upper=None, critical_value=None,
               production_critical_value=None, curvature=None, standardized_determinant=None,
               covered=False, lower_miss=False, upper_miss=False)
    task = manifest["tasks"][0]
    spec = MODULE._cell_spec(manifest, task)
    MODULE._validate_row(row, task, spec, 4000)
    summary = MODULE._summary([candidate for candidate in rows if candidate["target"] == "covariance"], 2)
    assert summary["point_estimates"] == 2
    assert summary["successes"] == summary["coverage_denominator"] == 1
    assert summary["success_rate"] == 0.5
    assert summary["failure_counts"] == {"target_unavailable:1:component_inference_q1": 1}
    assert MODULE._summary([candidate for candidate in rows if candidate["target"] == "worker"], 2)["success_rate"] == 1
    row["confidence_lower"] = 0.0
    with pytest.raises(MODULE.CampaignError, match="unavailable target has an interval"):
        MODULE._validate_row(row, task, spec, 4000)


def test_confirmation_freezes_independent_seed_and_inventory(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    _, manifest = _manifest(tmp_path, binary, "confirmation")
    assert manifest["scientific_contract"]["master_seed"] != MODULE.MASTER_SEED
    assert manifest["expected_row_count"] == 14 * 2500 * 4
    manifest["tasks"][0]["output"] = "../escape.jsonl"
    with pytest.raises(MODULE.CampaignError, match="metadata"):
        MODULE._validate_manifest(manifest)


def test_aggregate_rejects_outcome_dependent_folds(tmp_path: Path) -> None:
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    manifest_path, _ = _manifest(tmp_path, binary, "smoke")
    MODULE.run_task(manifest_path, 1, tmp_path / "tasks", binary)
    path = tmp_path / "tasks" / "task-00001.jsonl"
    rows = [json.loads(line) for line in path.read_text().splitlines()]
    rows[-1]["outer_fold_fingerprint"] += 1
    payload = b"".join(json.dumps(row).encode() + b"\n" for row in rows)
    path.write_bytes(payload)
    receipt_path = path.with_suffix(".receipt.json")
    receipt = json.loads(receipt_path.read_text())
    receipt["output_sha256"] = MODULE._sha256(payload)
    receipt_path.write_text(json.dumps(receipt))
    with pytest.raises(MODULE.CampaignError, match="folds changed"):
        MODULE.aggregate(manifest_path, tmp_path / "tasks", tmp_path / "aggregate")
