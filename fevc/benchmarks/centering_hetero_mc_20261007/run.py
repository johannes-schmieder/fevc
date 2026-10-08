"""Frozen, synthetic centering experiment; Python standard library only on SCC."""
from __future__ import annotations

import argparse
import concurrent.futures
import csv
import hashlib
import json
import math
import os
from pathlib import Path
import re
import subprocess
import sys
import time

MODES = ("none", "mean", "corrected")
FIELDS = ["case", "phase", "dataset", "rep", "algorithm", "centering", "probes", "seed",
          "outcome_seed", "rc", "n", "sample_ok", "available", "mcse_mode",
          "mcse_method", "mcse_status", "mcse_centering"]
FIELDS += [f"{prefix}{i}" for prefix in ("point", "plugin", "se") for i in range(1, 5)]
FIELDS += [f"cov{i}{j}" for i in range(1, 5) for j in range(1, 5)]
DGP = ("fesim, dgp(akm) preset(stylized) workers(100) firms(15) periods(6) "
       "burnin(5) seed(7102026) truth(basic) connectivity(keep) "
       "parameters(mu 3 sd_worker .45 sd_firm .3 sd_error 1.5 firm_size_sd .25 "
       "theta_sort 1.5 kappa_ee -.69314718056 kappa_eu -4 "
       "het_worker {het_worker} het_firm {het_firm} het_interaction {het_interaction}) noreport clear")
CASES = [dict(case="homo", het_worker=0, het_firm=0, het_interaction=0),
         dict(case="firm", het_worker=0, het_firm=2, het_interaction=0),
         dict(case="interaction", het_worker=0, het_firm=0, het_interaction=2)]
CASE_FIELDS = {"case", "het_worker", "het_firm", "het_interaction"}
FIXTURE_FIELDS = ["workerid", "time", "firmid", "lnwage", "alpha_true", "psi_true",
                  "lnwage_true", "epsilon_true", "conditional_mean_true", "sigma2_true", "signal"]
GENERATED_FIELDS = ["workerid", "time", "firmid", "employed", "alpha_true", "psi_true",
                    "conditional_mean_true", "sigma2_true"]
EXPECTED = dict(generated=600, employed=574, retained=573, workers=99, firms=15,
                generated_times=list(range(2000, 2006)))


def validate_cases(cases):
    if not isinstance(cases, list) or not cases:
        raise ValueError("cases must be a nonempty list")
    seen = set()
    for case in cases:
        if set(case) != CASE_FIELDS or not isinstance(case["case"], str) or not re.fullmatch(r"[a-z][a-z0-9_]*", case["case"]):
            raise ValueError("invalid case fields or name")
        if case["case"] in seen:
            raise ValueError("duplicate case")
        seen.add(case["case"])
        if any(type(case[k]) not in (int, float) or not math.isfinite(case[k]) for k in CASE_FIELDS - {"case"}):
            raise ValueError("invalid heteroskedasticity coefficient")


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def seed(*key):
    # Canonical semantic keys make tasks independent of order and process count.
    return 1 + int.from_bytes(hashlib.sha256(json.dumps(key).encode()).digest()[:8], "big") % 2147483646


def cells(profile, cases=CASES):
    if profile not in ("smoke", "main"):
        raise ValueError("unknown profile")
    validate_cases(cases)
    nr, ns = (2, 2) if profile == "smoke" else (2000, 300)
    units = []
    for rep in range(1, nr + 1):
        units.append(dict(phase="sampling", dataset=0, rep=rep,
                          outcome_seed=seed("outcome", rep), probes=[0, 256]))
    for dataset in range(1, 4):
        units.append(dict(phase="calibration", dataset=dataset, rep=0,
                          outcome_seed=seed("fixed-outcome", dataset), probes=[0]))
        for rep in range(1, ns + 1):
            units.append(dict(phase="calibration", dataset=dataset, rep=rep,
                              outcome_seed=seed("fixed-outcome", dataset), probes=[64, 256]))
    return [dict(case=c["case"], **u) for c in cases for u in units]


def keys(units):
    return {(u["case"], u["phase"], u["dataset"], u["rep"], "exact" if p == 0 else "jla", c, p)
            for u in units for p in u["probes"] for c in MODES}


