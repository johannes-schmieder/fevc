from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[3]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


MODULE = load("rc_observation_tests", ROOT / "fevc/tools/run_rc_observation_inference.py")
HELPERS = load("rc_observation_fixtures", Path(__file__).with_name("test_structured_inference_confirmation.py"))
REGISTRATION = MODULE._registration_identity


@pytest.fixture(autouse=True)
def identities(monkeypatch):
    monkeypatch.setattr(MODULE, "_registration_identity", lambda _: {
        "path": MODULE.REGISTRATION_PATH.as_posix(), "schema": MODULE.REGISTRATION_SCHEMA,
        "sha256": "c" * 64,
    })
    monkeypatch.setattr(MODULE, "_git_identity", lambda _: {
        "commit": "a" * 40, "dirty": False, "status": [], "source_file_count": 100,
        "worktree_diff_sha256": "b" * 64, "source_manifest_sha256": "d" * 64,
    })


def fake_binary(path, **kwargs):
    HELPERS._fake_binary(path, **kwargs)
    source = path.read_text().replace(
        'assert mode == "confirmation-matrix-v5"',
        'assert mode in ("confirmation-matrix-v5", "pipeline-matrix-rc-v1")')
    source = source.replace("0x7a93d4c152e86b0f", "(0xc247f9083ae16d5b if mode == 'confirmation-matrix-v5' else 0x9816ac5d72b0f3e4)")
    source = source.replace("fevc-structured-inference-confirmation-v5", MODULE.ROW_SCHEMA)
    source = source.replace('"status": "success",', '"status": "success", "outer_fold_fingerprint": 12345, "leading_eigenvalue": 0.01, "curvature": 0.02 / (0.2 ** 0.5),')
    preflight = []
    for cell in MODULE.ALL_CELLS:
        for target in MODULE.TARGETS:
            dominant = cell["dominant"]
            preflight.append({
                "schema": "fevc-rc-observation-preflight-v1", "cell": cell["cell"],
                "k": cell["k"], "target": target, "observations": 200,
                "parameters": 30, "maker_minimum": 0.8, "variance_minimum": 0.2,
                "mean_leading_share": (0.98 if target == "covariance" else 0.8) if dominant else (0.09 if cell["k"] == 12 else 0.07),
                "mean_remainder_share": 0.6 if dominant and target == "covariance" else 0.1,
                "variance_fold_seed": 8675309,
            })
    front = f"if sys.argv[1] == 'design-preflight-rc-v1':\n    for row in {preflight!r}: print(json.dumps(row))\n    raise SystemExit(0)\n"
    source = source.replace("_, mode, cell, k, start, count = sys.argv", front + "_, mode, cell, k, start, count = sys.argv")
    path.write_text(source)


def manifest(tmp_path, binary, profile="tiny", shard_size=None):
    MODULE.run_preflight(ROOT, tmp_path / "preflight", binary)
    path = tmp_path / "manifest.json"
    return path, MODULE.create_manifest(ROOT, profile, path, shard_size, tmp_path / "preflight/receipt.json")


def test_complete_tiny_and_reverse_sharding(tmp_path):
    binary = tmp_path / "fake.py"
    fake_binary(binary)
    outputs = []
    for label, shard in (("forward", 2), ("reverse", 1)):
        path, value = manifest(tmp_path / label, binary, shard_size=shard)
        assert value["expected_row_count"] == 160
        assert value["scientific_contract"]["outcome_seed_hex"] == hex(MODULE.PIPELINE_SEED)
        for task in reversed(value["tasks"]):
            MODULE.run_task(path, task["task_id"], tmp_path / label / "tasks", binary)
        result = MODULE.aggregate(path, tmp_path / label / "tasks", tmp_path / label / "aggregate")
        assert result["row_count"] == 160 and result["status"] == "COMPLETE"
        outputs.append((tmp_path / label / "aggregate/aggregate.jsonl").read_bytes())
    assert outputs[0] == outputs[1]


@pytest.mark.parametrize("keyword, message", [("duplicate", "duplicate"), ("partial", "inventory"), ("malformed", "JSON")])
def test_bad_rows_rejected(tmp_path, keyword, message):
    binary = tmp_path / "fake.py"
    fake_binary(binary, **{keyword: True})
    path, _ = manifest(tmp_path, binary, "smoke")
    with pytest.raises(MODULE.ConfirmationError, match=message):
        MODULE.run_task(path, 1, tmp_path / "tasks", binary)


@pytest.mark.parametrize("field, value", [("outcome_seed_hex", "0x1"), ("thresholds", {}), ("cells", []), ("variance_fold_seed", 1)])
def test_tampered_contract_rejected(tmp_path, field, value):
    binary = tmp_path / "fake.py"
    fake_binary(binary)
    path, result = manifest(tmp_path, binary, "smoke")
    result["scientific_contract"][field] = value
    path.write_text(json.dumps(result))
    with pytest.raises(MODULE.ConfirmationError):
        MODULE._read_manifest(path)


