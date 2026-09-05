#!/usr/bin/env python3
"""Independent, outcome-free fixed-offset covariance and quadratic-moment oracle.

This diagnostic never changes the production estimator or supplies corrected
standard errors to it. Covariance and quadratic kernels are diagonal plus
low rank, avoiding square matrices in the number of matches.
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import platform
import subprocess

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
REGISTRATION = Path("fevc/docs/fixed_offset_diagnostic_v1.json")
SOURCE_PATHS = (REGISTRATION, Path("fevc/tools/diagnose_fixed_offset.py"),
                Path("fevc/tests/python/test_fixed_offset_diagnostic.py"))
TARGETS = ("worker", "firm", "covariance", "total")


@dataclass
class Form:
    diagonal: np.ndarray
    factors: np.ndarray
    core: np.ndarray

    def apply(self, value):
        scale = self.diagonal if value.ndim == 1 else self.diagonal[:, None]
        return scale * value + self.factors @ (self.core @ (self.factors.T @ value))

    def diag(self):
        return self.diagonal + np.sum((self.factors @ self.core) * self.factors, axis=1)

    def trace_product(self, other):
        result = np.dot(self.diag(), other.diagonal)
        result += np.trace(other.core @ (other.factors.T @ self.apply(other.factors)))
        return float(result)

    def diagonal_sandwich_trace(self, variance):
        """tr(K D K D), including diagonal/low-rank cross terms."""
        gram = self.factors.T @ (variance[:, None] * self.factors)
        cross = self.factors.T @ ((self.diagonal * variance**2)[:, None] * self.factors)
        return float(np.dot(self.diagonal**2, variance**2)
                     + 2 * np.trace(self.core @ cross)
                     + np.trace(self.core @ gram @ self.core @ gram))

    def mixed_trace(self, variance, omega):
        """tr(K D K Omega), where Omega's base diagonal must equal D."""
        if not np.array_equal(variance, omega.diagonal):
            raise ValueError("mixed trace requires the same base diagonal")
        action = self.apply(omega.factors)
        return self.diagonal_sandwich_trace(variance) + float(
            np.trace(omega.core @ (action.T @ (variance[:, None] * action))))

    def sandwich_trace(self, omega):
        """tr(K Omega K Omega) without materializing either dense matrix."""
        action = self.apply(omega.factors)
        gram = omega.factors.T @ action
        cross = action.T @ (omega.diagonal[:, None] * action)
        return self.diagonal_sandwich_trace(omega.diagonal) + float(
            2 * np.trace(omega.core @ cross)
            + np.trace(omega.core @ gram @ omega.core @ gram))


