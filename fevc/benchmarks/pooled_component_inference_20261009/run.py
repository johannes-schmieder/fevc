"""Freeze, execute and audit a bounded paired Mean component assessment.

Results live outside the tracked harness. Every Stata process executes an
immutable package copy, not the moving development checkout.
"""
from __future__ import annotations

import argparse
import concurrent.futures
import csv
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import time

import numpy as np
import scipy

from pooled_oracle import TARGETS, build_design, evaluate, moments, seed
from pooled_numerical import numerical_cells, summarize_numerical

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
REGISTRATION = ROOT / "fevc/docs/pooled_component_inference_validation_v1.json"
AMENDMENT = ROOT / "fevc/docs/pooled_component_inference_validation_v1_amendment1.json"
NUMERICAL_REGISTRATION = ROOT / "fevc/docs/pooled_component_inference_numerical_v1.json"
SCHEMA = "FEVC_POOLED_COMPONENT_ASSESSMENT_ROW_V1"
FIELDS = ["cell", "replication", "arm", "target", "rc", "target_status", "sample_ok", "n", "physical_n",
          "point", "se", "lower", "upper", "centering", "inference_centering", "failure", "identity_error",
          "point_mcse", "mcse_available", "mcse_mode", "mcse_centering", "failure_phase",
          "q1_var_b", "q1_cov_br", "q1_var_r", "q1_b", "q1_remainder",
          "point_probes", "gram_probes", "covariance_simulations"]
ARMS = ("mean", "fixed_c0")
STATISTICAL_FAILURES = {"NEGATIVE_INFERENCE_VARIANCE", "INFERENCE_VARIANCE_INVALID",
                        "INFERENCE_COVARIANCE_NOT_PSD", "Q1_COVARIANCE_INVALID"}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def atomic_json(path, value):
    path = Path(path)
    partial = path.with_suffix(path.suffix + ".partial")
    partial.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")
    partial.replace(path)


def cell_id(cell):
    return f'{cell["route"]}_{cell["reference"]}_k{cell["k"]}_s{round(100*cell["stayer_share"])}_{cell["diagnostic"]}'


def cells(profile):
    registration = json.loads(REGISTRATION.read_text())
    if profile=="numerical": return numerical_cells(json.loads(NUMERICAL_REGISTRATION.read_text()))
    if profile == "diagnostic":
        values = [dict(k=registration["diagnostic_grid"]["dimension"], **c) for c in registration["diagnostic_grid"]["cells"]]
    elif profile in registration["profiles"]:
        values = [dict(route=r, reference=q, k=registration["primary"]["dimension"], stayer_share=share, diagnostic="primary")
                  for r in registration["primary"]["routes"] for q in registration["primary"]["references"]
                  for share in registration["primary"]["stayer_shares"]]
    else:
        raise ValueError("unknown profile")
    return [dict(cell=cell_id(c), **c) for c in values]


def numerical_seed(cell):
    return seed("pooled-component-v1", "fixed-numerical", cell)


def task_numerics(task):
    return task["cell"].get("numerics",dict(point_probes=200,gram_probes=2048,covariance_simulations=1000,seed=numerical_seed(task["cell"]["cell"])))


def outcome_cell(cell):
    return cell.get("outcome_cell",cell["cell"])


def row_key(row):
    return row["cell"], int(row["replication"]), row["arm"], row["target"]


def expected_keys(task):
    return {(task["cell"]["cell"], r, a, t) for r in task["replications"] for a in ARMS for t in TARGETS}


def parse_number(value):
    return None if str(value).strip() in ("", ".") else float(value)


def verify_oracle(actual, expected):
    """Certify arithmetic within the registered tolerance, not bitwise BLAS."""
    if isinstance(expected, dict):
        if set(actual) != set(expected): raise ValueError("oracle preflight fields changed")
        for key in expected: verify_oracle(actual[key], expected[key])
    elif isinstance(expected, list):
        if len(actual) != len(expected): raise ValueError("oracle preflight shape changed")
        for a, b in zip(actual, expected): verify_oracle(a, b)
    elif isinstance(expected, float):
        if not math.isfinite(actual) or abs(actual-expected)>1e-10*max(1.,abs(expected)):
            raise ValueError("oracle preflight arithmetic changed")
    elif actual != expected:
        raise ValueError("oracle preflight identity changed")


