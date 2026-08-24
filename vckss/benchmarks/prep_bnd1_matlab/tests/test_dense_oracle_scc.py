from __future__ import annotations

import csv
import json

import pytest
from common import EvidenceError, sha256
from validate_dense_oracle_scc import APPLICATION_FILES, validate

COMMIT = "c" * 40
BUNDLE = "b" * 64
GENERIC_HASH = "d" * 64


def tsv(path, header, rows) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(header)
        writer.writerows(rows)


def make_job(tmp_path):
    job = tmp_path / "job"
    job.mkdir()
    source_rows = [
        ("schema", "PREP-BND-1-DENSE-ORACLE-SOURCE-V1"),
        ("source_commit", COMMIT),
        ("bundle_sha256", BUNDLE),
        ("bundle_archive_sha256", BUNDLE),
        ("bundle_manifest_sha256", GENERIC_HASH),
        ("internal_manifest_sha256", GENERIC_HASH),
        ("matlab_oracle_sha256", GENERIC_HASH),
        ("stata_oracle_sha256", GENERIC_HASH),
        ("wrapper_sha256", GENERIC_HASH),
        ("post_validator_sha256", GENERIC_HASH),
    ]
    tsv(job / "source_receipt.tsv", ("key", "value"), source_rows)
    checked = (
        "SOURCE_COMMIT.txt",
        "vckss/benchmarks/oracle/stata_oracle.do",
        "vckss/benchmarks/oracle/vckss_dense_oracle.m",
        "vckss/benchmarks/prep_bnd1_matlab/run_dense_oracle.sge",
        "vckss/benchmarks/prep_bnd1_matlab/validate_dense_oracle.py",
        "vckss/benchmarks/prep_bnd1_matlab/validate_dense_oracle_scc.py",
    )
    (job / "source_manifest.check").write_text(
        "".join(f"{item}: OK\n" for item in checked), encoding="utf-8")
    (job / "matlab_oracle.csv").write_text("matlab\n", encoding="utf-8")
    (job / "stata_oracle.csv").write_text("stata\n", encoding="utf-8")
    dense = {
        "schema": "PREP-BND-1-DENSE-ORACLE-V1",
        "status": "PASS_EXACT_DENSE_ORACLE",
        "source_commit": COMMIT,
        "matlab_oracle_sha256": sha256(job / "matlab_oracle.csv"),
        "stata_oracle_sha256": sha256(job / "stata_oracle.csv"),
    }
    (job / "dense_oracle_validation.json").write_text(
        json.dumps(dense), encoding="utf-8")
    (job / "matlab.application.txt").write_text(
        f"VCKSS MATLAB ORACLE PASS: {COMMIT}\n", encoding="utf-8")
    (job / "stata.application.txt").write_text(
        "VCKSS STATA/MATLAB ORACLE PASS\n", encoding="utf-8")
    (job / "dense_oracle_validation.application.txt").write_text(
        f"PREP_BND1_DENSE_ORACLE_PASS {COMMIT}\n", encoding="utf-8")
    hashes = [(name, sha256(job / name)) for name in APPLICATION_FILES]
    tsv(job / "application_files.sha256.tsv", ("artifact", "sha256"), hashes)
    node_rows = [
        ("schema", "PREP-BND-1-DENSE-ORACLE-NODE-V1"),
        ("job_id", "123"), ("hostname", "node1"),
        ("source_commit", COMMIT), ("bundle_sha256", BUNDLE),
        ("bundle_manifest_sha256", GENERIC_HASH),
        ("application_manifest_sha256",
         sha256(job / "application_files.sha256.tsv")),
        ("requested_slots", "4"), ("actual_slots", "4"),
        ("matlab_module", "matlab/2025b"),
        ("stata_module", "stata-mp/19"),
    ]
    tsv(job / "node_receipt.tsv", ("key", "value"), node_rows)
    (job / "oracle.pass").write_text(
        f"PREP_BND1_DENSE_ORACLE_SCC_PASS {COMMIT} {BUNDLE}\n",
        encoding="utf-8")
    qacct_path = tmp_path / "qacct.txt"
    qacct_path.write_text(
        "jobnumber 123\ntaskid undefined\nproject welfgr\n"
        "granted_pe omp\nslots 4\nfailed 0\nexit_status 0\n"
        "ru_wallclock 3.0\ncpu 2.0\nmaxvmem 1.5G\nhostname node1\n",
        encoding="utf-8",
    )
    return job, qacct_path


def test_scc_dense_oracle_accepts_complete_hash_bound_evidence(tmp_path) -> None:
    job, qacct_path = make_job(tmp_path)
    rows = list(csv.reader((job / "node_receipt.tsv").open(
        newline="", encoding="utf-8"), delimiter="\t"))
    rows[3][1] = "node1.example.edu"
    tsv(job / "node_receipt.tsv", rows[0], rows[1:])
    result = validate(job, qacct_path, COMMIT, BUNDLE)
    assert result["status"] == "PASS_EXACT_DENSE_ORACLE_SCC"
    assert result["job_id"] == "123"


def test_scc_dense_oracle_rejects_application_tampering(tmp_path) -> None:
    job, qacct_path = make_job(tmp_path)
    with (job / "stata.application.txt").open("a", encoding="utf-8") as handle:
        handle.write("changed\n")
    with pytest.raises(EvidenceError, match="application artifact hash changed"):
        validate(job, qacct_path, COMMIT, BUNDLE)
