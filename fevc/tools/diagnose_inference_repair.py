#!/usr/bin/env python3
"""Paired historical replay and independent dense q1 covariance diagnosis.

This is development diagnosis, not confirmation evidence. Inputs are synthetic
and all attempted targets are retained, including unavailable intervals.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path

import numpy as np

TARGETS = ("worker", "firm", "covariance", "total")
CELLS = {
    "one_mode_equal_independent": 12,
    "one_mode_leverage_sensitivity": 11,
    "one_mode_equal_independent_cmg": 8,
}


def selection(rows):
    output = {}
    for cell, expected in CELLS.items():
        failed = {r["replication"] for r in rows if r["cell"] == cell and r["status"] != "success"}
        if len(failed) != expected:
            raise ValueError(f"{cell}: expected {expected} historical failures, got {len(failed)}")
        output[cell] = {
            "failed": sorted(failed),
            "selected": sorted({j for r in failed for j in (r - 1, r, r + 1) if 0 <= j < 400}),
        }
    return output


def covariance(mode, action, kernel, variance):
    leading = float(np.dot(mode * mode, variance))
    cross = float(2 * np.dot(mode * variance, action))
    influence = float(4 * np.dot(action * action, variance))
    trace = float(2 * np.sum(kernel * kernel * variance[:, None] * variance[None, :]))
    remainder = influence - trace
    determinant = 1 - cross * cross / (leading * remainder) if leading > 0 and remainder > 0 else None
    return dict(leading_variance=leading, cross_covariance=cross,
                influence_term=influence, trace_correction=trace,
                remainder_variance=remainder, standardized_determinant=determinant,
                available=determinant is not None and determinant > 1e-8)


def dense_replay(inputs, state, results):
    worker = np.unique(inputs["worker"], return_inverse=True)[1]
    firm = np.unique(inputs["firm"], return_inverse=True)[1]
    group = np.unique(inputs["deletion"], return_inverse=True)[1]
    nw, nf, ng = worker.max() + 1, firm.max() + 1, group.max() + 1
    zw = np.eye(nw)[worker]
    zf = np.eye(nf)[firm, :-1]
    z = np.column_stack((zw, zf))
    frequency = np.asarray(inputs["frequency"], dtype=float)
    outcome = np.asarray(inputs["outcome"], dtype=float)
    controls = np.asarray(inputs["controls"], dtype=float)
    if controls.size:
        full = np.column_stack((z, controls.T))
        beta = np.linalg.solve(full.T @ (frequency[:, None] * full), full.T @ (frequency * outcome))
        outcome = outcome - controls.T @ beta[z.shape[1]:]
    mass = np.bincount(group, weights=frequency)
    x = np.zeros((ng, z.shape[1]))
    np.add.at(x, group, frequency[:, None] * z)
    x /= np.sqrt(mass)[:, None]
    y = np.bincount(group, weights=frequency * outcome) / np.sqrt(mass)
    inv = np.linalg.inv(x.T @ x)
    root = np.linalg.cholesky(inv)
    loading = x @ inv
    maker = np.eye(ng) - loading @ x.T
    weight = np.asarray(inputs["target_weight"], dtype=float)
    weight /= weight.sum()
    zw = np.column_stack((zw, np.zeros_like(zf)))
    zf = np.column_stack((np.zeros((len(zf), nw)), zf))
    aw = zw.T @ (weight[:, None] * zw) - np.outer(weight @ zw, weight @ zw)
    af = zf.T @ (weight[:, None] * zf) - np.outer(weight @ zf, weight @ zf)
    ac = zw.T @ (weight[:, None] * zf) - np.outer(weight @ zw, weight @ zf)
    ac = (ac + ac.T) / 2
    targets = (aw, af, ac, aw + af + 2 * ac)
    source_diagonal = np.asarray(state["target_diagonal"])
    source_diagonal = np.vstack((source_diagonal, source_diagonal[0] + source_diagonal[1] + 2 * source_diagonal[2]))
    source_maker_inverse = np.asarray(state["maker_inverse"])
    fitted = np.asarray(state["fitted_variance"])
    known = np.asarray(inputs["aggregate_variance"])
    output = []
    for target, a in enumerate(targets):
        row = next(r for r in results if r["target"] == TARGETS[target])
        if "leading_eigenvalue" not in row:
            continue
        eigenvalues, eigenvectors = np.linalg.eigh(root.T @ a @ root)
        mode_index = np.argmin(abs(eigenvalues - row["leading_eigenvalue"]))
        eigenvalue = float(eigenvalues[mode_index])
        mode = x @ root @ eigenvectors[:, mode_index]
        mode /= np.linalg.norm(mode)
        if float(mode @ y) * row["leading_score"] < 0:
            mode = -mode
        b = loading @ a @ loading.T
        remainder_b = b - eigenvalue * np.outer(mode, mode)
        ratio = (source_diagonal[target] - eigenvalue * mode * mode) * source_maker_inverse
        kernel = remainder_b - (ratio[:, None] * maker + maker * ratio[None, :]) / 2
        action = kernel @ y
        fitted_covariance = covariance(mode, action, kernel, fitted)
        known_covariance = covariance(mode, action, kernel, known)
        exact_ratio = np.diag(remainder_b) / np.diag(maker)
        exact_kernel = remainder_b - (exact_ratio[:, None] * maker + maker * exact_ratio[None, :]) / 2
        exact_covariance = covariance(mode, exact_kernel @ y, exact_kernel, known)
        differences = {
            "leading_variance": fitted_covariance["leading_variance"] - row["leading_variance"],
            "cross_covariance": fitted_covariance["cross_covariance"] - row["leading_remainder_covariance"],
            "influence_term": fitted_covariance["influence_term"] - row["remainder_influence_variance"],
            "leading_score": float(mode @ y) - row["leading_score"],
            "direct_remainder": float(y @ action) - row["remainder_estimate"],
        }
        if max(abs(v) for v in differences.values()) > 1e-8:
            raise ValueError(f"dense/production state mismatch: {row['cell']} {row['replication']} {row['target']} {differences}")
        old_determinant = row["leading_variance"] * row["remainder_variance"] - row["leading_remainder_covariance"] ** 2
        old_gate = old_determinant > 1e-8 * max(row["leading_variance"], row["remainder_variance"]) ** 2
        output.append(dict(cell=row["cell"], replication=row["replication"], target=row["target"],
                           seed=row["semantic_seed"], production_status=row["status"],
                           q1_status=row["q1_status"], legacy_gate_on_replay_covariance=old_gate,
                           production_trace=row["remainder_trace_variance"], trace_mcse=row["remainder_trace_mcse"],
                           exact_trace_fitted=fitted_covariance, exact_trace_known=known_covariance,
                           exact_kernel_known=exact_covariance, dense_production_differences=differences))
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--historical", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--limit", type=int, help="Non-evidentiary harness smoke only")
    args = parser.parse_args()
    original = [json.loads(line) for line in args.historical.read_text().splitlines()]
    chosen = selection(original)
    args.output.mkdir(parents=True, exist_ok=False)
    (args.output / "selection.json").write_text(json.dumps(chosen, indent=2) + "\n")
    rows, comparison, count = [], [], 0
    for cell, selected in chosen.items():
        for replication in selected["selected"]:
            if args.limit is not None and count >= args.limit:
                break
            command = [str(args.binary.resolve()), "repair-replay-v1", cell, "20", str(replication), "1", "256", "512", "128", "256", "4000"]
            run = subprocess.run(command, capture_output=True, text=True, check=False)
            stem = args.output / f"{cell}-{replication:04d}"
            stem.with_suffix(".jsonl").write_text(run.stdout)
            stem.with_suffix(".stderr").write_text(run.stderr)
            if run.returncode:
                raise RuntimeError(f"replay process failed: {command}, exit {run.returncode}")
            records = [json.loads(line) for line in run.stdout.splitlines()]
            inputs = [r for r in records if r.get("kind") == "input"]
            states = [r for r in records if r.get("kind") == "state"]
            targets = [r for r in records if "target" in r]
            if len(inputs) != 1 or len(targets) != 4 or {r["target"] for r in targets} != set(TARGETS):
                raise ValueError(f"incomplete replay inventory: {cell}/{replication}")
            rows.extend(targets)
            if len(states) == 1:
                comparison.extend(dense_replay(inputs[0], states[0], targets))
            elif any(r["status"] == "success" for r in targets):
                raise ValueError("successful target without exported state")
            count += 1
            print(f"replayed {cell}/{replication}: {[r['status'] for r in targets]}", flush=True)
    (args.output / "comparison.json").write_text(json.dumps(comparison, indent=2, allow_nan=False) + "\n")
    (args.output / "rows.jsonl").write_text("".join(json.dumps(r, allow_nan=False) + "\n" for r in rows))
    receipt = dict(role="development_diagnosis", replications=count, target_rows=len(rows),
                   dense_comparisons=len(comparison), complete=args.limit is None,
                   binary_sha256=hashlib.sha256(args.binary.read_bytes()).hexdigest(),
                   historical_sha256=hashlib.sha256(args.historical.read_bytes()).hexdigest())
    (args.output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt), flush=True)


if __name__ == "__main__":
    main()
