"""Validate a tiny native test-only full-response cache independently.

Only cache inputs and final outputs cross this boundary. Derivatives use dense
inverses and complex-step expressions, not native derivative/score helpers.
"""
import json
import sys

import numpy as np

if __package__:
    from .all_probe_reference import block_adjustment, cross_covariance, derivative, observation_inverse
else:
    from all_probe_reference import block_adjustment, cross_covariance, derivative, observation_inverse


def finite_array(value, shape):
    value = np.asarray(value, dtype=float)
    if value.shape != shape or not np.isfinite(value).all():
        raise ValueError(f"expected finite cache array {shape}")
    return value


def compare(actual, expected, label):
    scale = max(np.max(np.abs(actual)), np.max(np.abs(expected)))
    if scale == 0:
        return
    actual, expected = actual / scale, expected / scale
    error = np.linalg.norm(expected - actual) / max(np.linalg.norm(expected), np.linalg.norm(actual))
    if not np.isfinite(error) or error > 1e-11:
        raise ValueError(f"{label} normalized error {error}")


def validate(cache):
    r, t = cache["r"], cache["t"]
    if type(r) is not int or type(t) is not int or min(r, t) < 2:
        raise ValueError("invalid cache probe counts")
    if not isinstance(cache["units"], list) or not cache["units"]:
        raise ValueError("empty cache inventory")
    actual_scores = finite_array(cache["scores"], (2, r, 3))
    actual = finite_array(cache["leverage"], (3, 3))
    scores = [np.zeros((r, 3)), np.zeros((r, 3))]
    for unit in cache["units"]:
        u = finite_array(unit["u"], (r, 5))
        means = u.mean(axis=0)
        if unit["kind"] == "observation":
            target = finite_array(unit["l"], (t, 1, 3))
            control = finite_array(unit["control"], ())
            jac = derivative(lambda u: observation_inverse(u, r, control), means)[None, :]
        elif unit["kind"] == "match":
            width = len(unit["v"])
            if width < 1:
                raise ValueError("empty match cache")
            target = finite_array(unit["l"], (t, width, 3))
            cp = finite_array(unit["control"], (width, width))
            v, e = finite_array(unit["v"], (width,)), finite_array(unit["e"], (width,))
            jac = derivative(lambda u: block_adjustment(u, r, cp, v, e), means)
        else:
            raise ValueError("unknown cached deletion kind")
        if not np.isfinite(jac).all():
            raise ValueError("nonfinite cache derivative")
        for fold in (0, 1):
            operator = target[fold::2].mean(axis=0).T
            scores[fold] -= (u - means) @ (operator @ jac).T
    expected = cross_covariance(*scores)
    if not np.isfinite(expected).all() or not np.isfinite(scores).all():
        raise ValueError("nonfinite cache reference")
    compare(actual, expected, "full-response/replay covariance")
    for fold in (0, 1):
        compare(actual_scores[fold], scores[fold], "forward/reverse score")


if __name__ == "__main__":
    validate(json.load(sys.stdin))