def validate_rows(rows, task, manifest):
    if len(rows) != len(expected_keys(task)):
        raise ValueError("partial or extra row count")
    keys = [row_key(row) for row in rows]
    if len(set(keys)) != len(keys):
        raise ValueError("duplicate result key")
    if set(keys) != expected_keys(task):
        raise ValueError("unexpected result inventory")
    geometry = manifest["cells"][task["cell"]["cell"]]["oracle"]["diagnostics"]
    calls={}
    route,reference=task["cell"]["route"],task["cell"]["reference"]
    allowed_statuses=({0,1} if reference=="highrank" else set(range(6))) if route=="mata" else ({0,1,4,6} if reference=="highrank" else {0,1,2,3,6})
    for row in rows:
        calls.setdefault((row["cell"],row["replication"],row["arm"]),[]).append(row)
        if row.get("schema") != SCHEMA or row.get("manifest_sha256") != manifest["identity"]:
            raise ValueError("wrong schema or source/manifest")
        if row.get("profile") != manifest["profile"]:
            raise ValueError("wrong profile")
        wanted = seed("pooled-component-v1", manifest["domain"], outcome_cell(task["cell"]), int(row["replication"]))
        numerics=task_numerics(task)
        if row.get("outcome_seed") != wanted or row.get("numerical_seed") != numerics["seed"]:
            raise ValueError("wrong seed")
        if row["rc"] == 0:
            if row["sample_ok"] != 1 or row["n"] != geometry["stored_rows"] or row["physical_n"] != geometry["physical_rows"]:
                raise ValueError("wrong retained sample/count")
            if row["centering"] != ("mean" if row["arm"] == "mean" else "none"):
                raise ValueError("wrong centering")
            convention="fixed observed mean" if row["arm"]=="mean" else "uncentered"
            if row["inference_centering"]!=convention or row["mcse_centering"]!=convention:
                raise ValueError("wrong inference or MCSE centering convention")
            if row["covariance_simulations"]!=numerics["covariance_simulations"] or (route!="mata" and (row["point_probes"]!=numerics["point_probes"] or row["gram_probes"]!=numerics["gram_probes"])):
                raise ValueError("wrong numerical budget receipt")
            if row["target_status"] not in allowed_statuses:
                raise ValueError("unknown target status")
            if row["point"] is None or not math.isfinite(row["point"]):
                raise ValueError("nonfinite point")
            interval_fields=("se","lower","upper") if reference=="highrank" else ("lower","upper","q1_var_b","q1_cov_br","q1_var_r","q1_b","q1_remainder")
            if row["target_status"] == 0 and any(row[k] is None or not math.isfinite(row[k]) for k in interval_fields):
                raise ValueError("successful interval has nonfinite values")
            if row["target_status"] == 0 and ((row["se"] is not None and row["se"] < 0) or row["lower"] > row["upper"]):
                raise ValueError("invalid interval")
            if reference=="q1" and row["target_status"]==0 and (row["q1_var_b"]<=0 or row["q1_var_r"]-row["q1_cov_br"]**2/row["q1_var_b"]<=0):
                raise ValueError("successful q1 has inadmissible joint covariance")
            if row["target_status"]!=0 and (row["lower"] is not None or row["upper"] is not None):
                raise ValueError("unavailable target has interval endpoints")
            identity = row.get("identity_error")
            if reference=="q1" and row["target_status"]==0 and identity is None:
                raise ValueError("successful q1 lacks remainder identity")
            if identity is not None and (not math.isfinite(identity) or identity > 1e-8):
                raise ValueError("q1 remainder identity")
            exact = row["oracle_point"]
            limit=1e-8*max(1.,abs(exact),abs(row["point"]))
            if route!="mata":
                if row["mcse_available"]!=1 or row["mcse_mode"]!="all" or row["point_mcse"] is None or not math.isfinite(row["point_mcse"]) or row["point_mcse"]<0:
                    raise ValueError("native point lacks usable MCSE")
                limit=max(limit,6*row["point_mcse"])
            if abs(row["point"]-exact)>limit:
                raise ValueError("independent dense point disagreement")
        elif row["rc"]!=498 or row.get("failure") not in STATISTICAL_FAILURES:
            raise ValueError("engineering or unclassified failure blocks assessment")
    atomic_fields=("rc","failure","failure_phase","sample_ok","n","physical_n","centering","inference_centering","mcse_available","mcse_mode","mcse_centering","point_probes","gram_probes","covariance_simulations")
    for group in calls.values():
        if any(any(row[field]!=group[0][field] for field in atomic_fields) for row in group[1:]):
            raise ValueError("inconsistent atomic call rows")
    return rows


def primary_targets(cell):
    return TARGETS if cell["reference"] == "highrank" else ("worker", "firm", "total")


