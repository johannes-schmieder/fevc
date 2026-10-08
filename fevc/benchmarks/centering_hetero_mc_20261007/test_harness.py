import copy
import csv
import importlib.util
import math
from pathlib import Path

import pytest

spec = importlib.util.spec_from_file_location("centering_hetero_mc", Path(__file__).with_name("run.py"))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


def fixture_rows():
    units = m.cells("smoke")[:1]
    u = units[0]
    rows = []
    for key in sorted(m.keys(units)):
        case, phase, dataset, rep, alg, mode, probes = key
        r = dict.fromkeys(m.FIELDS, "0")
        r.update(case=case, phase=phase, dataset=str(dataset), rep=str(rep), algorithm=alg,
                 centering=mode, probes=str(probes), seed=str(m.seed("sketch", phase, dataset, rep, probes) if probes else 0),
                 outcome_seed=str(u["outcome_seed"]), n="573", sample_ok="1", available="1",
                 mcse_mode="all", mcse_method="exact" if alg == "exact" else "crossfit_if_v1",
                 mcse_status="exact_zero" if alg == "exact" else "ok_local",
                 mcse_centering={"none": "uncentered", "mean": "fixed observed mean", "corrected": "fixed observed mean and fixed centering increment"}[mode])
        rows.append(r)
    return units, rows


def write(path, rows, fields=None):
    with path.open("w") as f:
        w = csv.DictWriter(f, fieldnames=fields or m.FIELDS)
        w.writeheader()
        w.writerows(rows)


@pytest.mark.parametrize("damage", ["missing", "duplicate", "partial", "seed", "outcome_seed", "sample", "identity", "mcse", "covariance", "metadata", "case", "method"])
def test_reject_invalid(tmp_path, damage):
    units, rows = fixture_rows()
    if damage == "missing": rows = []
    if damage == "partial": rows.pop()
    if damage == "duplicate": rows.append(copy.deepcopy(rows[0]))
    if damage == "seed": rows[0]["seed"] = "123"
    if damage == "outcome_seed": rows[0]["outcome_seed"] = "123"
    if damage == "sample": rows[0]["n"] = "572"
    if damage == "identity": rows[0]["point4"] = "1"
    if damage == "mcse": rows[0]["se1"] = "."
    if damage == "covariance": rows[0]["cov12"] = "1"
    if damage == "metadata": rows[0]["mcse_centering"] = "wrong"
    if damage == "case": rows[0]["case"] = "unexpected"
    if damage == "method": rows[0]["mcse_method"] = "unknown"
    path = tmp_path / "out.csv"
    write(path, rows)
    if damage in ("missing", "duplicate", "partial", "seed", "outcome_seed", "case"):
        with pytest.raises(ValueError): m.validate(path, units)
    else:
        result = m.validate(path, units)
        assert result["status"] == "FAIL" and result["failures"]


@pytest.mark.parametrize("contents", ["incorrect,schema\n1,2\n", ",".join(m.FIELDS)+"\n1\n", ",".join(m.FIELDS)+"\n"+",".join(["0"]*(len(m.FIELDS)+1))+"\n"])
def test_malformed(tmp_path, contents):
    path = tmp_path / "out.csv"
    path.write_text(contents)
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


def test_count_mixed_fit_and_scientific_failures(tmp_path):
    units, rows = fixture_rows()
    rows[0]["rc"] = "498"
    rows[1]["point4"] = "1"
    rows[2]["n"] = "570"
    path = tmp_path / "out.csv"
    write(path, rows)
    result = m.validate(path, units)
    assert result["status"] == "FAIL" and len(result["failures"]) == 3
    assert result["success_rate"] == .5
    assert [f["category"] for f in result["failures"]] == ["fit", "scientific", "scientific"]


def test_bad_numeric_cells_preserve_failure_audit(tmp_path):
    units, rows = fixture_rows()
    rows[0]["se1"] = "bad"
    rows[1]["cov11"] = "bad"
    rows[2]["rc"] = "430"
    path = tmp_path / "out.csv"
    write(path, rows)
    result = m.validate(path, units)
    assert result["status"] == "FAIL" and len(result["failures"]) == 3
    assert result["success_rate"] == .5


def test_valid_and_schedule_independent(tmp_path):
    units, rows = fixture_rows()
    path = tmp_path / "out.csv"
    write(path, rows[::-1])
    assert m.validate(path, units)["status"] == "PASS"
    all_units = m.cells("main")
    assert set.union(*(m.keys(all_units[i::4]) for i in range(4))) == m.keys(all_units)
    assert len(m.keys(all_units)) == 3*17409
    bycase = [[{k: v for k, v in u.items() if k != "case"} for u in all_units if u["case"] == c["case"]] for c in m.CASES]
    assert bycase[0] == bycase[1] == bycase[2]


