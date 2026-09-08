"""Bounded paired replay of saved outcomes; no new DGP or confirmation draws."""
import argparse
import collections
import concurrent.futures
import gzip
import json
import os
from pathlib import Path
import statistics
import subprocess
import tarfile

from validation import BUILD, OLD, ROOT, compare_points, load_parent, readiness, same, target_row, validate

HERE = Path(__file__).resolve().parent
REGISTRATION = ROOT / "fevc/docs/unified_residual_moments_v1.json"
sha = BUILD.sha
write = BUILD.write_json


def tasks(parent_manifest, profile):
    if profile not in ("tiny", "comparison"):
        raise ValueError("unknown profile")
    for cell in parent_manifest["cells"]:
        first = next(t for t in parent_manifest["tasks"]
                     if all(t[k] == cell[k] for k in ("family", "cell", "k")))
        if profile == "tiny":
            intervals = [(0, 2)]
        elif cell["family"] == "observation":
            intervals = [(rep, 1) for rep in (0, 133, 266, 399)]
        else:
            intervals = [(rep, 100) for rep in range(0, 400, 100)]
        for start, reps in intervals:
            yield {**first, "start": start, "reps": reps,
                   "id": f'{cell["family"]}-{cell["cell"]}-{cell["k"]}-{start:04d}'}


def freeze(build, parent, output, profile, workers, pipeline=None):
    parent_manifest, cells, input_files = load_parent(parent)
    receipt = json.loads((build / "receipt.json").read_text())
    if receipt["status"] != "BUILD_PASS" or receipt["sources"] != BUILD.source_files():
        raise ValueError("build source binding changed")
    binaries = {}
    for arm in BUILD.ARMS:
        binaries[arm] = {}
        for family, expected in receipt["arms"][arm]["binaries"].items():
            binary = build / arm / "bin" / family
            if sha(binary) != expected:
                raise ValueError("changed native executable")
            binaries[arm][family] = {"path": str(binary), "sha256": expected}
    if not 1 <= workers <= 8:
        raise ValueError("at most eight single-threaded calls")
    prerequisites = {}
    if profile == "comparison":
        if pipeline is None:
            raise ValueError("comparison requires the complete tiny pipeline")
        previous = json.loads((pipeline / "result.json").read_text())
        pm = json.loads((pipeline / "manifest.json").read_text())
        if (previous["status"] != "PIPELINE_PASS"
                or previous["manifest_sha256"] != sha(pipeline / "manifest.json")
                or pm["build_receipt_sha256"] != sha(build / "receipt.json")
                or pm["registration_sha256"] != sha(REGISTRATION)
                or pm["input_files"] != input_files):
            raise ValueError("tiny pipeline identity changed")
        for name, expected in pm["source_files"].items():
            if sha(ROOT / name) != expected:
                raise ValueError("source changed since tiny pipeline")
        prerequisites = {"pipeline": str(pipeline), "result_sha256": sha(pipeline / "result.json")}
    output.mkdir(parents=True, exist_ok=False)
    sources = dict(receipt["sources"])
    extra = list((ROOT / "fevc").glob("*.ado")) + list((ROOT / "fevc/tools").glob("*.py"))
    extra += [REGISTRATION, ROOT / "fevc/fevc.sthlp", ROOT / "finalize_fevc.md",
              ROOT / "fevc/docs/rc_observation_inference_v1.json",
              ROOT / "fevc/docs/individual_inference_development_v1.json",
              ROOT / "fevc/tests/python/test_unified_saved_draws.py"]
    sources.update({str(p.relative_to(ROOT)): sha(p) for p in extra})
    with tarfile.open(output / "source.tar.gz", "x:gz") as archive:
        for name in sorted(sources):
            archive.add(ROOT / name, arcname=name, recursive=False)
        archive.add(build / "receipt.json", arcname="build/receipt.json", recursive=False)
        for arm in BUILD.ARMS:
            for family, info in binaries[arm].items():
                archive.add(info["path"], arcname=f"bin/{arm}/{family}", recursive=False)
                archive.add(build / arm / "bin" / f"{family}.rs",
                            arcname=f"build/{arm}/{family}.rs", recursive=False)
            for path in sorted((build / arm / "adapter").iterdir()):
                archive.add(path, arcname=f"build/{arm}/adapter/{path.name}", recursive=False)
    if any(sha(ROOT / name) != expected for name, expected in sources.items()):
        raise ValueError("source changed during freeze")
    inventory = list(tasks(parent_manifest, profile))
    manifest = {"schema": "unified-saved-draw-manifest-v1", "profile": profile,
                "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                "source_binding": "immutable bundle of dirty development worktree; not a release tag",
                "source_files": sources, "bundle_sha256": sha(output / "source.tar.gz"),
                "registration": json.loads(REGISTRATION.read_text()), "registration_sha256": sha(REGISTRATION),
                "build": str(build), "build_receipt_sha256": sha(build / "receipt.json"),
                "binaries": binaries, "input_files": input_files, "parent": str(parent),
                "cells": parent_manifest["cells"], "tasks": inventory,
                "thresholds": parent_manifest["thresholds"], "arms": list(BUILD.ARMS),
                "workers": workers, "threads_per_call": 1, "prerequisites": prerequisites,
                "expected_calls": 2 * sum(t["reps"] for t in inventory),
                "expected_match_calls": 2 * sum(t["reps"] for t in inventory if t["family"] != "observation"),
                "new_outcome_draws": 0}
    write(output / "manifest.json", manifest)
    return manifest, cells