def make_do(root, output, units, case, fail=False):
    if not units or any(u["case"] != case["case"] for u in units):
        raise ValueError("a task must contain exactly one nonempty case")
    lines = ["version 19", "clear all", "set more off", "set processors 1",
             "set rng mt64", "set linesize 255",
             f'adopath ++ "{root}/input/fevc"', f'adopath ++ "{root}/input/fesim"',
             "quietly fevc_rust probe", "assert r(centering_api)==1", "assert r(numerical_api)==2",
             f"quietly {DGP.format(**case)}", "assert _N==600", "isid workerid time",
             "quietly count if employed", "assert r(N)==574",
             "assert sigma2_true>0 & sigma2_true<. if employed",
             "quietly summarize sigma2_true if employed, meanonly", "assert abs(r(mean)-2.25)<1e-10",
             "assert abs(conditional_mean_true-3-alpha_true-psi_true)<1e-12 if employed",
             "sort workerid time", "format alpha_true psi_true conditional_mean_true sigma2_true %24.17g",
             f'export delimited {" ".join(GENERATED_FIELDS)} using "{output}/fixture.generated.csv", replace datafmt',
             "keep if employed", "gen double y=lnwage",
             "quietly fevc y, worker(workerid) firm(firmid) algorithm(exact) "
             "backend(rust) centering(none) nativethreads(1) nodisplay",
             "keep if e(sample)", "assert _N==573", "isid workerid time", "sort workerid time",
             "gen double signal=conditional_mean_true",
             "assert abs(signal-3-alpha_true-psi_true)<1e-12",
             "assert abs(signal-lnwage_true+epsilon_true)<1e-12",
             "format lnwage alpha_true psi_true lnwage_true epsilon_true signal conditional_mean_true sigma2_true %24.17g",
             f'export delimited {" ".join(FIXTURE_FIELDS)} using "{output}/fixture.csv", replace datafmt',
             'file open mc_out using "' + str(output) + '/results.partial.csv", write replace',
             'file write mc_out "' + ",".join(FIELDS) + '" _n',
             "program define mc_fit",
             " args case phase dataset rep algorithm centering probes seed outcome_seed",
             " local opts",
             ' if "`algorithm\'"=="jla" local opts probes(`probes\') seed(`seed\') rng(counter_v1)',
             " capture quietly fevc y, worker(workerid) firm(firmid) algorithm(`algorithm') "
             "centering(`centering') backend(rust) deletion(match) stayers(both) mcse(all) "
             "nativethreads(1) tolerance(1e-10) nodisplay `opts'",
             " local rc=_rc",
             ' file write mc_out "`case\',`phase\',`dataset\',`rep\',`algorithm\',`centering\',`probes\',`seed\',`outcome_seed\',`rc\'"',
             " if `rc' {",
             '  file write mc_out ",.,0,0,all,failed,fit_failed,unavailable"',
             "  forvalues j=1/28 {", '   file write mc_out ",."', "  }",
             " }", " else {",
             "  quietly count if !e(sample)", "  local sample_ok=(r(N)==0)",
             '  file write mc_out "," (e(N)) ",`sample_ok\'," (e(mcse_available)) ",`e(mcse_mode)\',`e(mcse_method)\',`e(mcse_status)\',`e(mcse_centering)\'"',
             "  foreach name in kss plugin mcse {", "   matrix v=e(`name')",
             "   forvalues j=1/4 {", '    file write mc_out "," %24.17g (v[1,`j\'])', "   }", "  }",
             "  matrix v=e(mcse_cov_raw)", "  forvalues i=1/4 {", "   forvalues j=1/4 {",
             '    file write mc_out "," %24.17g (v[`i\',`j\'])', "   }", "  }", " }",
             " file write mc_out _n", "end"]
    if fail:
        lines += ["error 459"]
    for u in units:
        lines += [f'set seed {u["outcome_seed"]}', "quietly replace y=signal+sqrt(sigma2_true)*rnormal()"]
        if u["phase"] == "sampling" and u["rep"] == 1:
            lines += ["format y %24.17g", f'export delimited workerid time y using "{output}/oracle-y.csv", replace datafmt']
        for p in u["probes"]:
            s = seed("sketch", u["phase"], u["dataset"], u["rep"], p) if p else 0
            for c in MODES:
                lines += [f'mc_fit {u["case"]} {u["phase"]} {u["dataset"]} {u["rep"]} '
                          f'{"jla" if p else "exact"} {c} {p} {s} {u["outcome_seed"]}']
    lines += ["file close mc_out", 'display "CENTERING_HETERO_MC_STATA_PASS"']
    return "\n".join(lines) + "\n"


