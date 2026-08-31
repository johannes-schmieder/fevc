#!/usr/bin/env python3
"""Fail-closed post-job validator for the SCC clean-room dense oracle."""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path

from common import HEX40, HEX64, load_json, qacct, require, same_host, sha256

APPLICATION_FILES = (
    "dense_oracle_validation.application.txt",
    "dense_oracle_validation.json",
    "matlab.application.txt",
    "matlab_oracle.csv",
    "source_manifest.check",
    "source_receipt.tsv",
    "stata.application.txt",
    "stata_oracle.csv",
)


def key_values(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(), f"invalid TSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.reader(handle, delimiter="\t"))
    require(rows and rows[0] == ["key", "value"], "invalid key/value header")
    require(all(len(row) == 2 for row in rows[1:]), "invalid key/value row")
    result = {key: value for key, value in rows[1:]}
    require(len(result) == len(rows) - 1, "duplicate key/value row")
    return result


def application_hashes(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(),
            "invalid application hash manifest")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.reader(handle, delimiter="\t"))
    require(rows and rows[0] == ["artifact", "sha256"],
            "invalid application hash header")
    require(all(len(row) == 2 and HEX64.fullmatch(row[1]) for row in rows[1:]),
            "invalid application hash row")
    result = {artifact: digest for artifact, digest in rows[1:]}
    require(len(result) == len(rows) - 1 and
            tuple(sorted(result)) == APPLICATION_FILES,
            "application hash closure changed")
    return result


