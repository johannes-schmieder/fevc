#!/usr/bin/env python3
"""Verify one canonical exact-source artifact set before immutable import."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

try:
    from .common import key_values, load_json, require, sha256
    from .validate_pilot import RUN_SCHEMA
except ImportError:
    from common import key_values, load_json, require, sha256  # type: ignore
    from validate_pilot import RUN_SCHEMA  # type: ignore


ARTIFACT_SOURCE_SCHEMA = "VCKSS-COMPARATIVE-SCALING-ARTIFACT-SOURCE-V1"
PREPARATION_SCHEMA = "VCKSS-COMPARATIVE-SCALING-PREPARATION-V2"
PREPARATION_QACCT_SCHEMA = "VCKSS-COMPARATIVE-SCALING-PREPARATION-QACCT-V2"
SHARED_FIELDS = (
    "source_commit",
    "bundle_sha256",
    "source_manifest_sha256",
    "task_manifest_sha256",
    "stata_spi_manifest_sha256",
    "mem_per_core_gib",
    "command_memory_gib",
    "required_stata_processors",
    "maximum_mata_cores",
    "required_rust_threads",
    "required_matlab_workers",
)


def verify_manifest(source_run: Path, manifest_path: Path) -> int:
    require(manifest_path.is_file() and not manifest_path.is_symlink(),
            "canonical binary manifest is missing")
    artifacts = source_run / "artifacts"
    require(artifacts.is_dir() and not artifacts.is_symlink(),
            "canonical artifact directory is missing")
    count = 0
    seen: set[Path] = set()
    for line in manifest_path.read_text(encoding="utf-8").splitlines():
        fields = line.split("  ", 1)
        require(len(fields) == 2 and re.fullmatch(r"[0-9a-f]{64}", fields[0])
                is not None, "invalid canonical binary manifest row")
        relative = Path(fields[1])
        require(not relative.is_absolute() and ".." not in relative.parts and
                relative.parts and relative.parts[0] in {"package", "mex"} and
                relative not in seen,
                "unsafe canonical binary path")
        seen.add(relative)
        artifact = artifacts / relative
        ancestors = [artifacts.joinpath(*relative.parts[:index])
                     for index in range(1, len(relative.parts))]
        require(not any(path.is_symlink() for path in ancestors) and
                artifact.is_file() and not artifact.is_symlink() and
                sha256(artifact) == fields[0],
                f"canonical artifact changed: {relative.as_posix()}")
        count += 1
    require(count > 10, "canonical binary manifest is incomplete")
    return count


def verify(target_path: Path, source_run: Path) -> dict[str, Any]:
    target = load_json(target_path)
    source_identity_path = source_run / "run_identity.json"
    source_identity = load_json(source_identity_path)
    source_run_id = str(source_identity.get("run_id", ""))
    require(target.get("schema") == RUN_SCHEMA and target.get("status") == "PASS" and
            target.get("run_kind") in {"pilot-small", "pilot-worst", "production"},
            "target staged-run identity changed")
    require(target.get("artifact_source_run_id") == source_run_id and
            re.fullmatch(r"[A-Za-z0-9._-]+", source_run_id) is not None,
            "target canonical artifact source changed")
    require(source_identity.get("schema") == RUN_SCHEMA and
            source_identity.get("status") == "PASS" and
            source_identity.get("run_kind") == "preparation" and
            source_identity.get("artifact_source_run_id") is None and
            source_run.name == source_run_id,
            "canonical staged-run identity changed")
    for field in SHARED_FIELDS:
        require(target.get(field) == source_identity.get(field),
                f"canonical {field} differs from target")

    receipt_dir = source_run / "receipts" / "preparation"
    preparation_path = receipt_dir / "preparation.tsv"
    preparation = key_values(preparation_path)
    qacct_path = receipt_dir / "qacct.pass.json"
    qacct = load_json(qacct_path)
    require(preparation.get("schema") == PREPARATION_SCHEMA and
            preparation.get("status") == "PASS" and
            preparation.get("artifact_mode") == "BUILT_CANONICAL" and
            preparation.get("artifact_source_run_id") == "NONE" and
            preparation.get("source_commit") == source_identity.get("source_commit") and
            preparation.get("bundle_sha256") == source_identity.get("bundle_sha256"),
            "canonical preparation receipt changed")
    require(qacct.get("schema") == PREPARATION_QACCT_SCHEMA and
            qacct.get("status") == "PASS" and
            qacct.get("run_id") == source_run_id and
            qacct.get("run_kind") == "preparation" and
            qacct.get("artifact_mode") == "BUILT_CANONICAL" and
            qacct.get("artifact_source_run_id") is None and
            qacct.get("source_commit") == source_identity.get("source_commit") and
            qacct.get("bundle_sha256") == source_identity.get("bundle_sha256"),
            "canonical preparation accounting changed")
    manifest_path = receipt_dir / "binary_manifest.sha256"
    artifact_count = verify_manifest(source_run, manifest_path)
    manifest_sha = sha256(manifest_path)
    require(preparation.get("binary_manifest_sha256") == manifest_sha ==
            qacct.get("binary_manifest_sha256"),
            "canonical binary manifest identity changed")
    require((receipt_dir / "wrapper.pass").is_file() and
            not (receipt_dir / "wrapper.fail").exists(),
            "canonical preparation wrapper changed")
    return {
        "schema": ARTIFACT_SOURCE_SCHEMA,
        "status": "PASS",
        "artifact_source_run_id": source_run_id,
        "source_commit": target["source_commit"],
        "bundle_sha256": target["bundle_sha256"],
        "source_manifest_sha256": target["source_manifest_sha256"],
        "task_manifest_sha256": target["task_manifest_sha256"],
        "stata_spi_manifest_sha256": target["stata_spi_manifest_sha256"],
        "binary_manifest_sha256": manifest_sha,
        "artifact_files": artifact_count,
        "canonical_run_identity_sha256": sha256(source_identity_path),
        "canonical_preparation_receipt_sha256": sha256(preparation_path),
        "canonical_preparation_qacct_sha256": sha256(qacct_path),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", type=Path, required=True)
    parser.add_argument("--source-run", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "artifact-source receipt target exists")
    value = verify(args.target, args.source_run)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print("VCKSS_COMPARATIVE_SCALING_ARTIFACT_SOURCE_PASS "
          f"{value['artifact_source_run_id']} {value['binary_manifest_sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