def number(value):
    return math.nan if value.strip() in ("", ".") else float(value)


def read_csv(path, fields):
    with Path(path).open() as f:
        reader = csv.DictReader(f)
        if reader.fieldnames != fields:
            raise ValueError("CSV schema mismatch")
        rows = list(reader)
    if any(None in r or any(v is None for v in r.values()) for r in rows):
        raise ValueError("malformed CSV row")
    return rows


def centered_midranks(values):
    ordered = sorted(values.items(), key=lambda item: item[1])
    scores = {}
    start = 0
    while start < len(ordered):
        end = start + 1
        while end < len(ordered) and ordered[end][1] == ordered[start][1]:
            end += 1
        score = (start + end) / len(ordered) - 1
        scores.update((key, score) for key, _ in ordered[start:end])
        start = end
    return scores


def validate_fixture(output, case):
    generated = read_csv(output / "fixture.generated.csv", GENERATED_FIELDS)
    retained = read_csv(output / "fixture.csv", FIXTURE_FIELDS)
    if len(generated) != EXPECTED["generated"] or len(retained) != EXPECTED["retained"]:
        raise ValueError("fixture dimensions")
    for rows in (generated, retained):
        ids = [(int(r["workerid"]), int(r["time"])) for r in rows]
        if len(set(ids)) != len(ids) or ids != sorted(ids):
            raise ValueError("fixture duplicate or unordered row key")
    employed = [r for r in generated if r["employed"] == "1"]
    if len(employed) != EXPECTED["employed"] or any(r["employed"] not in ("0", "1") for r in generated):
        raise ValueError("fixture employment inventory")
    if {(int(r["workerid"]), int(r["time"])) for r in generated} != {(i, t) for i in range(1, 101) for t in EXPECTED["generated_times"]}:
        raise ValueError("fixture generated worker/time keys")
    alpha, psi = {}, {}
    for r in generated:
        worker = int(r["workerid"])
        value = number(r["alpha_true"])
        if not math.isfinite(value) or (worker in alpha and alpha[worker] != value):
            raise ValueError("fixture inconsistent worker effects")
        alpha[worker] = value
        if r["employed"] == "1":
            firm = int(r["firmid"])
            value = number(r["psi_true"])
            if not math.isfinite(value) or (firm in psi and psi[firm] != value):
                raise ValueError("fixture inconsistent firm effects")
            psi[firm] = value
        elif not math.isnan(number(r["sigma2_true"])) or not math.isnan(number(r["conditional_mean_true"])):
            raise ValueError("fixture nonemployment truth must be unavailable")
    if len(alpha) != 100 or len(psi) != 15:
        raise ValueError("fixture full latent population not observed")
    worker_score, firm_score = centered_midranks(alpha), centered_midranks(psi)
    eta = [case["het_worker"]*worker_score[int(r["workerid"])]+
           case["het_firm"]*firm_score[int(r["firmid"])]+
           case["het_interaction"]*worker_score[int(r["workerid"])]*firm_score[int(r["firmid"])]
           for r in employed]
    exp_eta = [math.exp(e-max(eta)) for e in eta]
    scale = sum(exp_eta)/len(exp_eta)
    for r, profile in zip(employed, exp_eta):
        if not math.isclose(number(r["sigma2_true"]), 2.25*profile/scale, rel_tol=2e-12, abs_tol=1e-12):
            raise ValueError("fixture variance profile differs from case parameters")
    gen_bykey = {(r["workerid"], r["time"]): r for r in employed}
    for r in employed + retained:
        vals = {k: number(r[k]) for k in ("alpha_true", "psi_true", "conditional_mean_true", "sigma2_true")}
        if not all(math.isfinite(v) for v in vals.values()) or vals["sigma2_true"] <= 0:
            raise ValueError("fixture invalid truth/variance")
        if abs(vals["conditional_mean_true"] - 3 - vals["alpha_true"] - vals["psi_true"]) > 1e-12:
            raise ValueError("fixture conditional mean identity")
    for r in retained:
        if not all(math.isfinite(number(r[k])) for k in FIXTURE_FIELDS):
            raise ValueError("fixture nonfinite retained values")
        g = gen_bykey.get((r["workerid"], r["time"]))
        if g is None or any(r[k] != g[k] for k in ("firmid", "alpha_true", "psi_true", "conditional_mean_true", "sigma2_true")):
            raise ValueError("retained/generated fixture mismatch")
        if abs(number(r["signal"])-number(r["conditional_mean_true"])) > 1e-12 or abs(number(r["signal"])-number(r["lnwage_true"])+number(r["epsilon_true"])) > 1e-12:
            raise ValueError("fixture signal identity")
    if len({r["workerid"] for r in retained}) != EXPECTED["workers"] or len({r["firmid"] for r in retained}) != EXPECTED["firms"]:
        raise ValueError("fixture worker/firm dimensions")
    variances = [number(r["sigma2_true"]) for r in employed]
    if abs(sum(variances)/len(variances)-2.25) > 1e-10:
        raise ValueError("fixture generated variance normalization")
    if all(case[k] == 0 for k in CASE_FIELDS - {"case"}) and any(abs(v-2.25) > 1e-12 for v in variances):
        raise ValueError("homoskedastic fixture variance")
    a = [number(r["alpha_true"]) for r in retained]
    b = [number(r["psi_true"]) for r in retained]
    covariance = sum(x*y for x, y in zip(a, b))/len(a)-(sum(a)/len(a))*(sum(b)/len(b))
    if covariance <= 0:
        raise ValueError("fixture nonpositive true covariance")
    # Exclude generated shocks and variance profile, but bind row ordering and all
    # mean-design truth so shared outcome/probe seeds imply paired experiments.
    invariant = [[r[k] for k in ("workerid", "time", "firmid", "alpha_true", "psi_true", "signal")] for r in retained]
    return dict(design_sha256=hashlib.sha256(json.dumps(invariant).encode()).hexdigest(),
                fixture_sha256=digest(output / "fixture.csv"),
                generated_fixture_sha256=digest(output / "fixture.generated.csv"),
                generated=len(generated), employed=len(employed), retained=len(retained),
                workers=len(set(r["workerid"] for r in retained)), firms=len(set(r["firmid"] for r in retained)),
                sigma2_generated_mean=sum(variances)/len(variances),
                sigma2_retained_mean=sum(number(r["sigma2_true"]) for r in retained)/len(retained),
                sigma2_min=min(variances), sigma2_max=max(variances),
                sigma2_ratio=max(variances)/min(variances), covariance_true=covariance)


