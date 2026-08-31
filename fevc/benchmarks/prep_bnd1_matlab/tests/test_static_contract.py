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


def test_pair_stata_marker_survives_stata_log_wrapping() -> None:
    source = (ROOT / "run_pair.sge").read_text(encoding="utf-8")
    assert 'grep -F "PREP_BND1_MATLAB_STATA_PASS $PBM_SOURCE_COMMIT"' in source
    assert ('PREP_BND1_MATLAB_STATA_PASS $PBM_SOURCE_COMMIT '
            '$PBM_TASK_SHA256') not in source


def test_matlab_pair_counts_unique_worker_firm_rows_not_matrix_elements() -> None:
    source = (ROOT / "prep_bnd1_matlab_run.m").read_text(encoding="utf-8")
    assert "size(unique([worker firm],'rows'),1)==expected_rows" in source
    assert "numel(unique([worker firm],'rows'))" not in source


def test_dense_oracle_wrapper_is_clean_room_non_submitting_and_source_bound() -> None:
    source = (ROOT / "run_dense_oracle.sge").read_text(encoding="utf-8")
    assert re.search(r"\bqsub\b", source) is None
    assert "LeaveOutTwoWay" not in source
    assert "PBM_MATLAB_ROOT" not in source
    for token in (
        "matlab/2025b", "stata-mp/19", "sha256sum -c BUNDLE_FILES.sha256",
        "vckss_dense_oracle", "stata_oracle.do",
        "validate_dense_oracle.py", "application_files.sha256.tsv",
        "PREP_BND1_DENSE_ORACLE_SCC_PASS",
    ):
        assert token in source


def test_bundle_allowlist_is_sorted_and_excludes_licensed_source() -> None:
    rows = [line for line in (ROOT / "bundle_allowlist.txt").read_text(
        encoding="utf-8").splitlines() if line]
    assert rows == sorted(set(rows))
    assert not any("LeaveOutTwoWay" in row for row in rows)
    assert "fevc/fevc.ado" in rows
    assert "fevc/benchmarks/prep_bnd1_matlab/run_pair.sge" in rows
    for required in (
        "fevc/benchmarks/matlab_scale/common.py",
        "fevc/benchmarks/oracle/stata_oracle.do",
        "fevc/benchmarks/oracle/vckss_dense_oracle.m",
        "fevc/benchmarks/prep_bnd1_matlab/run_dense_oracle.sge",
        "fevc/benchmarks/prep_bnd1_matlab/validate_dense_oracle_scc.py",
    ):
        assert required in rows
