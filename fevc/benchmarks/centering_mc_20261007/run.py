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
import time

MODES = ("none", "mean", "corrected")
FIELDS = ["phase", "dataset", "rep", "algorithm", "centering", "probes", "seed",
          "outcome_seed", "rc", "n", "sample_ok", "available", "mcse_mode",
          "mcse_method", "mcse_status", "mcse_centering"]
FIELDS += [f"{prefix}{i}" for prefix in ("point", "plugin", "se") for i in range(1, 5)]
FIELDS += [f"cov{i}{j}" for i in range(1, 5) for j in range(1, 5)]
DGP = ("fesim, dgp(akm) preset(stylized) workers(100) firms(15) periods(6) "
       "burnin(5) seed(7102026) truth(basic) connectivity(keep) "
       "parameters(mu 3 sd_worker .45 sd_firm .3 sd_error 1.5 firm_size_sd .25 "
       "theta_sort 1.5 kappa_ee -.69314718056 kappa_eu -4) noreport clear")


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def seed(*key):
    # Canonical semantic keys make tasks independent of order and process count.
    return 1 + int.from_bytes(hashlib.sha256(json.dumps(key).encode()).digest()[:8], "big") % 2147483646


def cells(profile):
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
    return units


def keys(units):
    return {(u["phase"], u["dataset"], u["rep"], "exact" if p == 0 else "jla", c, p)
            for u in units for p in u["probes"] for c in MODES}