def make_design(k, loadings):
    if not isinstance(k, int) or k < 3 or loadings not in ("historical", "bounded"):
        raise ValueError("invalid diagnostic design")
    groups = k * k
    counts = 2 + np.arange(groups) % 3
    group = np.repeat(np.arange(groups), counts)
    starts = np.r_[0, np.cumsum(counts)[:-1]]
    local = np.arange(len(group)) - np.repeat(starts, counts)
    worker, firm = group // k, group % k
    row = np.arange(len(group))
    frequency = (1 + (group * 17 + local * 11 + worker * 5 + firm * 3) % 13).astype(float)
    mass = np.bincount(group, weights=frequency)
    unique, inverse, ties = np.unique(mass, return_inverse=True, return_counts=True)
    del unique
    rank = (2 * (np.cumsum(ties) - 0.5 * ties) / groups - 1)[inverse]
    tau = 0.26 + 0.08 * rank
    fe_controls = np.column_stack((0.17 * worker - 0.11 * firm,
                                   -0.08 * worker + 0.14 * firm))
    within_controls = np.column_stack((0.37 * local + ((row * 7 + 3) % 19) / 23,
                                       -0.29 * local + ((row * 13 + 5) % 29) / 31))
    controls = fe_controls * (20 / k if loadings == "bounded" else 1) + within_controls
    wg, fg = np.arange(groups) // k, np.arange(groups) % k
    z = np.column_stack((np.eye(k)[wg], np.eye(k)[fg, :-1]))
    x = np.sqrt(mass)[:, None] * z
    inverse_gram = np.linalg.inv(x.T @ x)
    root = np.linalg.cholesky(inverse_gram)
    q = x @ root
    aggregate_controls = np.zeros((groups, 2))
    np.add.at(aggregate_controls, group, frequency[:, None] * controls)
    projected_controls = z @ (inverse_gram @ (z.T @ aggregate_controls))
    residual_controls = controls - projected_controls[group]
    control_gram = residual_controls.T @ (frequency[:, None] * residual_controls)
    h_transpose = (frequency[:, None] * residual_controls) @ np.linalg.inv(control_gram)
    u = aggregate_controls / np.sqrt(mass)[:, None]
    a = np.empty((groups, 2))
    gamma_covariance = np.zeros((2, 2))
    blocks = []
    for g, (start, count) in enumerate(zip(starts, counts)):
        indices = slice(start, start + count)
        freq = frequency[indices]
        correlation = 0.65 ** np.abs(np.subtract.outer(np.arange(count), np.arange(count)))
        sigma = correlation * (tau[g] * mass[g] / (freq @ correlation @ freq))
        h = h_transpose[indices]
        a[g] = freq @ sigma @ h / np.sqrt(mass[g])
        gamma_covariance += h.T @ sigma @ h
        blocks.append(sigma)
    correction = np.block([[gamma_covariance, -np.eye(2)], [-np.eye(2), np.zeros((2, 2))]])
    known = Form(tau, np.empty((groups, 0)), np.empty((0, 0)))
    estimated = Form(tau, np.column_stack((u, a)), correction)
    index, center = np.arange(k), (k - 1) / 2
    worker_effect = 2.5 * ((index - center) / k + 0.18 * np.sin((index + 1) * 0.73))
    firm_effect = 2.5 * (-0.8 * (index - center) / k + 0.14 * np.cos((index + 1) * 1.07))
    alpha = np.r_[worker_effect + firm_effect[-1], firm_effect[:-1] - firm_effect[-1]]
    mean = x @ alpha
    target_mass = 0.85 + ((wg * 19 + fg * 11 + 3) % 31) / 100
    target_mass[0] *= 1000
    weight = target_mass / target_mass.sum()
    zw = z.copy()
    zw[:, k:] = 0
    zf = z - zw
    aw = zw.T @ (weight[:, None] * zw) - np.outer(weight @ zw, weight @ zw)
    af = zf.T @ (weight[:, None] * zf) - np.outer(weight @ zf, weight @ zf)
    ac = zw.T @ (weight[:, None] * zf) - np.outer(weight @ zw, weight @ zf)
    ac = (ac + ac.T) / 2
    return dict(k=k, loadings=loadings, group=group, frequency=frequency, mass=mass,
                counts=counts, starts=starts, tau=tau, controls=controls, z=z, x=x,
                inverse_gram=inverse_gram, root=root, q=q, h_transpose=h_transpose,
                residual_controls=residual_controls, control_gram=control_gram,
                gamma_covariance=gamma_covariance, blocks=blocks, known=known,
                estimated=estimated, mean=mean, alpha=alpha, target_mass=target_mass,
                targets=(aw, af, ac, aw + af + 2 * ac))


def kernel(q, small_target):
    maker_diagonal = 1 - np.sum(q * q, axis=1)
    if maker_diagonal.min() <= 1e-10:
        raise ValueError("match deletion unidentified")
    ratio = np.sum((q @ small_target) * q, axis=1) / maker_diagonal
    p = q.shape[1]
    core = np.block([[small_target, 0.5 * np.eye(p)],
                     [0.5 * np.eye(p), np.zeros((p, p))]])
    return Form(-ratio, np.column_stack((q, ratio[:, None] * q)), core)


def moments(form, mean, omega, diagonal):
    action = form.apply(mean)
    linear = float(4 * action @ omega.apply(action))
    quadratic = 2 * form.sandwich_trace(omega)
    expected_naive = (float(4 * np.dot(action * action, diagonal))
                      + 4 * form.mixed_trace(diagonal, omega)
                      - 2 * form.diagonal_sandwich_trace(diagonal))
    total = linear + quadratic
    if total <= 0 or linear < -1e-10 or quadratic < -1e-10:
        raise ValueError("invalid population variance")
    return dict(bias=form.trace_product(omega), variance=total,
                linear_variance=linear, quadratic_variance=quadratic,
                expected_diagonal_variance_estimator=expected_naive,
                sd_over_sqrt_expected_diagonal_variance=(
                    float(np.sqrt(total / expected_naive)) if expected_naive > 0 else None))