def summarize(rows, manifest):
    """Enumerate all failures, with availability distinct from coverage."""
    summaries, failures = [], []
    screens = manifest["registration"]["descriptive_screens"]
    grouped = {}
    for row in rows:
        grouped.setdefault((row["cell"], row["arm"], row["target"]), []).append(row)
    for (name, arm, target), group in sorted(grouped.items()):
        cell = manifest["cells"][name]["spec"]
        oracle = manifest["cells"][name]["oracle"]
        index = TARGETS.index(target)
        truth = oracle["truth"][index]
        sd_true = math.sqrt(oracle["actual_mean" if arm == "mean" else "fixed_c0"]["covariance"][index][index])
        success = [r for r in group if r["rc"] == 0 and r["target_status"] == 0]
        finite_points = [r["point"] for r in group if r["rc"] == 0]
        failure_counts = {}
        for row in group:
            if row["rc"] or row["target_status"]:
                status = row["failure"] if row["rc"] else f'target_status_{row["target_status"]}'
                failure_counts[status] = failure_counts.get(status, 0) + 1
                failures.append(dict(key=list(row_key(row)), status=status))
        covered = [float(r["lower"] <= truth <= r["upper"]) for r in success]
        coverage = float(np.mean(covered)) if covered else None
        se = [r["se"] for r in success if r["se"] is not None and math.isfinite(r["se"])]
        point_errors = np.asarray(finite_points) - truth
        bias = float(point_errors.mean()) if len(point_errors) else None
        point_sd = float(np.std(finite_points, ddof=1)) if len(finite_points) > 1 else None
        se_ratio = float(np.sqrt(np.mean(np.square(se))) / point_sd) if se and point_sd else None
        n = len(group)
        item = dict(cell=name, arm=arm, target=target, primary=cell["diagnostic"] == "primary" and target in primary_targets(cell),
                    attempts=n, successes=len(success), point_successes=len(finite_points), success_rate=len(success)/n,
                    coverage_success=coverage, coverage_all=sum(covered)/n, failures=failure_counts,
                    bias=bias, bias_over_oracle_sd=bias/sd_true if bias is not None and sd_true else None,
                    empirical_sd=point_sd, oracle_sd=sd_true, highrank_se_available=len(se),rms_se_over_empirical_sd=se_ratio,
                    mean_interval_width=float(np.mean([r["upper"]-r["lower"] for r in success])) if success else None,
                    lower_miss=sum(r["lower"]>truth for r in success)/n,
                    upper_miss=sum(r["upper"]<truth for r in success)/n,
                    screens=[])
        if cell["reference"]=="q1":
            item["q1_joint_covariance"]=dict(
                comparison_population="computed q1 targets; availability and all attempts reported separately",
                mean_var_b=float(np.mean([r["q1_var_b"] for r in success])) if success else None,
                mean_cov_br=float(np.mean([r["q1_cov_br"] for r in success])) if success else None,
                mean_var_r=float(np.mean([r["q1_var_r"] for r in success])) if success else None,
                empirical_var_b=float(np.var([r["q1_b"] for r in success],ddof=1)) if len(success)>1 else None,
                empirical_cov_br=float(np.cov([r["q1_b"] for r in success],[r["q1_remainder"] for r in success])[0,1]) if len(success)>1 else None,
                empirical_var_r=float(np.var([r["q1_remainder"] for r in success],ddof=1)) if len(success)>1 else None)
        if item["primary"]:
            if item["success_rate"] < screens["success_rate_minimum"]: item["screens"].append("success_rate")
            if bias is not None and abs(bias) > screens["absolute_bias_over_oracle_sd_maximum"]*sd_true + screens["bias_mcse_multiplier"]*sd_true/math.sqrt(max(1,len(finite_points))): item["screens"].append("bias")
            mcse = math.sqrt(.95*.05/max(1,len(success)))
            if coverage is None or coverage + screens["coverage_mcse_multiplier"]*mcse < screens["coverage_lower"]: item["screens"].append("coverage")
            lo, hi = screens["q0_rms_se_over_empirical_sd"]
            if cell["reference"] == "highrank" and (se_ratio is None or not lo <= se_ratio <= hi): item["screens"].append("se_ratio")
        summaries.append(item)
    paired = []
    pairs = {}
    for row in rows:
        pairs.setdefault((row["cell"], row["replication"], row["target"]), {})[row["arm"]] = row
    for name in sorted(manifest["cells"]):
        for t, target in enumerate(TARGETS):
            relevant = [v for k,v in pairs.items() if k[0] == name and k[2] == target]
            if not relevant: continue
            truth = manifest["cells"][name]["oracle"]["truth"][t]
            contrasts = []
            for value in relevant:
                if set(value) != set(ARMS): raise ValueError("missing paired arm")
                indicators = {}
                for arm, row in value.items():
                    indicators[arm] = int(row["rc"] == 0 and row["target_status"] == 0 and row["lower"] <= truth <= row["upper"])
                contrasts.append(indicators["fixed_c0"] - indicators["mean"])
            drop = float(np.mean(contrasts))
            mcse = float(np.std(contrasts, ddof=1)/np.sqrt(len(contrasts))) if len(contrasts)>1 else 0.
            primary = manifest["cells"][name]["spec"]["diagnostic"] == "primary" and target in primary_targets(manifest["cells"][name]["spec"])
            paired.append(dict(cell=name,target=target,all_attempt_coverage_drop=drop,paired_mcse=mcse,
                               screen_failed=primary and drop > screens["paired_coverage_drop_maximum"] + screens["paired_mcse_multiplier"]*mcse))
    failed = [dict(cell=r["cell"],arm=r["arm"],target=r["target"],screens=r["screens"]) for r in summaries if r["screens"]]
    failed += [dict(cell=r["cell"],target=r["target"],screens=["paired_coverage_drop"]) for r in paired if r["screen_failed"]]
    approximation=[]
    for name,value in sorted(manifest["cells"].items()):
        for i,target in enumerate(TARGETS):
            ratio=value["oracle"]["difference"]["rms_over_actual_sd"][i]
            primary=value["spec"]["diagnostic"]=="primary" and target in primary_targets(value["spec"])
            item=dict(cell=name,target=target,rms_over_actual_sd=ratio,
                      rms_over_conditional_remainder_sd=value["oracle"]["q1"]["mean_discrepancy_rms_over_conditional_remainder_sd"][i],
                      screen_failed=primary and ratio>screens["mean_approximation_rms_over_actual_sd"])
            approximation.append(item)
            if item["screen_failed"]: failed.append(dict(cell=name,target=target,screens=["mean_approximation"]))
    return dict(status="SCREEN_FAILURE" if failed else "SCREENS_PASS", scope="bounded descriptive assessment, not general coverage qualification",
                profile=manifest["profile"],manifest_sha256=manifest["identity"],summaries=summaries,
                paired=paired,approximation=approximation,scientific_screen_failures=failed,attempt_failures=failures,total_rows=len(rows))


