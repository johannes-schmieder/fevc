from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import sys

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location(
    "fixed_offset_diagnostic", ROOT / "fevc/tools/diagnose_fixed_offset.py")
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def dense(form):
    return np.diag(form.diagonal) + form.factors @ form.core @ form.factors.T


def physical_matrices(design):
    group, frequency = design["group"], design["frequency"]
    n, g = len(group), len(design["mass"])
    sigma = np.zeros((n, n))
    for start, count, block in zip(design["starts"], design["counts"], design["blocks"]):
        sigma[start:start+count, start:start+count] = block
    collapse = np.zeros((g, n))
    collapse[group, np.arange(n)] = frequency / np.sqrt(design["mass"][group])
    return design["z"][group], sigma, collapse


@pytest.mark.parametrize("seed", [917, 1311])
def test_factorized_traces_against_dense(seed):
    rng = np.random.default_rng(seed)
    n = 11
    f, u = rng.normal(size=(n, 4)), rng.normal(size=(n, 2))
    h = rng.normal(size=(4, 4))
    h = (h+h.T)/2
    kernel = MODULE.Form(rng.normal(size=n), f, h)
    d = rng.uniform(0.5, 1.5, n)
    omega = MODULE.Form(d, u, np.array([[0.3, 0.02], [0.02, 0.2]]))
    k, o, diagonal = dense(kernel), dense(omega), np.diag(d)
    assert np.allclose(kernel.diag(), np.diag(k), atol=1e-12)
    assert np.allclose(kernel.apply(u), k @ u, atol=1e-12)
    assert kernel.trace_product(omega) == pytest.approx(np.trace(k@o), abs=1e-10)
    assert kernel.diagonal_sandwich_trace(d) == pytest.approx(np.trace(k@diagonal@k@diagonal), rel=1e-12)
    assert kernel.mixed_trace(d, omega) == pytest.approx(np.trace(k@diagonal@k@o), rel=1e-12)
    assert kernel.sandwich_trace(omega) == pytest.approx(np.trace(k@o@k@o), rel=1e-12)
    mean = rng.normal(size=n)
    moments = MODULE.moments(kernel, mean, omega, d)
    expected = 4*mean@k@o@k@mean + 2*np.trace(k@o@k@o)
    assert moments["variance"] == pytest.approx(expected, rel=1e-12)
    naive = 4*mean@k@diagonal@k@mean + 4*np.trace(k@diagonal@k@o)-2*np.trace(k@diagonal@k@diagonal)
    assert moments["expected_diagonal_variance_estimator"] == pytest.approx(naive, rel=1e-12)
    with pytest.raises(ValueError, match="same base"):
        kernel.mixed_trace(d*2, omega)


@pytest.mark.parametrize("k,loadings", [(3, "historical"), (5, "bounded")])
def test_physical_wls_covariance_and_quadratic_oracle(k, loadings):
    design = MODULE.make_design(k, loadings)
    z, sigma, collapse = physical_matrices(design)
    controls, w = design["controls"], np.diag(design["frequency"])
    full = np.column_stack((z, controls))
    joint_loading = np.linalg.solve(full.T@w@full, full.T@w)
    gamma_loading = joint_loading[-2:]
    assert np.allclose(design["h_transpose"].T, gamma_loading, atol=2e-10)
    adjusted = collapse@(np.eye(len(z))-controls@gamma_loading)
    exact_omega = adjusted@sigma@adjusted.T
    assert np.allclose(dense(design["known"]), collapse@sigma@collapse.T, atol=1e-11)
    assert np.allclose(dense(design["estimated"]), exact_omega, atol=1e-10)
    assert np.linalg.eigvalsh(exact_omega).min() > 0
    assert np.allclose(design["gamma_covariance"], gamma_loading@sigma@gamma_loading.T, atol=1e-10)
    loading = design["inverse_gram"]@design["x"].T
    assert np.allclose(loading@adjusted, joint_loading[:-2], atol=1e-10)
    p = design["x"]@loading
    maker = np.eye(k*k)-p
    for a in design["targets"]:
        b = loading.T@a@loading
        ratio = np.diag(b)/np.diag(maker)
        exact_kernel = b-(ratio[:, None]*maker+maker*ratio[None, :])/2
        form = MODULE.kernel(design["q"], design["root"].T@a@design["root"])
        assert np.allclose(dense(form), exact_kernel, atol=1e-11)
        # Independent fourth-moment formula on original correlated physical rows.
        physical_kernel = adjusted.T@exact_kernel@adjusted
        mu = z@design["alpha"]
        exact_variance = (4*mu@physical_kernel@sigma@physical_kernel@mu
                          + 2*np.trace(physical_kernel@sigma@physical_kernel@sigma))
        observed = MODULE.moments(form, design["mean"], design["estimated"], design["tau"])
        assert observed["variance"] == pytest.approx(exact_variance, rel=1e-9, abs=1e-11)
        assert observed["bias"] == pytest.approx(np.trace(physical_kernel@sigma), abs=1e-10)


