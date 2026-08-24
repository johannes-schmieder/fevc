from __future__ import annotations

import importlib.util
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "vckss/tools/check_legacy_names.py"
SPEC = importlib.util.spec_from_file_location("legacy_name_audit", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def test_relocation_inventory_is_complete_and_hash_bound() -> None:
    frozen, errors = MODULE.verify_inventory(MODULE.git_candidates())
    assert not errors
    assert len(frozen) > 1_600

    relocation = MODULE.load_v2_inventory()
    predecessor = "varcomp" + "_kss"
    public_ado = relocation[f"{predecessor}/{predecessor}.ado"]
    assert public_ado["new_path"] == "vckss/vckss.ado"
    assert public_ado["byte_identity_required"] is False

    historical = relocation[
        f"{predecessor}/qualification/rename_equivalence/receipt.json"
    ]
    assert historical["new_path"] == "vckss/qualification/rename_equivalence/receipt.json"
    assert historical["byte_identity_required"] is True

    # latest.json is a mutable convenience pointer. Per-SHA receipts are the
    # source-bound evidence and remain byte-locked by the inventory.
    latest = relocation[".ci/stata/latest.json"]
    assert latest["new_path"] == ".ci/stata/latest.json"
    assert latest["byte_identity_required"] is False


def test_active_occurrence_requires_an_exact_exception() -> None:
    errors: list[str] = []
    findings = MODULE.check_occurrences(
        "active/example.txt", "content", MODULE.TOKENS[0], errors
    )
    assert findings == 1
    assert len(errors) == 1
    assert "allowed maximum is 0" in errors[0]


def test_exceptions_are_exactly_count_bounded() -> None:
    for relative in (
        MODULE.SELF_REL,
        MODULE.V2_INVENTORY_REL,
        MODULE.V2_EQ_DRIVER_REL,
        MODULE.V2_EQ_RUNNER_REL,
        MODULE.V2_EQ_BASELINE_MANIFEST_REL,
        MODULE.V2_EQ_BASELINE_JSON_REL,
        MODULE.V2_EQ_BASELINE_TSV_REL,
        MODULE.V2_EQ_RECEIPT_REL,
    ):
        text = (REPO_ROOT / relative).read_text(encoding="utf-8")
        for token in MODULE.TOKENS:
            count = text.count(token)
            maximum = MODULE.EXCEPTIONS.get((relative, token, "content"), 0)
            assert maximum == count
            errors: list[str] = []
            MODULE.check_occurrences(
                relative, "content", text + token, errors
            )
            assert len(errors) == 1
            assert f"allowed maximum is {maximum}" in errors[0]


def test_relocated_frozen_hash_mismatch_fails(
    tmp_path: Path, monkeypatch
) -> None:
    original = "archive/frozen.txt"
    relocated = "vckss/archive/frozen.txt"
    frozen_path = tmp_path / relocated
    frozen_path.parent.mkdir(parents=True)
    frozen_path.write_bytes(b"changed frozen evidence\n")

    record = {
        "original_path": original,
        "new_path": relocated,
        "sha256": "0" * 64,
        "size": frozen_path.stat().st_size,
        "byte_identity_required": True,
    }
    monkeypatch.setattr(MODULE, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(MODULE, "load_v1_inventory", lambda: {})
    monkeypatch.setattr(MODULE, "load_v2_inventory", lambda: {original: record})
    monkeypatch.setattr(MODULE, "git_tree_paths", lambda _commit: {original})

    frozen, errors = MODULE.verify_inventory([relocated])
    assert frozen == {relocated}
    assert any("relocated frozen hash mismatch" in error for error in errors)