def execute(binary, folder, task, arm, manifest_hash, fixture=False):
    folder.mkdir(parents=True, exist_ok=False)
    raw = folder / "rows.jsonl.partial"
    command = [str(binary), task["profile"], task["cell"], str(task["k"]), str(task["start"]),
               str(task["reps"]), str(task["master"]), "individual-v1"]
    environment = dict(os.environ)
    # A caller's diagnostic environment must not alter task output or overwrite
    # a fixture across replications. Captures are explicit single-call checks.
    environment.pop("FEVC_INDIVIDUAL_FIXTURE", None)
    if fixture:
        if task["reps"] != 1:
            raise ValueError("fixture capture requires a single saved draw")
        environment["FEVC_INDIVIDUAL_FIXTURE"] = str(folder / "fixture.csv")
    receipt = {"task": task, "arm": arm, "manifest_sha256": manifest_hash,
               "binary_sha256": sha(binary), "command": command}
    try:
        with raw.open("x") as log, (folder / "stderr.log").open("x") as err:
            completed = subprocess.run(command, stdout=log, stderr=err, env=environment, timeout=1800)
        if completed.returncode:
            raise ValueError(f"process exit {completed.returncode}")
        records = [json.loads(line) for line in raw.read_text().splitlines()]
        design, calls = validate(records, task, arm)
        compressed = folder / "rows.jsonl.gz"
        temporary = folder / "rows.jsonl.gz.partial"
        with temporary.open("xb") as stream:
            stream.write(gzip.compress(raw.read_bytes(), mtime=0))
        temporary.rename(compressed)
        raw.rename(folder / "stdout.jsonl")
        receipt.update(status="VALIDATED", calls=len(calls), targets=4*len(calls), design=design,
                       output_sha256=sha(compressed), shared_failures=sum(r["status"] != "success" for r in calls))
    except (ValueError, OSError, subprocess.TimeoutExpired) as error:
        receipt.update(status="EXECUTION_FAILURE", detail=str(error))
    write(folder / "receipt.json", receipt)
    return receipt


def execute_pair(task, ordinal, manifest, output):
    order = BUILD.ARMS if ordinal % 2 == 0 else tuple(reversed(BUILD.ARMS))
    receipts = []
    for arm in order:
        binary = Path(manifest["binaries"][arm][task["family"]]["path"])
        receipts.append(execute(binary, output / "tasks" / task["id"] / arm, task, arm,
                                sha(output / "manifest.json")))
    return receipts


def read_task(output, manifest, task, arm):
    folder = output / "tasks" / task["id"] / arm
    receipt = json.loads((folder / "receipt.json").read_text())
    if (receipt.get("status") != "VALIDATED" or receipt.get("task") != task or receipt.get("arm") != arm
            or receipt.get("manifest_sha256") != sha(output / "manifest.json")
            or receipt.get("binary_sha256") != manifest["binaries"][arm][task["family"]]["sha256"]
            or receipt.get("output_sha256") != sha(folder / "rows.jsonl.gz")):
        raise ValueError(f"invalid task receipt: {receipt.get('detail', '')}")
    records = [json.loads(line) for line in gzip.open(folder / "rows.jsonl.gz", "rt")]
    design, calls = validate(records, task, arm)
    if (receipt["calls"] != len(calls) or receipt["targets"] != 4*len(calls) or receipt["design"] != design
            or receipt["shared_failures"] != sum(r["status"] != "success" for r in calls)):
        raise ValueError("receipt inventory mismatch")
    return design, calls


