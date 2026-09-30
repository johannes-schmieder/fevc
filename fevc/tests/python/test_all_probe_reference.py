from __future__ import annotations

import copy
import itertools

import numpy as np
import pytest

from fevc.tools.all_probe_reference import (
    PROPAGATION, block_adjustment, cross_covariance, derivative, execute,
    finalize, finite_terms, mean_covariance, moments, nested_summary,
    observation_inverse, prepare, run, validate_inventory,
)


def gradients(u, r):
    p, m, a, b, c = u
    h, residual, _, _ = finite_terms(u, r)
    dh = np.array([m, -p, 0, 0, 0]) / (p + m)**2
    dm = -dh
    basis = np.eye(5)
    db = (a * dm + residual * basis[2] - b * dh - h * basis[3]
          + c * (dm - dh) + (residual - h) * basis[4]) / r
    dv = (2 * residual * a * dm + residual**2 * basis[2]
          + 2 * h * b * dh + h**2 * basis[3]
          - 2 * c * (residual * dh + h * dm) - 2 * h * residual * basis[4]) / r
    return dh, dm, db, dv


def observation_gradient(u, r, control):
    _, m, b, v = finite_terms(u, r)
    _, dm, db, dv = gradients(u, r)
    ell = m - control
    return (-ell**-2 - 2 * b * ell**-3 + 3 * v * ell**-4) * dm + db / ell**2 - dv / ell**3


def normalized_error(a, b):
    return np.linalg.norm(np.asarray(a) - b) / max(np.linalg.norm(a), np.linalg.norm(b), 1e-20)


@pytest.mark.parametrize("r", [3, 32, 200])
def test_five_moment_and_observation_derivatives(r):
    rng = np.random.default_rng(70929)
    for _ in range(100):
        projected = rng.normal(0, .2, 21)
        residual = rng.normal(0, .9, 21)
        u = moments(projected, residual).mean(axis=0)
        control = rng.uniform(0, .1)
        analytical = np.array(gradients(u, r))
        assert normalized_error(analytical, derivative(lambda u: np.array(finite_terms(u, r)), u)) <= 1e-9
        obs = observation_gradient(u, r, control)
        assert normalized_error(obs, derivative(lambda u: observation_inverse(u, r, control), u)) <= 1e-9
        direction = rng.normal(size=5)
        step = 2e-6
        numerical = (observation_inverse(u + step * direction, r, control)
                     - observation_inverse(u - step * direction, r, control)) / (2 * step)
        scale = np.linalg.norm(obs) * np.linalg.norm(direction)
        assert abs(obs @ direction - numerical) / scale <= 1e-9


def test_varying_control_block_complete_inverse_derivative():
    rng = np.random.default_rng(70930)
    signs = set()
    for _ in range(100):
        f = rng.integers(1, 5, 4)
        vdir = np.sqrt(f / f.sum())
        controls = rng.normal(size=(4, 2)) * .04
        controls = controls @ controls.T
        u = moments(rng.normal(0, .3, 40), rng.normal(0, .7, 40)).mean(axis=0)
        h, _, b, v = finite_terms(u, 32)
        signs.add(np.sign(b))
        maker = np.eye(4) - controls - h * np.outer(vdir, vdir)
        assert np.linalg.eigvalsh(maker).min() > .1
        e = rng.normal(size=4) * np.sqrt(f)
        inv = np.linalg.inv(maker)
        # Dense and Woodbury maker representations, independent of native code.
        base = np.linalg.inv(np.eye(4) - controls)
        w0 = base @ vdir
        woodbury = base + h * np.outer(w0, w0) / (1 - h * (vdir @ w0))
        np.testing.assert_allclose(inv, woodbury, atol=1e-13, rtol=1e-13)
        w, t, k = inv @ vdir, vdir @ (inv @ e), vdir @ (inv @ vdir)
        dh, _, db, dv = gradients(u, 32)
        beta = (1 + 2 * b * k - 3 * v * k**2) * dh + db - k * dv
        direct = derivative(lambda u: block_adjustment(u, 32, controls, vdir, e), u)
        assert normalized_error(np.outer(w * t, beta), direct) < 1e-9
    assert signs == {-1, 1}