def diagnose(design):
    q, root, mean = (design[name] for name in ("q", "root", "mean"))
    rows = []
    for name, a in zip(TARGETS, design["targets"]):
        small = root.T @ a @ root
        small = (small + small.T) / 2
        eigenvalues, vectors = np.linalg.eigh(small)
        order = np.argsort(-np.abs(eigenvalues), kind="stable")
        leading, second = eigenvalues[order[:2]]
        vector = vectors[:, order[0]]
        mode = q @ vector
        squares = float(eigenvalues @ eigenvalues)
        remainder_square = float(np.sum(eigenvalues[order[1:]]**2))
        full = kernel(q, small)
        remainder = kernel(q, small - leading * np.outer(vector, vector))
        if np.max(np.abs(full.diag())) > 1e-9:
            raise ValueError("leave-out kernel is not zero diagonal")
        result = dict(k=design["k"], loadings=design["loadings"], target=name,
                      matches=len(mean), stored_rows=len(design["group"]),
                      frequency_mass=float(design["mass"].sum()),
                      truth=float(design["alpha"] @ a @ design["alpha"]),
                      leading_share=float(leading**2 / squares),
                      remainder_concentration=float(second**2 / remainder_square),
                      maximum_mode_weight_squared=float(np.max(mode**2)),
                      minimum_maker_diagonal=float(1 - np.max(np.sum(q*q, axis=1))),
                      gamma_standard_deviations=np.sqrt(np.diag(design["gamma_covariance"])).tolist(),
                      within_control_condition=float(np.linalg.cond(design["control_gram"])))
        for arm in ("known", "estimated"):
            omega = design[arm]
            point = moments(full, mean, omega, design["tau"])
            rem = moments(remainder, mean, omega, design["tau"])
            action = remainder.apply(mean)
            leading_variance = float(mode @ omega.apply(mode))
            cross = float(2 * mode @ omega.apply(action))
            naive_leading = float(np.dot(mode * mode, design["tau"]))
            naive_cross = float(2 * np.dot(mode * design["tau"], action))
            result[arm] = dict(point=point, remainder=rem,
                               leading_variance=leading_variance,
                               leading_remainder_covariance=cross,
                               population_standardized_determinant=1-cross**2/(leading_variance*rem["variance"]),
                               diagonal_leading_variance=naive_leading,
                               expected_diagonal_cross_covariance=naive_cross)
        result["estimated_over_known_point_sd"] = float(np.sqrt(
            result["estimated"]["point"]["variance"] / result["known"]["point"]["variance"]))
        rows.append(result)
    return rows


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_export(path):
    """Check baseline generator and exact fit using production's probe diagonal."""
    records = [json.loads(line) for line in path.read_text().splitlines()]
    inputs = [r for r in records if r.get("kind") == "input"]
    states = [r for r in records if r.get("kind") == "state"]
    rows = [r for r in records if "target" in r]
    if len(inputs) != 1 or len(states) != 1 or len(rows) != 4:
        raise ValueError("incomplete production export")
    inputs, state = inputs[0], states[0]
    if inputs["cell"] != "controls_varying_fixedoffset":
        raise ValueError("wrong production design")
    k = len(set(inputs["worker"]))
    design = make_design(k, "historical")
    expected = dict(worker=10000 + design["group"]//k,
                    firm=20000 + design["group"]%k,
                    deletion=100000 + design["group"], frequency=design["frequency"],
                    controls=design["controls"].T, aggregate_variance=design["tau"],
                    target_weight=design["target_mass"][design["group"]]/design["counts"][design["group"]])
    differences = {}
    for key, values in expected.items():
        actual = np.asarray(inputs[key])
        if actual.shape != values.shape or not np.allclose(actual, values, rtol=1e-12, atol=1e-12):
            raise ValueError(f"production input mismatch: {key}")
        differences[key] = float(np.max(np.abs(actual-values)))
    outcome = np.asarray(inputs["outcome"])
    gamma = design["h_transpose"].T @ outcome
    adjusted = outcome-design["controls"]@gamma
    y = np.bincount(design["group"], weights=design["frequency"]*adjusted)/np.sqrt(design["mass"])
    beta = design["inverse_gram"]@design["x"].T@y
    residual = y-design["x"]@beta
    source_diagonal = np.asarray(state["target_diagonal"])
    source_diagonal = np.vstack((source_diagonal, source_diagonal[0]+source_diagonal[1]+2*source_diagonal[2]))
    for index, (target, a) in enumerate(zip(TARGETS, design["targets"])):
        matches = [r for r in rows if r["target"] == target]
        if len(matches) != 1:
            raise ValueError("production target inventory mismatch")
        row = matches[0]
        point = beta@a@beta - np.dot(source_diagonal[index]*np.asarray(state["maker_inverse"]), y*residual)
        truth = design["alpha"]@a@design["alpha"]
        differences[f"{target}_point"] = float(abs(point-row["point_estimate"]))
        differences[f"{target}_truth"] = float(abs(truth-row["truth"]))
        if differences[f"{target}_point"] > 1e-8 or differences[f"{target}_truth"] > 1e-10:
            raise ValueError("production point/truth mismatch")
    return dict(status="PASS", export_sha256=digest(path), k=k, differences=differences,
                comparison="independent exact fit with exported production randomized correction diagonal")