def aggregate(output, cells):
    manifest = json.loads((output / "manifest.json").read_text())
    errors, regressions, attempts = [], [], 0
    groups, cell_calls = collections.defaultdict(list), collections.defaultdict(list)
    expected_dirs = {t["id"] for t in manifest["tasks"]}
    if {p.name for p in (output / "tasks").iterdir()} != expected_dirs:
        errors.append("unexpected or missing task directories")
    for task in manifest["tasks"]:
        key = task["family"], task["cell"], task["k"]
        cell = OLD.spec_for(task)
        paired = {}
        folder = output / "tasks" / task["id"]
        if folder.is_dir() and {p.name for p in folder.iterdir()} != set(BUILD.ARMS):
            errors.append(f'{task["id"]}: unexpected or missing arms')
        for arm in BUILD.ARMS:
            try:
                design, calls = read_task(output, manifest, task, arm)
                if design != cells[key]["design"]:
                    raise ValueError("saved design changed")
                paired[arm] = {r["replication"]: r for r in calls}
                attempts += len(calls)
                cell_calls[(arm, *key)].extend(calls)
                for call in calls:
                    for t, name in enumerate(OLD.TARGETS):
                        groups[(arm, *key, name)].append(target_row(call, design, cell, t))
            except (ValueError, OSError, KeyError) as error:
                errors.append(f'{task["id"]}/{arm}: {error}')
        if len(paired) == 2:
            for rep, baseline in paired["baseline"].items():
                for failure in compare_points(baseline, paired["unified"][rep], cells[key]["calls"][rep]):
                    regressions.append(f'{task["id"]}/{rep}: {failure}')
    if attempts != manifest["expected_calls"]:
        errors.append(f'call inventory: {attempts} != {manifest["expected_calls"]}')
    summaries = []
    for (arm, family, cell_name, k, target), rows in sorted(groups.items()):
        cell = next(c for c in manifest["cells"] if (c["family"], c["cell"], c["k"]) == (family, cell_name, k))
        summary = OLD.Q1._summary(rows, len(rows))
        summary.update(cell, arm=arm, target=target)
        summary["se_denominator"] = sum(r["status"] == "success" and r.get("estimated_sd") is not None for r in rows)
        widths = [r["interval_width"] for r in rows if r["status"] == "success"]
        summary["mean_interval_width"] = statistics.fmean(widths) if widths else None
        if family == "observation":
            d, index = cells[(family, cell_name, k)]["design"], OLD.TARGETS.index(target)
            summary.update(mean_leading_share=d["leading_share"][index], mean_remainder_share=d["remainder_share"][index])
        summaries.append(summary)
    diagnostics = []
    for (arm, family, name, k), calls in sorted(cell_calls.items()):
        successful = [r for r in calls if r["status"] == "success"]
        rconds = [r["gram_rcond"][0] for r in successful if r["gram_rcond"][0] is not None]
        diagnostics.append({"arm": arm, "family": family, "cell": name, "k": k,
                            "calls": len(calls), "shared_success_rate": len(successful)/len(calls),
                            "shared_failure_counts": dict(collections.Counter(r["detail"] for r in calls if r["status"] != "success")),
                            "mean_seconds": statistics.fmean(r["seconds"] for r in calls),
                            "median_seconds": statistics.median(r["seconds"] for r in calls),
                            "minimum_gram_rcond": min(rconds) if rconds else None,
                            "maximum_floor_share": max((r["floored"]/r["units"] for r in successful), default=None),
                            "maximum_nonpositive_share": max((r["nonpositive"]/r["units"] for r in successful), default=None)})
    historical, safeguards = {}, []
    if not errors and manifest["profile"] == "comparison":
        safeguards = readiness(summaries)
        for arm in BUILD.ARMS:
            historical[arm] = {}
            for family in OLD.GATES:
                selected = [s for s in summaries if s["arm"] == arm and s["family"] == family]
                failures, stress = OLD.GATES[family]._scientific_failures(
                    selected, "confirmation" if family == "observation" else "development")
                historical[arm][family] = {"failures": failures, "stress": stress,
                                          "scope": "four-draw regression slice only" if family == "observation" else "saved development draws"}
    status = "ACCOUNTING_FAIL" if errors else "REGRESSION_FAIL" if regressions else (
        "READINESS_FAIL" if safeguards else "PIPELINE_DATA_VALID" if manifest["profile"] == "tiny" else "BOUNDED_READINESS_PASS")
    return {"status": status, "manifest_sha256": sha(output / "manifest.json"),
            "native_calls": attempts, "target_attempts": 4*attempts,
            "accounting_failures": errors, "regressions": regressions, "readiness_failures": safeguards,
            "historical_two_sided": historical, "summaries": summaries, "diagnostics": diagnostics,
            "claim": "bounded saved-draw development; not independent confirmation"}


