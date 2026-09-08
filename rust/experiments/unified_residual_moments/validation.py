"""Saved-input integrity, paired-result accounting, and prospective readiness gates."""
import copy
import gzip
import importlib.util
import json
import math
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    sys.modules[name] = value
    spec.loader.exec_module(value)
    return value


OLD = module("unified_historical_validation", HERE.parent / "individual_inference/run.py")
BUILD = module("unified_builder", HERE / "build.py")
sha = BUILD.sha


def load_parent(parent):
    manifest = json.loads((parent / "manifest.json").read_text())
    result = json.loads((parent / "result.json").read_text())
    if (result["manifest_sha256"] != sha(parent / "manifest.json")
            or manifest["bundle_sha256"] != sha(parent / "source.tar.gz")
            or manifest["cells"] != OLD.CELLS
            or manifest["tasks"] != list(OLD.tasks("development"))):
        raise ValueError("saved parent identity or inventory changed")
    files = {str(parent / p): sha(parent / p)
             for p in ("manifest.json", "result.json", "source.tar.gz")}
    cells = {}
    for task in manifest["tasks"]:
        folder = parent / "tasks" / task["id"]
        path = folder / "rows.jsonl.gz"
        receipt = json.loads((folder / "receipt.json").read_text())
        if (receipt["task"] != task or receipt["output_sha256"] != sha(path)
                or receipt["binary_sha256"] != manifest["binaries"][task["family"]]):
            raise ValueError("saved task hash or identity changed")
        records = [json.loads(line) for line in gzip.open(path, "rt")]
        design, calls = OLD.validate(records, task)
        key = task["family"], task["cell"], task["k"]
        entry = cells.setdefault(key, {"design": design, "calls": {}, "master": task["master"]})
        if entry["design"] != design or entry["master"] != task["master"]:
            raise ValueError("saved design or master seed changed across shards")
        for call in calls:
            rep = call["replication"]
            if rep in entry["calls"]:
                raise ValueError("duplicate saved replication")
            entry["calls"][rep] = call
        files.update({str(p): sha(p) for p in (path, folder / "receipt.json")})
    if len(cells) != 48 or any(set(c["calls"]) != set(range(400)) for c in cells.values()):
        raise ValueError("missing saved cell or replication")
    return manifest, cells, files


def validate(records, task, arm):
    """Validate actual fitter metadata, then reuse unchanged statistical checks.

    The historical validator couples statistical checks to its old fitter codes.
    A private metadata-only view lets us reuse those *unchanged* interval, seed,
    covariance and accounting checks. Actual new fields are checked first and
    raw records are never rewritten or used with substituted metadata in reports.
    """
    if arm not in BUILD.ARMS:
        raise ValueError("unknown arm")
    if len(records) < 2 or records[1].get("kind") != "design" or any(
            row.get("kind") != "call" for row in records[2:]):
        raise ValueError("record ordering")
    OLD.finite(records)
    observation = task["family"] == "observation"
    residual = observation or arm == "unified"
    q1 = OLD.is_q1(OLD.spec_for(task))
    for row in records[2:]:
        if row.get("status") != "success":
            continue
        if row.get("fit") != (2 if residual else 1):
            raise ValueError("incorrect actual fitter")
        if type(row.get("units")) is not int or row["units"] <= 0:
            raise ValueError("independent units")
        if residual:
            expected_order = 2 if observation else 3
            if row.get("gram_probes") != 512 or row.get("ordering") != expected_order:
                raise ValueError("Gram budget or unit ordering")
            for field in ("gram_rcond", "gram_inverse_relres", "fit_relres", "positivity_floor"):
                value = OLD.array(row, field, 1)[0]
                if value < 0 or (field in ("gram_rcond", "positivity_floor") and value == 0):
                    raise ValueError(f"invalid {field}")
            if row["gram_inverse_relres"][0] > 1e-9 or row["fit_relres"][0] > 1e-9:
                raise ValueError("uncertified moment system")
            atoms = (1000 + 128 + 2 + 512) * row.get("units", 0) + 2 * row.get("critical_draws", 0)
            if row.get("counter_atoms") != atoms or row.get("counter_words") != 2 * atoms:
                raise ValueError("independent-unit Counter accounting")
        elif (row.get("gram_probes") != 0 or row.get("ordering") != 0
              or any(row.get(k) != [None] for k in
                     ("gram_rcond", "gram_inverse_relres", "fit_relres", "positivity_floor"))):
            raise ValueError("legacy arm has residual diagnostics")
        if type(row.get("nonpositive")) is not int or not 0 <= row["nonpositive"] <= row.get("floored", -1):
            raise ValueError("nonpositive/floor count")
        iterations = 512 if observation or not q1 else 128
        spectrum = OLD.array(row, "spectrum", 60, True)
        if any(spectrum[15*t+12] != 128 or spectrum[15*t+13] != iterations for t in range(4)):
            raise ValueError("spectral budget changed")
    view = copy.deepcopy(records)
    # Only the legacy validator's three method fields differ. All scientific
    # quantities, Counter counts, solver certificates and outcomes remain actual.
    if not observation and arm == "unified":
        for row in view[2:]:
            if row.get("status") == "success":
                row.update(fit=1, gram_probes=0, gram_rcond=[None])
    design, _ = OLD.validate(view, task)
    for row in records[2:]:
        if row["status"] != "success":
            continue
        if not math.isclose(row["point"][3], row["point"][0] + row["point"][1] + 2*row["point"][2],
                            rel_tol=1e-10, abs_tol=1e-12):
            raise ValueError("four-target point identity")
        if row["joint"] == 0:
            mapping = ((1, 0, 0), (0, 1, 0), (0, 0, 1), (1, 1, 2))
            for a in range(4):
                for b in range(4):
                    expected = sum(mapping[a][i]*row["primitive"][3*i+j]*mapping[b][j]
                                   for i in range(3) for j in range(3))
                    if not math.isclose(row["covariance"][4*a+b], expected, rel_tol=1e-10, abs_tol=1e-12):
                        raise ValueError("four-target covariance identity")
    return design, records[2:]


