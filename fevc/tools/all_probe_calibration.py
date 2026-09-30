"""Prospective small-system raw-covariance audit; never a native qualification.

Consumes the complete dense reference inventory. Fixed contrasts, independent
64-by-64 blocks and delta-method ratio uncertainty implement issue #7 section
12.B. Missing raw diagnostics cannot shrink the calibration population.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path

import numpy as np

if __package__:
    from .all_probe_reference import validate_inventory
else:
    from all_probe_reference import validate_inventory


CONTRASTS = np.array([[1,0,0], [0,1,0], [0,0,1], [1,1,0], [1,-1,0],
                      [1,0,1], [1,0,-1], [0,1,1], [0,1,-1], [1,1,2]])
POLICY = dict(blocks=64, block_size=64, relative_limit=.15, uncertainty_multiplier=2.576,
              minimum_usable_rate=.99, maximum_point_failures=0, near_zero_relative=1e-10)


def rate_interval(successes, total):
    """Wilson 95% interval; observed gates do not assert zero population risk."""
    z = 1.959963984540054
    rate = successes / total
    denominator = 1 + z*z/total
    center = (rate + z*z/(2*total)) / denominator
    radius = z * math.sqrt(rate*(1-rate)/total + z*z/(4*total*total)) / denominator
    return [max(0., center-radius), min(1., center+radius)]


def contrast_screen(estimated, reference, policy=POLICY):
    estimated, reference = np.asarray(estimated), np.asarray(reference)
    if estimated.shape != (64,3,3) or reference.shape != (64,3,3) or not np.isfinite([estimated,reference]).all():
        raise ValueError("calibration needs 64 complete finite covariance pairs")
    answers = []
    scale = max(np.max(np.abs(reference)), np.max(np.abs(estimated)))
    if scale == 0:
        # Sampled zeros do not establish a deterministic zero direction.
        return [dict(contrast=c.tolist(), status="near_zero_inconclusive",
                     estimated_variance=0., reference_variance=0.,
                     mean_difference=0., difference_standard_error=0.) for c in CONTRASTS]
    estimated, reference = estimated / scale, reference / scale
    typical = np.max(np.abs(reference.mean(axis=0)))
    for c in CONTRASTS:
        a = np.einsum('i,kij,j->k', c, estimated, c)
        b = np.einsum('i,kij,j->k', c, reference, c)
        ma, mb = a.mean(), b.mean()
        difference = a-b
        difference_se = np.std(difference, ddof=1) / math.sqrt(len(a))
        common = dict(contrast=c.tolist(), estimated_variance=float(ma*scale),
                      reference_variance=float(mb*scale), mean_difference=float((ma-mb)*scale),
                      difference_standard_error=float(difference_se*scale))
        if mb <= policy["near_zero_relative"] * typical * (c @ c):
            answers.append(dict(common, status="near_zero_inconclusive"))
            continue
        discrepancy = ma / mb - 1
        # Joint delta method includes covariance between the estimated and
        # repeated-run block variances; treating the denominator as fixed is wrong.
        influence = ((a-ma) - (ma/mb)*(b-mb)) / mb
        se = np.std(influence, ddof=1) / math.sqrt(len(a))
        bound = abs(discrepancy) + policy["uncertainty_multiplier"]*se
        answers.append(dict(common, relative_discrepancy=float(discrepancy),
                            relative_standard_error=float(se), equivalence_bound=float(bound),
                            status="pass" if bound <= policy["relative_limit"] else "fail"))
    return answers


def audit(result):
    native=result.get('schema')=='FEVC_ALL_PROBE_NATIVE_RESULT_V1'
    if native:
        if __package__:
            from .all_probe_native import validate_inventory as native_validate, check_audit_dependencies
        else:
            from all_probe_native import validate_inventory as native_validate, check_audit_dependencies
        check_audit_dependencies(result['manifest'])
        native_validate(result)
    else:
        validate_inventory(result)
    m = result["manifest"]
    profile='confirmation_native_small_v1' if native else 'confirmation_dense_small_v1'
    if m.get("profile") != profile or m.get("acceptance") != POLICY:
        raise ValueError("confirmation policy was not frozen exactly")
    if (m["K"],m["L"]) != (4096,1) or (m["R"],m["T"]) not in ((200,200),(400,200),(200,400)):
        raise ValueError("invalid prospective public-budget confirmation inventory")
    if m.get("calibration_sha256") != hashlib.sha256(Path(__file__).read_bytes()).hexdigest():
        raise ValueError("frozen calibration source mismatch")
    attempts = sorted(result["attempts"], key=lambda a: a["key"])
    failures = result["summary"]["point_failures"]
    raw_count = result["summary"]["finite_raw_count"]
    usable = result["summary"]["usable_diagnostics"]
    answer = dict(schema="FEVC_ALL_PROBE_CALIBRATION_AUDIT_V1", manifest=m,
                  implementation="rust_core_counter_v1" if native else "independent_dense_python_only", attempted=4096,
                  point_failures=failures, usable_diagnostics=usable, finite_raw_count=raw_count,
                  point_failure_rate=len(failures)/4096, usable_rate=usable/4096,
                  point_failure_interval=rate_interval(len(failures),4096),
                  usable_interval=rate_interval(usable,4096),
                  conditioning=result["summary"]["conditioning"],
                  true_maker_margin=result["summary"]["true_maker_margin"],
                  acceptance_pass=False)
    if failures or raw_count != 4096:
        answer.update(status="failed_or_incomplete", contrasts=[])
        return answer
    points = np.array([a["point"] for a in attempts]).reshape(64,64,3)
    raw = np.array([a["all_raw"] for a in attempts]).reshape(64,64,3,3)
    reference = np.array([np.cov(block.T, ddof=1) for block in points])
    screens = contrast_screen(raw.mean(axis=1), reference)
    passed = usable/4096 >= POLICY["minimum_usable_rate"] and all(c["status"] == "pass" for c in screens)
    answer.update(status="pass" if passed else "fail_or_inconclusive", contrasts=screens,
                  acceptance_pass=passed, bias_against_exact=result["summary"].get("bias_against_exact"))
    return answer


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    answer = audit(json.loads(args.result.read_text()))
    tmp = args.output.with_suffix(args.output.suffix + ".tmp")
    tmp.write_text(json.dumps(answer, indent=2, allow_nan=False) + "\n")
    tmp.replace(args.output)
    return 0 if answer["acceptance_pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