def validate_success(row, key):
    withheld = None
    if number(row["n"]) != EXPECTED["retained"] or row["sample_ok"] != "1":
        raise ValueError(f"sample mismatch {key}")
    convention = {"none": "uncentered", "mean": "fixed observed mean",
                  "corrected": "fixed observed mean and fixed centering increment"}[key[5]]
    if row["mcse_mode"] != "all" or row["mcse_centering"] != convention:
        raise ValueError(f"MCSE metadata {key}")
    if row["mcse_method"] != ("exact" if key[4] == "exact" else "crossfit_if_v1"):
        raise ValueError(f"MCSE method {key}")
    for prefix in ("point", "plugin"):
        v = [number(row[f"{prefix}{i}"]) for i in range(1, 5)]
        if not all(math.isfinite(x) for x in v) or abs(v[3]-v[0]-v[1]-2*v[2]) > 1e-8:
            raise ValueError(f"target identity {key}")
    se = [number(row[f"se{i}"]) for i in range(1, 5)]
    if row["available"] == "1":
        if not all(math.isfinite(x) and x >= 0 for x in se):
            raise ValueError(f"invalid usable SE {key}")
        if row["mcse_status"] not in ("exact_zero", "ok_local", "ok_local_psd_adjusted"):
            raise ValueError(f"MCSE status {key}")
    elif row["available"] == "0" and all(math.isnan(x) for x in se):
        withheld = dict(key=key, status=row["mcse_status"])
    else:
        raise ValueError(f"invalid withholding {key}")
    cov = [[number(row[f"cov{i}{j}"]) for j in range(1, 5)] for i in range(1, 5)]
    if all(math.isfinite(x) for rr in cov for x in rr):
        for i in range(4):
            for j in range(4):
                if abs(cov[i][j]-cov[j][i]) > 1e-8:
                    raise ValueError(f"asymmetric covariance {key}")
            if abs(cov[i][3]-cov[i][0]-cov[i][1]-2*cov[i][2]) > 1e-8:
                raise ValueError(f"total covariance identity {key}")
    elif row["available"] == "1":
        raise ValueError(f"missing usable covariance {key}")
    if key[4] == "exact" and (any(se) or row["mcse_method"] != "exact"):
        raise ValueError(f"exact MCSE {key}")
    return withheld


