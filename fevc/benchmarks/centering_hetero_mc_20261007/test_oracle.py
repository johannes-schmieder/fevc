"""Independent small-fixture checks for the heteroskedastic Gaussian audit."""
import numpy as np
import pandas as pd
import pytest

from exact_oracle import build_matrices, gaussian_moments, oracle


def fixture():
    # Uneven complete worker--firm support produces nontrivial target weights.
    rows = [(w, f, k) for w in range(4) for f in range(3)
            for k in range(2 + ((w + f) % 3))]
    d = pd.DataFrame(rows, columns=["workerid", "firmid", "time"])
    d["alpha_true"] = d.workerid.map({0:-.5, 1:-.1, 2:.3, 3:.7})
    d["psi_true"] = d.firmid.map({0:-.3, 1:.1, 2:.4})
    d["signal"] = 3 + d.alpha_true + d.psi_true
    d["sigma2_true"] = np.exp(2*d.alpha_true*d.psi_true + .5*d.firmid)
    return d


def test_gaussian_moments_diagonal_closed_form():
    weights, mu, v = np.array([2., -1., .5]), np.array([1., 3., -2.]), np.array([.5, 2., 3.])
    mean, sd = gaussian_moments(np.diag(weights), mu, v)
    assert mean == pytest.approx(np.sum(weights*(mu**2+v)))
    assert sd**2 == pytest.approx(np.sum(weights**2*(2*v**2+4*v*mu**2)))


def test_corrected_unbiased_for_arbitrary_diagonal_variances():
    d = fixture()
    result = oracle(d)
    moments = pd.DataFrame(result["moments"])
    assert moments[moments["mode"].isin(["none", "corrected"])].bias.abs().max() < 1e-11
    assert moments[moments["mode"] == "mean"].bias.abs().max() > 1e-6
    assert result["diagnostics"]["residualized_variance_rms"] > 0


def test_firm_only_heterogeneity_has_no_mean_bias():
    d = fixture()
    d.sigma2_true = np.exp(d.firmid)
    result = oracle(d)
    assert result["diagnostics"]["residualized_variance_rms"] < 1e-12
    assert max(abs(r["bias"]) for r in result["moments"] if r["mode"] != "plugin") < 1e-11


def test_dense_quadratics_match_direct_corrected_formula():
    d = fixture()
    matrices, _, _ = build_matrices(d)
    D = pd.get_dummies(d.workerid, dtype=float).to_numpy()
    F = pd.get_dummies(d.firmid, dtype=float).to_numpy()[:, 1:]
    X = np.column_stack([D, F]); n = len(d)
    A = np.linalg.inv(X.T @ X); M = np.eye(n)-X@A@X.T
    groups = list(d.groupby(["workerid", "firmid"]).indices.values())
    Z = np.column_stack([np.isin(np.arange(n), g).astype(float) for g in groups])
    masses = np.array([len(g) for g in groups])
    aa = np.array([M[np.ix_(g,g)].sum()/len(g) for g in groups])
    K = n*np.diag(aa)-Z.T@M@Z
    rng = np.random.default_rng(73951)
    for _ in range(3):
        y = d.signal.to_numpy()+np.sqrt(d.sigma2_true)*rng.normal(size=n)
        z, e = y-y.mean(), M@y
        r, xi, t = np.zeros(n), np.zeros(n), np.zeros(n)
        for g in groups:
            r[g] = np.linalg.solve(M[np.ix_(g,g)], e[g])
            t[g] = z[g]*r[g].sum()
            xi[g] = (t[g]-r[g]*z[g].sum())/len(g)
        b = Z.T@M@t - n*np.array([np.sum(M[np.ix_(g,g)]@xi[g]) for g in groups])
        kappa = np.linalg.solve(K,b)
        for modes in matrices.values():
            B = modes["plugin"]
            none, mean, delta = y@B@y, y@B@y, 0.
            for j,g in enumerate(groups):
                Bgg = B[np.ix_(g,g)]
                none -= y[g]@Bgg@r[g]
                mean -= z[g]@Bgg@r[g]
                delta -= np.ones(len(g))@Bgg@(xi[g]+kappa[j]/masses[j])
            np.testing.assert_allclose([y@modes["none"]@y,y@modes["mean"]@y,y@modes["corrected"]@y],
                                       [none,mean,mean+delta], atol=2e-12,rtol=0)


def test_gaussian_paired_moments_and_point_outputs():
    d = fixture(); n = len(d)
    draws = np.column_stack([d.signal.to_numpy(), d.signal.to_numpy()+np.arange(n)/n])
    result = oracle(d,draws, sampling_reps=128)
    matrices,_,_ = build_matrices(d)
    for row in result["paired_centering"]:
        modes = matrices[row["target"]]
        delta = modes["mean"]-modes["corrected"]
        mean,sd = gaussian_moments(delta,d.signal.to_numpy(),d.sigma2_true.to_numpy())
        assert row["expected_mean_minus_corrected"] == pytest.approx(mean)
        assert row["simulation_se"] == pytest.approx(sd/np.sqrt(128))
    for row in result["moments"]:
        matrix = matrices[row["target"]][row["mode"]]
        np.testing.assert_allclose(row["points"],[y@matrix@y for y in draws.T], atol=1e-13)


@pytest.mark.parametrize("bad", [np.full(36,-1.), np.full(36,np.nan), np.ones(1)])
def test_invalid_variance_rejected(bad):
    with pytest.raises(ValueError, match="conditional variances"):
        oracle(fixture(),variance=bad)


def test_missing_variance_and_non_akm_signal_rejected():
    d = fixture()
    with pytest.raises(ValueError, match="sigma2_true"):
        oracle(d.drop(columns="sigma2_true"))
    d.loc[0,"signal"] += .1
    with pytest.raises(ValueError, match="signal"):
        oracle(d)
