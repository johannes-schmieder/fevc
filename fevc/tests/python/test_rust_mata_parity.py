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
    record = "native/windows-cmg-20261107/manifest.json"
    assert record in windows["evidence"]
    manifest_path = REPO_ROOT / record
    adoption = json.loads(manifest_path.read_text(encoding="utf-8"))
    sys.path.insert(0, str(REPO_ROOT / "fevc/tools"))
    import build_github_release

    # Current qualification requires every registered runtime and comparison
    # group; the previous single full-profile receipt cannot qualify 0.5.2.
    result = build_github_release.require_windows_cmg_qualification(
        REPO_ROOT, manifest_path, adoption
    )
    assert result["status"] == "PASS" and result["groups"] == 18
    assert len(result["cells"]) == 12
    assert all(cell["science"] == cell["performance"] == "PASS"
               for cell in result["cells"])
    binary = next(row for row in adoption["binaries"] if "windows" in row["name"])
    assert binary["sha256"] == hashlib.sha256(
        (REPO_ROOT / "fevc" / binary["name"]).read_bytes()
    ).hexdigest()
    # The previous adoption remains an unqualified historical exception.
    previous = json.loads((REPO_ROOT / "native/pooled-component-20261010/windows-manual-adoption.json").read_text())
    assert previous["automated_private_runtime"] == "NOT_RUN"
    assert previous["owner_manual_runtime_test"] == "PENDING"
