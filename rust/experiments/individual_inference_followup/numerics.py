"""Independent saved-output diagnostics; no production numerical routines."""
import math
from functools import lru_cache

import numpy as np

NORMAL_RADIUS = 1.959963984540054


def ellipse(center, covariance, radius, eigenvalue):
    """Stationary quartic, independent of production's angular grid/refinement."""
    center = np.asarray(center, dtype=float)
    covariance = np.asarray(covariance, dtype=float)
    if center.shape != (2,) or covariance.shape != (2, 2):
        raise ValueError("ellipse dimensions")
    if not np.isfinite([*center, *covariance.flat, radius, eigenvalue]).all():
        raise ValueError("nonfinite ellipse")
    if radius <= 0 or not np.allclose(covariance, covariance.T, rtol=0, atol=1e-12):
        raise ValueError("invalid ellipse")
    root = np.linalg.cholesky(covariance)
    a = eigenvalue * (radius * root[0, 0]) ** 2
    b = radius * (2 * eigenvalue * center[0] * root[0, 0] + root[1, 0])
    c = radius * root[1, 1]
    d = eigenvalue * center[0] ** 2 + center[1]
    polynomial = np.array([-c, 4*a-2*b, 0., -4*a-2*b, c])
    roots = np.roots(polynomial / np.max(np.abs(polynomial)))
    angles = [0., math.pi]
    angles += [2*math.atan(float(t.real)) for t in roots if abs(t.imag) < 1e-8]
    values = [a*math.cos(t)**2+b*math.cos(t)+c*math.sin(t)+d for t in angles]
    return [min(values), max(values)]


@lru_cache(maxsize=4)
def quadrature(nodes):
    x, w = np.polynomial.legendre.leggauss(nodes)
    return (x+1)/2, w/2


def critical_cdf(radius, curvature, nodes=96):
    if radius <= 0 or curvature < 0 or not math.isfinite(curvature):
        raise ValueError("invalid critical CDF inputs")
    if curvature <= 1e-10:
        return math.erf(radius / math.sqrt(2))
    t, w = quadrature(nodes)
    # x=r(1-t^2) removes the square-root endpoint in the half-normal integral.
    x = radius * (1-t*t)
    y = np.sqrt((radius-x)*(2/curvature+radius+x))
    cdf = np.array([math.erf(float(v)/math.sqrt(2)) for v in y])
    return float(np.dot(w, 2*radius*t*math.sqrt(2/math.pi)*np.exp(-x*x/2)*cdf))


def critical(curvature, nodes=96):
    if curvature <= 1e-10:
        return NORMAL_RADIUS
    lo, hi = NORMAL_RADIUS, math.sqrt(-2*math.log(.05))
    for _ in range(42):
        mid = (lo+hi)/2
        if critical_cdf(mid, curvature, nodes) < .95:
            lo = mid
        else:
            hi = mid
    return (lo+hi)/2


def summarize_intervals(intervals, truth):
    x = np.asarray(intervals)
    if x.ndim != 2 or x.shape[1] != 2 or not np.isfinite(x).all() or np.any(x[:, 0] > x[:, 1]):
        raise ValueError("invalid interval inventory")
    return {"attempts": len(x), "coverage": float(np.mean((x[:, 0] <= truth) & (truth <= x[:, 1]))),
            "truth_below_lower": float(np.mean(truth < x[:, 0])),
            "truth_above_upper": float(np.mean(truth > x[:, 1])),
            "mean_width": float(np.mean(x[:, 1]-x[:, 0]))}


