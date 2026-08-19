from __future__ import annotations

import csv

import pytest
from common import EvidenceError
from validate_dense_oracle import TARGETS, validate

COMMIT = "c" * 40


def write(path, delta=0.0) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(
            handle,
            fieldnames=["source_commit", "oracle", "target", "plugin",
                        "correction", "corrected"],
            lineterminator="\n",
        )
        writer.writeheader()
        for index, target in enumerate(TARGETS):
            writer.writerow({
                "source_commit": COMMIT,
                "oracle": "test",
                "target": target,
                "plugin": 1 + index + delta,
                "correction": .1 + index + delta,
                "corrected": .9 + delta,
            })


def test_dense_oracle_is_a_hard_equality_gate(tmp_path) -> None:
    matlab = tmp_path / "matlab.csv"
    stata = tmp_path / "stata.csv"
    write(matlab)
    write(stata, 1e-10)
    result = validate(matlab, stata, COMMIT)
    assert result["status"] == "PASS_EXACT_DENSE_ORACLE"


def test_dense_oracle_rejects_gap(tmp_path) -> None:
    matlab = tmp_path / "matlab.csv"
    stata = tmp_path / "stata.csv"
    write(matlab)
    write(stata, 3e-9)
    with pytest.raises(EvidenceError, match="dense-oracle"):
        validate(matlab, stata, COMMIT)
