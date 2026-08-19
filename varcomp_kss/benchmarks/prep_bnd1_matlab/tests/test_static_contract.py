from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def test_scc_wrapper_is_non_submitting_and_hash_bound() -> None:
    source = (ROOT / "run_pair.sge").read_text(encoding="utf-8")
    assert re.search(r"\bqsub\b", source) is None
    for token in (
        "PBM_SOURCE_COMMIT", "PBM_BUNDLE_SHA256", "PBM_BUNDLE_MANIFEST",
        "PBM_TASK_SHA256", "sha256sum -c", "verify_numopt2_matlab_source.py",
        "monitor_process_tree.py", "stata_matlab|matlab_stata",
    ):
        assert token in source


def test_bundle_allowlist_is_sorted_and_excludes_licensed_source() -> None:
    rows = [line for line in (ROOT / "bundle_allowlist.txt").read_text(
        encoding="utf-8").splitlines() if line]
    assert rows == sorted(set(rows))
    assert not any("LeaveOutTwoWay" in row for row in rows)
    assert "varcomp_kss/varcomp_kss.ado" in rows
    assert "varcomp_kss/benchmarks/prep_bnd1_matlab/run_pair.sge" in rows