def analyze_target(calls, truth, target):
    if len(calls) != 400 or [r['replication'] for r in calls] != list(range(400)):
        raise ValueError("expected every saved replication exactly once")
    q = np.array([r['q1'][20*target:20*target+20] for r in calls], dtype=float)
    s = np.array([r['spectrum'][15*target:15*target+15] for r in calls], dtype=float)
    if not np.isfinite(q).all() or np.any(q[:, 16] != 0):
        raise ValueError("q1 target failure: retain and diagnose explicitly")
    if np.ptp(s[:, 0]) > 1e-12 * max(1., np.max(abs(s[:, 0]))):
        raise ValueError("changing spectral geometry prevents pooled score diagnostic")
    eigenvalue = s[0, 0]
    if not np.allclose(q[:, 0], q[:, 3]+q[:, 4], rtol=1e-12, atol=1e-12):
        raise ValueError("point decomposition")
    coords = q[:, [1, 4]]
    empirical = np.cov(coords, rowvar=False, ddof=1)
    fold_cov = [np.cov(coords[np.arange(400) % 2 != f], rowvar=False, ddof=1) for f in (0, 1)]
    model = np.array([[[r[2], r[5]], [r[5], r[6]]] for r in q])
    variants = {name: [] for name in ('original', 'integrated_radius', 'normal_radius_diagnostic', 'opposite_half_covariance_diagnostic')}
    endpoint_error = cdf_error = curvature_error = 0.
    for i, r in enumerate(q):
        center, cov = coords[i], model[i]
        endpoints = ellipse(center, cov, r[9], eigenvalue)
        endpoint_error = max(endpoint_error, max(abs(np.array(endpoints)-r[10:12]))/max(1., max(abs(r[10:12]))))
        cdf_error = max(cdf_error, abs(critical_cdf(r[9], r[8])-.95))
        curvature = 2*abs(eigenvalue)*cov[0, 0]/math.sqrt(cov[1, 1]-cov[0, 1]**2/cov[0, 0])
        curvature_error = max(curvature_error, abs(curvature-r[8])/max(1., abs(r[8])))
        variants['original'].append(r[10:12].tolist())
        variants['integrated_radius'].append(ellipse(center, cov, critical(curvature), eigenvalue))
        variants['normal_radius_diagnostic'].append(ellipse(center, cov, NORMAL_RADIUS, eigenvalue))
        other = fold_cov[i % 2]
        curvature = 2*abs(eigenvalue)*other[0, 0]/math.sqrt(other[1, 1]-other[0, 1]**2/other[0, 0])
        variants['opposite_half_covariance_diagnostic'].append(ellipse(center, other, critical(curvature), eigenvalue))
    if endpoint_error > 1e-9 or curvature_error > 1e-12 or cdf_error > 6*math.sqrt(.95*.05/100000):
        raise ValueError(f"numerical oracle failure: {endpoint_error}, {curvature_error}, {cdf_error}")
    points = q[:, 0]
    scalar = np.array([r['targets'][2*target] for r in calls])
    positive = scalar > 0
    mean_model = np.mean(model, axis=0)
    centered = points-np.mean(points)
    summary = {
        "target": ('worker', 'firm', 'covariance', 'total')[target], "primary": target != 2,
        "truth": truth, "bias": float(np.mean(points)-truth), "bias_mcse": float(np.std(points, ddof=1)/20),
        "point_skewness": float(np.mean(centered**3)/np.mean(centered**2)**1.5),
        "q0_scalar_se_available": int(np.sum(positive)),
        "empirical_sd_over_mean_q0_scalar_se": float(np.std(points, ddof=1)/np.mean(np.sqrt(scalar[positive]))),
        "empirical_coordinate_covariance": empirical.tolist(), "mean_modeled_coordinate_covariance": mean_model.tolist(),
        "empirical_over_mean_modeled_coordinate_variance": (np.diag(empirical)/np.diag(mean_model)).tolist(),
        "opposite_half_coordinate_covariances": [c.tolist() for c in fold_cov],
        "mean_curvature": float(np.mean(q[:, 8])), "mean_critical_radius": float(np.mean(q[:, 9])),
        "leading_share": float(s[0, 6]), "remainder_share": float(s[0, 8]),
        "max_scaled_endpoint_error": endpoint_error, "max_scaled_curvature_error": curvature_error,
        "max_critical_cdf_error": cdf_error,
        "variants": {name: summarize_intervals(value, truth) for name, value in variants.items()},
    }
    raw = [{"replication": i, "intervals": {name: value[i] for name, value in variants.items()}} for i in range(400)]
    return summary, raw