def pipeline_checks(output, manifest):
    checks = []
    for family in OLD.GATES:
        task = next(t for t in manifest["tasks"] if t["family"] == family)
        for arm in BUILD.ARMS:
            _, original = read_task(output, manifest, task, arm)
            binary = Path(manifest["binaries"][arm][family]["path"])
            for rep in (1, 0):
                split = {**task, "start": rep, "reps": 1}
                folder = output / "pipeline-checks" / f"{family}-{arm}-{rep}"
                receipt = execute(binary, folder, split, arm, sha(output / "manifest.json"), fixture=rep == 0)
                if receipt["status"] != "VALIDATED":
                    raise ValueError("split task failed")
                records = [json.loads(line) for line in gzip.open(folder / "rows.jsonl.gz", "rt")]
                _, calls = validate(records, split, arm)
                strip_time = lambda r: {k: v for k, v in r.items() if k != "seconds"}
                if strip_time(calls[0]) != strip_time(original[rep]):
                    raise ValueError("split/reversed task changed results")
                checks.append(str(folder.relative_to(output)))
            invalid = subprocess.run([str(binary), "invalid-cli"], capture_output=True, text=True, timeout=30)
            if invalid.returncode == 0:
                raise ValueError("malformed CLI did not fail")
            write(output / "pipeline-checks" / f"{family}-{arm}-invalid.json",
                  {"returncode": invalid.returncode, "stderr": invalid.stderr})
    return checks


def run(build, parent, output, profile, workers, pipeline):
    manifest, cells = freeze(build, parent, output, profile, workers, pipeline)
    with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
        futures = [pool.submit(execute_pair, task, i, manifest, output)
                   for i, task in enumerate(manifest["tasks"])]
        for future in concurrent.futures.as_completed(futures):
            for receipt in future.result():
                print(json.dumps({"task": receipt["task"]["id"], "arm": receipt["arm"], "status": receipt["status"]}), flush=True)
    result = aggregate(output, cells)
    if profile == "tiny" and result["status"] == "PIPELINE_DATA_VALID":
        try:
            result["pipeline_checks"] = pipeline_checks(output, manifest)
            result["status"] = "PIPELINE_PASS"
        except (ValueError, OSError, subprocess.TimeoutExpired) as error:
            result.update(status="PIPELINE_FAIL", pipeline_error=str(error))
    # Detect concurrent source/input changes even when all native calls succeeded.
    changed = [name for name, expected in manifest["source_files"].items() if sha(ROOT / name) != expected]
    changed += [name for name, expected in manifest["input_files"].items() if sha(name) != expected]
    if changed:
        result.update(status="BINDING_FAIL", changed_files=changed)
    write(output / "result.json", result)
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("profile", choices=("tiny", "comparison"))
    parser.add_argument("build", type=Path)
    parser.add_argument("parent", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--workers", type=int, default=8)
    parser.add_argument("--pipeline", type=Path)
    args = parser.parse_args()
    result = run(args.build.resolve(), args.parent.resolve(), args.output.resolve(),
                 args.profile, args.workers, args.pipeline.resolve() if args.pipeline else None)
    print(json.dumps({k: result[k] for k in ("status", "native_calls", "target_attempts")}))
    raise SystemExit(result["status"] not in ("PIPELINE_PASS", "BOUNDED_READINESS_PASS"))