def same(left, right):
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(same(left[k], right[k]) for k in left)
    if isinstance(left, list):
        return len(left) == len(right) and all(same(x, y) for x, y in zip(left, right))
    if isinstance(left, float) and isinstance(right, (float, int)):
        return math.isclose(left, right, rel_tol=1e-12, abs_tol=1e-12)
    return left == right


def compare_points(baseline, unified, saved):
    failures = []
    if baseline["seed"] != saved["seed"] or unified["seed"] != saved["seed"]:
        failures.append("saved outcome seed changed")
    # The baseline uses the current q0 iteration repair, but the repair changes
    # neither point nor covariance inputs. Updated memory forecasts may differ.
    if baseline["status"] == saved["status"] == "success":
        for field in ("point", "point_mcse", "primitive", "covariance"):
            if not same(baseline[field], saved[field]):
                failures.append(f"legacy replay changed {field}")
    elif baseline["status"] != saved["status"]:
        failures.append("legacy replay changed shared success status")
    if baseline["status"] == unified["status"] == "success":
        for field in ("point", "point_mcse"):
            if not same(baseline[field], unified[field]):
                failures.append(f"unified changed {field}")
        if baseline["units"] != unified["units"]:
            failures.append("independent units changed")
    return failures


def target_row(call, design, cell, target):
    if call["status"] != "success":
        return {"status": "shared_failure", "point_error": None,
                "error_phase": call["detail"]}
    variance, status = call["targets"][2*target:2*target+2]
    point, truth = call["point"][target], design["truth"][target]
    if OLD.is_q1(cell):
        q = call["q1"][20*target:20*target+20]
        status, lower, upper = q[16], q[10], q[11]
    else:
        radius = 1.959963984540054 * math.sqrt(variance) if status == 0 else 0
        lower, upper = point - radius, point + radius
    row = {"status": "success" if status == 0 else f"target_{int(status)}",
           "point_error": point-truth, "estimated_sd": math.sqrt(variance) if variance > 0 else None}
    if status == 0:
        row.update(covered=lower <= truth <= upper, lower_miss=truth < lower,
                   upper_miss=truth > upper, interval_width=upper-lower)
    return row


def readiness(summaries):
    """One-sided safeguards from the saved plan, not historical confirmation."""
    failures = []
    for row in summaries:
        if row["arm"] != "unified" or row["family"] == "observation":
            continue
        eligible = row["gate"] in ("correct", "q0_comparator") and (
            row["target"] in row.get("coverage_targets", OLD.TARGETS))
        if not eligible:
            continue
        threshold = OLD.GATES[row["family"]].THRESHOLDS
        label = f'{row["family"]}/{row["cell"]}/{row["target"]}'
        if row["success_rate"] < threshold["correct_success_rate"]:
            failures.append(f"{label}: availability")
        if any(row.get(k) is None for k in ("coverage", "coverage_mcse", "se_ratio")):
            failures.append(f"{label}: unavailable calibration")
            continue
        bound = max(threshold["coverage_absolute_tolerance"],
                    threshold["coverage_mcse_multiplier"] * row["coverage_mcse"])
        if row["coverage"] < .95 - bound:
            failures.append(f"{label}: undercoverage")
        if row["se_ratio"] > threshold["correct_se_ratio_upper"]:
            failures.append(f"{label}: underestimated SE")
        if row["se_denominator"] != row["successes"]:
            failures.append(f"{label}: incomplete SE calibration")
    return failures
