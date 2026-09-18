from __future__ import annotations

import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[3]


def _load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


ADAPTER = _load("test_rc_q0_adapter", ROOT / "fevc/tools/run_rc_match_q0.py")
MODULE = ADAPTER.CAMPAIGN
HELPERS = _load("rc_q0_fixture_helpers", Path(__file__).with_name("test_inference_repair_campaign.py"))
HELPERS.MODULE = MODULE
_original_fake_binary = HELPERS._fake_binary


@pytest.fixture(autouse=True)
def identities(monkeypatch):
    monkeypatch.setattr(MODULE, "_registration_identity", lambda _: {
        "path": MODULE.REGISTRATION_PATH.as_posix(),
        "schema": MODULE.REGISTRATION_SCHEMA, "sha256": "c" * 64,
    })
    monkeypatch.setattr(MODULE, "_git_identity", lambda _: {
        "commit": "a" * 40, "dirty": False, "status": [],
        "worktree_diff_sha256": "b" * 64, "source_file_count": 100,
        "source_manifest_sha256": "d" * 64,
    })


def _fake_binary(path, **kwargs):
    _original_fake_binary(path, **kwargs)
    text = path.read_text().replace("0xb73490d26a18ce55", "0xeca18f437d60b295")
    text = text.replace("0x4d617463685131a1", "0x73bd6a80912fc4e5")
    text = text.replace("fevc-inference-repair-match-row-v1", MODULE.ROW_SCHEMA)
    path.write_text(text)


def test_full_tiny_pipeline_and_independent_confirmation_inventory(tmp_path):
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    path, manifest = HELPERS._manifest(tmp_path, binary, "tiny")
    assert manifest["expected_row_count"] == 112
    for task in reversed(manifest["tasks"]):
        MODULE.run_task(path, task["task_id"], tmp_path / "tasks", binary)
    receipt = MODULE.aggregate(path, tmp_path / "tasks", tmp_path / "aggregate")
    assert receipt["status"] == "COMPLETE"
    assert receipt["row_count"] == 112
    with pytest.raises(MODULE.CampaignError, match="replace existing"):
        MODULE.aggregate(path, tmp_path / "tasks", tmp_path / "aggregate")
    _, confirmation = HELPERS._manifest(tmp_path, binary, "confirmation")
    assert confirmation["task_count"] == 70
    assert confirmation["expected_row_count"] == 140000
    assert confirmation["scientific_contract"]["master_seed"] not in {
        MODULE.MASTER_SEED, 0x98D3407AF6512CBE, 0xB73490D26A18CE55,
    }
    assert sum(len(cell["coverage_targets"]) for cell in MODULE.CELLS if cell["gate"] == "correct") == 24


@pytest.mark.parametrize("keyword, expected", [
    ("duplicate", "duplicate key"), ("partial", "inventory mismatch"),
    ("malformed", "non-JSON"),
])
def test_bad_task_output_rejected(tmp_path, keyword, expected):
    binary = tmp_path / "fake.py"
    _fake_binary(binary, **{keyword: True})
    path, _ = HELPERS._manifest(tmp_path, binary)
    with pytest.raises(MODULE.CampaignError, match=expected):
        MODULE.run_task(path, 1, tmp_path / "tasks", binary)


def test_scientific_gate_enumerates_all_failed_correct_targets():
    rows = HELPERS._good_development_summaries()
    for row in rows:
        if row["gate"] == "correct":
            row["coverage"] = 0.0
        if row["cell"] == "diffuse_severe_omitted":
            row["coverage"] = 0.8
    failures, _ = MODULE._scientific_failures(rows, "confirmation")
    assert len([failure for failure in failures if failure.endswith(": coverage")]) == 24
    assert not MODULE._scientific_failures(rows, "tiny")[0]


@pytest.mark.parametrize("test_name", [
    "test_manifest_rejects_tampered_preflight_payload",
    "test_aggregate_is_shard_invariant",
    "test_aggregate_enumerates_every_missing_task",
    "test_aggregate_rejects_mixed_source_receipt",
    "test_local_aggregate_rejects_binary_different_from_preflight",
    "test_aggregate_rejects_build_receipt_for_another_source",
    "test_typed_failures_are_counted_without_success_conditioning",
    "test_manifest_rejects_overlapping_task_ranges",
    "test_aggregate_rejects_outcome_dependent_folds",
])
def test_inherited_boundary(tmp_path, monkeypatch, test_name):
    monkeypatch.setattr(HELPERS, "_fake_binary", _fake_binary)
    getattr(HELPERS, test_name)(tmp_path)


def test_dirty_confirmation_rejected(tmp_path, monkeypatch):
    binary = tmp_path / "fake.py"
    _fake_binary(binary)
    preflight = HELPERS._preflight(tmp_path, binary, "confirmation")
    identity = MODULE._git_identity(ROOT)
    identity["dirty"] = True
    monkeypatch.setattr(MODULE, "_git_identity", lambda _: identity)
    with pytest.raises(MODULE.CampaignError, match="clean source tree"):
        MODULE.create_manifest(ROOT, "confirmation", tmp_path / "manifest.json", preflight, None)


CONFIRMATION_SOURCE = "eedc2d662022c887994c6943ee76fc70ac756cd5"  # Review-purged equivalent of bb580fe69085d1f98c9151cea2de038aec0a8ba6


def _confirmed_file(relative):
    return subprocess.check_output(
        ["git", "show", f"{CONFIRMATION_SOURCE}:{relative}"], cwd=ROOT,
    )


def test_registration_binds_original_confirmation_source(tmp_path):
    # The completed registration stays immutable. Later test-only maintenance
    # must not silently admit a new source for another confirmation run.
    registration = json.loads((ROOT / MODULE.REGISTRATION_PATH).read_text())
    for relative in [MODULE.REGISTRATION_PATH.as_posix(),
                     *registration["source_binding"]["frozen_file_sha256"]]:
        destination = tmp_path / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(_confirmed_file(relative))
    identity = ADAPTER._registration_identity(tmp_path)
    assert identity["schema"] == "FEVC_RC_MATCH_Q0_V1"


def test_unregistered_current_source_cannot_relaunch_confirmation():
    with pytest.raises(MODULE.CampaignError, match="frozen hash mismatch"):
        ADAPTER._registration_identity(ROOT)


def test_post_confirmation_repair_changes_only_rust_test_module():
    relative = "rust/crates/vckss-core/examples/rc_match_q0.rs"
    pattern = r"    #\[cfg\(test\)\]\n    mod q[01]_tests \{.*?\n    \}\n(?=\})"
    original, original_count = re.subn(
        pattern, "", _confirmed_file(relative).decode(), flags=re.DOTALL,
    )
    current, current_count = re.subn(
        pattern, "", (ROOT / relative).read_text(), flags=re.DOTALL,
    )
    assert original_count == current_count == 1
    assert current == original


@pytest.mark.parametrize("status, exit_code", [("PASS", 0), ("COMPLETE", 0), ("FAIL", 1)])
def test_cli_scientific_failure_exits_nonzero(monkeypatch, status, exit_code):
    monkeypatch.setattr(MODULE, "aggregate", lambda *args: {"status": status})
    assert ADAPTER.main(["aggregate", "manifest.json", "tasks", "output"]) == exit_code
