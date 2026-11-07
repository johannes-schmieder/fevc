from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "fevc/tools/render_rust_mata_parity.py"
SPEC = importlib.util.spec_from_file_location("render_rust_mata_parity", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def test_generated_parity_matrix_is_current() -> None:
    document = MODULE.load()
    assert MODULE.OUTPUT.read_text(encoding="utf-8") == MODULE.render(document)


def test_every_alpha_gap_is_explicit() -> None:
    rows = MODULE.load()["rows"]
    gaps = {
        row["id"]
        for row in rows
        if row["alpha_required"] and row["rust"] != "qualified"
    }
    assert gaps == set()
    windows = next(row for row in rows if row["id"] == "windows")
    assert windows["mata"] == windows["rust"] == "qualified"
    record = "native/cmg-consolidation-20261106/manifest.json"
    assert record in windows["evidence"]
    manifest_path = REPO_ROOT / record
    adoption = json.loads(manifest_path.read_text(encoding="utf-8"))
    evidence = adoption["windows_qualification"]
    payload = (manifest_path.parent / evidence["path"]).read_bytes()
    assert hashlib.sha256(payload).hexdigest() == evidence["sha256"]
    checks = json.loads(payload)
    assert checks["status"] == "PASS" and checks["runtime_profile"] == "full"
    assert set(checks["gates"].values()) <= {"PASS", "HOSTED_CI_PASS"}
    assert checks["production_commit"] == adoption["build_source_commit"]
    binary = next(row for row in adoption["binaries"] if "windows" in row["name"])
    assert checks["binary_sha256"] == binary["sha256"] == hashlib.sha256(
        (REPO_ROOT / "fevc" / binary["name"]).read_bytes()
    ).hexdigest()
    # The previous adoption remains an unqualified historical exception.
    previous = json.loads((REPO_ROOT / "native/pooled-component-20261010/windows-manual-adoption.json").read_text())
    assert previous["automated_private_runtime"] == "NOT_RUN"
    assert previous["owner_manual_runtime_test"] == "PENDING"