def validate(path, units):
    rows = read_csv(path, FIELDS)
    expected = keys(units)
    seen, failures, withheld = set(), [], []
    mapping = {(u["case"], u["phase"], u["dataset"], u["rep"]): u for u in units}
    for row in rows:
        key = (row["case"], row["phase"], int(row["dataset"]), int(row["rep"]), row["algorithm"],
               row["centering"], int(row["probes"]))
        if key not in expected or key in seen:
            raise ValueError(f"unexpected/duplicate key {key}")
        seen.add(key)
        u = mapping[key[:4]]
        want_seed = seed("sketch", *key[1:4], key[6]) if key[6] else 0
        if int(row["seed"]) != want_seed or int(row["outcome_seed"]) != u["outcome_seed"]:
            raise ValueError(f"seed mismatch {key}")
        if int(row["rc"]) != 0:
            failures.append(dict(key=key, rc=int(row["rc"]), category="fit"))
            continue
        try:
            withheld_cell = validate_success(row, key)
            if withheld_cell:
                withheld.append(withheld_cell)
        except (ValueError, OverflowError) as exc:
            failures.append(dict(key=key, rc=0, category="scientific", error=str(exc)))
    if seen != expected:
        raise ValueError(f"incomplete inventory: got {len(seen)}, expected {len(expected)}")
    bykey = {(r["case"], r["phase"], r["dataset"], r["rep"], r["algorithm"], r["probes"], r["centering"]): r for r in rows}
    invalid_keys = {tuple(failure["key"]) for failure in failures}
    for k, r in bykey.items():
        if k[-1] != "corrected" or r["rc"] != "0":
            continue
        mean = bykey[k[:-1] + ("mean",)]
        if mean["rc"] != "0":
            continue
        key = (r["case"], r["phase"], int(r["dataset"]), int(r["rep"]), r["algorithm"], r["centering"], int(r["probes"]))
        if key in invalid_keys or key[:5] + ("mean", key[6]) in invalid_keys:
            continue
        for f in [f"se{i}" for i in range(1,5)] + [f"cov{i}{j}" for i in range(1,5) for j in range(1,5)]:
            x, y = number(r[f]), number(mean[f])
            if (math.isnan(x) != math.isnan(y)) or (not math.isnan(x) and abs(x-y) > 1e-12):
                if not any(tuple(failure["key"]) == key for failure in failures):
                    failures.append(dict(key=key, rc=0, category="scientific", error=f"Mean/Corrected MCSE mismatch {k}"))
                break
    return dict(rows=len(rows), failures=failures, withheld=withheld,
                success_rate=(len(rows)-len(failures))/len(rows), status="PASS" if not failures else "FAIL")


