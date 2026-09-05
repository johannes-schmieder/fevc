#!/usr/bin/env python3
"""Frozen local four-arm controls diagnostic; no production nuisance correction."""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import time

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
REGISTRATION = Path("fevc/docs/fixed_offset_paired_v1.json")
TARGETS = ("worker", "firm", "covariance", "total")
ARMS = ("known_known", "known_fitted", "estimated_known", "estimated_fitted")


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


ORACLE = load_module("offset_paired_oracle", ROOT/"fevc/tools/diagnose_fixed_offset.py")
BASE = load_module("offset_paired_base", ROOT/"fevc/tools/run_match_inference_q0_campaign.py")


def payload(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False)+"\n").encode()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def strict_json(data):
    def pairs(values):
        result = {}
        for key, value in values:
            if key in result:
                raise ValueError(f"duplicate JSON key {key}")
            result[key] = value
        return result

    def constant(value):
        raise ValueError(f"nonfinite JSON number {value}")
    return json.loads(data, object_pairs_hook=pairs, parse_constant=constant)


def finite(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def seeds(registration, profile, replication):
    domain = "tiny_master_hex" if profile == "tiny" else "diagnostic_master_hex"
    root = BASE._splitmix64(int(registration["rng"][domain], 16)^BASE._splitmix64(replication))
    return BASE._splitmix64(root^1), BASE._splitmix64(root^2)


def prepare_design():
    design = ORACLE.make_design(20, "historical")
    reference = ORACLE.diagnose(design)
    design["cholesky"] = [np.linalg.cholesky(block) for block in design["blocks"]]
    modes, kernels = [], []
    for row, a in zip(reference, design["targets"]):
        small = design["root"].T@a@design["root"]
        eigenvalues, vectors = np.linalg.eigh((small+small.T)/2)
        mode = design["q"]@vectors[:, np.argmax(np.abs(eigenvalues))]
        sign = 1 if mode@design["mean"] >= 0 else -1
        modes.append(mode*sign)
        kernels.append(ORACLE.kernel(design["q"], small))
        for arm in ("known", "estimated"):
            row[arm]["leading_remainder_covariance"] *= sign
            row[arm]["expected_diagonal_cross_covariance"] *= sign
    design["modes"], design["kernels"] = np.array(modes), kernels
    return design, reference


def preflight(registration, design, reference):
    rules = registration["preflight"]
    failures = []
    if (len(design["group"]) != registration["design"]["stored_rows"]
            or int(design["frequency"].sum()) != registration["design"]["frequency_mass"]
            or len(design["mass"]) != registration["design"]["independent_original_matches"]):
        failures.append("design inventory")
    for row in reference:
        if row["target"] in registration["primary_targets"]:
            if (row["leading_share"] < rules["primary_minimum_leading_share"]
                    or row["remainder_concentration"] > rules["primary_maximum_remainder_concentration"]):
                failures.append(row["target"]+": spectrum")
        elif row["remainder_concentration"] < rules["covariance_minimum_remainder_concentration"]:
            failures.append("covariance: multi-mode")
        if row["minimum_maker_diagonal"] < rules["minimum_maker_denominator"]:
            failures.append(row["target"]+": identification")
        if row["within_control_condition"] > rules["maximum_within_control_condition"]:
            failures.append("control rank")
    minimum_eigenvalues = {}
    for arm in ("known", "estimated"):
        covariance = design[arm]
        matrix = np.diag(covariance.diagonal)+covariance.factors@covariance.core@covariance.factors.T
        minimum_eigenvalues[arm] = float(np.linalg.eigvalsh(matrix).min())
        if minimum_eigenvalues[arm] <= 0:
            failures.append(arm+": covariance positivity")
    if failures:
        raise ValueError(f"outcome-free preflight failed: {failures}")
    return dict(reference=reference, minimum_covariance_eigenvalues=minimum_eigenvalues,
                status="PASS", orientation="exact leading mode has nonnegative population score")


def create_manifest(path, binary, profile):
    registration = strict_json((ROOT/REGISTRATION).read_bytes())
    if profile not in registration["profiles"]:
        raise ValueError("unknown profile")
    source = BASE._git_identity(ROOT)
    if source["dirty"]:
        raise ValueError("manifest requires clean committed source")
    design, reference = prepare_design()
    settings = registration["profiles"][profile]
    tasks = [dict(task_id=i+1, start=start, count=min(settings["shard_size"], settings["replications"]-start))
             for i, start in enumerate(range(0, settings["replications"], settings["shard_size"]))]
    manifest = dict(schema="fevc-offset-paired-manifest-v1", profile=profile,
                    source=source, registration=registration,
                    binary_sha256=sha(binary.read_bytes()),
                    runtime=dict(python=platform.python_version(), numpy=np.__version__, platform=platform.platform()),
                    preflight=preflight(registration, design, reference), tasks=tasks,
                    expected_rows=16*settings["replications"])
    BASE._write_new(path, payload(manifest))
    return manifest


def read_manifest(path, binary=None, check_source=False):
    manifest = strict_json(path.read_bytes())
    registration = strict_json((ROOT/REGISTRATION).read_bytes())
    if (manifest["schema"] != "fevc-offset-paired-manifest-v1" or manifest["registration"] != registration
            or manifest["profile"] not in registration["profiles"]):
        raise ValueError("manifest registration mismatch")
    setting = registration["profiles"][manifest["profile"]]
    expected = [dict(task_id=i+1, start=start, count=min(setting["shard_size"], setting["replications"]-start))
                for i, start in enumerate(range(0, setting["replications"], setting["shard_size"]))]
    if manifest["tasks"] != expected or manifest["expected_rows"] != 16*setting["replications"]:
        raise ValueError("manifest task inventory mismatch")
    design, reference = prepare_design()
    if manifest["preflight"] != preflight(registration, design, reference):
        raise ValueError("preflight identity mismatch")
    if manifest["runtime"] != dict(python=platform.python_version(), numpy=np.__version__, platform=platform.platform()):
        raise ValueError("outcome generator runtime mismatch")
    if binary is not None and sha(binary.read_bytes()) != manifest["binary_sha256"]:
        raise ValueError("binary identity mismatch")
    if check_source and manifest["source"] != BASE._git_identity(ROOT):
        raise ValueError("source identity mismatch")
    return manifest


def make_input(manifest, replication, design):
    count = manifest["registration"]["profiles"][manifest["profile"]]["replications"]
    if not isinstance(replication, int) or not 0 <= replication < count:
        raise ValueError("invalid replication key")
    outcome_seed, seed = seeds(manifest["registration"], manifest["profile"], replication)
    rng = np.random.Generator(np.random.PCG64(outcome_seed))
    independent = rng.standard_normal(len(design["group"]))
    error = np.empty_like(independent)
    for start, count, root in zip(design["starts"], design["counts"], design["cholesky"]):
        error[start:start+count] = root@independent[start:start+count]
    group = design["group"]
    control_index = design["controls"]@np.array(manifest["registration"]["design"]["gamma"])
    outcome = (design["mean"]/np.sqrt(design["mass"]))[group]+control_index+error
    truth = [float(design["alpha"]@a@design["alpha"]) for a in design["targets"]]
    fold = int(manifest["registration"]["rng"]["fixed_fold_seed_hex"], 16)
    lines = [f"FEVC_OFFSET_INPUT_V1 {len(group)} {len(design['mass'])} {seed} {fold}",
             " ".join(format(v, ".17g") for v in truth)]
    weight = design["target_mass"][group]/design["counts"][group]
    for i, g in enumerate(group):
        values = (weight[i], outcome[i], *design["controls"][i])
        lines.append(f"{10000+g//20} {20000+g%20} {100000+g} {int(design['frequency'][i])} "+
                     " ".join(format(v, ".17g") for v in values))
    lines.append(" ".join(format(v, ".17g") for v in design["tau"]))
    return ("\n".join(lines)+"\n").encode(), outcome, seed, outcome_seed


def validate_replication(rows, seed, reference):
    expected = {(arm, target) for arm in ARMS for target in TARGETS}
    keys = [(r.get("arm"), r.get("target")) for r in rows]
    if len(keys) != len(set(keys)) or set(keys) != expected:
        raise ValueError("partial or duplicate arm-target inventory")
    truth = {r["target"]: r["truth"] for r in reference}
    for row in rows:
        if row.get("schema") != "fevc-offset-paired-row-v1" or row.get("seed") != seed:
            raise ValueError("row schema/seed mismatch")
        if not finite(row.get("truth")) or abs(row["truth"]-truth[row["target"]]) > 1e-10:
            raise ValueError("row truth mismatch")
        if row.get("status") == "backend_failure":
            if not row.get("error_code") or not row.get("error_phase"):
                raise ValueError("unclassified failure")
            continue
        if row.get("status") not in ("success", "target_unavailable"):
            raise ValueError("unknown status")
        for name in ("point", "sd", "q0_lower", "q0_upper", "maximum_complete_residual", "mean_variance"):
            if not finite(row.get(name)):
                raise ValueError(f"missing/nonfinite {name}")
        if row["sd"] <= 0 or row["mean_variance"] <= 0 or row["maximum_complete_residual"] > 1e-10:
            raise ValueError("invalid variance or solve residual")
        if row["arm"].endswith("fitted"):
            fold = row.get("fold_hash")
            if not isinstance(fold, str) or len(fold) != 16 or any(c not in "0123456789abcdef" for c in fold):
                raise ValueError("missing fixed folds")
        elif row.get("fold_hash") is not None:
            raise ValueError("oracle has fitted folds")
        if row["status"] == "success":
            if row.get("q1_status") != 0:
                raise ValueError("status contradiction")
            for name in ("lower", "upper", "critical", "leading_variance", "remainder_variance", "cross_covariance",
                         "curvature", "remainder_identity_error", "point_identity_error"):
                if not finite(row.get(name)):
                    raise ValueError(f"missing/nonfinite q1 {name}")
            if (row["lower"] > row["upper"] or row["critical"] <= 0
                    or row["leading_variance"] <= 0 or row["remainder_variance"] <= 0
                    or row["point_identity_error"] > 1e-9 or row["remainder_identity_error"] > 1e-8):
                raise ValueError("invalid computed q1 result")
            conditional = row["remainder_variance"]-row["cross_covariance"]**2/row["leading_variance"]
            if conditional <= 0:
                raise ValueError("invalid q1 covariance")
            curvature = 2*abs(row["leading_eigenvalue"])*row["leading_variance"]/math.sqrt(conditional)
            if not math.isclose(row["curvature"], curvature, rel_tol=1e-8, abs_tol=1e-10):
                raise ValueError("curvature identity")
        elif row.get("q1_status") not in (1, 2, 3, 6) or row.get("lower") is not None or row.get("upper") is not None:
            raise ValueError("invalid withheld interval")
    for offset in ("known", "estimated"):
        for target in TARGETS:
            pair = [r for r in rows if r["arm"].startswith(offset+"_") and r["target"] == target and "point" in r]
            if len(pair) == 2 and pair[0]["point"] != pair[1]["point"]:
                raise ValueError("variance model changed point estimate")
    folds = {r["fold_hash"] for r in rows if r.get("fold_hash") is not None}
    if len(folds) > 1:
        raise ValueError("paired folds differ")


def enrich(rows, outcome, replication, design):
    adjusted = {
        "known": outcome-design["controls"]@np.array([0.72, -0.43]),
        "estimated": outcome-design["controls"]@(design["h_transpose"].T@outcome),
    }
    collapsed = {key: np.bincount(design["group"], weights=design["frequency"]*value)/np.sqrt(design["mass"])
                 for key, value in adjusted.items()}
    for row in rows:
        row["replication"] = replication
        target = TARGETS.index(row["target"])
        y = collapsed[row["arm"].split("_")[0]]
        row["exact_point"] = float(y@design["kernels"][target].apply(y))
        exact_score = float(design["modes"][target]@y)
        row["exact_leading_score"] = exact_score
        if finite(row.get("leading_score")):
            sign = 1 if row["leading_score"]*exact_score >= 0 else -1
            row["aligned_leading_score"] = sign*row["leading_score"]
            row["aligned_cross_covariance"] = sign*row["cross_covariance"] if finite(row.get("cross_covariance")) else None
        else:
            row["aligned_leading_score"] = row["aligned_cross_covariance"] = None
    return rows


def verify_export(records, outcome, design, reference):
    """Independent physical-row WLS and dense Gaussian moment oracle, tiny only."""
    states = [r for r in records if r.get("kind") == "state"]
    rows = [r for r in records if r.get("schema") == "fevc-offset-paired-row-v1"]
    if len(states) != 4 or {r["arm"] for r in states} != set(ARMS) or len(rows) != 16:
        raise ValueError("incomplete tiny export")
    z, w, c = design["z"][design["group"]], design["frequency"], design["controls"]
    full = np.column_stack((z, c))
    projection = np.linalg.solve(full.T@(w[:, None]*full), full.T*w)
    collapse = np.zeros((len(design["mass"]), len(w)))
    collapse[design["group"], np.arange(len(w))] = w/np.sqrt(design["mass"])[design["group"]]
    sigma = np.zeros((len(w), len(w)))
    for start, count, block in zip(design["starts"], design["counts"], design["blocks"]):
        sigma[start:start+count, start:start+count] = block
    transforms = {"known": collapse, "estimated": collapse-(collapse@c)@projection[-2:]}
    covariance = {arm: transform@sigma@transform.T for arm, transform in transforms.items()}
    adjusted = {"known": outcome-c@np.array([.72, -.43]),
                "estimated": outcome-c@(projection[-2:]@outcome)}
    differences, folds = {}, []
    for state in states:
        arm = state["arm"]
        offset = arm.split("_")[0]
        y = collapse@adjusted[offset]
        beta = np.linalg.solve(z.T@(w[:, None]*z), z.T@(w*adjusted[offset]))
        residual = y-design["x"]@beta
        diagonal = np.asarray(state["target_diagonal"])
        maker = np.asarray(state["maker_inverse"])
        variance = np.asarray(state["variance"])
        if (diagonal.shape != (3, 400) or maker.shape != (400,) or variance.shape != (400,)
                or not all(np.isfinite(v).all() for v in (diagonal, maker, variance))
                or np.min(maker) <= 0 or np.min(variance) <= 0):
            raise ValueError("invalid exported state")
        if arm.endswith("known"):
            if state["folds"] or not np.array_equal(variance, design["tau"]):
                raise ValueError("oracle state mismatch")
        else:
            if len(state["folds"]) != 400 or set(state["folds"]) != set(range(5)):
                raise ValueError("invalid exported folds")
            folds.append(state["folds"])
        diagonal = np.vstack((diagonal, diagonal[0]+diagonal[1]+2*diagonal[2]))
        for index, target in enumerate(TARGETS):
            matches = [r for r in rows if r["arm"] == arm and r["target"] == target]
            if len(matches) != 1 or "point" not in matches[0]:
                raise ValueError("incomplete tiny point inventory")
            point = beta@design["targets"][index]@beta-np.dot(diagonal[index]*maker, y*residual)
            difference = abs(float(point)-matches[0]["point"])
            differences[arm+"/"+target] = difference
            if difference > 1e-8:
                raise ValueError("physical-row point mismatch")
    if folds[0] != folds[1]:
        raise ValueError("exported paired folds differ")
    moment_differences = {}
    for target, form, row in zip(TARGETS, design["kernels"], reference):
        kernel = form.apply(np.eye(400))
        for arm, omega in covariance.items():
            bias = float(np.trace(kernel@omega))
            variance = float(4*design["mean"]@kernel@omega@kernel@design["mean"]
                             +2*np.trace(kernel@omega@kernel@omega))
            difference = max(abs(bias-row[arm]["point"]["bias"]), abs(variance-row[arm]["point"]["variance"]))
            moment_differences[arm+"/"+target] = difference
            if difference > 1e-10:
                raise ValueError("physical-row moment mismatch")
    return dict(status="PASS", point_differences=differences, moment_differences=moment_differences,
                folds=folds[0], independent_fit="physical-row joint frequency-weighted least squares",
                diagonal="exported production randomized diagonal, not substituted exact diagonal")


def verify_tiny(manifest_path, task_root, binary, output):
    manifest = read_manifest(manifest_path, binary, check_source=True)
    if manifest["profile"] != "tiny":
        raise ValueError("dense export checks are tiny-only")
    output.mkdir(parents=True, exist_ok=False)
    design, reference = prepare_design()
    results = []
    for task in manifest["tasks"]:
        rep = task["start"]
        directory = task_root/f"task-{task['task_id']:04d}"
        input_path = directory/f"rep-{rep:04d}.txt"
        contents, outcome, seed, _ = make_input(manifest, rep, design)
        if input_path.read_bytes() != contents:
            raise ValueError("tiny input identity mismatch")
        completed = subprocess.run([str(binary.resolve()), str(input_path.resolve()), "export-state"], capture_output=True)
        BASE._write_new(output/f"rep-{rep:04d}.jsonl", completed.stdout)
        BASE._write_new(output/f"rep-{rep:04d}.stderr", completed.stderr)
        if completed.returncode:
            raise ValueError("tiny export process failed")
        records = [strict_json(line) for line in completed.stdout.splitlines()]
        raw = [r for r in records if r.get("kind") != "state"]
        validate_replication(raw, seed, reference)
        if raw != [strict_json(line) for line in (directory/f"rep-{rep:04d}.jsonl").read_bytes().splitlines()]:
            raise ValueError("export changed production results")
        results.append(dict(replication=rep, export_sha256=sha(completed.stdout),
                            **verify_export(records, outcome, design, reference)))
    if any(r["folds"] != results[0]["folds"] for r in results):
        raise ValueError("exported folds changed across outcomes")
    if manifest["source"] != BASE._git_identity(ROOT) or sha(binary.read_bytes()) != manifest["binary_sha256"]:
        raise ValueError("tiny source/binary changed")
    receipt = dict(status="PASS", manifest_sha256=sha(manifest_path.read_bytes()), results=results)
    BASE._write_new(output/"receipt.json", payload(receipt))
    return receipt


def run_task(manifest, manifest_hash, task, binary, task_dir, design):
    task_dir.mkdir(parents=True, exist_ok=False)
    rows, inventory = [], []
    start_time = time.monotonic()
    for replication in range(task["start"], task["start"]+task["count"]):
        contents, outcome, seed, outcome_seed = make_input(manifest, replication, design)
        stem = f"rep-{replication:04d}"
        input_path = task_dir/(stem+".txt")
        BASE._write_new(input_path, contents)
        completed = subprocess.run([str(binary.resolve()), str(input_path.resolve())], capture_output=True)
        stdout, stderr = task_dir/(stem+".jsonl"), task_dir/(stem+".stderr")
        BASE._write_new(stdout, completed.stdout)
        BASE._write_new(stderr, completed.stderr)
        if completed.returncode:
            raise ValueError(f"replication {replication} process exit {completed.returncode}; raw output retained")
        current = [strict_json(line) for line in completed.stdout.splitlines()]
        validate_replication(current, seed, manifest["preflight"]["reference"])
        rows.extend(enrich(current, outcome, replication, design))
        inventory.append(dict(replication=replication, seed=seed, outcome_seed=outcome_seed,
                              files={input_path.name: sha(contents), stdout.name: sha(completed.stdout), stderr.name: sha(completed.stderr)}))
    rows.sort(key=lambda r: (r["replication"], r["arm"], r["target"]))
    BASE._write_new(task_dir/"rows.json", payload(rows))
    receipt = dict(schema="fevc-offset-paired-task-v1", task=task, manifest_sha256=manifest_hash,
                   source=manifest["source"], binary_sha256=sha(binary.read_bytes()), inventory=inventory,
                   rows=len(rows), rows_sha256=sha((task_dir/"rows.json").read_bytes()),
                   elapsed_seconds=time.monotonic()-start_time)
    if receipt["binary_sha256"] != manifest["binary_sha256"]:
        raise ValueError("binary changed during task")
    BASE._write_new(task_dir/"receipt.json", payload(receipt))
    return receipt


def average(values):
    return statistics.fmean(values) if values else None


def mcse(values):
    return statistics.stdev(values)/math.sqrt(len(values)) if len(values) > 1 else None


def summarize(rows, registration, profile):
    summaries, failures = [], []
    for arm in ARMS:
        for target in TARGETS:
            cell = [r for r in rows if r["arm"] == arm and r["target"] == target]
            available = [r for r in cell if r["status"] == "success"]
            points = [r for r in cell if "point" in r]
            errors = [r["point"]-r["truth"] for r in points]
            covered = [float(r["lower"] <= r["truth"] <= r["upper"]) for r in available]
            all_covered = [float(r["status"] == "success" and r["lower"] <= r["truth"] <= r["upper"]) for r in cell]
            counts = {}
            for r in cell:
                if r["status"] != "success":
                    label = f"{r['status']}:{r.get('q1_status', r.get('error_code'))}:{r.get('error_phase','component_inference_q1')}"
                    counts.setdefault(label, []).append(r["replication"])
            sd = statistics.stdev(errors) if len(errors) > 1 else None
            mean_se = average([r["sd"] for r in points])
            summary = dict(arm=arm, target=target, attempts=len(cell), successes=len(available),
                           availability=len(available)/len(cell), point_count=len(points),
                           bias=average(errors), bias_mcse=mcse(errors), empirical_sd=sd,
                           mean_se=mean_se, se_ratio=sd/mean_se if sd is not None and mean_se else None,
                           coverage=average(covered), coverage_mcse=mcse(covered),
                           coverage_all_attempts=average(all_covered), all_attempts_mcse=mcse(all_covered),
                           mean_width=average([r["upper"]-r["lower"] for r in available]),
                           lower_miss=average([float(r["truth"] < r["lower"]) for r in available]),
                           upper_miss=average([float(r["truth"] > r["upper"]) for r in available]),
                           q0_coverage_diagnostic=average([float(r["q0_lower"] <= r["truth"] <= r["q0_upper"]) for r in points]),
                           exact_point_sd=statistics.stdev([r["exact_point"] for r in cell]) if len(cell)>1 else None,
                           failures=counts)
            for field in ("leading_variance", "remainder_variance", "aligned_cross_covariance",
                          "leading_variance_correction", "remainder_influence_variance", "remainder_trace_variance",
                          "mean_variance", "variance_floor_share", "leading_share", "remainder_share"):
                summary["mean_"+field] = average([r[field] for r in cell if finite(r.get(field))])
            summaries.append(summary)
            if profile == "diagnostic" and arm.startswith("known_") and target in registration["primary_targets"]:
                gate = registration["known_offset_calibration_checks"]
                label = arm+"/"+target
                if summary["availability"] < gate["minimum_availability"]:
                    failures.append(label+": availability")
                if len(available) < 2 or len(points) < 2:
                    failures.append(label+": insufficient results")
                    continue
                if abs(summary["coverage"]-gate["coverage"]) > max(gate["coverage_absolute_tolerance"], gate["coverage_mcse_multiplier"]*summary["coverage_mcse"]):
                    failures.append(label+": coverage")
                if not gate["se_ratio"][0] <= summary["se_ratio"] <= gate["se_ratio"][1]:
                    failures.append(label+": SE ratio")
                if abs(summary["bias"]) > max(1e-12, gate["bias_mcse_multiplier"]*summary["bias_mcse"]):
                    failures.append(label+": bias")
    by_key = {(r["replication"], r["arm"], r["target"]): r for r in rows}
    pairs = []
    contrasts = {
        "offset_at_known_variance": {"estimated_known": 1, "known_known": -1},
        "offset_at_fitted_variance": {"estimated_fitted": 1, "known_fitted": -1},
        "variance_fit_at_known_offset": {"known_fitted": 1, "known_known": -1},
        "variance_fit_at_estimated_offset": {"estimated_fitted": 1, "estimated_known": -1},
        "interaction": {"estimated_fitted": 1, "known_fitted": -1, "estimated_known": -1, "known_known": 1},
    }
    for target in TARGETS:
        for name, weights in contrasts.items():
            all_values, joint_values = [], []
            for rep in sorted({r["replication"] for r in rows}):
                cell = {arm: by_key[(rep, arm, target)] for arm in weights}
                value = sum(w*float(cell[arm]["status"] == "success" and cell[arm]["lower"] <= cell[arm]["truth"] <= cell[arm]["upper"]) for arm, w in weights.items())
                all_values.append(value)
                if all(r["status"] == "success" for r in cell.values()):
                    joint_values.append(value)
            pairs.append(dict(target=target, contrast=name, attempts=len(all_values),
                              coverage_difference_all_attempts=average(all_values), mcse=mcse(all_values),
                              jointly_available=len(joint_values),
                              conditional_difference=average(joint_values), conditional_mcse=mcse(joint_values)))
    return summaries, pairs, failures


def aggregate(manifest_path, task_root, output):
    manifest = read_manifest(manifest_path)
    expected_dirs = {f"task-{task['task_id']:04d}" for task in manifest["tasks"]}
    if {p.name for p in task_root.iterdir()} != expected_dirs:
        raise ValueError("task directory inventory mismatch")
    design, _ = prepare_design()
    rows, task_hashes = [], {}
    for task in manifest["tasks"]:
        directory = task_root/f"task-{task['task_id']:04d}"
        receipt = strict_json((directory/"receipt.json").read_bytes())
        if (receipt["schema"] != "fevc-offset-paired-task-v1" or receipt["task"] != task
                or receipt["manifest_sha256"] != sha(manifest_path.read_bytes())
                or receipt["source"] != manifest["source"] or receipt["binary_sha256"] != manifest["binary_sha256"]):
            raise ValueError("task receipt identity mismatch")
        current, expected_files = [], {"receipt.json", "rows.json"}
        replications = list(range(task["start"], task["start"]+task["count"]))
        if [r["replication"] for r in receipt["inventory"]] != replications:
            raise ValueError("replication receipt inventory mismatch")
        for item in receipt["inventory"]:
            rep = item["replication"]
            contents, outcome, seed, outcome_seed = make_input(manifest, rep, design)
            if (item["seed"], item["outcome_seed"]) != (seed, outcome_seed):
                raise ValueError("replication seed mismatch")
            stem = f"rep-{rep:04d}"
            files = {stem+".txt", stem+".jsonl", stem+".stderr"}
            if set(item["files"]) != files:
                raise ValueError("replication file inventory mismatch")
            for filename, digest in item["files"].items():
                if sha((directory/filename).read_bytes()) != digest:
                    raise ValueError("replication artifact hash mismatch")
            if (directory/(stem+".txt")).read_bytes() != contents:
                raise ValueError("input generation mismatch")
            raw = [strict_json(line) for line in (directory/(stem+".jsonl")).read_bytes().splitlines()]
            validate_replication(raw, seed, manifest["preflight"]["reference"])
            current.extend(enrich(raw, outcome, rep, design))
            expected_files |= files
        current.sort(key=lambda r: (r["replication"], r["arm"], r["target"]))
        if ({p.name for p in directory.iterdir()} != expected_files
                or receipt["rows"] != 16*task["count"]
                or sha((directory/"rows.json").read_bytes()) != receipt["rows_sha256"]
                or strict_json((directory/"rows.json").read_bytes()) != current):
            raise ValueError("task result inventory/hash mismatch")
        rows.extend(current)
        task_hashes[directory.name] = sha((directory/"receipt.json").read_bytes())
    if len(rows) != manifest["expected_rows"]:
        raise ValueError("total target inventory mismatch")
    if len({r["fold_hash"] for r in rows if r.get("fold_hash") is not None}) > 1:
        raise ValueError("folds changed across outcome replications")
    summaries, pairs, failures = summarize(rows, manifest["registration"], manifest["profile"])
    output.mkdir(parents=True, exist_ok=False)
    BASE._write_new(output/"rows.json", payload(rows))
    receipt = dict(schema="fevc-offset-paired-result-v1", profile=manifest["profile"],
                   status="FAIL_DIAGNOSTIC_CHECKS" if failures else ("PASS_DIAGNOSTIC_CHECKS" if manifest["profile"] == "diagnostic" else "COMPLETE_TINY"),
                   manifest_sha256=sha(manifest_path.read_bytes()), source=manifest["source"],
                   binary_sha256=manifest["binary_sha256"], rows=len(rows), rows_sha256=sha((output/"rows.json").read_bytes()),
                   task_receipt_hashes=task_hashes, summaries=summaries, paired_contrasts=pairs,
                   diagnostic_check_failures=failures, public_qualification=False)
    BASE._write_new(output/"receipt.json", payload(receipt))
    return receipt


def run_local(manifest_path, binary, output, workers, reverse=False):
    manifest = read_manifest(manifest_path, binary, check_source=True)
    if not isinstance(workers, int) or not 1 <= workers <= manifest["registration"]["execution"]["maximum_parallel_tasks"]:
        raise ValueError("invalid local worker count")
    output.mkdir(parents=True, exist_ok=False)
    task_root = output/"tasks"
    task_root.mkdir()
    design, _ = prepare_design()
    tasks = list(reversed(manifest["tasks"])) if reverse else manifest["tasks"]
    start = time.monotonic()
    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = {pool.submit(run_task, manifest, sha(manifest_path.read_bytes()), task, binary,
                               task_root/f"task-{task['task_id']:04d}", design): task for task in tasks}
        for future in as_completed(futures):
            result = future.result()
            print(f"completed task {result['task']['task_id']}/{len(tasks)}: {result['rows']} target attempts", flush=True)
    if manifest["source"] != BASE._git_identity(ROOT):
        raise ValueError("source changed during run")
    result = aggregate(manifest_path, task_root, output/"aggregate")
    BASE._write_new(output/"execution.json", payload(dict(workers=workers, reverse=reverse,
                    elapsed_seconds=time.monotonic()-start, status=result["status"], platform=platform.platform())))
    print(json.dumps({k: result[k] for k in ("status", "rows", "diagnostic_check_failures")}), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    create = sub.add_parser("create-manifest")
    create.add_argument("manifest", type=Path)
    create.add_argument("--binary", type=Path, required=True)
    create.add_argument("--profile", choices=("tiny", "diagnostic"), required=True)
    run = sub.add_parser("run-local")
    run.add_argument("manifest", type=Path)
    run.add_argument("output", type=Path)
    run.add_argument("--binary", type=Path, required=True)
    run.add_argument("--workers", type=int, default=4)
    run.add_argument("--reverse", action="store_true")
    collect = sub.add_parser("aggregate")
    collect.add_argument("manifest", type=Path)
    collect.add_argument("tasks", type=Path)
    collect.add_argument("output", type=Path)
    verify = sub.add_parser("verify-tiny")
    verify.add_argument("manifest", type=Path)
    verify.add_argument("tasks", type=Path)
    verify.add_argument("output", type=Path)
    verify.add_argument("--binary", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "create-manifest":
        create_manifest(args.manifest, args.binary, args.profile)
    elif args.command == "run-local":
        run_local(args.manifest, args.binary, args.output, args.workers, args.reverse)
    elif args.command == "verify-tiny":
        print(json.dumps(verify_tiny(args.manifest, args.tasks, args.binary, args.output)["status"]))
    else:
        print(json.dumps(aggregate(args.manifest, args.tasks, args.output)["status"]))


if __name__ == "__main__":
    main()
