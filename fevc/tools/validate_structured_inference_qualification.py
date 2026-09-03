#!/usr/bin/env python3
"""Validate deterministic fitted-variance component-inference evidence."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence


SCHEMA = "fevc-structured-component-qualification-v1"
CSV_SCHEMA = "1"
TARGETS = ("worker", "firm", "covariance", "total")
EXTRA_CELLS = {
    "diffuse_homoskedastic",
    "diffuse_leverage",
    "dominant_leverage",
    "diffuse_common_t8",
    "dominant_common_t8",
    "diffuse_common_controls",
    "dominant_common_controls",
    "diffuse_mild_functional",
    "dominant_mild_functional",
    "diffuse_mild_omitted",
    "dominant_mild_omitted",
    "diffuse_severe_omitted",
    "dominant_severe_omitted",
    "diffuse_common_null",
    "dominant_common_null_q0",
    "dominant_common_null",
}
REQUIRED_COLUMNS = {
    "schema",
    "profile",
    "cell",
    "gate",
    "k",
    "controls",
    "dominant",
    "variance_model",
    "variance_dgp",
    "error_dgp",
    "reference",
    "beta",
    "target",
    "replications",
    "successes",
    "bias",
    "bias_mcse",
    "coverage",
    "coverage_mcse",
    "empirical_sd",
    "mean_se",
    "se_ratio",
    "mean_estimated_variance",
    "leading_share",
    "remainder_share",
    "mean_floor_share",
    "mean_boundary_share",
}
THRESHOLDS = {
    "bias_mcse_multiplier": 4.0,
    "coverage_absolute_tolerance": 0.015,
    "coverage_mcse_multiplier": 3.0,
    "correct_success_rate": 0.99,
    "q1_nonprimary_success_rate": 0.95,
    "correct_se_ratio_lower": 0.90,
    "correct_se_ratio_upper": 1.10,
    "mild_success_rate": 0.98,
    "mild_coverage_floor": 0.88,
    "mild_coverage_drop": 0.04,
    "severe_success_rate": 0.90,
}


class QualificationError(RuntimeError):
    """The evidence is incomplete, malformed, or outside a registered gate."""


@dataclass(frozen=True)
class Row:
    values: dict[str, str]

    def text(self, name: str) -> str:
        return self.values[name]

    def integer(self, name: str) -> int:
        try:
            return int(self.values[name])
        except ValueError as error:
            raise QualificationError(f"{self.label}: invalid integer {name}") from error

    def number(self, name: str) -> float:
        try:
            value = float(self.values[name])
        except ValueError as error:
            raise QualificationError(f"{self.label}: invalid number {name}") from error
        if not math.isfinite(value):
            raise QualificationError(f"{self.label}: nonfinite {name}")
        return value

    @property
    def label(self) -> str:
        return "/".join(
            self.values.get(name, "?") for name in ("cell", "k", "target")
        )


def _expected_keys(profile: str) -> set[tuple[str, int, str]]:
    if profile == "development":
        dimensions = (12,)
    elif profile == "confirmation":
        dimensions = (12, 16)
    else:
        raise QualificationError(f"unsupported profile {profile!r}")
    keys = {
        (cell, k, target)
        for k in dimensions
        for cell in ("diffuse_common", "dominant_common")
        for target in TARGETS
    }
    final_k = dimensions[-1]
    keys.update((cell, final_k, target) for cell in EXTRA_CELLS for target in TARGETS)
    return keys


def _read(path: Path) -> tuple[list[Row], str]:
    payload = path.read_bytes()
    try:
        text = payload.decode("utf-8")
    except UnicodeDecodeError as error:
        raise QualificationError("qualification CSV is not UTF-8") from error
    reader = csv.DictReader(text.splitlines())
    if reader.fieldnames is None or set(reader.fieldnames) != REQUIRED_COLUMNS:
        raise QualificationError("qualification CSV columns do not match schema v1")
    rows = [Row(dict(values)) for values in reader]
    return rows, hashlib.sha256(payload).hexdigest()


def _promotion_row(row: Row) -> bool:
    if row.text("gate") != "correct":
        return False
    return row.text("reference") == "Q0" or row.text("target") != "covariance"


def _check_integrity(rows: Sequence[Row], profile: str) -> dict[tuple[str, int, str], Row]:
    expected = _expected_keys(profile)
    by_key: dict[tuple[str, int, str], Row] = {}
    expected_replications = 100 if profile == "development" else 2_500
    for row in rows:
        if row.text("schema") != CSV_SCHEMA or row.text("profile") != profile:
            raise QualificationError(f"{row.label}: schema or profile mismatch")
        key = (row.text("cell"), row.integer("k"), row.text("target"))
        if key in by_key:
            raise QualificationError(f"duplicate qualification row {key}")
        by_key[key] = row
        if row.text("target") not in TARGETS:
            raise QualificationError(f"{row.label}: unknown target")
        attempts = row.integer("replications")
        successes = row.integer("successes")
        if attempts != expected_replications or not 0 <= successes <= attempts:
            raise QualificationError(f"{row.label}: invalid replication counts")
        for name in ("leading_share", "remainder_share"):
            value = row.number(name)
            if not 0.0 <= value <= 1.0 + 1.0e-10:
                raise QualificationError(f"{row.label}: invalid {name}")
        if successes >= 2:
            for name in REQUIRED_COLUMNS - {
                "schema",
                "profile",
                "cell",
                "gate",
                "variance_model",
                "variance_dgp",
                "error_dgp",
                "reference",
                "beta",
                "target",
                "k",
                "controls",
                "dominant",
                "replications",
                "successes",
                "leading_share",
                "remainder_share",
            }:
                row.number(name)
    missing = expected - set(by_key)
    extra = set(by_key) - expected
    if missing or extra:
        raise QualificationError(
            f"qualification cell matrix mismatch: missing={sorted(missing)} extra={sorted(extra)}"
        )
    return by_key


def _check_spectral_cases(by_key: dict[tuple[str, int, str], Row], profile: str) -> None:
    final_k = 12 if profile == "development" else 16
    for target in ("worker", "firm"):
        diffuse = by_key[("diffuse_common", final_k, target)]
        dominant = by_key[("dominant_common", final_k, target)]
        if diffuse.number("leading_share") >= 0.11:
            raise QualificationError(f"{diffuse.label}: diffuse fixture is not diffuse")
        if dominant.number("leading_share") <= 0.75:
            raise QualificationError(f"{dominant.label}: bottleneck fixture lacks one leading mode")
        if dominant.number("remainder_share") >= 0.15:
            raise QualificationError(f"{dominant.label}: q=1 remainder is not diffuse")
    covariance = by_key[("dominant_common", final_k, "covariance")]
    if covariance.number("leading_share") <= 0.95 or covariance.number("remainder_share") <= 0.50:
        raise QualificationError(
            f"{covariance.label}: deliberate q>1 limitation is not present"
        )
    unreliable = by_key[("dominant_common_null_q0", final_k, "worker")]
    if unreliable.number("leading_share") <= 0.75:
        raise QualificationError(f"{unreliable.label}: q=0 stress case lacks concentration")
    if profile == "confirmation":
        for target in ("worker", "firm", "total"):
            first = by_key[("diffuse_common", 12, target)].number("leading_share")
            second = by_key[("diffuse_common", 16, target)].number("leading_share")
            if second >= first:
                raise QualificationError(
                    f"diffuse_common/{target}: concentration did not fall with dimension"
                )


def _correct_gate(rows: Iterable[Row]) -> list[str]:
    checked: list[str] = []
    for row in rows:
        if not _promotion_row(row):
            continue
        attempts = row.integer("replications")
        successes = row.integer("successes")
        if successes / attempts < THRESHOLDS["correct_success_rate"]:
            raise QualificationError(f"{row.label}: correct-model success rate is too low")
        bias = abs(row.number("bias"))
        bias_bound = THRESHOLDS["bias_mcse_multiplier"] * row.number("bias_mcse")
        if bias > max(bias_bound, 1.0e-12):
            raise QualificationError(f"{row.label}: point bias exceeds four Monte Carlo SEs")
        coverage = row.number("coverage")
        coverage_bound = max(
            THRESHOLDS["coverage_absolute_tolerance"],
            THRESHOLDS["coverage_mcse_multiplier"] * row.number("coverage_mcse"),
        )
        if abs(coverage - 0.95) > coverage_bound:
            raise QualificationError(f"{row.label}: correct-model coverage misses its gate")
        ratio = row.number("se_ratio")
        if not (
            THRESHOLDS["correct_se_ratio_lower"]
            <= ratio
            <= THRESHOLDS["correct_se_ratio_upper"]
        ):
            raise QualificationError(f"{row.label}: empirical/estimated SE ratio misses its gate")
        checked.append(row.label)
    return checked


def _mild_gate(by_key: dict[tuple[str, int, str], Row], profile: str) -> list[str]:
    final_k = 12 if profile == "development" else 16
    checked: list[str] = []
    for shape in ("diffuse", "dominant"):
        for suffix in ("mild_functional", "mild_omitted"):
            cell = f"{shape}_{suffix}"
            base_cell = f"{shape}_common"
            for target in TARGETS:
                row = by_key[(cell, final_k, target)]
                if row.text("reference") == "Q1" and target == "covariance":
                    continue
                if row.integer("successes") / row.integer("replications") < THRESHOLDS[
                    "mild_success_rate"
                ]:
                    raise QualificationError(f"{row.label}: mild-case success rate is too low")
                coverage = row.number("coverage")
                if coverage < THRESHOLDS["mild_coverage_floor"]:
                    raise QualificationError(f"{row.label}: mild misspecification coverage is too low")
                base = by_key[(base_cell, final_k, target)]
                combined_mcse = math.hypot(
                    row.number("coverage_mcse"), base.number("coverage_mcse")
                )
                allowed_drop = THRESHOLDS["mild_coverage_drop"] + 3.0 * combined_mcse
                if base.number("coverage") - coverage > allowed_drop:
                    raise QualificationError(
                        f"{row.label}: mild misspecification degradation is too large"
                    )
                checked.append(row.label)
    return checked


def _severe_receipt(by_key: dict[tuple[str, int, str], Row], profile: str) -> dict[str, object]:
    final_k = 12 if profile == "development" else 16
    values = []
    for shape in ("diffuse", "dominant"):
        for target in TARGETS:
            row = by_key[(f"{shape}_severe_omitted", final_k, target)]
            if row.integer("successes") / row.integer("replications") < THRESHOLDS[
                "severe_success_rate"
            ]:
                raise QualificationError(f"{row.label}: severe-case execution rate is too low")
            values.append(
                {
                    "row": row.label,
                    "coverage": row.number("coverage"),
                    "se_ratio": row.number("se_ratio"),
                }
            )
    visibly_misspecified = any(
        value["coverage"] < 0.90
        or value["coverage"] > 0.985
        or value["se_ratio"] < 0.85
        or value["se_ratio"] > 1.15
        for value in values
    )
    if profile == "confirmation" and not visibly_misspecified:
        raise QualificationError(
            "severe omitted-variance cases do not reveal the structured model's limitation"
        )
    return {"visibly_misspecified": visibly_misspecified, "rows": values}


def validate(path: Path, profile: str, repeat: Path | None = None) -> dict[str, object]:
    rows, digest = _read(path)
    if repeat is not None:
        _, repeat_digest = _read(repeat)
        if path.read_bytes() != repeat.read_bytes():
            raise QualificationError("repeat run is not byte-for-byte deterministic")
    else:
        repeat_digest = None
    by_key = _check_integrity(rows, profile)
    _check_spectral_cases(by_key, profile)
    if profile == "confirmation":
        correct = _correct_gate(rows)
        mild = _mild_gate(by_key, profile)
    else:
        for row in rows:
            if row.text("gate") != "diagnostic" and row.integer("successes") < 2:
                raise QualificationError(f"{row.label}: development cell did not execute")
        correct = [row.label for row in rows if _promotion_row(row)]
        mild = [row.label for row in rows if row.text("gate") == "mild"]
    final_k = 12 if profile == "development" else 16
    q1_nonprimary = [
        by_key[(cell, final_k, "covariance")]
        for cell in (
            "dominant_common",
            "dominant_leverage",
            "dominant_common_t8",
            "dominant_common_controls",
        )
    ]
    if profile == "confirmation":
        for row in q1_nonprimary:
            if row.integer("successes") / row.integer("replications") < THRESHOLDS[
                "q1_nonprimary_success_rate"
            ]:
                raise QualificationError(f"{row.label}: q=1 atomic usability is too low")
    severe = _severe_receipt(by_key, profile)
    return {
        "schema": SCHEMA,
        "status": "PASS",
        "decision": "promotion_evidence_pass" if profile == "confirmation" else "development_pass",
        "profile": profile,
        "csv_sha256": digest,
        "repeat_sha256": repeat_digest,
        "rows": len(rows),
        "correct_rows_checked": correct,
        "mild_rows_checked": mild,
        "severe_misspecification": severe,
        "thresholds": THRESHOLDS,
        "interpretation": (
            "Assumption-conditional evidence for FEVC's structured common variance models; "
            "not evidence for unrestricted heteroskedastic KSS variance products. Spectral "
            "fixture thresholds validate the test designs and are not public routing cutoffs."
        ),
    }


def _parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("csv", type=Path)
    parser.add_argument("--profile", choices=("development", "confirmation"), required=True)
    parser.add_argument("--repeat", type=Path)
    parser.add_argument("--receipt", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = _parse_args(sys.argv[1:] if argv is None else argv)
    try:
        receipt = validate(arguments.csv, arguments.profile, arguments.repeat)
    except (OSError, QualificationError) as error:
        receipt = {
            "schema": SCHEMA,
            "status": "FAIL",
            "profile": arguments.profile,
            "error": str(error),
        }
        status = 1
    else:
        status = 0
    rendered = json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    if arguments.receipt is not None:
        arguments.receipt.write_text(rendered, encoding="utf-8")
    sys.stdout.write(rendered)
    return status


if __name__ == "__main__":
    raise SystemExit(main())
