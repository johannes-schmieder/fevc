#!/usr/bin/env python3
"""Deterministic review of existing RC outputs; no estimator calls or RNG."""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import hashlib
import json
import math
from pathlib import Path
import statistics


Z95 = 1.959963984540054


def sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def log_interval(value, mcse):
    return [value * math.exp(sign * Z95 * mcse) for sign in (-1, 1)]


def ratio_diagnostics(errors, standard_errors):
    """Paired delta influence and exact delete-one jackknife for SD/mean(SE)."""
    n = len(errors)
    if (n < 4 or len(standard_errors) != n
            or any(not math.isfinite(x) for x in errors)
            or any(not math.isfinite(s) or s <= 0 for s in standard_errors)):
        raise ValueError("need at least four finite paired errors and positive SEs")
    mean_error = statistics.fmean(errors)
    mean_se = statistics.fmean(standard_errors)
    squared = [(x - mean_error) ** 2 for x in errors]
    variance_n = statistics.fmean(squared)
    if variance_n <= 0:
        raise ValueError("point errors must have positive variance")
    empirical_sd = math.sqrt(variance_n * n / (n - 1))
    ratio = empirical_sd / mean_se
    mean_variance = statistics.fmean(s * s for s in standard_errors)
    rms_se = math.sqrt(mean_variance)
    # Both performance measures come from the same replications. Their
    # covariance matters; treating the mean SE as fixed is not this delta SE.
    influence = [(a - variance_n) / (2 * variance_n) - (s - mean_se) / mean_se
                 for a, s in zip(squared, standard_errors)]
    log_mcse = statistics.stdev(influence) / math.sqrt(n)
    rms_influence = [(a - variance_n) / (2 * variance_n)
                     - (s * s - mean_variance) / (2 * mean_variance)
                     for a, s in zip(squared, standard_errors)]
    rms_log_mcse = statistics.stdev(rms_influence) / math.sqrt(n)
    loo_ratios = []
    for a, s in zip(squared, standard_errors):
        loo_variance = (n * variance_n - n * a / (n - 1)) / (n - 2)
        loo_se = (n * mean_se - s) / (n - 1)
        if loo_variance <= 0 or loo_se <= 0:
            raise ValueError("delete-one ratio is unidentified")
        loo_ratios.append(math.sqrt(loo_variance) / loo_se)
    loo_logs = [math.log(x) for x in loo_ratios]
    mean_loo_log = statistics.fmean(loo_logs)
    jackknife_log_mcse = math.sqrt((n - 1) / n * math.fsum(
        (x - mean_loo_log) ** 2 for x in loo_logs))
    return {
        "replications": n, "bias": mean_error, "empirical_sd": empirical_sd,
        "mean_se": mean_se, "se_ratio": ratio,
        "delta_log_ratio_mcse": log_mcse, "delta_ratio_mcse": ratio * log_mcse,
        "delta_ratio_interval95": log_interval(ratio, log_mcse),
        "jackknife_log_ratio_mcse": jackknife_log_mcse,
        "jackknife_ratio_mcse": ratio * jackknife_log_mcse,
        "jackknife_ratio_interval95": log_interval(ratio, jackknife_log_mcse),
        "delete_one_ratio_range": [min(loo_ratios), max(loo_ratios)],
        "rms_se": rms_se, "sd_over_rms_se": empirical_sd / rms_se,
        "rms_ratio_interval95": log_interval(empirical_sd / rms_se, rms_log_mcse),
        "rms_over_mean_se": rms_se / mean_se,
        "mean_variance_over_empirical_variance": mean_variance / empirical_sd ** 2,
        "variance_ratio_interval95": log_interval(
            mean_variance / empirical_sd ** 2, 2 * rms_log_mcse),
        "point_error_kurtosis": statistics.fmean(a * a for a in squared) / variance_n ** 2,
    }


