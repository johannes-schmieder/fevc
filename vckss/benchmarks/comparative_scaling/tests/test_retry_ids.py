from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from expand_task_ids import expand  # noqa: E402
from common import EvidenceError  # noqa: E402
from verify_retry import classify  # noqa: E402


def test_retry_task_ids_expand_without_reordering() -> None:
    assert expand("1,3-5,300") == [1, 3, 4, 5, 300]


@pytest.mark.parametrize("value", ("0", "301", "5-3", "1,1", "1,,2", "x"))
def test_retry_task_ids_reject_ambiguous_or_out_of_range_values(value: str) -> None:
    with pytest.raises(ValueError):
        expand(value)


def test_retry_classification_accepts_only_missing_or_scheduler_failed(
        tmp_path: Path) -> None:
    (tmp_path / "attempts" / "first" / "qacct").mkdir(parents=True)
    qacct = tmp_path / "attempts" / "first" / "qacct" / "2.txt"
    qacct.write_text(
        "jobnumber 123\ntaskid 2\nfailed 37\nexit_status 1\n",
        encoding="utf-8",
    )
    assert classify(tmp_path, 1) == "MISSING_TASK_AND_ACCOUNTING"
    assert classify(tmp_path, 2) == "SCHEDULER_FAILED"

    success = tmp_path / "attempts" / "first" / "qacct" / "3.txt"
    success.write_text(
        "jobnumber 123\ntaskid 3\nfailed 0\nexit_status 0\n",
        encoding="utf-8",
    )
    with pytest.raises(EvidenceError, match="non-infrastructure"):
        classify(tmp_path, 3)
