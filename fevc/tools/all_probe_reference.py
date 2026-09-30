"""Independent, deliberately dense small-system reference for issue #7.

This is a development oracle, not a production estimator. Physical expansion,
dense inverses and cached responses make all copy and cross-unit terms explicit.
No native influence-covariance helper is imported. Seeds here use NumPy's PCG64
and do not claim parity with Counter-V1 or Stata's RNG.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import subprocess
import time
from pathlib import Path

import numpy as np


def finite_terms(u, r):
    """Unclipped analytic expression; complex inputs support differentiation."""
    p, m, a, b, c = u
    h, residual = p / (p + m), m / (p + m)
    bias = (residual * a - h * b + (residual - h) * c) / r
    variance = (residual**2 * a + h**2 * b - 2 * h * residual * c) / r
    return h, residual, bias, variance


def observation_inverse(u, r, control=0):
    _, m, b, v = finite_terms(u, r)
    ell = m - control
    return 1 / ell + b / ell**2 - v / ell**3


def block_adjustment(u, r, controls, direction, residual):
    h, _, b, v = finite_terms(u, r)
    maker = np.eye(len(direction)) - controls - h * np.outer(direction, direction)
    a = np.linalg.solve(maker, residual)
    w = np.linalg.solve(maker, direction)
    return a + (b - v * (direction @ w)) * w * (direction @ a)


def derivative(fun, u):
    """Independent complex-step derivative of the complete expression."""
    return np.stack([
        np.imag(fun(np.asarray(u, dtype=complex) + 1e-25j * basis)) / 1e-25
        for basis in np.eye(5)
    ], axis=-1)


def moments(projected, residual):
    projected, residual = np.broadcast_arrays(projected, residual)
    return np.stack((projected**2, residual**2, projected**4,
                     residual**4, projected**2 * residual**2), axis=-1)


def mean_covariance(draws):
    draws = np.asarray(draws, dtype=float)
    if draws.ndim != 2 or len(draws) < 2 or not np.isfinite(draws).all():
        raise ValueError("covariance needs at least two finite vector draws")
    return cross_covariance(draws, draws)


def cross_covariance(a, b):
    a, b = np.asarray(a), np.asarray(b)
    if a.shape != b.shape or a.ndim != 2 or len(a) < 2:
        raise ValueError("cross-fold scores have invalid dimensions/counts")
    if not np.isfinite(a).all() or not np.isfinite(b).all():
        raise ValueError("nonfinite scores")
    scale = max(np.max(np.abs(a)), np.max(np.abs(b)))
    if scale == 0:
        return np.zeros((a.shape[1], a.shape[1]))
    a, b = a / scale, b / scale
    a, b = a - a.mean(axis=0), b - b.mean(axis=0)
    normalized = (a.T @ b + b.T @ a) / (2 * len(a) * (len(a) - 1))
    return (normalized * scale) * scale


PROPAGATION = np.array([[1., 0, 0], [0, 1, 0], [0, 0, 1], [1, 1, 2]])


def finalize(conditional, leverage):
    conditional, leverage = np.asarray(conditional), np.asarray(leverage)
    if conditional.shape != (3, 3) or leverage.shape != (3, 3):
        raise ValueError("primitive covariance must be 3 by 3")
    if not np.array_equal(conditional, conditional.T, equal_nan=True) or not np.array_equal(leverage, leverage.T, equal_nan=True):
        raise ValueError("asymmetric covariance")
    raw = conditional + leverage
    if not np.isfinite(raw).all():
        return {"status": "nonfinite_derivative", "raw": None, "usable": None}
    scale = max(np.max(np.abs(conditional)), np.max(np.abs(leverage)))
    if scale == 0:
        return {"status": "ok_local", "raw": raw.tolist(), "usable": raw.tolist(),
                "mcse": [0.] * 4, "psd_adjustment": 0.}
    # Scaling prevents an absolute unit floor, norm overflow and underflow.
    tau = 1e-12 * (np.linalg.norm(conditional / scale) + np.linalg.norm(leverage / scale))
    sym = (raw / scale + raw.T / scale) / 2
    values, vectors = np.linalg.eigh(sym)
    if values.min() < -tau:
        return {"status": "unstable_nonpsd", "raw": raw.tolist(), "usable": None,
                "mcse": None, "psd_adjustment": 0.}
    usable_scaled = (vectors * np.maximum(values, 0)) @ vectors.T
    usable = usable_scaled * scale
    return {"status": "ok_local_psd_adjusted" if values.min() < 0 else "ok_local",
            "raw": raw.tolist(), "usable": usable.tolist(),
            "mcse": (np.sqrt(np.maximum(np.diag(PROPAGATION @ usable_scaled @ PROPAGATION.T), 0)) * np.sqrt(scale)).tolist(),
            "psd_adjustment": float(np.linalg.norm(usable_scaled - sym) * scale)}


def prepare(data):
    workers, firms, y = (np.asarray(data[k]) for k in ("worker", "firm", "outcome"))
    frequency = np.asarray(data.get("frequency", np.ones(len(y))), dtype=float)
    mass = np.asarray(data.get("target_mass", frequency), dtype=float)
    controls = np.asarray(data.get("controls", np.empty((len(y), 0))), dtype=float)
    if controls.ndim == 1:
        controls = controls[:, None]
    deletion = np.asarray(data.get("deletion_id", list(zip(workers, firms))))
    if deletion.ndim > 1:
        _, deletion = np.unique(deletion, axis=0, return_inverse=True)
    else:
        _, deletion = np.unique(deletion, return_inverse=True)
    stayers = data.get("stayers", "both")
    if stayers not in ("both", "movers"):
        raise ValueError("invalid stayer population")
    classification = firms if data.get("deletion", "observation") == "observation" else deletion
    stayer = np.array([len(np.unique(classification[workers == w])) == 1 for w in workers])
    if stayers == "movers" and stayer.any():
        selected = ~stayer
        workers, firms, y, frequency, mass, controls, deletion = [v[selected] for v in (workers, firms, y, frequency, mass, controls, deletion)]
    if (len(y) == 0 or any(len(v) != len(y) for v in (workers, firms, frequency, mass, controls, deletion))
            or not np.isfinite(y).all() or not np.isfinite(controls).all()
            or not np.isfinite(frequency).all() or (frequency < 1).any()
            or not np.equal(frequency, np.floor(frequency)).all()
            or not np.isfinite(mass).all() or (mass < 0).any() or mass.sum() <= 0):
        raise ValueError("invalid fixed input")
    if frequency.sum() > 512:
        raise ValueError("dense reference limited to 512 physical rows")
    copies = np.repeat(np.arange(len(y)), frequency.astype(int))
    _, wi = np.unique(workers, return_inverse=True)
    _, fi = np.unique(firms, return_inverse=True)
    xw = np.eye(wi.max() + 1)[wi[copies]]
    xf = np.eye(fi.max() + 1)[fi[copies]]
    xfe = np.column_stack((xw, xf))
    xc = controls[copies]
    x = np.column_stack((xfe, xc))
    inverse = np.linalg.pinv(x, rcond=1e-13)
    fit = inverse @ y[copies]
    fit_rhs = x.T @ y[copies]
    fit_residual = np.linalg.norm(x.T @ (x @ fit) - fit_rhs) / max(np.linalg.norm(fit_rhs), 1e-30)
    pfe = xfe @ np.linalg.pinv(xfe, rcond=1e-13)
    qcontrols = xc - pfe @ xc
    cp = qcontrols @ np.linalg.pinv(qcontrols, rcond=1e-13)
    nuisance = data.get("nuisance", "joint")
    if nuisance not in ("joint", "fixedoffset"):
        raise ValueError("invalid nuisance")
    working_y = y[copies] - (xc @ fit[xfe.shape[1]:] if nuisance == "fixedoffset" else 0)
    working_inverse = np.linalg.pinv(xfe, rcond=1e-13) if nuisance == "fixedoffset" else inverse
    working_fit = working_inverse @ working_y
    fitted = (xfe if nuisance == "fixedoffset" else x) @ working_fit
    cw = working_fit[:xw.shape[1]][wi[copies]]
    cf = working_fit[xw.shape[1]:xfe.shape[1]][fi[copies]]
    tmass = (mass / frequency)[copies]
    tmass /= tmass.sum()
    center = np.eye(len(copies)) - np.ones((len(copies), 1)) @ tmass[None, :]
    centered = center @ np.column_stack((cw, cf))
    cov = centered.T @ (tmass[:, None] * centered)
    plugin = np.array([cov[0, 0], cov[1, 1], cov[0, 1]])
    kind = data.get("deletion", "observation")
    if kind not in ("observation", "match"):
        raise ValueError("invalid deletion")
    groups = [np.array([i]) for i in range(len(copies))]
    if kind == "match":
        groups = [np.flatnonzero(deletion[copies] == g) for g in np.unique(deletion)]
        if stayers == "both":
            stayer = np.array([len(np.unique(deletion[wi == w])) == 1 for w in wi])
            groups = [g for g in groups if not stayer[copies[g[0]]]]
            groups += [np.array([i]) for i in np.flatnonzero(stayer[copies])]
    for g in groups:
        if len(np.unique(wi[copies[g]])) != 1 or len(np.unique(fi[copies[g]])) != 1:
            raise ValueError("deletion block crosses FE coordinates")
    control_projection = cp if nuisance == "joint" else np.zeros_like(cp)
    true_margins = [float(np.linalg.eigvalsh(np.eye(len(g)) - (pfe + control_projection)[np.ix_(g, g)]).min())
                    for g in groups]
    return {"xw": xw, "xf": xf, "x": x if nuisance == "joint" else xfe,
            "inverse": working_inverse, "pfe": pfe, "cp": control_projection,
            "y": working_y, "e": working_y - fitted, "mass": tmass, "groups": groups,
            "plugin": plugin, "true_maker_margin": min(true_margins), "copies": copies,
            "full_fit_complete_residual": float(fit_residual)}


def run(prepared, r, t, leverage_seed, target_seed, *, all_probe=True):
    if r < 2 or t < 2:
        raise ValueError("R and T must be at least two")
    n = len(prepared["y"])
    q = np.random.default_rng(leverage_seed).choice([-1., 1.], size=(r, n))
    projected = q @ prepared["pfe"].T
    z = np.zeros(n)
    states = []
    margins, ratios, constrained = [], [], []
    nonsmooth = False
    for g in prepared["groups"]:
        vdir = np.ones(len(g)) / math.sqrt(len(g))
        pu = projected[:, g] @ vdir
        mu = q[:, g] @ vdir - pu
        responses = moments(pu, mu)
        u = responses.mean(axis=0)
        h, m, b, variance = finite_terms(u, r)
        if not np.isfinite(u).all() or u[0] + u[1] <= 1e-10 or variance < -1e-8:
            return {"point_status": "moment_failed", "diagnostic_status": "point_failed"}
        if variance < 0:
            nonsmooth = True
        constrained.append(float(u[0] + u[1]))
        cg = prepared["cp"][np.ix_(g, g)]
        maker = np.eye(len(g)) - cg - h * np.outer(vdir, vdir)
        margin = float(np.linalg.eigvalsh(maker).min())
        if margin <= 1e-10:
            return {"point_status": "maker_failed", "diagnostic_status": "point_failed",
                    "minimum_margin": margin}
        if len(g) == 1:
            inverse_weight = observation_inverse(u, r, cg[0, 0])
            if not np.isfinite(inverse_weight) or inverse_weight <= 0:
                return {"point_status": "inverse_failed", "diagnostic_status": "point_failed"}
        a = np.linalg.solve(maker, prepared["e"][g])
        w = np.linalg.solve(maker, vdir)
        k = vdir @ w
        z[g] = a + (b - max(variance, 0) * k) * w * (vdir @ a)
        margins.append(margin)
        ratios.append(float(np.sqrt(max(variance, 0)) * k))
        if all_probe:
            jacobian = derivative(lambda u: block_adjustment(u, r, cg, vdir, prepared["e"][g]), u)
            states.append((g, jacobian, responses - u))
    qt = np.random.default_rng(target_seed).choice([-1., 1.], size=(t, n))
    direction = qt * np.sqrt(prepared["mass"])
    direction -= direction.sum(axis=1)[:, None] * prepared["mass"]
    # Batched dense equations retain the full observation-space operator and
    # each logical draw. This only bounds local harness runtime; no production
    # derivative, solve or covariance helper is used.
    rw = np.zeros((t, prepared["x"].shape[1]))
    rf = rw.copy()
    nw, nf = prepared["xw"].shape[1], prepared["xf"].shape[1]
    rw[:, :nw] = direction @ prepared["xw"]
    rf[:, nw:nw + nf] = direction @ prepared["xf"]
    gram_inverse = prepared["inverse"] @ prepared["inverse"].T
    sw = (rw @ gram_inverse) @ prepared["x"].T
    sf = (rf @ gram_inverse) @ prepared["x"].T
    maximum_residual = max(float(np.max(np.linalg.norm(prediction @ prepared["x"] - rhs, axis=1)
                           / np.maximum(np.linalg.norm(rhs, axis=1), 1e-30)))
                           for prediction, rhs in ((sw, rw), (sf, rf)))
    maps = np.zeros((t, 3, n))
    for g in prepared["groups"]:
        aw, af = sw[:, g] @ prepared["y"][g], sf[:, g] @ prepared["y"][g]
        maps[:, 0, g] = aw[:, None] * sw[:, g]
        maps[:, 1, g] = af[:, None] * sf[:, g]
        maps[:, 2, g] = (aw[:, None] * sf[:, g] + af[:, None] * sw[:, g]) / 2
    draws = maps @ z
    conditional = mean_covariance(draws)
    correction = draws.mean(axis=0)
    answer = {"point_status": "ok", "plugin": prepared["plugin"].tolist(),
              "correction": correction.tolist(), "point": (prepared["plugin"] - correction).tolist(),
              "conditional": conditional.tolist(), "conditional_mcse": finalize(conditional, np.zeros((3, 3)))["mcse"],
              "minimum_margin": min(margins), "minimum_constrained": min(constrained),
              "sensitivity_ratio": max(ratios), "rhs": r + 2 * t,
              "maximum_target_complete_residual": maximum_residual,
              "replay_rhs": 0, "diagnostic_status": "disabled", "point_variance_clipped": nonsmooth}
    if all_probe:
        folds = [maps[parity::2].mean(axis=0) for parity in (0, 1)]
        scores = [np.zeros((r, 3)), np.zeros((r, 3))]
        for g, jacobian, centered in states:
            for fold in (0, 1):
                scores[fold] -= centered @ (folds[fold][:, g] @ jacobian).T
        leverage = cross_covariance(*scores)
        diagnostic = finalize(conditional, leverage)
        if nonsmooth:
            diagnostic.update(status="nonsmooth_adjustment", usable=None, mcse=None)
        answer.update(leverage=leverage.tolist(), all_raw=diagnostic["raw"],
                      all_usable=diagnostic["usable"], all_mcse=diagnostic.get("mcse"),
                      diagnostic_status=diagnostic["status"],
                      score_means=[s.mean(axis=0).tolist() for s in scores])
    return answer


def nested_summary(points):
    points = np.asarray(points, dtype=float)
    if points.ndim != 3 or points.shape[0] < 2 or points.shape[1] < 2 or not np.isfinite(points).all():
        raise ValueError("nested reference needs complete finite K by L by coordinate inventory")
    k, l, _ = points.shape
    within = np.mean([mean_covariance(p) * l for p in points], axis=0)
    between = mean_covariance(points.mean(axis=1)) * k
    return {"target": within.tolist(), "leverage": (between - within / l).tolist(),
            "all": (between + (1 - 1 / l) * within).tolist()}


def execute(manifest):
    if manifest.get("schema") != "FEVC_ALL_PROBE_REFERENCE_V1":
        raise ValueError("invalid manifest schema")
    r, t, k, l = (manifest[name] for name in ("R", "T", "K", "L"))
    if any(type(v) is not int for v in (r, t, k, l)) or min(r, t, k) < 2 or l < 1:
        raise ValueError("invalid reference counts")
    if k * l > 100_000:
        raise ValueError("reference attempt cap exceeded")
    p = prepare(manifest["input"])
    attempts = []
    for i in range(k):
        for j in range(l):
            seeds = [int(np.random.SeedSequence([manifest["seed"], domain, i, j if domain == 1 else 0]).generate_state(1)[0]) for domain in (0, 1)]
            start = time.perf_counter()
            answer = run(p, r, t, *seeds)
            attempts.append(dict(key=[i, j], leverage_seed=seeds[0], target_seed=seeds[1],
                                 R=r, T=t, fold_a=(t + 1) // 2, fold_b=t // 2,
                                 seconds=time.perf_counter() - start, route="dense_physical_cache",
                                 **answer))
    points = [a["point"] for a in attempts if a["point_status"] == "ok"]
    failures = [a["key"] for a in attempts if a["point_status"] != "ok"]
    usable = sum(a["diagnostic_status"] in ("ok_local", "ok_local_psd_adjusted") for a in attempts)
    # A failed inventory is retained; no unfiltered covariance is claimed.
    summary = {"attempted": k * l, "point_failures": failures, "usable_diagnostics": usable,
               "conditioning": "success_conditional" if failures else "observed_complete_unfiltered",
               "true_maker_margin": p["true_maker_margin"]}
    if len(points) >= 2 and l == 1:
        summary["one_run_covariance"] = (mean_covariance(points) * len(points)).tolist()
        summary["bias_against_exact"] = (np.mean(points, axis=0) - exact_point(p)).tolist()
    if not failures and l >= 2:
        summary["nested"] = nested_summary(np.asarray(points).reshape(k, l, 3))
    finite_raw = [a["all_raw"] for a in attempts if a.get("all_raw") is not None]
    summary["finite_raw_count"] = len(finite_raw)
    if finite_raw:
        summary["mean_raw_including_nonpsd"] = np.mean(finite_raw, axis=0).tolist()
    return {"schema": "FEVC_ALL_PROBE_REFERENCE_RESULT_V1", "manifest": manifest,
            "attempts": attempts, "summary": summary}


def exact_point(p):
    # Dense correction under exact FE/control block deletion, independent of probes.
    correction = np.zeros(3)
    n = len(p["y"])
    gram_inverse = p["inverse"] @ p["inverse"].T
    nw, nf = p["xw"].shape[1], p["xf"].shape[1]
    rw, rf = np.zeros((nw + nf, n)), np.zeros((nw + nf, n))
    rw[:nw] = p["xw"].T
    rf[nw:nw + nf] = p["xf"].T
    if p["x"].shape[1] > nw + nf:
        rw = np.pad(rw, ((0, p["x"].shape[1] - nw - nf), (0, 0)))
        rf = np.pad(rf, ((0, p["x"].shape[1] - nw - nf), (0, 0)))
    sw, sf = p["x"] @ (gram_inverse @ rw), p["x"] @ (gram_inverse @ rf)
    target_cov = np.diag(p["mass"]) - np.outer(p["mass"], p["mass"])
    for g in p["groups"]:
        z = np.linalg.solve(np.eye(len(g)) - (p["pfe"] + p["cp"])[np.ix_(g, g)], p["e"][g])
        sigma = (np.outer(p["y"][g], z) + np.outer(z, p["y"][g])) / 2
        correction += [np.trace(sw[g] @ target_cov @ sw[g].T @ sigma),
                       np.trace(sf[g] @ target_cov @ sf[g].T @ sigma),
                       np.trace(sw[g] @ target_cov @ sf[g].T @ sigma)]
    return p["plugin"] - correction


def validate_inventory(result):
    if result.get("schema") != "FEVC_ALL_PROBE_REFERENCE_RESULT_V1":
        raise ValueError("invalid result schema")
    m = result["manifest"]
    expected = {(k, l) for k in range(m["K"]) for l in range(m["L"])}
    keys = [tuple(a["key"]) for a in result["attempts"]]
    if len(keys) != len(expected) or len(set(keys)) != len(keys) or set(keys) != expected:
        raise ValueError("missing, duplicate or partial attempt inventory")
    for a in result["attempts"]:
        k, l = a["key"]
        for domain, name in ((0, "leverage_seed"), (1, "target_seed")):
            seed = int(np.random.SeedSequence([m["seed"], domain, k, l if domain else 0]).generate_state(1)[0])
            if a[name] != seed:
                raise ValueError("seed identity mismatch")
        if [a["R"], a["T"], a["fold_a"], a["fold_b"]] != [m["R"], m["T"], (m["T"] + 1) // 2, m["T"] // 2]:
            raise ValueError("count identity mismatch")
        if a["point_status"] not in ("ok", "moment_failed", "maker_failed", "inverse_failed"):
            raise ValueError("unknown point status")
        if a["point_status"] == "ok":
            for name, shape in (("point", (3,)), ("plugin", (3,)), ("correction", (3,)),
                                ("conditional", (3, 3)), ("conditional_mcse", (4,)), ("leverage", (3, 3))):
                v = np.asarray(a[name])
                if v.shape != shape or not np.isfinite(v).all():
                    raise ValueError("invalid successful attempt")
            expected_conditional = finalize(a["conditional"], np.zeros((3, 3)))["mcse"]
            if not np.allclose(a["conditional_mcse"], expected_conditional, atol=0, rtol=1e-13):
                raise ValueError("conditional MCSE mismatch")
            for name in ("maximum_target_complete_residual", "minimum_margin", "minimum_constrained", "sensitivity_ratio", "seconds"):
                if not math.isfinite(a[name]) or a[name] < 0:
                    raise ValueError("invalid residual, boundary or timing receipt")
            if a["route"] != "dense_physical_cache" or a["rhs"] != m["R"] + 2 * m["T"] or a["replay_rhs"] != 0:
                raise ValueError("route/work mismatch")
            if (a["diagnostic_status"] == "nonsmooth_adjustment") != a["point_variance_clipped"]:
                raise ValueError("clipping status mismatch")
            if not np.allclose(a["point"], np.array(a["plugin"]) - a["correction"], atol=0, rtol=1e-14):
                raise ValueError("point accounting mismatch")
            diagnostic = finalize(a["conditional"], a["leverage"])
            if a["diagnostic_status"] not in (diagnostic["status"], "nonsmooth_adjustment"):
                raise ValueError("diagnostic status mismatch")
            if not np.array_equal(a["all_raw"], diagnostic["raw"]):
                raise ValueError("raw covariance mismatch")
            expected_usable = None if a["diagnostic_status"] == "nonsmooth_adjustment" else diagnostic["usable"]
            if not np.array_equal(a["all_usable"], expected_usable):
                raise ValueError("usable covariance mismatch")
            expected_mcse = None if expected_usable is None else diagnostic["mcse"]
            if not np.array_equal(a["all_mcse"], expected_mcse):
                raise ValueError("MCSE propagation mismatch")
        elif a["diagnostic_status"] != "point_failed":
            raise ValueError("failed point has a diagnostic")
    if result["summary"]["attempted"] != len(keys):
        raise ValueError("attempt summary mismatch")
    failures = [a["key"] for a in result["attempts"] if a["point_status"] != "ok"]
    usable = sum(a["diagnostic_status"] in ("ok_local", "ok_local_psd_adjusted") for a in result["attempts"])
    raw_count = sum(a.get("all_raw") is not None for a in result["attempts"])
    if result["summary"]["point_failures"] != failures or result["summary"]["usable_diagnostics"] != usable or result["summary"]["finite_raw_count"] != raw_count:
        raise ValueError("failure/availability summary mismatch")
    if result["summary"]["conditioning"] != ("success_conditional" if failures else "observed_complete_unfiltered"):
        raise ValueError("conditioning mismatch")
    if "input_sha256" in m and m["input_sha256"] != input_identity(m["input"]):
        raise ValueError("input identity mismatch")


def input_identity(data):
    return hashlib.sha256(json.dumps(data, sort_keys=True).encode()).hexdigest()


def bind_identity(manifest, name, value):
    if name in manifest and manifest[name] != value:
        raise ValueError(f"frozen {name} mismatch")
    manifest[name] = value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    m = json.loads(args.manifest.read_text())
    current_sha = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    if "source_sha" in m and m["source_sha"] != current_sha:
        raise ValueError("manifest source does not match checkout")
    m["source_sha"] = current_sha
    bind_identity(m, "reference_sha256", hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
    bind_identity(m, "input_sha256", input_identity(m["input"]))
    result = execute(m)
    result["environment"] = {"numpy": np.__version__, "rng": "NumPy-PCG64",
                             "profile": "development_dense_small"}
    validate_inventory(result)
    tmp = args.output.with_suffix(args.output.suffix + ".tmp")
    tmp.write_text(json.dumps(result, indent=2, allow_nan=False) + "\n")
    tmp.replace(args.output)
    return 1 if result["summary"]["point_failures"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