def review(raw: Path, result_path: Path):
    accepted = json.loads(result_path.read_text())
    audit = accepted["audit"]
    expected = audit["hashes"]["output/aggregate/aggregate.jsonl"]
    if sha256(raw) != expected:
        raise ValueError("raw output does not match the frozen audit hash")
    original = {(x["cell"], x["k"], x["target"]): x for x in audit["summaries"]}
    groups = defaultdict(list)
    statuses = Counter()
    seen = set()
    with raw.open() as stream:
        for line in stream:
            row = json.loads(line)
            key = (row["cell"], row["k"], row["target"])
            attempt = (*key, row["replication"])
            if attempt in seen or key not in original:
                raise ValueError("duplicate or unexpected target attempt")
            seen.add(attempt)
            statuses[row["status"]] += 1
            groups[key].append(row)
    if len(seen) != audit["row_count"] or dict(statuses) != audit["attempt_status_counts"]:
        raise ValueError("full attempt accounting does not match the audit")
    output = []
    for key, summary in sorted(original.items()):
        rows = sorted(groups[key], key=lambda x: x["replication"])
        if [x["replication"] for x in rows] != list(range(summary["attempts"])):
            raise ValueError("missing or unexpected replication keys")
        primary = summary["gate"] == "correct" and (
            summary["reference"] == "Q0" or summary["target"] != "covariance")
        if not primary:
            continue
        if any(x["status"] != "success" for x in rows):
            raise ValueError("this all-attempt primary review requires full availability")
        errors = [x["point_error"] for x in rows]
        ses = [x["estimated_sd"] for x in rows]
        diagnostics = ratio_diagnostics(errors, ses)
        if not math.isclose(diagnostics["se_ratio"], summary["se_ratio"], rel_tol=1e-12):
            raise ValueError("independent ratio differs from frozen summary")
        coverage = statistics.fmean(row["covered"] for row in rows)
        if coverage != summary["coverage"]:
            raise ValueError("independent coverage differs from frozen summary")
        coverage_mcse = math.sqrt(coverage * (1 - coverage) / len(rows))
        diagnostics.update({
            "cell": key[0], "k": key[1], "target": key[2],
            "reference": summary["reference"], "availability": 1.0,
            "coverage": coverage, "coverage_mcse": coverage_mcse,
            "coverage_interval95": [max(0, coverage - Z95 * coverage_mcse),
                                    min(1, coverage + Z95 * coverage_mcse)],
        })
        output.append(diagnostics)
    if len(output) != 39:
        raise ValueError("expected exactly the 39 original primary target rows")
    by_key = {(row["cell"], row["k"], row["target"]): row for row in output}
    failed = by_key[("dominant_leverage", 16, "firm")]
    common = by_key[("dominant_common", 16, "firm")]
    difference = failed["se_ratio"] - common["se_ratio"]
    difference_mcse = math.hypot(failed["delta_ratio_mcse"], common["delta_ratio_mcse"])
    coverage_difference = failed["coverage"] - common["coverage"]
    coverage_difference_mcse = math.hypot(failed["coverage_mcse"], common["coverage_mcse"])
    cutoff = audit["thresholds"]["correct_se_ratio_upper"]
    return {
        "schema": "FEVC_RC_OBSERVATION_RATIO_REVIEW_V1", "status": "DIAGNOSTIC_ONLY",
        "original_confirmation_status": accepted["status"],
        "original_scientific_failures": audit["scientific_failures"],
        "scientific_source": accepted["scientific_source"],
        "inputs": {"raw_sha256": expected, "result_sha256": sha256(result_path),
                   "review_script_sha256": sha256(Path(__file__))},
        "full_attempt_count": len(seen), "attempt_status_counts": dict(statuses),
        "reviewed_primary_rows": len(output), "rows": output,
        "failed_row_cutoff": cutoff,
        "failed_row_excess": failed["se_ratio"] - cutoff,
        "failed_row_excess_in_mcse": (failed["se_ratio"] - cutoff) / failed["delta_ratio_mcse"],
        "unpaired_common_comparison": {
            "ratio_difference": difference, "difference_mcse": difference_mcse,
            "difference_interval95": [difference - Z95 * difference_mcse,
                                      difference + Z95 * difference_mcse],
            "coverage_difference": coverage_difference,
            "coverage_difference_mcse": coverage_difference_mcse,
            "coverage_difference_interval95": [coverage_difference - Z95 * coverage_difference_mcse,
                                               coverage_difference + Z95 * coverage_difference_mcse],
            "limitation": "Different variance DGPs and independent semantic outcome seeds; not a paired comparison of two variance models on identical data.",
        },
        "interpretation_limits": [
            "Deterministic post-result diagnosis, not a new simulation or confirmation.",
            "MC intervals quantify uncertainty from finite independent replications, not uncertainty for an empirical FEVC estimate.",
            "Pointwise delta/jackknife approximations, not simultaneous or selection-adjusted intervals.",
            "RMS and mean-variance diagnostics do not replace the registered mean-SE ratio gate.",
            "No rows are dropped, cutoffs changed, failures waived or public routes promoted.",
        ],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw", type=Path)
    parser.add_argument("--result", type=Path,
                        default=Path("fevc/docs/rc_observation_inference_v1_result.json"))
    args = parser.parse_args()
    print(json.dumps(review(args.raw, args.result), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
