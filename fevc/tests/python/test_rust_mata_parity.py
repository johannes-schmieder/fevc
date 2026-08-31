from __future__ import annotations

import importlib.util
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