def make_do(task, root, data_path, result_path, fail=False):
    c = task["cell"]
    numerics=task_numerics(task)
    model = "structured_leverage" if c["diagnostic"] == "leverage_sensitivity" else "structured_common"
    common = f"worker(worker) firm(firm) targetweight(target) stayers(both) mcse(all) nodisplay deletion(match) deletionid(original_unit) nuisance(fixedoffset) inferencemodel({model}) "
    if c["route"] == "mata":
        common += "algorithm(exact) backend(mata) rng(stata) "
    else:
        common += f'algorithm(jla) engine(generic) backend(rust) rng(counter_v1) probes({numerics["point_probes"]}) preconditioner(diagonal) nativethreads(1) inferencegramprobes({numerics["gram_probes"]}) '
    common += f'inference({c["reference"]}) inferencesimulations({numerics["covariance_simulations"]}) inferencebins(1000) '
    weight = "[fw=frequency]"
    control = "control" if c["diagnostic"] == "estimated_offset" else ""
    lines = ["version 18", "clear all", "set more off", "set varabbrev off", "set processors 1", "set linesize 255",
             'display "POOLED_COMPONENT_STATA_VERSION `c(stata_version)\'"',
             f'adopath ++ "{root}/input/fevc"',f'adopath ++ "{root}/input/plugin"',
             f'import delimited using "{data_path}", clear varnames(1) numericcols(_all) asdouble',
             "generate double working_outcome=.",
             f'file open mc_out using "{result_path}", write replace',
             'file write mc_out "'+','.join(FIELDS)+'" _n',
             "program define mc_fit", " args arm replication c0",
             " local mode mean", ' if "`arm\'"=="fixed_c0" local mode none',
             f' capture quietly fevc working_outcome {control} {weight}, {common} centering(`mode\') seed({numerics["seed"]}) inferenceseed({numerics["seed"]})',
             " local rc=_rc", " local sample_ok=0", " local n=.", " local physical_n=.",
             ' local failure "`e(withholding_status)\'"',
             " if `rc'==0 {", "  quietly count if !e(sample)", "  local sample_ok=(r(N)==0)",
             "  local n=e(N_stored)", "  local physical_n=e(N_physical)", " }", " else {",
             '  if "`failure\'"=="" local failure "stata_rc_`rc\'"'," }",
             " forvalues t=1/4 {", "  local target : word `t' of worker firm covariance total",
             "  scalar point=.","  scalar se=.","  scalar lower=.","  scalar upper=.","  scalar identity=.","  local target_status=.",
             "  scalar point_mcse=.", "  local mcse_available=.",
             "  scalar q1_var_b=.","  scalar q1_cov_br=.","  scalar q1_var_r=.","  scalar q1_b=.","  scalar q1_remainder=.",
             "  if `rc'==0 {", "   matrix b=e(b)", "   scalar point=b[1,`t']", "   local target_status=0"]
    lines += ["   local mcse_available=e(mcse_available)","   if `mcse_available'==1 {","    matrix point_mcse_values=e(mcse)","    scalar point_mcse=point_mcse_values[1,`t']","   }"]
    if c["reference"] == "q1":
        lines += ["   matrix intervals=e(q1_inference)", "   matrix statuses=e(q1_status)", "   local target_status=statuses[`t',1]",
                  "   scalar se=intervals[`t',2]", "   scalar lower=intervals[`t',5]", "   scalar upper=intervals[`t',6]",
                  "   scalar q1_var_b=intervals[`t',10]","   scalar q1_cov_br=intervals[`t',11]","   scalar q1_var_r=intervals[`t',12]",
                  "   scalar q1_b=intervals[`t',13]","   scalar q1_remainder=intervals[`t',14]",
                  "   capture scalar identity=e(component_q1_diagnostics)[`t',\"remainder_identity_error\"]",
                  "   if _rc capture scalar identity=e(q1_failure_diagnostics)[`t',6]"]
    else:
        lines += ["   matrix intervals=e(component_inference)", "   scalar se=intervals[`t',2]", "   scalar lower=intervals[`t',3]", "   scalar upper=intervals[`t',4]"]
        lines += ["   matrix statuses=e(q0_status)","   local target_status=statuses[`t',1]"]
    lines += ["  }", f'  file write mc_out "{c["cell"]},`replication\',`arm\',`target\',`rc\',`target_status\',`sample_ok\',`n\',`physical_n\'"',
              '  foreach scalar in point se lower upper {', '   file write mc_out "," %24.17g (`scalar\')',"  }",
              '  file write mc_out ",`e(centering)\',`e(inference_centering)\',`failure\'," %24.17g (identity)',
              '  file write mc_out "," %24.17g (point_mcse) ",`mcse_available\',`e(mcse_mode)\',`e(mcse_centering)\',`e(native_error_phase)\'"',
              "  foreach scalar in q1_var_b q1_cov_br q1_var_r q1_b q1_remainder {",'   file write mc_out "," %24.17g (`scalar\')',"  }",
              '  file write mc_out "," (e(probes)) "," (e(inference_gram_probes)) "," (e(inference_simulations))',
              "  file write mc_out _n"," }","end"]
    if fail: lines += ["error 459"]
    for rep in task["replications"]:
        for arm in ARMS:
            lines += [f'quietly replace working_outcome=y{rep}' + (f'-{task["c0"]:.17g}' if arm == "fixed_c0" else ""),
                      f'mc_fit {arm} {rep} {task["c0"]:.17g}']
    lines += ["file close mc_out", 'display "POOLED_COMPONENT_ASSESSMENT_STATA_PASS"', "exit, clear"]
    return "\n".join(lines)+"\n"


