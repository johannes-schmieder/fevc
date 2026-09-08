"""Algebra, independent interval geometry, and fail-closed capture inventory."""
from pathlib import Path
import copy
import importlib.util
import math
import numpy as np
import pytest

spec = importlib.util.spec_from_file_location('q1_target_run', Path(__file__).with_name('run.py'))
run = importlib.util.module_from_spec(spec)
spec.loader.exec_module(run)
a = run.oracle


def test_capture_is_only_one_insertion_and_retains_original_gate():
    source = 'before\n'+run.MARKER+'arguments\n)?;\nafter\n'
    observed = run.instrument(source, 'capture;')
    assert observed.replace('capture;\n', '', 1) == source
    for malformed in ('none', source+source):
        with pytest.raises(ValueError, match='anchor'):
            run.instrument(malformed, 'capture;')


@pytest.mark.parametrize('vb,cross,vr,certified,status', [
    (1., .2, 2., True, 0), (1., 0., -1., True, 1),
    (1., 2., 1., True, 2), (1., 1., 1., True, 2), (1., 0., 2., False, 6)])
def test_target_covariance_status_is_not_marginal_positivity(vb, cross, vr, certified, status):
    assert a.pair_status(vb, cross, vr, certified) == status


def test_dense_remainder_and_raw_leaveout_identity():
    rng = np.random.default_rng(6341)
    x = np.linalg.qr(rng.normal(size=(15, 3)))[0]
    hat = x@x.T
    maker = np.eye(15)-hat
    inverse = 1/np.diag(maker)
    plugin = (x*np.array([2., .2, -.1]))@x.T
    ratio = np.diag(plugin)*inverse
    point = plugin-(ratio[:, None]*maker+maker*ratio[None, :])/2
    mode, eigenvalue = x[:, 0], 2.
    remainder = a.remainder_kernel(point, maker, inverse, mode, eigenvalue)
    direct_plugin = plugin-eigenvalue*np.outer(mode, mode)
    direct_ratio = np.diag(direct_plugin)*inverse
    direct = direct_plugin-(direct_ratio[:, None]*maker+maker*direct_ratio[None, :])/2
    np.testing.assert_allclose(remainder, direct, atol=2e-16)
    y = rng.normal(size=15)
    recenter = np.sum(mode**2*y*(maker@y)*inverse)
    assert abs(y@point@y-eigenvalue*((mode@y)**2-recenter)-y@remainder@y) < 1e-14
    s = rng.uniform(.5, 2., 15)
    assert a.exact_trace(remainder, s) == pytest.approx(2*np.trace(remainder@np.diag(s)@remainder@np.diag(s)))


@pytest.mark.parametrize('eigenvalue', [-2., 0., 1.3])
def test_polynomial_ellipse_image_against_dense_angular_oracle(eigenvalue):
    center, covariance, radius = [1.2, -.4], [1.3, -.2, .7], 2.2
    endpoints = a.ellipse(center, covariance, radius, eigenvalue)
    angles = np.arange(262144)*2*np.pi/262144
    root = np.linalg.cholesky(np.array([[1.3, -.2], [-.2, .7]]))
    points = np.array(center)[:, None]+radius*root@np.array([np.cos(angles), np.sin(angles)])
    values = eigenvalue*points[0]**2+points[1]
    np.testing.assert_allclose(endpoints, [values.min(), values.max()], rtol=1e-8, atol=1e-8)


def test_independent_quadrature_limits_and_convergence():
    radius = 2.1
    assert a.critical_cdf(radius, 0) == pytest.approx(math.erf(radius/math.sqrt(2)))
    assert a.critical_cdf(radius, 1e10) == pytest.approx(1-math.exp(-radius*radius/2), abs=1e-9)
    for curvature in (.001, .1, 1., 100.):
        critical = a.critical_value(curvature)
        assert abs(a.critical_cdf(critical, curvature)-.95) < 1e-10
        assert abs(a.critical_cdf(critical, curvature, 64)-.95) < 1e-7


def fixture():
    q = [1.]*len(a.Q_FIELDS)
    q[a.Q_FIELDS.index('upper')] = 2.
    rows = [dict(kind='exact_modes', truth=[0.]*4),
            dict(kind='start', replication=0, seed=18, order=[0, 1], y=[.5, -.3]),
            dict(kind='capture_variance', variance=[1., 1.])]
    rows += [dict(kind='capture_target', target=t, status=0, mode=[1., 0.], ratio=[.1, .2],
                  influence=[.2, .1], q=q.copy(), critical_draws=100000) for t in range(4)]
    rows += [dict(kind='end', replication=0, status='success', points=[1.]*4,
                  covariance=[1.]*16, widths=[1.]*4, critical=[1.]*4)]
    base = {(0, t): dict(seed=18, status='success', point_error=1., width=1., critical=1., variance=1.)
            for t in a.TARGETS}
    return rows, base


def test_valid_capture_inventory():
    rows, base = fixture()
    assert len(run.validate(rows, [0], [], base, 2)[1]) == 1


@pytest.mark.parametrize('problem', ['missing', 'duplicate', 'partial', 'seed', 'row_order',
    'nan', 'null', 'target', 'status', 'critical_draws', 'variance', 'changed_baseline', 'full_rejection'])
def test_rejects_malformed_or_scientifically_changed_capture(problem):
    rows, base = fixture()
    if problem == 'missing': rows.pop()
    elif problem == 'duplicate': rows.append(copy.deepcopy(rows[-1]))
    elif problem == 'partial': rows[3]['q'].pop()
    elif problem == 'seed': rows[1]['seed'] = 19
    elif problem == 'row_order': rows[1]['order'] = [0, 0]
    elif problem == 'nan': rows[3]['q'][0] = float('nan')
    elif problem == 'null': rows[3]['q'][0] = None
    elif problem == 'target': rows[4]['target'] = 0
    elif problem == 'status': rows[3]['status'] = 9
    elif problem == 'critical_draws': rows[3]['critical_draws'] = 0
    elif problem == 'variance': rows[2]['variance'][0] = 0.
    elif problem == 'changed_baseline': base[0, 'firm']['width'] = 2.
    elif problem == 'full_rejection': rows[-1]['status'] = 'failed'
    with pytest.raises((ValueError, KeyError)):
        run.validate(rows, [0], [], base, 2)