@pytest.mark.parametrize("change", ["fold", "curvature", "source", "missing"])
def test_aggregation_and_row_guards(tmp_path, change):
    binary = tmp_path / "fake.py"
    fake_binary(binary)
    path, value = manifest(tmp_path, binary, "smoke")
    MODULE.run_task(path, 1, tmp_path / "tasks", binary)
    raw = tmp_path / "tasks/task-00001.jsonl"
    rows = [json.loads(line) for line in raw.read_text().splitlines()]
    if change == "curvature":
        rows[0]["curvature"] *= 2
        with pytest.raises(MODULE.ConfirmationError, match="curvature"):
            MODULE._validate_row(rows[0], value["tasks"][0], MODULE._cell_spec(value, value["tasks"][0]))
        return
    receipt_path = raw.with_suffix(".receipt.json")
    receipt = json.loads(receipt_path.read_text())
    if change == "fold":
        rows[-1]["outer_fold_fingerprint"] += 1
        payload = b"".join(json.dumps(row).encode() + b"\n" for row in rows)
        raw.write_bytes(payload)
        receipt["output_sha256"] = MODULE._sha256(payload)
    elif change == "source":
        receipt["source"]["commit"] = "e" * 40
    else:
        raw.unlink()
    receipt_path.write_text(json.dumps(receipt))
    with pytest.raises(MODULE.ConfirmationError):
        MODULE.aggregate(path, tmp_path / "tasks", tmp_path / "aggregate")


def test_failure_accounting_and_unchanged_thresholds():
    result = MODULE._summary([{"status": "variance_fit_failed"}, {"status": "q1_failed"}], 2)
    assert result["attempts"] == 2 and result["successes"] == 0
    assert sum(result["failure_counts"].values()) == 2
    assert MODULE.THRESHOLDS == HELPERS.MODULE.THRESHOLDS
    assert MODULE.ALL_CELLS == HELPERS.MODULE.ALL_CELLS


def test_registration_real_source():
    assert REGISTRATION(ROOT)["schema"] == MODULE.REGISTRATION_SCHEMA


def test_confirmation_inventory_and_dirty_source(tmp_path, monkeypatch):
    binary = tmp_path / "fake.py"
    fake_binary(binary)
    path, result = manifest(tmp_path, binary, "confirmation")
    assert result["task_count"] == 100 and result["expected_row_count"] == 200000
    assert result["scientific_contract"]["outcome_seed_hex"] == hex(MODULE.V5_CONFIRMATION_SEED)
    assert MODULE.V5_CONFIRMATION_SEED not in {MODULE.PIPELINE_SEED, 0x7A93D4C152E86B0F}
    result["tasks"][0]["output"] = "../escape.jsonl"
    path.write_text(json.dumps(result))
    with pytest.raises(MODULE.ConfirmationError, match="noncanonical"):
        MODULE._read_manifest(path)
    identity = MODULE._git_identity(ROOT)
    identity["dirty"] = True
    monkeypatch.setattr(MODULE, "_git_identity", lambda _: identity)
    with pytest.raises(MODULE.ConfirmationError, match="clean source"):
        MODULE.create_manifest(ROOT, "confirmation", tmp_path / "bad.json", None, tmp_path / "preflight/receipt.json")


def test_all_scientific_failures_are_enumerated():
    rows = []
    for cell in MODULE.ALL_CELLS:
        for target in MODULE.TARGETS:
            rows.append({**cell, "target": target, "success_rate": 1.0, "successes": 2500,
                         "bias": 0.0, "bias_mcse": 0.01, "coverage_mcse": 0.004,
                         "coverage": 0.0 if cell["gate"] == "correct" else 0.8 if cell["gate"] == "descriptive" else 0.95,
                         "se_ratio": 1.0, "mean_leading_share": (0.98 if target == "covariance" else 0.8) if cell["dominant"] else (0.09 if cell["k"] == 12 else 0.07),
                         "mean_remainder_share": 0.6 if cell["dominant"] and target == "covariance" else 0.1})
    failures, _ = MODULE._scientific_failures(rows, "confirmation")
    assert len([failure for failure in failures if failure.endswith(": coverage")]) == 39


@pytest.mark.parametrize("status, code", [("PASS", 0), ("COMPLETE", 0), ("FAIL", 1)])
def test_scientific_cli_exit(monkeypatch, status, code):
    monkeypatch.setattr(MODULE, "aggregate", lambda *args: {"status": status})
    assert MODULE.main(["aggregate", "manifest.json", "tasks", "output"]) == code