def test_copy_pullback_keeps_nonlinearity_and_shared_direction():
    rng = np.random.default_rng(71001)
    q = rng.choice([-1., 1.], size=(17, 7))
    p = rng.normal(size=17) * .15
    a2, a4 = np.mean(p**2), np.mean(p**4)
    c1, c3 = np.mean(q * p[:, None], axis=0), np.mean(q * p[:, None]**3, axis=0)
    u = np.array([a2 + np.zeros(7), 1 + a2 - 2 * c1, a4 + np.zeros(7),
                  1 + 6 * a2 + a4 - 4 * c1 - 4 * c3, a2 + a4 - 2 * c3]).T
    lam = np.array([observation_gradient(copy, 17, .02) for copy in u])
    g2 = lam[:, 0] + lam[:, 1] + 6 * lam[:, 3] + lam[:, 4]
    g4 = lam[:, 2] + lam[:, 3] + lam[:, 4]
    g1, g3 = -2 * lam[:, 1] - 4 * lam[:, 3], -4 * lam[:, 3] - 2 * lam[:, 4]
    offset = g2.sum() * a2 + g4.sum() * a4 + g1 @ c1 + g3 @ c3
    replay = g2.sum() * p**2 + g4.sum() * p**4 + p * (q @ g1) + p**3 * (q @ g3) - offset
    direct = np.einsum("rai,ai->r", moments(p[:, None], q - p[:, None]) - u, lam)
    np.testing.assert_allclose(direct, replay, atol=1e-12, rtol=1e-11)
    # Inversion before copy averaging is essential.
    per_copy = np.mean([observation_inverse(copy, 17, .02) for copy in u])
    assert abs(per_copy - observation_inverse(u.mean(axis=0), 17, .02)) > 1e-6


@pytest.mark.parametrize("t", [2, 4, 5])
@pytest.mark.parametrize("means", [(0., 0.), (.7, -.4)])
def test_exact_bilinear_interaction_and_unequal_folds(t, means):
    r = 3
    mu_x, mu_y = means
    estimates, conditional, cross, naive = [], [], [], []
    for x in itertools.product([-1., 1.], repeat=r):
        x = np.array(x) * .3 + mu_x
        for y in itertools.product([-1., 1.], repeat=t):
            y = np.array(y) * .8 + mu_y
            estimates.append(x.mean() * y.mean())
            conditional.append(mean_covariance((x.mean() * y)[:, None])[0, 0])
            centered_x = x - x.mean()
            cross.append(cross_covariance((centered_x * y[::2].mean())[:, None],
                                          (centered_x * y[1::2].mean())[:, None])[0, 0])
            naive.append(mean_covariance((centered_x * y.mean())[:, None])[0, 0])
    truth = mu_y**2 * .3**2 / r + mu_x**2 * .8**2 / t + .3**2 * .8**2 / (r * t)
    assert normalized_error(np.var(estimates), truth) < 1e-11
    assert normalized_error(np.mean(conditional) + np.mean(cross), truth) < 1e-11
    assert np.mean(conditional) + np.mean(naive) > truth * (1 + 1e-3)
    if means == (0, 0):
        assert normalized_error(np.mean(conditional) + np.mean(naive), 2 * truth) < 1e-11


