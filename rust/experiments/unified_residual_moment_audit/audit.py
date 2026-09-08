"""Independent raw-output audit; does not import the campaign or estimator."""
import argparse
import collections
import gzip
import hashlib
import json
import math
from pathlib import Path
import statistics
import tarfile

TARGETS = ("worker", "firm", "covariance", "total")
ARMS = ("baseline", "unified")


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def close(a, b):
    if a is None or b is None:
        return a is b
    return math.isclose(a, b, rel_tol=1e-10, abs_tol=1e-12)


def mean(values):
    return statistics.fmean(values) if values else None


def semantic_seed(master, cell, dimension, replication):
    mask = (1 << 64) - 1
    def mix(value):
        value = (value + 0x9e3779b97f4a7c15) & mask
        value = ((value ^ (value >> 30)) * 0xbf58476d1ce4e5b9) & mask
        value = ((value ^ (value >> 27)) * 0x94d049bb133111eb) & mask
        return value ^ (value >> 31)
    tag = 0xcbf29ce484222325
    # The archived generator uses this literal, not standard FNV's multiplier.
    for byte in cell.encode():
        tag = ((tag ^ byte) * 0x1000000001b3) & mask
    return mix(master ^ mix(tag) ^ mix(dimension) ^ mix(replication))


def read_jsonl(path):
    with gzip.open(path, "rt") as stream:
        return [json.loads(line) for line in stream]


def target_record(call, truth, index, q1):
    if call["status"] == "shared_failure":
        require(isinstance(call.get("detail"), str), "unclassified shared failure")
        return {"error": None, "available": False, "covered": False, "se": None, "width": None}
    require(call["status"] == "success", "unknown native status")
    point = call["point"][index]
    variance, q0_status = call["targets"][2*index:2*index+2]
    require(math.isfinite(point) and math.isfinite(variance), "nonfinite point or target variance")
    require(q0_status in (0, 1, 4, 6), "unknown q0 status")
    require(q0_status != 0 or variance > 0, "nonpositive reported q0 variance")
    if q1:
        row = call["q1"][20*index:20*index+20]
        status, lo, hi = row[16], row[10], row[11]
        require(status in (0, 1, 2, 3, 6), "unknown q1 status")
        available = status == 0
        if available:
            require(all(row[j] is not None and math.isfinite(row[j]) for j in (2, 5, 6, 9, 10, 11, 17)),
                    "missing q1 covariance or endpoints")
            require(row[2] > 0 and row[6] > 0 and row[9] > 0, "nonpositive q1 covariance/critical")
            determinant = 1 - row[5]**2/(row[2]*row[6])
            require(determinant > 1e-8 and close(determinant, row[17]), "q1 covariance determinant")
        else:
            require(lo is None and hi is None, "unavailable q1 endpoints exported")
    else:
        available = q0_status == 0
        radius = 1.959963984540054 * math.sqrt(variance) if available else 0
        lo, hi = point-radius, point+radius
    require(not available or lo <= hi, "reversed interval")
    return {"error": point-truth, "available": available,
            "covered": available and lo <= truth <= hi,
            "se": math.sqrt(variance) if variance > 0 else None,
            "width": hi-lo if available else None}


def summarize(records):
    errors = [r["error"] for r in records if r["error"] is not None]
    successes = [r for r in records if r["available"]]
    ses = [r["se"] for r in successes if r["se"] is not None]
    covered = sum(r["covered"] for r in successes)
    sd = statistics.stdev(errors) if len(errors) > 1 else None
    se = mean(ses)
    coverage = covered/len(successes) if successes else None
    return {"attempts": len(records), "point_estimates": len(errors), "successes": len(successes),
            "success_rate": len(successes)/len(records), "bias": mean(errors),
            "bias_mcse": sd/math.sqrt(len(errors)) if sd is not None else None,
            "empirical_sd": sd, "mean_se": se, "se_denominator": len(ses),
            "se_ratio": sd/se if sd is not None and se is not None and se > 0 else None,
            "coverage_denominator": len(successes), "coverage": coverage,
            "coverage_mcse": math.sqrt(coverage*(1-coverage)/len(successes)) if successes else None,
            "coverage_among_all_attempts": covered/len(records),
            "mean_interval_width": mean([r["width"] for r in successes])}