def run_task(root, profile, case, task, units, stata, fail=False):
    output = root / "output" / profile / case["case"] / f"task-{task:02d}"
    output.mkdir(parents=True, exist_ok=False)
    (output / "analysis.do").write_text(make_do(root, output, units, case, fail))
    (output / "driver.do").write_text(f'version 19\ncapture noisily do "{output}/analysis.do"\nlocal rc=_rc\nexit `rc\', clear\n')
    start = time.monotonic()
    with (output / "console.txt").open("w") as console:
        run = subprocess.run([stata, "-b", "do", str(output / "driver.do")], cwd=output,
                             stdout=console, stderr=subprocess.STDOUT)
    log = output / "driver.log"
    raw = log.read_text(errors="replace") if log.exists() else ""
    # Keep license-bearing startup banners out of collected logs.
    raw = raw[raw.find(". "):] if ". " in raw else ""
    raw = "\n".join(x for x in raw.splitlines() if not re.search(r"Licensed to:|Serial number:", x))
    log.write_text(raw + "\n")
    receipt = dict(task=task, profile=profile, case=case["case"], case_parameters=case,
                   expected_rows=len(keys(units)), expected_units=units,
                   output_path=str(output.relative_to(root)), seconds=time.monotonic()-start,
                   process_rc=run.returncode, manifest_sha256=digest(root / "manifest.json"),
                   stata_success=bool(re.search(r"^CENTERING_HETERO_MC_STATA_PASS$", raw, re.M)),
                   job_id=os.environ.get("JOB_ID"), host=os.uname().nodename)
    try:
        if run.returncode or not receipt["stata_success"]:
            raise ValueError("Stata process/application failure")
        receipt["fixture"] = validate_fixture(output, case)
        receipt.update(validate(output / "results.partial.csv", units))
        receipt["results_sha256"] = digest(output / "results.partial.csv")
        (output / "results.partial.csv").rename(output / "results.csv")
    except Exception as exc:
        receipt.update(status="FAIL", error=str(exc))
    (output / "receipt.partial.json").write_text(json.dumps(receipt, indent=2)+"\n")
    (output / "receipt.partial.json").rename(output / "receipt.json")
    return receipt


def main():
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    p.add_argument("--profile", choices=["smoke", "main"], required=True)
    p.add_argument("--stata", required=True)
    p.add_argument("--workers", type=int, default=1)
    p.add_argument("--deliberate-failure", action="store_true")
    a = p.parse_args()
    root = a.root.resolve()
    manifest = json.loads((root / "manifest.json").read_text())
    if manifest["schema"] != "FEVC_CENTERING_HETERO_MC_V1" or manifest["expected"] != EXPECTED:
        raise ValueError("unexpected campaign schema/dimensions")
    validate_cases(manifest["cases"])
    for rel, sha in manifest["files"].items():
        if digest(root / rel) != sha:
            raise ValueError(f"changed frozen input {rel}")
    if sys.platform == "darwin" and digest(root / "input/fevc/fevc_rust_macos_arm64.plugin") != manifest["local_validation_plugin_sha256"]:
        raise ValueError("changed local validation plugin")
    units = manifest["profiles"][a.profile]
    if units != cells(a.profile, manifest["cases"]):
        raise ValueError("profile differs from registered task manifest")
    if not 1 <= a.workers <= int(os.environ.get("NSLOTS", "4")):
        raise ValueError("invalid worker allocation")
    groups = []
    for case in manifest["cases"]:
        case_units = [u for u in units if u["case"] == case["case"]]
        for i in range(min(a.workers, len(case_units))):
            groups.append((case, i+1, case_units[i::a.workers]))
    with concurrent.futures.ThreadPoolExecutor(a.workers) as pool:
        futures = [pool.submit(run_task, root, a.profile, c, i, u, a.stata, a.deliberate_failure)
                   for c, i, u in groups]
        receipts = [f.result() for f in futures]
    print(json.dumps([{k: v for k, v in r.items() if k not in ("expected_units",)}
                      for r in receipts], indent=2), flush=True)
    if not all(r["status"] == "PASS" for r in receipts):
        raise SystemExit(1)
    if len({r["fixture"]["design_sha256"] for r in receipts}) != 1:
        raise ValueError("design/mean truth differs across tasks or variance cases")
    for case in manifest["cases"]:
        if len({r["fixture"]["fixture_sha256"] for r in receipts if r["case"] == case["case"]}) != 1:
            raise ValueError("case fixture differs across tasks")
    print("CENTERING_HETERO_MC_RUN_PASS")


if __name__ == "__main__":
    main()