def test_matrix_affine_cached_forward_reverse_and_hybrid_terms():
    rng = np.random.default_rng(71002)
    u = rng.normal(size=(7, 13))
    l = rng.normal(size=(5, 3, 9))
    d = rng.normal(size=(9, 13))
    centered = u - u.mean(axis=0)
    forward = [-np.einsum("ij,rj->ri", fold @ d, centered)
               for fold in (l[::2].mean(axis=0), l[1::2].mean(axis=0))]
    response = centered @ d.T
    reverse = [-response @ fold.T for fold in (l[::2].mean(axis=0), l[1::2].mean(axis=0))]
    np.testing.assert_allclose(cross_covariance(*forward), cross_covariance(*reverse), rtol=1e-13)
    # Shared shifts and recentering are harmless, population-wise outer products aren't.
    np.testing.assert_allclose(cross_covariance(forward[0] + 17, forward[1] - 9),
                               cross_covariance(*forward), rtol=1e-13)
    pieces = [[-response[:, :4] @ fold[:, :4].T, -response[:, 4:] @ fold[:, 4:].T]
              for fold in (l[::2].mean(axis=0), l[1::2].mean(axis=0))]
    wrongly_separate = sum(cross_covariance(pieces[0][i], pieces[1][i]) for i in (0, 1))
    assert np.linalg.norm(wrongly_separate - cross_covariance(*forward)) > 1


def test_covariance_policy_propagation_and_outcome_scale():
    cond = np.diag([1., 2., 3.])
    signed = np.diag([-.2, .4, -.1])
    result = finalize(cond, signed)
    raw = np.array(result["raw"])
    assert result["status"] == "ok_local"
    assert result["mcse"][0] < 1
    assert np.isclose(result["mcse"][3]**2, np.array([1, 1, 2]) @ raw @ [1, 1, 2])
    np.testing.assert_allclose(PROPAGATION @ raw @ PROPAGATION.T,
                               (PROPAGATION @ raw) @ PROPAGATION.T)
    for scale in (1e-120, 1e120):
        scaled = finalize(cond * scale, signed * scale)
        assert scaled["status"] == result["status"]
        np.testing.assert_allclose(np.array(scaled["mcse"]) / np.sqrt(scale), result["mcse"])
    assert finalize(cond, -2 * cond)["status"] == "unstable_nonpsd"
    assert finalize(cond, np.diag([-1 - 1e-13, 0., 0.]))["status"] == "ok_local_psd_adjusted"
    assert finalize(np.zeros((3, 3)), np.zeros((3, 3)))["mcse"] == [0.] * 4
    assert finalize(cond, np.full((3, 3), np.nan))["status"] == "nonfinite_derivative"


def fixture(deletion="observation", nuisance="joint"):
    worker = np.repeat(np.arange(4), 9)
    firm = np.tile(np.repeat(np.arange(3), 3), 4)
    control = np.sin(np.arange(36) * 1.7) * .2
    return dict(worker=worker.tolist(), firm=firm.tolist(),
                outcome=(worker * .4 + firm * -.3 + np.cos(np.arange(36) * .7)).tolist(),
                frequency=np.tile([1, 2, 1], 12).tolist(),
                target_mass=(1 + np.arange(36) % 5).tolist(),
                controls=control[:, None].tolist(), deletion=deletion, nuisance=nuisance)


@pytest.mark.parametrize("deletion", ["observation", "match"])
@pytest.mark.parametrize("nuisance", ["joint", "fixedoffset"])
def test_complete_reference_fresh_preparation_and_literal_copies(deletion, nuisance):
    data = fixture(deletion, nuisance)
    p = prepare(data)
    assert p["true_maker_margin"] > .3
    a = run(p, 32, 33, 17, 19)
    b = run(prepare(data), 32, 33, 17, 19)
    assert a == b
    disabled = run(p, 32, 33, 17, 19, all_probe=False)
    assert a["point"] == disabled["point"]
    assert a["conditional_mcse"] == disabled["conditional_mcse"]
    # Physical expansion with the original declared deletion IDs and copy mass.
    copies = p["copies"]
    expanded = copy.deepcopy(data)
    for key in ("worker", "firm", "outcome", "controls"):
        expanded[key] = np.asarray(data[key])[copies].tolist()
    expanded["frequency"] = [1] * len(copies)
    expanded["target_mass"] = (np.asarray(data["target_mass"])[copies] / np.asarray(data["frequency"])[copies]).tolist()
    e = run(prepare(expanded), 32, 33, 17, 19)
    np.testing.assert_allclose(a["point"], e["point"], atol=1e-12)
    np.testing.assert_allclose(a["all_raw"], e["all_raw"], atol=1e-12)
    scaled = copy.deepcopy(data)
    scaled["outcome"] = (np.asarray(data["outcome"]) * -3).tolist()
    s = run(prepare(scaled), 32, 33, 17, 19)
    np.testing.assert_allclose(s["point"], np.array(a["point"]) * 9, atol=1e-11)
    np.testing.assert_allclose(s["all_raw"], np.array(a["all_raw"]) * 81, atol=1e-11)


