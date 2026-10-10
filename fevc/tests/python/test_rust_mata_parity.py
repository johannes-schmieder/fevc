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
    # Repository adoption is owner-authorized before Windows runtime testing;
    # do not let the ledger silently turn that exception into qualification.
    assert gaps == {"windows"}
    windows = next(row for row in rows if row["id"] == "windows")
    assert windows["mata"] == windows["rust"] == "pending"
    record = "native/pooled-component-20261010/windows-manual-adoption.json"
    assert record in windows["evidence"]
    adoption = json.loads((REPO_ROOT / record).read_text(encoding="utf-8"))
    assert adoption["status"] == "ADOPTED_FOR_OWNER_MANUAL_TEST_RUNTIME_UNQUALIFIED"
    assert adoption["automated_private_runtime"] == "NOT_RUN"
    assert adoption["owner_manual_runtime_test"] == "PENDING"
    assert adoption["owner_authorization"]
    assert adoption["sha256"] == hashlib.sha256(
        (REPO_ROOT / adoption["path"]).read_bytes()
    ).hexdigest()