def validate(
    job_dir: Path,
    qacct_path: Path,
    expected_source_commit: str,
    expected_bundle: str,
) -> dict[str, object]:
    require(HEX40.fullmatch(expected_source_commit) is not None,
            "invalid expected source commit")
    require(HEX64.fullmatch(expected_bundle) is not None,
            "invalid expected bundle")
    source = key_values(job_dir / "source_receipt.tsv")
    require(source.get("schema") == "PREP-BND-1-DENSE-ORACLE-SOURCE-V1" and
            source.get("source_commit") == expected_source_commit and
            source.get("bundle_sha256") == expected_bundle and
            source.get("bundle_archive_sha256") == expected_bundle,
            "source binding changed")
    for field in (
        "bundle_manifest_sha256", "internal_manifest_sha256",
        "matlab_oracle_sha256", "stata_oracle_sha256", "wrapper_sha256",
        "post_validator_sha256",
    ):
        require(HEX64.fullmatch(source.get(field, "")) is not None,
                f"invalid source hash: {field}")
    require(source["bundle_manifest_sha256"] ==
            source["internal_manifest_sha256"], "bundle manifests differ")

    source_check = job_dir / "source_manifest.check"
    require(source_check.is_file() and not source_check.is_symlink(),
            "source-manifest check missing")
    source_text = source_check.read_text(encoding="utf-8", errors="replace")
    require("FAILED" not in source_text, "source-manifest check failed")
    for relative in (
        "SOURCE_COMMIT.txt",
        "fevc/benchmarks/oracle/stata_oracle.do",
        "fevc/benchmarks/oracle/vckss_dense_oracle.m",
        "fevc/benchmarks/prep_bnd1_matlab/run_dense_oracle.sge",
        "fevc/benchmarks/prep_bnd1_matlab/validate_dense_oracle.py",
        "fevc/benchmarks/prep_bnd1_matlab/validate_dense_oracle_scc.py",
    ):
        require(f"{relative}: OK" in source_text,
                f"source check omitted {relative}")

    application_manifest = job_dir / "application_files.sha256.tsv"
    hashes = application_hashes(application_manifest)
    for artifact, digest in hashes.items():
        artifact_path = job_dir / artifact
        require(artifact_path.is_file() and not artifact_path.is_symlink(),
                f"invalid application artifact: {artifact}")
        require(sha256(artifact_path) == digest,
                f"application artifact hash changed: {artifact}")

    dense = load_json(job_dir / "dense_oracle_validation.json")
    require(dense.get("schema") == "PREP-BND-1-DENSE-ORACLE-V1" and
            dense.get("status") == "PASS_EXACT_DENSE_ORACLE" and
            dense.get("source_commit") == expected_source_commit,
            "dense-oracle validation failed")
    require(dense.get("matlab_oracle_sha256") ==
            hashes["matlab_oracle.csv"] and
            dense.get("stata_oracle_sha256") == hashes["stata_oracle.csv"],
            "dense-oracle application hashes changed")
    matlab_log = (job_dir / "matlab.application.txt").read_text(
        encoding="utf-8", errors="replace")
    stata_log = (job_dir / "stata.application.txt").read_text(
        encoding="utf-8", errors="replace")
    validation_log = (job_dir / "dense_oracle_validation.application.txt").read_text(
        encoding="utf-8", errors="replace")
    require(f"FEVC MATLAB ORACLE PASS: {expected_source_commit}" in matlab_log,
            "MATLAB application marker missing")
    require("FEVC STATA/MATLAB ORACLE PASS" in stata_log,
            "Stata application marker missing")
    require(f"PREP_BND1_DENSE_ORACLE_PASS {expected_source_commit}" in
            validation_log, "Python application marker missing")

    node = key_values(job_dir / "node_receipt.tsv")
    accounting = qacct(qacct_path)
    require(node.get("schema") == "PREP-BND-1-DENSE-ORACLE-NODE-V1" and
            node.get("source_commit") == expected_source_commit and
            node.get("bundle_sha256") == expected_bundle and
            node.get("bundle_manifest_sha256") ==
            source["bundle_manifest_sha256"] and
            node.get("application_manifest_sha256") ==
            sha256(application_manifest), "node receipt binding changed")
    require(node.get("job_id") == accounting["jobnumber"] and
            same_host(node.get("hostname", ""), accounting["hostname"]),
            "scheduler identity changed")
    require(node.get("requested_slots") == node.get("actual_slots") == "4" and
            node.get("matlab_module") == "matlab/2025b" and
            node.get("stata_module") == "stata-mp/19",
            "application resource contract changed")
    marker_path = job_dir / "oracle.pass"
    require(marker_path.is_file() and not marker_path.is_symlink(),
            "invalid terminal marker")
    marker = marker_path.read_text(encoding="utf-8").strip()
    require(marker ==
            f"PREP_BND1_DENSE_ORACLE_SCC_PASS {expected_source_commit} "
            f"{expected_bundle}" and not (job_dir / "wrapper.fail").exists(),
            "terminal marker failed")
    return {
        "schema": "PREP-BND-1-DENSE-ORACLE-SCC-VALIDATION-V1",
        "status": "PASS_EXACT_DENSE_ORACLE_SCC",
        "source_commit": expected_source_commit,
        "bundle_sha256": expected_bundle,
        "job_id": accounting["jobnumber"],
        "hostname": accounting["hostname"],
        "qacct_wall_seconds": float(accounting["ru_wallclock"]),
        "qacct_cpu_seconds": float(accounting["cpu"]),
        "application_manifest_sha256": sha256(application_manifest),
        "artifact_sha256": {
            **hashes,
            "application_files.sha256.tsv": sha256(application_manifest),
            "node_receipt.tsv": sha256(job_dir / "node_receipt.tsv"),
            "oracle.pass": sha256(job_dir / "oracle.pass"),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--job-dir", type=Path, required=True)
    parser.add_argument("--qacct", type=Path, required=True)
    parser.add_argument("--expected-source-commit", required=True)
    parser.add_argument("--expected-bundle", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "validation target already exists")
    payload = validate(args.job_dir, args.qacct, args.expected_source_commit,
                       args.expected_bundle)
    args.output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print("PREP_BND1_DENSE_ORACLE_SCC_VALIDATION_PASS "
          f"{args.expected_source_commit}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