def test_known_offset_zero_bias_and_unbiased_variance():
    rows = MODULE.diagnose(MODULE.make_design(4, "historical"))
    assert [r["target"] for r in rows] == list(MODULE.TARGETS)
    for row in rows:
        for component in ("point", "remainder"):
            moments = row["known"][component]
            assert moments["bias"] == pytest.approx(0, abs=1e-11)
            assert moments["expected_diagonal_variance_estimator"] == pytest.approx(moments["variance"], rel=1e-10)
        assert 0 < row["leading_share"] <= 1
        assert 0 < row["remainder_concentration"] <= 1+1e-10


def test_fe_loading_changes_do_not_change_control_identification():
    historical = MODULE.make_design(5, "historical")
    bounded = MODULE.make_design(5, "bounded")
    for key in ("residual_controls", "control_gram", "gamma_covariance"):
        assert np.allclose(historical[key], bounded[key], rtol=1e-10, atol=1e-10)
    assert not np.allclose(dense(historical["estimated"]), dense(bounded["estimated"]))
    baseline = MODULE.make_design(20, "historical")
    same = MODULE.make_design(20, "bounded")
    assert np.array_equal(baseline["controls"], same["controls"])
    assert np.array_equal(baseline["estimated"].factors, same["estimated"].factors)


@pytest.mark.parametrize("k,loadings", [(2, "historical"), (3, "invalid"), (3.2, "bounded")])
def test_invalid_design(k, loadings):
    with pytest.raises(ValueError, match="invalid"):
        MODULE.make_design(k, loadings)


def test_output_inventory_rejects_missing_duplicate_wrong_and_nonfinite():
    registration = {"cases": [{"k": 4, "loadings": "historical"}], "expected_target_rows": 4}
    rows = MODULE.diagnose(MODULE.make_design(4, "historical"))
    MODULE.validate_rows(rows, registration)
    for bad in (rows[:-1], rows+[rows[0]], [{**rows[0], "k": 5}]+rows[1:]):
        with pytest.raises(ValueError, match="inventory"):
            MODULE.validate_rows(bad, registration)
    bad = copy.deepcopy(rows)
    bad[0]["truth"] = float("nan")
    with pytest.raises(ValueError):
        MODULE.validate_rows(bad, registration)


def test_manifest_new_only_and_full_pipeline(tmp_path, monkeypatch):
    root = tmp_path / "source"
    (root / "fevc/docs").mkdir(parents=True)
    registration = {"cases": [{"k": 4, "loadings": "historical"}], "expected_target_rows": 4}
    (root / MODULE.REGISTRATION).write_text(json.dumps(registration))
    monkeypatch.setattr(MODULE, "ROOT", root)
    monkeypatch.setattr(MODULE, "source_identity", lambda: {"commit": "a"*40, "files": {}})
    manifest, output = tmp_path/"manifest.json", tmp_path/"output"
    MODULE.create_manifest(manifest)
    with pytest.raises(FileExistsError):
        MODULE.create_manifest(manifest)
    MODULE.run(manifest, output)
    receipt = json.loads((output/"receipt.json").read_text())
    assert receipt["rows"] == 4 and receipt["coverage_claim"] is False
    assert receipt["rows_sha256"] == MODULE.digest(output/"rows.json")
    assert not list(output.glob("*.partial"))
    with pytest.raises(FileExistsError):
        MODULE.run(manifest, output)
    for key, value in (("source", {}), ("expected_rows", 3), ("registration", {}),
                       ("random_outcomes", True), ("outputs", ["rows.json"])):
        broken = json.loads(manifest.read_text())
        broken[key] = value
        altered = tmp_path/f"{key}.json"
        altered.write_text(json.dumps(broken))
        with pytest.raises(ValueError, match="identity"):
            MODULE.run(altered, tmp_path/f"out-{key}")
        assert not (tmp_path/f"out-{key}").exists()


def test_registration_is_deterministic_and_excludes_promotion():
    registration = json.loads((ROOT/MODULE.REGISTRATION).read_text())
    assert registration["expected_target_rows"] == 20
    assert registration["methods"]["random_outcomes"] is False
    assert registration["methods"]["production_modification"] is False
    assert registration["methods"]["second_stage_correction"] is False


def test_reject_incomplete_production_export(tmp_path):
    path = tmp_path/"empty.jsonl"
    path.write_text("{}\n")
    with pytest.raises(ValueError, match="incomplete"):
        MODULE.verify_export(path)