def freeze(output, profile, plugin_dir=None, selected=None, shard_size=25):
    output=Path(output).resolve()
    if output.exists(): raise ValueError("output must be new; frozen runs are immutable")
    if shard_size < 1: raise ValueError("shard size must be positive")
    reg=json.loads(REGISTRATION.read_text())
    specs=cells(profile)
    if selected:
        specs=[c for c in specs if c["cell"] in set(selected)]
        if len(specs)!=len(set(selected)): raise ValueError("unknown selected cell")
    if not specs: raise ValueError("empty grid")
    if any(c["route"] != "mata" for c in specs) and plugin_dir is None: raise ValueError("native cells require an explicit source-local plugin directory")
    output.mkdir(parents=True)
    (output/"input/fevc").mkdir(parents=True)
    (output/"input/plugin").mkdir()
    (output/"input/harness").mkdir()
    package=ROOT/"fevc"
    runtime=["fevc.pkg","stata.toc"]+[line[2:].strip() for line in (package/"fevc.pkg").read_text().splitlines() if line.startswith("f ")]
    hashes={}
    for name in runtime:
        path=package/name
        destination=output/"input/fevc"/name
        destination.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(path,destination)
        hashes[str(destination.relative_to(output))]=digest(destination)
    for source in (HERE/"run.py",HERE/"pooled_oracle.py",HERE/"pooled_numerical.py",REGISTRATION,AMENDMENT,NUMERICAL_REGISTRATION):
        dest=output/"input/harness"/source.name
        shutil.copyfile(source,dest); hashes[str(dest.relative_to(output))]=digest(dest)
    native_sources=subprocess.check_output(["git","ls-files","-co","--exclude-standard","rust"],cwd=ROOT,text=True).splitlines()
    native_source_hashes={name:digest(ROOT/name) for name in sorted(set(native_sources)) if (ROOT/name).is_file()}
    diff=subprocess.check_output(["git","diff","--binary","HEAD"],cwd=ROOT)
    (output/"input/source.patch").write_bytes(diff)
    hashes["input/source.patch"]=digest(output/"input/source.patch")
    if plugin_dir:
        native=list(Path(plugin_dir).glob("*.plugin"))
        if not native: raise ValueError("no .plugin files in provided directory")
        for source in native:
            dest=output/"input/plugin"/source.name
            shutil.copyfile(source,dest); hashes[str(dest.relative_to(output))]=digest(dest)
    numerical_reg=json.loads(NUMERICAL_REGISTRATION.read_text())
    profile_spec=numerical_reg["profile"] if profile=="numerical" else reg["profiles"][profile]
    manifest=dict(schema="FEVC_POOLED_COMPONENT_ASSESSMENT_MANIFEST_V1",profile=profile,
                  domain=("assessment_structural_zero_repair" if profile=="assessment" else profile_spec["domain"]), registration=reg,amendments=[json.loads(AMENDMENT.read_text())],numerical_registration=numerical_reg,
                  git_head=subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip(),
                  git_status=subprocess.check_output(["git","status","--porcelain"],cwd=ROOT,text=True),
                  source_files=hashes,native_source_hashes=native_source_hashes,
                  replications=profile_spec["replications_per_cell"],
                  cells={},tasks=[],versions=dict(python=sys.version,numpy=np.__version__,scipy=scipy.__version__,
                    platform=platform.platform(),machine=platform.machine(),openblas_threads=os.environ.get("OPENBLAS_NUM_THREADS")))
    for cell in specs:
        d=build_design(cell["k"],cell["route"],cell["reference"],cell["diagnostic"],cell["stayer_share"])
        oracle=moments(d)
        manifest["cells"][cell["cell"]]=dict(spec=cell,oracle=oracle)
        for start in range(1,manifest["replications"]+1,shard_size):
            manifest["tasks"].append(dict(id=f'{cell["cell"]}_{start:05d}',cell=cell,c0=d.c0,
                replications=list(range(start,min(start+shard_size,manifest["replications"]+1)))))
    manifest["expected_rows"]=sum(len(expected_keys(task)) for task in manifest["tasks"])
    manifest["identity"]=hashlib.sha256(json.dumps(manifest,sort_keys=True,allow_nan=False).encode()).hexdigest()
    atomic_json(output/"manifest.json",manifest)
    return manifest