def manifest(**updates):
    return dict(schema="FEVC_ALL_PROBE_REFERENCE_V1", input=fixture(), R=8, T=9,
                K=2, L=2, seed=801, **updates)


def test_nested_counts_summary_and_inventory_failures():
    result = execute(manifest())
    validate_inventory(result)
    assert result["summary"]["attempted"] == 4
    points = np.array([a["point"] for a in result["attempts"]]).reshape(2, 2, 3)
    summary = nested_summary(points)
    within = np.mean([np.cov(p.T) for p in points], axis=0)
    between = np.cov(points.mean(axis=1).T)
    np.testing.assert_allclose(summary["all"], between + within / 2, atol=1e-14)
    np.testing.assert_allclose(summary["leverage"], between - within / 2, atol=1e-14)
    for mutate in (lambda a: a.pop(), lambda a: a.append(a[0]), lambda a: a.__setitem__(1, a[0])):
        broken = copy.deepcopy(result)
        mutate(broken["attempts"])
        with pytest.raises(ValueError, match="inventory"):
            validate_inventory(broken)
    with pytest.raises(ValueError):
        execute({})
    with pytest.raises(ValueError):
        execute(dict(manifest(), R=1))


def test_point_failures_are_not_silently_filtered():
    bad = dict(worker=[0], firm=[0], outcome=[1.], deletion="match")
    result = execute(dict(manifest(), input=bad))
    validate_inventory(result)
    assert len(result["summary"]["point_failures"]) == 4
    assert result["summary"]["conditioning"] == "success_conditional"
    assert "nested" not in result["summary"]


def test_default_hybrid_and_parallel_original_ids():
    data = fixture("match")
    data.pop("controls")
    for key, values in (("worker", [9] * 6), ("firm", [0] * 6),
                        ("outcome", [1., 2., 3., 2., 1., 0.]),
                        ("frequency", [1] * 6), ("target_mass", [2] * 6)):
        data[key] += values
    p = prepare(data)
    assert p["true_maker_margin"] > .3
    default = run(p, 32, 33, 17, 19)
    assert default == run(prepare(dict(data, stayers="both")), 32, 33, 17, 19)
    movers = prepare(dict(data, stayers="movers"))
    assert len(movers["y"]) == 48
    assert len(p["groups"]) == 18  # 12 whole mover matches plus six stayer copies.
    parallel = dict(data, deletion_id=list(range(42)))
    parallel_p = prepare(parallel)
    assert len(parallel_p["groups"]) == 42  # Stayer worker now has six original blocks.


def test_scaled_reductions_when_intermediate_products_overflow():
    draws = np.array([[1e154], [-1e154], [1e154], [-1e154]])
    assert np.isclose(mean_covariance(draws)[0, 0] / 1e307, 10 / 3)
    result = finalize(np.eye(3) * 1e308, np.zeros((3, 3)))
    assert np.isfinite(result["mcse"]).all()
    assert np.isclose(result["mcse"][3] / 1e154, np.sqrt(6))


