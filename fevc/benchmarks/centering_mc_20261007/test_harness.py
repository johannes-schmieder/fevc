import copy
import csv
import importlib.util
from pathlib import Path

import pytest

spec = importlib.util.spec_from_file_location("centering_mc", Path(__file__).with_name("run.py"))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


def fixture_rows():
    units = m.cells("smoke")[:1]
    u = units[0]
    rows = []
    for key in sorted(m.keys(units)):
        phase, dataset, rep, alg, mode, probes = key
        r = dict.fromkeys(m.FIELDS, "0")
        r.update(phase=phase, dataset=str(dataset), rep=str(rep), algorithm=alg,
                 centering=mode, probes=str(probes), seed=str(m.seed("sketch", phase, dataset, rep, probes) if probes else 0),
                 outcome_seed=str(u["outcome_seed"]), n="573", sample_ok="1", available="1",
                 mcse_mode="all", mcse_method="exact" if alg=="exact" else "crossfit_if_v1",
                 mcse_status="exact_zero" if alg=="exact" else "ok_local",
                 mcse_centering={"none":"uncentered", "mean":"fixed observed mean", "corrected":"fixed observed mean and fixed centering increment"}[mode])
        rows.append(r)
    return units, rows


def write(path, rows):
    with path.open("w") as f:
        w = csv.DictWriter(f, fieldnames=m.FIELDS)
        w.writeheader()
        w.writerows(rows)


@pytest.mark.parametrize("damage", ["missing", "duplicate", "partial", "seed", "sample", "identity", "mcse", "covariance", "metadata"])
def test_reject_invalid(tmp_path, damage):
    units, rows = fixture_rows()
    if damage == "missing": rows = []
    if damage == "partial": rows.pop()
    if damage == "duplicate": rows.append(copy.deepcopy(rows[0]))
    if damage == "seed": rows[0]["seed"] = "123"
    if damage == "sample": rows[0]["n"] = "572"
    if damage == "identity": rows[0]["point4"] = "1"
    if damage == "mcse": rows[0]["se1"] = "."
    if damage == "covariance": rows[0]["cov12"] = "1"
    if damage == "metadata": rows[0]["mcse_centering"] = "wrong"
    path = tmp_path / "out.csv"
    write(path, rows)
    with pytest.raises(ValueError): m.validate(path, units)


def test_malformed(tmp_path):
    path = tmp_path / "out.csv"
    path.write_text("incorrect,schema\n1,2\n")
    with pytest.raises(ValueError): m.validate(path, m.cells("smoke"))


def test_count_all_fit_failures(tmp_path):
    units, rows = fixture_rows()
    rows[0]["rc"] = "498"
    rows[1]["rc"] = "430"
    path = tmp_path / "out.csv"
    write(path, rows)
    result = m.validate(path, units)
    assert result["status"] == "FAIL" and len(result["failures"]) == 2
    assert result["success_rate"] == 4/6


def test_valid_and_schedule_independent(tmp_path):
    units, rows = fixture_rows()
    path = tmp_path / "out.csv"
    write(path, rows[::-1])
    assert m.validate(path, units)["status"] == "PASS"
    all_units = m.cells("main")
    assert set.union(*(m.keys(all_units[i::4]) for i in range(4))) == m.keys(all_units)
    assert len(m.keys(all_units)) == 17409


def test_noiseless_signal_explicit(tmp_path):
    code = m.make_do(tmp_path, tmp_path, m.cells("smoke")[:1])
    assert "gen double signal=lnwage_true-epsilon_true" in code
    assert "assert abs(signal-3-alpha_true-psi_true)<1e-12" in code
    assert "replace y=signal+1.5*rnormal()" in code
    assert "replace y=lnwage_true+" not in code