def load_manifest(root):
    root=Path(root)
    m=json.loads((root/"manifest.json").read_text())
    identity=m.pop("identity")
    if hashlib.sha256(json.dumps(m,sort_keys=True,allow_nan=False).encode()).hexdigest()!=identity: raise ValueError("manifest identity changed")
    m["identity"]=identity
    for relative,expected in m["source_files"].items():
        if digest(root/relative)!=expected: raise ValueError("immutable source changed")
    # The runner itself must match its frozen source. Importing a new checkout's
    # oracle to execute an old manifest would invalidate the source identity.
    for name in ("run.py","pooled_oracle.py","pooled_numerical.py"):
        if digest(HERE/name)!=digest(root/"input/harness"/name): raise ValueError("run with the frozen input/harness/run.py")
    return m


def parse_task_output(out, oracles, manifest, task):
    with (out/"results.partial.csv").open() as f:
        reader=csv.DictReader(f)
        if reader.fieldnames!=FIELDS: raise ValueError("malformed result schema")
        raw=list(reader)
    rows=[]
    for r in raw:
        if None in r or any(v is None for v in r.values()): raise ValueError("malformed result row")
        numeric_fields=("n","physical_n","point","se","lower","upper","identity_error","point_mcse","mcse_available","q1_var_b","q1_cov_br","q1_var_r","q1_b","q1_remainder","point_probes","gram_probes","covariance_simulations")
        row={k:(parse_number(v) if k in numeric_fields else v.strip()) for k,v in r.items()}
        for key in ("replication","rc","sample_ok"): row[key]=int(row[key])
        row["target_status"]=-1 if row["target_status"]=="." else int(row["target_status"])
        index=TARGETS.index(row["target"])
        ov=oracles[row["replication"]]
        row.update(schema=SCHEMA,profile=manifest["profile"],manifest_sha256=manifest["identity"],
                   outcome_seed=seed("pooled-component-v1",manifest["domain"],outcome_cell(task["cell"]),row["replication"]),
                   numerical_seed=task_numerics(task)["seed"],
                   oracle_point=ov["point" if row["arm"]=="mean" else "fixed_point"][index],observed_mean=ov["mean"])
        rows.append(row)
    return rows