def test_corrupt_seed_status_raw_counts_and_frozen_identities():
    from fevc.tools.all_probe_reference import bind_identity
    result = execute(manifest())
    edits = [("schema", "BAD"), ("all_raw", [[0.]]), ("target_seed", 0),
             ("diagnostic_status", "ok_fabricated"), ("fold_a", 7)]
    for name, value in edits:
        broken = copy.deepcopy(result)
        if name == "schema":
            broken[name] = value
        else:
            broken["attempts"][0][name] = value
        with pytest.raises(ValueError):
            validate_inventory(broken)
    m = {"input_sha256": "frozen"}
    with pytest.raises(ValueError, match="frozen"):
        bind_identity(m, "input_sha256", "changed")


def test_matrix_affine_exact_unbiasedness_and_deterministic_leverage():
    outcomes, estimators = [], []
    base = np.array([[.2, -.7], [1., .3], [-.4, .5]])
    noise = np.array([[1., .2], [-.2, .8], [.7, -.3]])
    mu = np.array([.3, -.5])
    delta = np.array([.4, .9])
    for xsign in itertools.product([-1., 1.], repeat=3):
        u = mu + np.array(xsign)[:, None] * delta
        for ysign in itertools.product([-1., 1.], repeat=3):
            ops = base + np.array(ysign)[:, None, None] * noise
            draws = ops @ u.mean(axis=0)
            outcomes.append(draws.mean(axis=0))
            scores = [-(u - u.mean(axis=0)) @ fold.T for fold in (ops[::2].mean(axis=0), ops[1::2].mean(axis=0))]
            estimators.append(mean_covariance(draws) + cross_covariance(*scores))
    np.testing.assert_allclose(np.mean(estimators, axis=0), np.cov(np.array(outcomes).T, ddof=0), atol=1e-13, rtol=1e-11)
    deterministic = np.ones((3, 3))
    np.testing.assert_array_equal(cross_covariance(deterministic, deterministic * -2), np.zeros((3, 3)))


def test_target_operator_against_stored_weighted_parameter_space():
    data = fixture("match", "joint")
    p = prepare(data)
    f, mass = np.asarray(data["frequency"]), np.asarray(data["target_mass"])
    worker, firm = np.asarray(data["worker"]), np.asarray(data["firm"])
    # Independent grounded parameterization rather than the physical pseudoinverse.
    xw = np.eye(4)[worker]
    xf = np.eye(3)[firm][:, 1:]
    x = np.column_stack((xw, xf, data["controls"]))
    hinv = np.linalg.inv(x.T @ (f[:, None] * x))
    q = np.random.default_rng(19).choice([-1., 1.], len(p["y"]))
    sums = np.bincount(p["copies"], weights=q)
    a = np.sqrt(mass / (f * mass.sum())) * sums
    direction = a - mass / mass.sum() * a.sum()
    rhs = np.zeros((x.shape[1], 2))
    rhs[:4, 0], rhs[4:6, 1] = xw.T @ direction, xf.T @ direction
    prediction = x @ hinv @ rhs
    physical_dir = q * np.sqrt(p["mass"])
    physical_dir -= p["mass"] * physical_dir.sum()
    physical_rhs = np.zeros((p["x"].shape[1], 2))
    physical_rhs[:4, 0] = p["xw"].T @ physical_dir
    physical_rhs[4:7, 1] = p["xf"].T @ physical_dir
    reference = p["x"] @ (p["inverse"] @ p["inverse"].T) @ physical_rhs
    np.testing.assert_allclose(prediction[p["copies"]], reference, rtol=1e-11, atol=1e-12)
    kappa = np.column_stack((prediction[:, 0]**2, prediction[:, 1]**2, prediction[:, 0] * prediction[:, 1]))
    arbitrary_z = np.sin(np.arange(len(f)))
    contraction = np.sum(f[:, None] * np.asarray(data["outcome"])[:, None] * arbitrary_z[:, None] * kappa, axis=0)
    physical_kappa = np.column_stack((reference[:, 0]**2, reference[:, 1]**2, reference[:, 0] * reference[:, 1]))
    np.testing.assert_allclose(contraction, np.sum(p["y"][:, None] * arbitrary_z[p["copies"], None] * physical_kappa, axis=0), atol=1e-12)


