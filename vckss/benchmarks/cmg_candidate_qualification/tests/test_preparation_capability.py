from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from common import EvidenceError, RUN_SCHEMA, sha256  # noqa: E402
from validate_preparation import validate  # noqa: E402


def write_capability(path: Path, licensed: int) -> str:
    path.write_text(
        "key\tvalue\n"
        "schema\tVCKSS-STATA-PROCESSOR-CAPABILITY-V1\n"
        f"status\t{'PASS' if licensed >= 16 else 'FAIL'}\n"
        "required_processors\t16\n"
        f"licensed_processors\t{licensed}\n"
        "initial_processors\t4\n"
        "stata_version\t19\n"
        "stata_flavor\tIC\n"
        "stata_mp\t1\n",
        encoding="utf-8",
    )
    return sha256(path)


def staged(tmp_path: Path, licensed: int) -> None:
    identity = {
        "schema": RUN_SCHEMA,
        "status": "PASS",
        "candidate_commit": "1" * 40,
        "comparison_commit": "2" * 40,
        "required_stata_processors": 16,
    }
    (tmp_path / "run_identity.json").write_text(
        json.dumps(identity) + "\n", encoding="utf-8")
    receipts = tmp_path / "receipts" / "preparation"
    receipts.mkdir(parents=True)
    capability_sha = write_capability(
        receipts / "stata_processor_capability.tsv", licensed)
    (receipts / "preparation.tsv").write_text(
        "key\tvalue\n"
        "schema\tVCKSS-CMG-CANDIDATE-QUALIFICATION-PREPARATION-V1\n"
        "status\tPASS\n"
        "job_id\t123\n"
        f"candidate_commit\t{'1' * 40}\n"
        f"comparison_commit\t{'2' * 40}\n"
        f"required_stata_processors\t16\n"
        f"licensed_stata_processors\t{licensed}\n"
        f"stata_processor_capability_sha256\t{capability_sha}\n"
        f"candidate_binary_manifest_sha256\t{'a' * 64}\n"
        f"comparison_binary_manifest_sha256\t{'b' * 64}\n",
        encoding="utf-8",
    )
    (receipts / "wrapper.pass").write_text(
        f"VCKSS_CMG_CANDIDATE_QUALIFICATION_PREPARE_PASS {'1' * 40} {'2' * 40}\n",
        encoding="utf-8",
    )
    (receipts / "qacct.txt").write_text(
        "jobnumber 123\nproject welfgr\ngranted_pe omp\nslots 4\n"
        "failed 0\nexit_status 0\n",
        encoding="utf-8",
    )


def test_candidate_preparation_requires_sixteen_licensed_processors(
        tmp_path: Path) -> None:
    staged(tmp_path, 16)
    value = validate(tmp_path)
    assert value["required_stata_processors"] == 16
    assert value["licensed_stata_processors"] == 16


def test_candidate_preparation_rejects_four_core_entitlement(
        tmp_path: Path) -> None:
    staged(tmp_path, 4)
    with pytest.raises(EvidenceError, match="processor capability"):
        validate(tmp_path)