def run_task(root, task_id, stata, fail=False):
    root=Path(root).resolve(); manifest=load_manifest(root)
    task=next((t for t in manifest["tasks"] if t["id"]==task_id),None)
    if task is None: raise ValueError("unknown task")
    out=root/"tasks"/task_id
    if out.exists(): raise ValueError("task already exists; preserve its outcome")
    out.mkdir(parents=True)
    c=task["cell"]
    d=build_design(c["k"],c["route"],c["reference"],c["diagnostic"],c["stayer_share"])
    verify_oracle(moments(d),manifest["cells"][c["cell"]]["oracle"])
    row_values={k:v.copy() for k,v in d.rows.items()}
    if c["diagnostic"]=="estimated_offset":
        idx=np.arange(len(row_values["worker"]))
        row_values["control"]=np.sin(.17*idx)+.3*(idx%2)
    oracles={}
    for rep in task["replications"]:
        y=d.draw(manifest["domain"],outcome_cell(c),rep,error="t8" if c["diagnostic"]=="heavy_tail" else "gaussian")
        stored=d.physical_outcome(y)
        if c["diagnostic"]=="estimated_offset":
            unit=d.rows["unit"].astype(int);freq=d.rows["frequency"]
            rng=np.random.default_rng(seed("offset-contrast",manifest["domain"],c["cell"],rep))
            contrast=rng.normal(size=len(stored))
            contrast-= (np.bincount(unit,weights=freq*contrast)/(d.direction**2))[unit]
            stored += .7*row_values["control"]+contrast
            xphysical=d.x[unit]/d.direction[unit,None]
            full=np.column_stack((xphysical,row_values["control"]))
            coefficients=np.linalg.lstsq(np.sqrt(freq)[:,None]*full,np.sqrt(freq)*stored,rcond=None)[0]
            y=np.bincount(unit,weights=freq*(stored-coefficients[-1]*row_values["control"]))/d.direction
        row_values[f"y{rep}"]=stored
        oracles[rep]=evaluate(d,y)
    with (out/"data.csv").open("w",newline="") as f:
        writer=csv.writer(f); writer.writerow(row_values)
        writer.writerows(zip(*row_values.values()))
    atomic_json(out/"oracle.json",oracles)
    script=out/"run.do"
    script.write_text(make_do(task,root,out/"data.csv",out/"results.partial.csv",fail=fail))
    started=time.monotonic()
    completed=subprocess.run([str(stata),"-q","do",str(script)],cwd=out,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,stdin=subprocess.DEVNULL,check=False)
    elapsed=time.monotonic()-started
    (out/"stata.log").write_text(completed.stdout)
    receipt=dict(task=task_id,manifest_sha256=manifest["identity"],elapsed_seconds=elapsed,
                 process_returncode=completed.returncode,stata_executable=str(stata),
                 stata_executable_sha256=digest(stata),status="FAILED")
    versions=re.findall(r"POOLED_COMPONENT_STATA_VERSION ([0-9.]+)",completed.stdout)
    receipt["stata_version"]=versions[-1] if versions else None
    if completed.returncode or "POOLED_COMPONENT_ASSESSMENT_STATA_PASS" not in completed.stdout:
        receipt["failure_reason"]="missing Stata PASS marker or nonzero process status"
        atomic_json(out/"receipt.json",receipt)
        raise RuntimeError(f"Stata application failure; inspect {out / 'stata.log'}")
    try:
        rows=parse_task_output(out,oracles,manifest,task)
        validate_rows(rows,task,manifest)
    except Exception as error:
        receipt["failure_reason"]=f"{type(error).__name__}: {error}"
        atomic_json(out/"receipt.json",receipt)
        raise
    partial=out/"results.jsonl.partial"
    partial.write_text("".join(json.dumps(row,sort_keys=True,allow_nan=False)+"\n" for row in rows))
    partial.replace(out/"results.jsonl")
    (out/"results.partial.csv").replace(out/"results.csv")
    receipt.update(status="COMPLETE",rows=len(rows),result_sha256=digest(out/"results.jsonl"),
                   input_sha256=digest(out/"data.csv"),
                   atomic_failures=len({(r["cell"],r["replication"],r["arm"]) for r in rows if r["rc"]!=0}),
                   atomic_failure_rows=sum(r["rc"]!=0 for r in rows))
    atomic_json(out/"receipt.json",receipt)
    return receipt