def test_cli_atomic_success_failure_and_frozen_input(tmp_path):
    import hashlib
    import json
    import subprocess
    import sys
    from pathlib import Path
    script = Path(__file__).resolve().parents[2] / "tools/all_probe_reference.py"
    path, output = tmp_path / "manifest.json", tmp_path / "result.json"
    m = manifest()
    path.write_text(json.dumps(m))
    process = subprocess.run([sys.executable, str(script), str(path), str(output)], capture_output=True, text=True)
    assert process.returncode == 0, process.stderr
    result = json.loads(output.read_text())
    validate_inventory(result)
    assert result["manifest"]["reference_sha256"] == hashlib.sha256(script.read_bytes()).hexdigest()
    assert not list(tmp_path.glob("*.tmp"))
    frozen = result["manifest"]
    frozen["input"]["outcome"][0] += 1
    path.write_text(json.dumps(frozen))
    process = subprocess.run([sys.executable, str(script), str(path), str(output)], capture_output=True, text=True)
    assert process.returncode != 0 and "frozen input_sha256 mismatch" in process.stderr
    m["input"] = dict(worker=[0], firm=[0], outcome=[1.], deletion="match")
    path.write_text(json.dumps(m))
    process = subprocess.run([sys.executable, str(script), str(path), str(output)], capture_output=True, text=True)
    assert process.returncode == 1
    assert len(json.loads(output.read_text())["attempts"]) == 4


def test_cache_oracle_rejects_nonfinite_malformed_and_partial_outputs():
    from fevc.tools.all_probe_cache_oracle import validate
    r, t = 3, 5
    u = moments(np.array([.2, .3, -.1]), np.array([.8, -.7, 1.1]))
    jac = derivative(lambda x: observation_inverse(x, r, .02), u.mean(axis=0))
    score = -(u - u.mean(axis=0)) @ jac
    scores = np.tile(score[:, None], (1, 3))
    cache = dict(r=r, t=t, scores=[scores.tolist(), scores.tolist()],
                 leverage=cross_covariance(scores, scores).tolist(),
                 units=[dict(kind="observation", u=u.tolist(), control=.02,
                             l=np.ones((t, 1, 3)).tolist())])
    validate(cache)
    edits = [("scores", np.full((2,r,3), np.nan).tolist()),
             ("scores", np.full((2,r,3), np.inf).tolist()),
             ("leverage", np.full((3,3), np.nan).tolist()),
             ("leverage", [[0.]]), ("scores", [scores.tolist()]), ("r", 1)]
    for name, value in edits:
        broken = copy.deepcopy(cache)
        broken[name] = value
        with pytest.raises(ValueError):
            validate(broken)
    for field, value in (("u", u[:-1].tolist()), ("l", np.ones((t-1,1,3)).tolist()),
                         ("kind", "unknown"), ("control", np.nan)):
        broken = copy.deepcopy(cache)
        broken["units"][0][field] = value
        with pytest.raises(ValueError):
            validate(broken)
    broken = copy.deepcopy(cache)
    broken["scores"][0][0][0] += 1e-6
    with pytest.raises(ValueError, match="normalized error"):
        validate(broken)