def audit(output):
    manifest = json.loads((output / "manifest.json").read_text())
    result = json.loads((output / "result.json").read_text())
    require(manifest["schema"] == "unified-saved-draw-manifest-v1" and manifest["profile"] == "comparison", "wrong run scope")
    require(result["manifest_sha256"] == sha(output / "manifest.json"), "result/manifest binding")
    require(manifest["bundle_sha256"] == sha(output / "source.tar.gz"), "source bundle binding")
    require(manifest["arms"] == list(ARMS) and manifest["new_outcome_draws"] == 0, "arm or draw scope")
    for path, digest in manifest["input_files"].items():
        require(sha(path) == digest, f"changed saved input: {path}")
    with tarfile.open(output / "source.tar.gz") as bundle:
        members = bundle.getnames()
        require(len(members) == len(set(members)), "duplicate source bundle members")
        for name, digest in manifest["source_files"].items():
            stream = bundle.extractfile(name)
            require(stream is not None and hashlib.file_digest(stream, "sha256").hexdigest() == digest,
                    f"source member binding: {name}")
        require("rust/crates/vckss-core/src/cmg_impl.rs.in" in manifest["source_files"], "missing CMG build template")
        stream = bundle.extractfile("build/receipt.json")
        require(hashlib.file_digest(stream, "sha256").hexdigest() == manifest["build_receipt_sha256"], "bundled build receipt")
        require(sha(Path(manifest["build"]) / "receipt.json") == manifest["build_receipt_sha256"], "current build receipt")
        for arm in ARMS:
            for family, entry in manifest["binaries"][arm].items():
                require(sha(entry["path"]) == entry["sha256"], "changed executable")
                stream = bundle.extractfile(f"bin/{arm}/{family}")
                require(hashlib.file_digest(stream, "sha256").hexdigest() == entry["sha256"], "bundled executable")
    specifications = {(c["family"], c["cell"], c["k"]): c for c in manifest["cells"]}
    require(len(specifications) == 48, "cell inventory")
    require(sum(k[0] == "observation" for k in specifications) == 20, "observation inventory")
    groups = collections.defaultdict(list)
    calls_by_key, calls_by_cell, truth_by_key = {}, collections.defaultdict(list), {}
    ids = [t["id"] for t in manifest["tasks"]]
    require(len(ids) == len(set(ids)) == 192, "task ID inventory")
    require({p.name for p in (output / "tasks").iterdir()} == set(ids), "task directory inventory")
    for task in manifest["tasks"]:
        key = task["family"], task["cell"], task["k"]
        spec = specifications[key]
        q1 = spec.get("reference", spec.get("reference_distribution", "")).lower() == "q1"
        require({p.name for p in (output / "tasks" / task["id"]).iterdir()} == set(ARMS), "arm directory inventory")
        for arm in ARMS:
            folder = output / "tasks" / task["id"] / arm
            receipt = json.loads((folder / "receipt.json").read_text())
            require(receipt["status"] == "VALIDATED" and receipt["task"] == task and receipt["arm"] == arm, "task receipt identity/status")
            require(receipt["manifest_sha256"] == result["manifest_sha256"], "task manifest binding")
            require(receipt["binary_sha256"] == manifest["binaries"][arm][task["family"]]["sha256"], "task binary binding")
            require(receipt["output_sha256"] == sha(folder / "rows.jsonl.gz"), "task output binding")
            rows = read_jsonl(folder / "rows.jsonl.gz")
            require(len(rows) == task["reps"]+2 and rows[1]["kind"] == "design", "partial task")
            expected_header = {"kind": "task", "schema": "individual-v1",
                               **{k: task[k] for k in ("family", "profile", "cell", "k", "start", "reps", "master", "numseed")}}
            require(rows[0] == expected_header, "task header")
            require(receipt["calls"] == task["reps"] and receipt["targets"] == 4*task["reps"], "receipt counts")
            require(receipt["design"] == rows[1], "receipt design")
            require(receipt["shared_failures"] == sum(r["status"] != "success" for r in rows[2:]), "shared failure count")
            require(key not in truth_by_key or truth_by_key[key] == rows[1]["truth"], "changing truth")
            truth_by_key[key] = rows[1]["truth"]
            replications = []
            for call in rows[2:]:
                rep = call["replication"]
                require(call["kind"] == "call" and call["seed"] == semantic_seed(task["master"], task["cell"], task["k"], rep), "semantic outcome identity")
                identity = arm, *key, rep
                require(identity not in calls_by_key, "duplicate replication")
                calls_by_key[identity] = call
                calls_by_cell[(arm, *key)].append(call)
                replications.append(rep)
                residual = arm == "unified" or key[0] == "observation"
                if call["status"] == "success":
                    require(call["fit"] == (2 if residual else 1), "actual fitter")
                    require(call["max_residual"] <= call["residual_gate"], "solver certificate")
                    require(0 <= call["nonpositive"] <= call["floored"] < call["units"], "positivity count")
                    if residual:
                        require(call["gram_probes"] == 512 and call["gram_rcond"][0] > 0, "Gram diagnostics")
                        atoms = 1642*call["units"] + 2*call["critical_draws"]
                        require(call["counter_atoms"] == atoms and call["counter_words"] == 2*atoms, "Counter accounting")
                    require(close(call["point"][3], call["point"][0]+call["point"][1]+2*call["point"][2]), "point adding-up")
                    if call["joint"] != 0:
                        require(all(x is None for x in call["primitive"]+call["covariance"]), "invalid joint covariance exported")
                target_rows = [target_record(call, rows[1]["truth"][i], i, q1) for i in range(4)]
                if call["status"] == "success":
                    require(call["computed"] == sum(r["available"] for r in target_rows), "computed-target accounting")
                for i, record in enumerate(target_rows):
                    groups[(arm, *key, TARGETS[i])].append(record)
            require(sorted(replications) == list(range(task["start"], task["start"]+task["reps"])), "replication range")
    require(len(calls_by_key) == result["native_calls"] == manifest["expected_calls"] == 22560, "complete call count")
    require(result["target_attempts"] == 90240, "complete target count")
    require(sum(k[1] != "observation" for k in calls_by_key) == manifest["expected_match_calls"] == 22400, "match call count")
    for key in specifications:
        expected = {0, 133, 266, 399} if key[0] == "observation" else set(range(400))
        for arm in ARMS:
            require({r["replication"] for r in calls_by_cell[(arm, *key)]} == expected, "per-cell replication inventory")
        for rep in expected:
            a, b = (calls_by_key[(arm, *key, rep)] for arm in ARMS)
            if a["status"] == b["status"] == "success":
                require(all(close(x, y) for field in ("point", "point_mcse") for x, y in zip(a[field], b[field])), "paired point regression")
    parent = Path(manifest["parent"])
    old_manifest = json.loads((parent / "manifest.json").read_text())
    saved_replays = 0
    for task in old_manifest["tasks"]:
        key = task["family"], task["cell"], task["k"]
        rows = read_jsonl(parent / "tasks" / task["id"] / "rows.jsonl.gz")
        require(rows[1]["truth"] == truth_by_key[key], "saved truth regression")
        for saved in rows[2:]:
            baseline = calls_by_key.get(("baseline", *key, saved["replication"]))
            if baseline is None:
                continue
            saved_replays += 1
            require(baseline["seed"] == saved["seed"] and baseline["status"] == saved["status"], "saved outcome/status regression")
            if baseline["status"] == "success":
                for field in ("point", "point_mcse", "primitive", "covariance"):
                    require(len(baseline[field]) == len(saved[field]) and
                            all(close(x, y) for x, y in zip(baseline[field], saved[field])), "saved numerical regression")
    require(saved_replays == 11280, "saved replay inventory")
    diagnostics = {(d["arm"], d["family"], d["cell"], d["k"]): d for d in result["diagnostics"]}
    require(len(diagnostics) == len(result["diagnostics"]) == 96, "diagnostic inventory")
    for key, calls in calls_by_cell.items():
        successes = [r for r in calls if r["status"] == "success"]
        rconds = [r["gram_rcond"][0] for r in successes if r["gram_rcond"][0] is not None]
        expected = {"calls": len(calls), "shared_success_rate": len(successes)/len(calls),
                    "mean_seconds": mean([r["seconds"] for r in calls]),
                    "median_seconds": statistics.median(r["seconds"] for r in calls),
                    "minimum_gram_rcond": min(rconds) if rconds else None,
                    "maximum_floor_share": max((r["floored"]/r["units"] for r in successes), default=None),
                    "maximum_nonpositive_share": max((r["nonpositive"]/r["units"] for r in successes), default=None)}
        for field, value in expected.items():
            require(close(diagnostics[key][field], value), f"diagnostic mismatch {key}/{field}")
        failures = collections.Counter(r["detail"] for r in calls if r["status"] != "success")
        require(diagnostics[key]["shared_failure_counts"] == dict(failures), "shared failure classification")
    published = {(s["arm"], s["family"], s["cell"], s["k"], s["target"]): s for s in result["summaries"]}
    require(len(published) == len(result["summaries"]) == len(groups) == 384, "summary inventory")
    failures, conservative = [], []
    for key, records in sorted(groups.items()):
        actual = summarize(records)
        for field, value in actual.items():
            require(close(published[key][field], value), f"summary mismatch {key}/{field}")
        arm, family, cell, k, target = key
        spec = specifications[(family, cell, k)]
        if arm != "unified" or family == "observation" or spec["gate"] not in ("correct", "q0_comparator"):
            continue
        if target not in spec.get("coverage_targets", TARGETS):
            continue
        cutoff = manifest["thresholds"][family]
        label = f"{family}/{cell}/{target}"
        if actual["success_rate"] < cutoff["correct_success_rate"]:
            failures.append(f"{label}: availability")
        if any(actual[x] is None for x in ("coverage", "coverage_mcse", "se_ratio")):
            failures.append(f"{label}: unavailable calibration")
            continue
        allowance = max(cutoff["coverage_absolute_tolerance"], cutoff["coverage_mcse_multiplier"]*actual["coverage_mcse"])
        if actual["coverage"] < .95-allowance:
            failures.append(f"{label}: undercoverage")
        if actual["se_ratio"] > cutoff["correct_se_ratio_upper"]:
            failures.append(f"{label}: underestimated SE")
        if actual["se_denominator"] != actual["successes"]:
            failures.append(f"{label}: incomplete SE calibration")
        if actual["coverage"] > .95+allowance or actual["se_ratio"] < cutoff["correct_se_ratio_lower"]:
            conservative.append({"row": label, "coverage": actual["coverage"], "se_ratio": actual["se_ratio"]})
    require(set(failures) == set(result["readiness_failures"]), "readiness decision mismatch")
    require(not result["accounting_failures"] and not result["regressions"], "campaign reported accounting/regression failure")
    expected_status = "READINESS_FAIL" if failures else "BOUNDED_READINESS_PASS"
    require(result["status"] == expected_status, "overall decision mismatch")
    return {"schema": "unified-independent-audit-v1", "accounting_status": "PASS",
            "readiness_status": expected_status, "calls": len(calls_by_key), "target_attempts": 90240,
            "summaries_checked": len(published), "readiness_failures": failures,
            "saved_replays_checked": saved_replays, "diagnostics_checked": len(diagnostics),
            "conservative_primary_rows": conservative, "result_sha256": sha(output / "result.json"),
            "manifest_sha256": sha(output / "manifest.json"), "auditor_sha256": sha(Path(__file__)),
            "claim": "independent inventory and summary reconstruction; not new outcome evidence or asymptotic proof"}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    report = audit(args.run.resolve())
    with args.output.open("x") as stream:
        json.dump(report, stream, indent=2, allow_nan=False)
    print(json.dumps({k: report[k] for k in ("accounting_status", "readiness_status", "calls")}))