def test_noiseless_signal_explicit_and_profile_matched(tmp_path):
    code = m.make_do(tmp_path, tmp_path, m.cells("smoke")[:1], m.CASES[0])
    assert "gen double signal=conditional_mean_true" in code
    assert "assert abs(signal-3-alpha_true-psi_true)<1e-12" in code
    assert "replace y=signal+sqrt(sigma2_true)*rnormal()" in code
    assert "oracle-y.csv" in code
    with pytest.raises(ValueError): m.make_do(tmp_path, tmp_path, m.cells("smoke")[:1], m.CASES[1])


@pytest.mark.parametrize("cases", [[m.CASES[0], m.CASES[0]], [dict(m.CASES[0], unknown=1)], [dict(m.CASES[0], case="../unsafe")], [dict(m.CASES[0], het_firm=math.inf)], []])
def test_invalid_cases(cases):
    with pytest.raises(ValueError): m.validate_cases(cases)


def generated_fixture(case):
    rows = []
    for i in range(1, 101):
        for t in range(1, 7):
            # 99 retained workers; worker 100 contributes one extra employed row.
            employed = i < 96 or (i in (96, 97, 98) and t == 1) or (i == 99 and t == 1) or (i == 100 and t == 1)
            # 570 + 3 + 1 + 1 = 575; remove worker95,time6 for 574.
            employed = employed and (i, t) != (95, 6)
            firm = 1+(i-1) % 15
            alpha, psi = i/100, firm/100
            rows.append(dict(workerid=str(i), time=str(t+1999), firmid=str(firm if employed else 0),
                             employed=str(int(employed)), alpha_true=str(alpha), psi_true=str(psi if employed else "."),
                             conditional_mean_true=str(3+alpha+psi if employed else "."), sigma2_true="."))
    employed = [r for r in rows if r["employed"] == "1"]
    a = m.centered_midranks({i: i/100 for i in range(1, 101)})
    b = m.centered_midranks({j: j/100 for j in range(1, 16)})
    weights = [math.exp(case["het_worker"]*a[int(r["workerid"])]+case["het_firm"]*b[int(r["firmid"])]+case["het_interaction"]*a[int(r["workerid"])]*b[int(r["firmid"])]) for r in employed]
    mean = sum(weights)/len(weights)
    for r, w in zip(employed, weights): r["sigma2_true"] = str(2.25*w/mean)
    kept = []
    for r in employed[:-1]:
        mean = float(r["conditional_mean_true"])
        kept.append({k: r[k] for k in m.FIXTURE_FIELDS if k in r} | dict(lnwage=str(mean+.5), lnwage_true=str(mean+.5), epsilon_true="0.5", signal=str(mean)))
    return rows, kept


@pytest.mark.parametrize("case", m.CASES)
def test_fixture_profiles_valid(tmp_path, case):
    gen, retained = generated_fixture(case)
    write(tmp_path / "fixture.generated.csv", gen, m.GENERATED_FIELDS)
    write(tmp_path / "fixture.csv", retained, m.FIXTURE_FIELDS)
    receipt = m.validate_fixture(tmp_path, case)
    assert receipt["retained"] == 573 and receipt["covariance_true"] > 0


@pytest.mark.parametrize("damage", ["missing", "duplicate", "variance", "variance_nan", "signal", "nonemployed", "worker", "firm", "profile", "row_order", "sigma2_retained", "nan_error"])
def test_fixture_reject_invalid(tmp_path, damage):
    case = m.CASES[2]
    gen, retained = generated_fixture(case)
    if damage == "missing": gen.pop()
    if damage == "duplicate": gen[-1] = copy.deepcopy(gen[0])
    if damage == "variance": gen[0]["sigma2_true"] = "-1"
    if damage == "variance_nan": gen[0]["sigma2_true"] = "."
    if damage == "signal": retained[0]["signal"] = "-10"
    if damage == "nonemployed": gen[-1]["sigma2_true"] = "2.25"
    if damage == "worker": gen[0]["alpha_true"] = "0.22"
    if damage == "firm": gen[0]["psi_true"] = "0.22"
    if damage == "profile": case = m.CASES[1]
    if damage == "row_order": retained.reverse()
    if damage == "sigma2_retained": retained[0]["sigma2_true"] = "2.25"
    if damage == "nan_error": retained[0]["epsilon_true"] = "."
    write(tmp_path / "fixture.generated.csv", gen, m.GENERATED_FIELDS)
    write(tmp_path / "fixture.csv", retained, m.FIXTURE_FIELDS)
    with pytest.raises(ValueError): m.validate_fixture(tmp_path, case)


def test_midrank_ties():
    assert m.centered_midranks({1: 0, 2: 0}) == {1: 0, 2: 0}
    assert m.centered_midranks({1: 1, 2: 2, 3: 2, 4: 3}) == {1: -.75, 2: 0, 3: 0, 4: .75}