def test_prospective_contrast_screen_uses_joint_ratio_uncertainty_and_units():
    from fevc.tools.all_probe_calibration import contrast_screen, rate_interval, audit
    rng=np.random.default_rng(4071)
    reference=np.array([np.diag(rng.uniform(.5,1.5,3)) for _ in range(64)])
    identical=contrast_screen(reference,reference)
    assert all(c["status"]=="pass" and c["relative_standard_error"]==0 for c in identical)
    correlated=contrast_screen(reference*1.05,reference)
    assert all(c["relative_standard_error"]<1e-15 for c in correlated)
    for c in correlated:
        direction=np.array(c['contrast'])
        differences=.05*np.einsum('i,kij,j->k',direction,reference,direction)
        np.testing.assert_allclose(c['mean_difference'],differences.mean(),rtol=1e-13)
        np.testing.assert_allclose(c['difference_standard_error'],differences.std(ddof=1)/8,rtol=1e-13)
    rejected=contrast_screen(reference*1.2,reference)
    assert all(c["status"]=="fail" for c in rejected)
    for multiplier in (1e-200,1e200):
        scaled=contrast_screen(reference*1.05*multiplier,reference*multiplier)
        np.testing.assert_allclose([c["equivalence_bound"] for c in scaled],
                                   [c["equivalence_bound"] for c in correlated],rtol=1e-12)
    assert rate_interval(0,4096)[1]>0
    with pytest.raises(ValueError,match="policy"):
        audit(execute(manifest()))
    with pytest.raises(ValueError,match="complete finite"):
        contrast_screen(reference[:63],reference[:63])
    assert all(c['status']=='near_zero_inconclusive'
               for c in contrast_screen(np.zeros_like(reference),np.zeros_like(reference)))


def test_nested_jackknife_matches_independent_leave_one_references():
    from fevc.tools.all_probe_nested_audit import estimates, comparisons
    points=np.random.default_rng(9128).normal(size=(7,8,3))+[100.,-31.,74.]
    actual=estimates(points)
    expected=nested_summary(points)
    for name in ('target','leverage','all'):
        np.testing.assert_allclose(actual[name],expected[name],rtol=1e-11,atol=1e-12)
        leave=np.array([nested_summary(np.delete(points,i,axis=0))[name] for i in range(7)])
        se=np.sqrt(6/7*np.sum((leave-leave.mean(axis=0))**2,axis=0))
        np.testing.assert_allclose(actual[name+'_standard_error'],se,rtol=1e-11,atol=1e-12)
    with pytest.raises(ValueError,match='complete finite'):
        estimates(points[:2])
    diagnostics={name:np.random.default_rng(i).normal(size=(7,8,3,3))
                 for i,name in enumerate(('target','leverage','all'))}
    compared=comparisons(points,diagnostics)
    for name in diagnostics:
        expected_difference=diagnostics[name].mean(axis=(0,1))-expected[name]
        leave=np.array([np.delete(diagnostics[name],i,axis=0).mean(axis=(0,1))-
                        nested_summary(np.delete(points,i,axis=0))[name] for i in range(7)])
        se=np.sqrt(6/7*np.sum((leave-leave.mean(axis=0))**2,axis=0))
        np.testing.assert_allclose(compared[name]['mean_difference'],expected_difference,rtol=1e-11,atol=1e-12)
        np.testing.assert_allclose(compared[name]['difference_standard_error'],se,rtol=1e-11,atol=1e-12)
    diagnostics['all'][0,0,0,0]=np.nan
    with pytest.raises(ValueError,match='every finite'):
        comparisons(points,diagnostics)


def test_confirmation_audits_reject_changed_frozen_source(monkeypatch):
    from fevc.tools import all_probe_calibration as calibration, all_probe_nested_audit as nested
    for module,profile,key,counts in [
            (calibration,'confirmation_dense_small_v1','calibration_sha256',dict(K=4096,L=1)),
            (nested,'confirmation_dense_nested_v1','nested_audit_sha256',dict(K=128,L=8))]:
        monkeypatch.setattr(module,'validate_inventory',lambda result:None)
        m=dict(profile=profile,R=200,T=200,acceptance=calibration.POLICY,**counts)
        for value in (None,'0'*64):
            m[key]=value
            with pytest.raises(ValueError,match='frozen .* source mismatch'):
                module.audit(dict(manifest=m))
