import math
import numpy as np
import pytest
from numerics import ellipse, critical, critical_cdf, summarize_intervals, analyze_target


@pytest.mark.parametrize('eigenvalue', [-2., -.01, 0., .01, 2.])
def test_quartic_against_dense_angular_grid(eigenvalue):
    center = np.array([.7, -.2]); cov = np.array([[1.3, -.17], [-.17, .2]])
    theta = np.arange(500000)*2*math.pi/500000
    x = center[:, None]+2.13*np.linalg.cholesky(cov)@np.array([np.cos(theta), np.sin(theta)])
    y = eigenvalue*x[0]**2+x[1]
    assert np.allclose(ellipse(center, cov, 2.13, eigenvalue), [min(y), max(y)], rtol=1e-9, atol=1e-9)


@pytest.mark.parametrize('curvature', [0., .005, .01, .05, .25, 1., 4., 20., 100.])
def test_quadrature_resolution(curvature):
    r = critical(curvature)
    assert abs(r-critical(curvature, 192)) < 1e-8
    assert abs(critical_cdf(r, curvature)-.95) < 1e-10


def test_fail_closed():
    with pytest.raises(np.linalg.LinAlgError):
        ellipse([0., 0.], [[1., 2.], [2., 1.]], 2., 1.)
    with pytest.raises(ValueError):
        ellipse([math.nan, 0.], [[1., 0.], [0., 1.]], 2., 1.)
    with pytest.raises(ValueError):
        summarize_intervals([[2., 1.]], 1.)
    with pytest.raises(ValueError):
        analyze_target([{'replication': 0}]*400, 0., 0)
