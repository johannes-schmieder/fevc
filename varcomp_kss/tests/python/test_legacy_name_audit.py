from __future__ import annotations

import importlib.util
import json
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "varcomp_kss/tools/check_legacy_names.py"
SPEC = importlib.util.spec_from_file_location("legacy_name_audit", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def test_frozen_inventory_is_complete_and_hash_bound() -> None:
    frozen, errors = MODULE.verify_inventory(MODULE.git_candidates())
    assert not errors
    assert len(frozen) > 1_000


def test_active_occurrence_requires_an_exact_exception() -> None:
    errors: list[str] = []
    findings = MODULE.check_occurrences(
        "active/example.txt", "content", MODULE.TOKENS[0], errors
    )
    assert findings == 1
    assert len(errors) == 1
    assert "allowed maximum is 0" in errors[0]


def test_exception_is_count_bounded() -> None:
    token = MODULE.TOKENS[0]
    maximum = MODULE.EXCEPTIONS[(MODULE.RENAME_NOTE_REL, token, "content")]
    errors: list[str] = []
    MODULE.check_occurrences(
        MODULE.RENAME_NOTE_REL, "content", (token + " ") * (maximum + 1), errors
    )
    assert len(errors) == 1
    assert f"allowed maximum is {maximum}" in errors[0]


def test_frozen_hash_mismatch_fails(tmp_path: Path, monkeypatch) -> None:
    frozen_rel = "docs/migration/README.md"
    frozen_path = tmp_path / frozen_rel
    frozen_path.parent.mkdir(parents=True)
    frozen_path.write_bytes(b"changed frozen evidence\n")

    inventory_path = tmp_path / MODULE.INVENTORY_REL
    inventory_path.parent.mkdir(parents=True, exist_ok=True)
    inventory_path.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "algorithm": "sha256",
                "predecessor_commit": MODULE.PREDECESSOR_COMMIT,
                "scope": {
                    "prefixes": list(MODULE.FROZEN_PREFIXES),
                    "required_exact": sorted(MODULE.FROZEN_EXACT),
                    "optional_exact": sorted(MODULE.OPTIONAL_FROZEN_EXACT),
                },
                "files": [
                    {
                        "path": frozen_rel,
                        "sha256": "0" * 64,
                        "size": frozen_path.stat().st_size,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    monkeypatch.setattr(MODULE, "REPO_ROOT", tmp_path)
    frozen, errors = MODULE.verify_inventory([frozen_rel])
    assert frozen == {frozen_rel}
    assert any("frozen hash mismatch" in error for error in errors)