def make_do(root, output, units, fail=False):
    lines = ["version 19", "clear all", "set more off", "set processors 1",
             "set rng mt64", "set linesize 255",
             f'adopath ++ "{root}/input/fevc"', f'adopath ++ "{root}/input/fesim"',
             "quietly fevc_rust probe", "assert r(centering_api)==1", "assert r(numerical_api)==2",
             f"quietly {DGP}", "keep if employed", "gen double y=lnwage",
             "quietly fevc y, worker(workerid) firm(firmid) algorithm(exact) "
             "backend(rust) centering(none) nativethreads(1) nodisplay",
             "keep if e(sample)", "assert _N==573", "isid workerid time", "sort workerid time",
             "gen double signal=lnwage_true-epsilon_true",
             "assert abs(signal-3-alpha_true-psi_true)<1e-12",
             "format lnwage alpha_true psi_true lnwage_true epsilon_true signal %24.17g",
             f'export delimited workerid time firmid lnwage alpha_true psi_true lnwage_true epsilon_true signal using "{output}/fixture.csv", replace datafmt',
             'file open mc_out using "' + str(output) + '/results.partial.csv", write replace',
             'file write mc_out "' + ",".join(FIELDS) + '" _n',
             "program define mc_fit",
             " args phase dataset rep algorithm centering probes seed outcome_seed",
             " local opts",
             ' if "`algorithm\'"=="jla" local opts probes(`probes\') seed(`seed\') rng(counter_v1)',
             " capture quietly fevc y, worker(workerid) firm(firmid) algorithm(`algorithm') "
             "centering(`centering') backend(rust) deletion(match) stayers(both) mcse(all) "
             "nativethreads(1) tolerance(1e-10) nodisplay `opts'",
             " local rc=_rc",
             ' file write mc_out "`phase\',`dataset\',`rep\',`algorithm\',`centering\',`probes\',`seed\',`outcome_seed\',`rc\'"',
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
        lines += [f'set seed {u["outcome_seed"]}', "quietly replace y=signal+1.5*rnormal()"]
        for p in u["probes"]:
            s = seed("sketch", u["phase"], u["dataset"], u["rep"], p) if p else 0
            for c in MODES:
                lines += [f'mc_fit {u["phase"]} {u["dataset"]} {u["rep"]} '
                          f'{"jla" if p else "exact"} {c} {p} {s} {u["outcome_seed"]}']
    lines += ["file close mc_out", 'display "CENTERING_MC_STATA_PASS"']
    return "\n".join(lines) + "\n"


def number(value):
    return math.nan if value.strip() in ("", ".") else float(value)


def validate(path, units):
    with Path(path).open() as f:
        reader = csv.DictReader(f)
        if reader.fieldnames != FIELDS:
            raise ValueError("CSV schema mismatch")
        rows = list(reader)
    expected = keys(units)
    seen, failures, withheld = set(), [], []
    mapping = {(u["phase"], u["dataset"], u["rep"]): u for u in units}
    for row in rows:
        key = (row["phase"], int(row["dataset"]), int(row["rep"]), row["algorithm"],
               row["centering"], int(row["probes"]))
        if key not in expected or key in seen:
            raise ValueError(f"unexpected/duplicate key {key}")
        seen.add(key)
        u = mapping[key[:3]]
        want_seed = seed("sketch", *key[:3], key[5]) if key[5] else 0
        if int(row["seed"]) != want_seed or int(row["outcome_seed"]) != u["outcome_seed"]:
            raise ValueError(f"seed mismatch {key}")
        if int(row["rc"]) != 0:
            failures.append(dict(key=key, rc=int(row["rc"])))
            continue
        if number(row["n"]) != 573 or row["sample_ok"] != "1":
            raise ValueError(f"sample mismatch {key}")
        convention = {"none": "uncentered", "mean": "fixed observed mean",
                      "corrected": "fixed observed mean and fixed centering increment"}[key[4]]
        if row["mcse_mode"] != "all" or row["mcse_centering"] != convention:
            raise ValueError(f"MCSE metadata {key}")
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
            withheld.append(dict(key=key, status=row["mcse_status"]))
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
        if key[3] == "exact" and (any(se) or row["mcse_method"] != "exact"):
            raise ValueError(f"exact MCSE {key}")
    if seen != expected:
        raise ValueError(f"incomplete inventory: got {len(seen)}, expected {len(expected)}")
    bykey = {(r["phase"], r["dataset"], r["rep"], r["algorithm"], r["probes"], r["centering"]): r for r in rows}
    for k, r in bykey.items():
        if k[-1] != "corrected" or r["rc"] != "0":
            continue
        mean = bykey[k[:-1] + ("mean",)]
        if mean["rc"] != "0":
            continue
        for f in [f"se{i}" for i in range(1,5)] + [f"cov{i}{j}" for i in range(1,5) for j in range(1,5)]:
            x, y = number(r[f]), number(mean[f])
            if not (math.isnan(x) and math.isnan(y)) and abs(x-y) > 1e-12:
                raise ValueError(f"Mean/Corrected MCSE mismatch {k}")
    return dict(rows=len(rows), failures=failures, withheld=withheld,
                success_rate=(len(rows)-len(failures))/len(rows), status="PASS" if not failures else "FAIL")


def run_task(root, profile, task, units, stata, fail=False):
    output = root / "output" / profile / f"task-{task:02d}"
    output.mkdir(parents=True, exist_ok=False)
    (output / "analysis.do").write_text(make_do(root, output, units, fail))
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
    receipt = dict(task=task, profile=profile, seconds=time.monotonic()-start,
                   process_rc=run.returncode, manifest_sha256=digest(root / "manifest.json"),
                   stata_success=bool(re.search(r"^CENTERING_MC_STATA_PASS$", raw, re.M)),
                   job_id=os.environ.get("JOB_ID"), host=os.uname().nodename)
    try:
        if run.returncode or not receipt["stata_success"]:
            raise ValueError("Stata process/application failure")
        receipt.update(validate(output / "results.partial.csv", units))
        receipt["results_sha256"] = digest(output / "results.partial.csv")
        (output / "results.partial.csv").rename(output / "results.csv")
    except Exception as exc:
        receipt.update(status="FAIL", error=str(exc))
    (output / "receipt.json").write_text(json.dumps(receipt, indent=2)+"\n")
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
    for rel, sha in manifest["files"].items():
        if digest(root / rel) != sha:
            raise ValueError(f"changed frozen input {rel}")
    units = manifest["profiles"][a.profile]
    if units != cells(a.profile):
        raise ValueError("profile differs from registered task manifest")
    if not 1 <= a.workers <= int(os.environ.get("NSLOTS", "4")):
        raise ValueError("invalid worker allocation")
    groups = [units[i::a.workers] for i in range(a.workers)]
    with concurrent.futures.ThreadPoolExecutor(a.workers) as pool:
        futures = [pool.submit(run_task, root, a.profile, i+1, u, a.stata, a.deliberate_failure)
                   for i, u in enumerate(groups)]
        receipts = [f.result() for f in futures]
    print(json.dumps(receipts, indent=2), flush=True)
    if not all(r["status"] == "PASS" for r in receipts):
        raise SystemExit(1)
    print("CENTERING_MC_RUN_PASS")


if __name__ == "__main__":
    main()