def source_identity():
    status = subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True)
    if status.strip():
        raise ValueError("diagnostic requires a clean committed source")
    return dict(commit=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                files={str(path): digest(ROOT / path) for path in SOURCE_PATHS})


def create_manifest(path):
    registration = json.loads((ROOT / REGISTRATION).read_text())
    manifest = dict(schema="fevc-fixed-offset-diagnostic-manifest-v1", source=source_identity(),
                    registration=registration, expected_rows=registration["expected_target_rows"],
                    outputs=["rows.json", "receipt.json"], random_outcomes=False)
    with path.open("x") as handle:
        handle.write(json.dumps(manifest, indent=2, allow_nan=False) + "\n")


def validate_rows(rows, registration):
    expected = {(case["k"], case["loadings"], target)
                for case in registration["cases"] for target in TARGETS}
    keys = [(r["k"], r["loadings"], r["target"]) for r in rows]
    if len(keys) != len(set(keys)) or set(keys) != expected:
        raise ValueError("incomplete or duplicate case-target inventory")
    if len(rows) != registration["expected_target_rows"]:
        raise ValueError("unexpected row count")
    json.dumps(rows, allow_nan=False)


def run(manifest_path, output):
    manifest = json.loads(manifest_path.read_text())
    if manifest["schema"] != "fevc-fixed-offset-diagnostic-manifest-v1":
        raise ValueError("wrong manifest schema")
    registration = json.loads((ROOT / REGISTRATION).read_text())
    if (manifest["source"] != source_identity() or manifest["registration"] != registration
            or manifest["expected_rows"] != registration["expected_target_rows"]
            or manifest["outputs"] != ["rows.json", "receipt.json"]
            or manifest["random_outcomes"] is not False):
        raise ValueError("manifest identity mismatch")
    output.mkdir(parents=True, exist_ok=False)
    rows = []
    for case in registration["cases"]:
        rows.extend(diagnose(make_design(**case)))
        print(f"completed {case}", flush=True)
    validate_rows(rows, registration)
    temporary = output / "rows.json.partial"
    temporary.write_text(json.dumps(rows, indent=2, allow_nan=False) + "\n")
    temporary.rename(output / "rows.json")
    receipt = dict(schema="fevc-fixed-offset-diagnostic-receipt-v1", status="COMPLETE_DIAGNOSTIC",
                   source=manifest["source"], manifest_sha256=digest(manifest_path),
                   rows_sha256=digest(output / "rows.json"), rows=len(rows),
                   python=platform.python_version(), numpy=np.__version__,
                   platform=platform.platform(), random_outcomes=False, coverage_claim=False)
    (output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    create = sub.add_parser("create-manifest")
    create.add_argument("manifest", type=Path)
    execute = sub.add_parser("run")
    execute.add_argument("manifest", type=Path)
    execute.add_argument("output", type=Path)
    verify = sub.add_parser("verify-export")
    verify.add_argument("export", type=Path)
    args = parser.parse_args()
    if args.command == "create-manifest":
        create_manifest(args.manifest)
    elif args.command == "run":
        run(args.manifest, args.output)
    else:
        print(json.dumps(verify_export(args.export), indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