def aggregate(root):
    root=Path(root); manifest=load_manifest(root); rows=[]; seconds=0.;input_hashes={}
    for task in manifest["tasks"]:
        out=root/"tasks"/task["id"]
        receipt=json.loads((out/"receipt.json").read_text())
        if receipt["status"]!="COMPLETE" or receipt["manifest_sha256"]!=manifest["identity"] or receipt["result_sha256"]!=digest(out/"results.jsonl"):
            raise ValueError("incomplete or inconsistent task receipt")
        current=[json.loads(line) for line in (out/"results.jsonl").read_text().splitlines()]
        validate_rows(current,task,manifest); rows.extend(current); seconds+=receipt["elapsed_seconds"]
        if receipt["input_sha256"]!=digest(out/"data.csv"): raise ValueError("task input hash changed")
        if manifest["profile"]=="numerical":
            key=outcome_cell(task["cell"])
            if key in input_hashes and input_hashes[key]!=receipt["input_sha256"]: raise ValueError("fixed numerical outcome changed across settings")
            input_hashes[key]=receipt["input_sha256"]
    if len(rows)!=manifest["expected_rows"] or len({row_key(r) for r in rows})!=len(rows): raise ValueError("aggregate inventory")
    result=summarize_numerical(rows,manifest) if manifest["profile"]=="numerical" else summarize(rows,manifest)
    if manifest["profile"]=="numerical": result["fixed_input_hashes"]=input_hashes
    result.update(task_count=len(manifest["tasks"]),elapsed_process_seconds=seconds,
                  calls=len(rows)//4,seconds_per_paired_outcome=seconds/(len(rows)/8))
    atomic_json(root/"assessment.json",result)
    return result


def run_all(root, stata, workers):
    if not 1 <= workers <= 4: raise ValueError("use one through four local tasks")
    root=Path(root).resolve(); manifest=load_manifest(root)
    def execute(task):
        command=[sys.executable,str(HERE/"run.py"),"run-task",str(root),task["id"],"--stata",str(stata)]
        process=subprocess.run(command,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
        return dict(task=task["id"],returncode=process.returncode,output=process.stdout)
    statuses=[]
    with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
        for future in concurrent.futures.as_completed([pool.submit(execute,t) for t in manifest["tasks"]]):
            result=future.result();statuses.append(result)
            print(json.dumps(result),flush=True)
    atomic_json(root/"execution.json",dict(manifest_sha256=manifest["identity"],workers=workers,tasks=statuses))
    if any(t["returncode"] for t in statuses): raise RuntimeError("one or more tasks failed; preserve and inspect execution.json")
    return aggregate(root)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    sub=p.add_subparsers(dest="command",required=True)
    f=sub.add_parser("freeze"); f.add_argument("output",type=Path); f.add_argument("--profile",choices=("smoke","pilot","assessment","diagnostic","numerical"),required=True)
    f.add_argument("--plugin-dir",type=Path); f.add_argument("--cell",action="append"); f.add_argument("--shard-size",type=int,default=25)
    r=sub.add_parser("run-task");r.add_argument("root",type=Path);r.add_argument("task");r.add_argument("--stata",type=Path,required=True);r.add_argument("--deliberate-failure",action="store_true")
    a=sub.add_parser("aggregate");a.add_argument("root",type=Path)
    allp=sub.add_parser("run-all");allp.add_argument("root",type=Path);allp.add_argument("--stata",type=Path,required=True);allp.add_argument("--workers",type=int,default=2)
    args=p.parse_args()
    if args.command=="freeze":
        m=freeze(args.output,args.profile,args.plugin_dir,args.cell,args.shard_size); print(json.dumps(dict(identity=m["identity"],tasks=len(m["tasks"]),rows=m["expected_rows"])))
    elif args.command=="run-task": print(json.dumps(run_task(args.root,args.task,args.stata,args.deliberate_failure)))
    else:
        result=run_all(args.root,args.stata,args.workers) if args.command=="run-all" else aggregate(args.root)
        print(json.dumps({k:result[k] for k in ("status","total_rows","calls","seconds_per_paired_outcome","scientific_screen_failures")}))
        if result["scientific_screen_failures"]: raise SystemExit(1)


if __name__=="__main__": main()
