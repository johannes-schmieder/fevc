from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from authorize_replacement import (  # noqa: E402
    PILOT_TASK_IDS,
    replacement_stage,
)
from common import EvidenceError  # noqa: E402
from common import TASK_FIELDS, sha256, write_tsv  # noqa: E402
from collect_generation import frozen_manifest  # noqa: E402


def test_replacement_submission_is_full_or_pilot_then_exact_remainder() -> None:
    expected = [1, 2, 3, 16, 17, 18, 226, 227, 228]
    assert replacement_stage(expected, expected, []) == ("FULL", expected)
    assert replacement_stage(PILOT_TASK_IDS, expected, []) == (
        "PILOT", expected)
    remainder = [2, 3, 16, 17, 18, 227, 228]
    assert replacement_stage(remainder, expected, PILOT_TASK_IDS) == (
        "REMAINDER", remainder)


def test_replacement_submission_rejects_arbitrary_or_duplicate_work() -> None:
    expected = [1, 2, 3, 226, 227, 228]
    with pytest.raises(EvidenceError, match="registered pilot or full"):
        replacement_stage([1], expected, [])
    with pytest.raises(EvidenceError, match="exact remaining"):
        replacement_stage([2], expected, PILOT_TASK_IDS)
    with pytest.raises(EvidenceError, match="duplicate successful"):
        replacement_stage([2, 3, 227, 228], expected, [1, 1, 226])


def test_frozen_manifest_is_read_under_its_byte_bound_historical_schema(
        tmp_path: Path) -> None:
    rows = []
    for task_id in range(1, 301):
        rows.append({field: "legacy" for field in TASK_FIELDS} | {
            "task_id": str(task_id),
            "active_cores": "1",
            "structure": "weak_d3",
            "rows": "7680",
        })
    path = tmp_path / "input" / "tasks.tsv"
    path.parent.mkdir()
    write_tsv(path, TASK_FIELDS, rows)
    identity = {"task_manifest_sha256": sha256(path)}
    assert len(frozen_manifest(tmp_path, identity)) == 300

    path.write_text(path.read_text(encoding="utf-8") + "\n", encoding="utf-8")
    with pytest.raises(EvidenceError, match="identity changed"):
        frozen_manifest(tmp_path, identity)
