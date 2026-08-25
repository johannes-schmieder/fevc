#!/usr/bin/env python3
"""Run the source-bound local A/C/maintained-MATLAB full-CMG spike."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import platform
import re
import shutil
import socket
import statistics
import subprocess
import tarfile
import time
from datetime import UTC, datetime
from pathlib import Path

ROWS = 1_966_080
WORKERS = 327_680
FIRMS = 8_192
DEGREE = 6
PROBES = 200
SEED = 2_026_082_501
THREADS = 4
WARM_REPETITIONS = 5
BASELINE_COMMIT = "4124b34f3ca216dcc3aae27e4b31bbac9e011f11"
CMG_COMMIT = "dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10"
STATA_MARKER = "VCKSS_FULL_CMG_SPIKE_STATA_PASS"
MATLAB_MARKER = "PAPER MATLAB SCALING MATLAB PASS"
POLICY_PATH = Path(__file__).resolve().parents[2] / "docs/development_acceptance_v1.json"
POLICY = json.loads(POLICY_PATH.read_text(encoding="utf-8"))
SCALE_RELATIVE_TOLERANCE = float(
    POLICY["point_estimate_equivalence"]["scale_relative_tolerance"]
)
COMMON_DRAW_MCSE_FRACTION = float(
    POLICY["point_estimate_equivalence"]["common_draw_mcse_fraction"]
)
DIAGNOSTIC_PREFIX = "CMG_FULL_SPIKE_V1"
RUN_ORDERS = (
    ("baseline", "candidate", "matlab"),
    ("candidate", "matlab", "baseline"),
    ("matlab", "baseline", "candidate"),
    ("baseline", "matlab", "candidate"),
    ("candidate", "baseline", "matlab"),
    ("matlab", "candidate", "baseline"),
)
RESULT_FIELDS = tuple(
    f"{kind}{column}"
    for kind in ("plugin", "correction", "corrected", "mcse")
    for column in range(1, 5)
)
PRIMARY_RESULT_FIELDS = tuple(f"corrected{column}" for column in range(1, 5))
STRUCTURAL_FIELDS = (
    "rows",
    "workers",
    "firms",
    "cells_per_worker",
    "probes",
    "seed",
    "processors",
    "engine",
    "preconditioner",
    "sample_count",
)
PHASE_FIELDS = (
    "ingest_seconds",
    "canonical_seconds",
    "native_graph_seconds",
    "native_compression_seconds",
    "native_plan_seconds",
    "native_solve_seconds",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def git(repo: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", *arguments],
        cwd=repo,
        check=True,
        text=True,
        capture_output=True,
    ).stdout.strip()


def archive_package(repo: Path, commit: str, destination: Path) -> None:
    tar_path = destination.with_suffix(".tar")
    destination.mkdir()
    with tar_path.open("wb") as handle:
        subprocess.run(
            ["git", "archive", commit, "vckss"],
            cwd=repo,
            check=True,
            stdout=handle,
        )
    with tarfile.open(tar_path, "r:") as archive:
        archive.extractall(destination, filter="data")
    tar_path.unlink()


def read_key_values(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        key, separator, value = line.partition("=")
        require(bool(separator) and key not in values, f"invalid key-value receipt: {path}")
        values[key] = value
    return values


def read_one_csv(path: Path) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"expected one CSV row: {path}")
    return rows[0]


def parse_peak_rss(path: Path) -> int:
    match = re.search(
        r"^\s*(\d+)\s+maximum resident set size\s*$",
        path.read_text(encoding="utf-8"),
        re.MULTILINE,
    )
    require(match is not None, f"could not parse macOS peak RSS: {path}")
    return int(match.group(1))


def collect_stata_log(directory: Path, console: Path, destination: Path) -> None:
    batch_logs = [
        path
        for path in directory.glob("*.log")
        if path not in (console, destination)
    ]
    require(len(batch_logs) == 1, f"expected one Stata batch log in {directory}")
    destination.write_bytes(batch_logs[0].read_bytes() + console.read_bytes())
    batch_logs[0].unlink()
    console.unlink()


def parse_diagnostics(log: Path) -> dict[str, object]:
    setup: dict[str, str] | None = None
    batches: list[dict[str, str]] = []
    for line in log.read_text(encoding="utf-8").splitlines():
        if DIAGNOSTIC_PREFIX not in line:
            continue
        payload = line[line.index(DIAGNOSTIC_PREFIX) :].split()
        require(len(payload) >= 3, f"malformed full-CMG diagnostic: {line}")
        values = dict(token.split("=", 1) for token in payload[2:] if "=" in token)
        if payload[1] == "SETUP":
            require(setup is None, "candidate emitted multiple full-CMG setup receipts")
            setup = values
        elif payload[1] == "BATCH":
            batches.append(values)
    require(setup is not None and batches, "candidate full-CMG diagnostics are missing")
    integer_fields = (
        "rhs",
        "concurrency",
        "rhs_ns",
        "solve_ns",
        "extraction_ns",
        "max_iterations",
        "total_iterations",
        "total_operator_applications",
        "total_preconditioner_applications",
    )
    for batch in batches:
        require(all(field in batch for field in integer_fields), "incomplete batch receipt")
    rhs_count = sum(int(batch["rhs"]) for batch in batches)
    require(rhs_count == 1 + 3 * PROBES, f"expected 601 repeated RHS, found {rhs_count}")
    require(int(setup["threads"]) == THREADS, "full-CMG thread receipt changed")
    require(max(int(batch["concurrency"]) for batch in batches) <= THREADS,
            "full-CMG concurrency exceeds requested threads")
    return {
        "setup": setup,
        "batch_count": len(batches),
        "rhs_count": rhs_count,
        "rhs_seconds": sum(int(batch["rhs_ns"]) for batch in batches) / 1e9,
        "solve_seconds": sum(int(batch["solve_ns"]) for batch in batches) / 1e9,
        "extraction_seconds": sum(int(batch["extraction_ns"]) for batch in batches) / 1e9,
        "maximum_iterations": max(int(batch["max_iterations"]) for batch in batches),
        "total_iterations": sum(int(batch["total_iterations"]) for batch in batches),
        "total_operator_applications": sum(
            int(batch["total_operator_applications"]) for batch in batches
        ),
        "total_preconditioner_applications": sum(
            int(batch["total_preconditioner_applications"]) for batch in batches
        ),
        "maximum_reduced_residual": max(
            float(batch["max_reduced_residual"]) for batch in batches
        ),
        "maximum_complete_residual": max(
            float(batch["max_complete_residual"]) for batch in batches
        ),
        "executions": sorted({batch["execution"] for batch in batches}),
        "maximum_concurrency": max(int(batch["concurrency"]) for batch in batches),
    }


def process_snapshot() -> dict[int, tuple[int, int]]:
    completed = subprocess.run(
        ["ps", "-axo", "pid=,ppid=,rss="],
        check=True,
        text=True,
        capture_output=True,
    )
    result: dict[int, tuple[int, int]] = {}
    for line in completed.stdout.splitlines():
        fields = line.split()
        if len(fields) == 3:
            result[int(fields[0])] = (int(fields[1]), int(fields[2]))
    return result


def descendants(snapshot: dict[int, tuple[int, int]], root_pid: int) -> set[int]:
    selected = {root_pid}
    changed = True
    while changed:
        changed = False
        for pid, (parent, _) in snapshot.items():
            if pid not in selected and parent in selected:
                selected.add(pid)
                changed = True
    return selected.intersection(snapshot)


def monitor_macos_tree(
    process: subprocess.Popen[bytes], identity_path: Path, output: Path
) -> dict[str, object]:
    peak_rss_kib = 0
    peak_process_count = 0
    identity_observations = 0
    identity: dict[str, object] | None = None
    samples = 0
    while process.poll() is None:
        snapshot = process_snapshot()
        selected = descendants(snapshot, process.pid)
        if selected:
            samples += 1
            peak_rss_kib = max(peak_rss_kib, sum(snapshot[pid][1] for pid in selected))
            peak_process_count = max(peak_process_count, len(selected))
        if identity is None and identity_path.is_file():
            identity = json.loads(identity_path.read_text(encoding="utf-8"))
        if identity is not None:
            named = {int(identity["client_pid"]), *(int(pid) for pid in identity["worker_pids"])}
            if named.issubset(selected):
                identity_observations += 1
        time.sleep(0.25)
    require(samples > 0 and peak_rss_kib > 0, "MATLAB process-tree monitor saw no RSS")
    require(identity is not None and identity_observations > 0,
            "MATLAB client/worker process identity was not observed")
    receipt = {
        "schema": "VCKSS-FULL-CMG-SPIKE-MACOS-PROCESS-TREE-V1",
        "status": "PASS",
        "root_pid": process.pid,
        "sample_count": samples,
        "peak_rss_kib": peak_rss_kib,
        "peak_process_count": peak_process_count,
        "identity_observation_count": identity_observations,
        "identity_sha256": sha256(identity_path),
    }
    output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return receipt


def validate_stata(row: dict[str, str], role: str, commit: str, task_sha: str,
                   input_sha: str) -> None:
    require(row["schema"] == "VCKSS-FULL-CMG-SPIKE-STATA-V1", "Stata schema changed")
    require(row["role"] == role and row["source_commit"] == commit, "Stata source binding failed")
    require(row["task_sha256"] == task_sha and row["input_sha256"] == input_sha,
            "Stata input binding failed")
    require(int(float(row["rows"])) == ROWS and int(float(row["workers"])) == WORKERS and
            int(float(row["firms"])) == FIRMS, "Stata dimensions changed")
    require(int(float(row["processors"])) == THREADS, "Stata processor count changed")
    for field in ("data_restored", "rng_restored", "sort_rng_restored"):
        require(int(float(row[field])) == 1, f"Stata failed {field}")
    require(float(row["max_complete_residual"]) <= float(row["residual_acceptance"]),
            "Stata complete residual gate failed")
    require(abs(float(row["target_identity_residual"])) <= 1e-12,
            "Stata target identity gate failed")


def common_draw_acceptance(
    left: dict[str, object], right: dict[str, object], column: int
) -> tuple[float, float, float]:
    field = f"corrected{column}"
    left_value = float(left[field])
    right_value = float(right[field])
    difference = abs(left_value - right_value)
    scale_floor = SCALE_RELATIVE_TOLERANCE * max(
        1.0, abs(left_value), abs(right_value)
    )
    mcse_limit = COMMON_DRAW_MCSE_FRACTION * max(
        float(left[f"mcse{column}"]), float(right[f"mcse{column}"])
    )
    limit = max(scale_floor, mcse_limit)
    return difference, limit, difference / limit


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--baseline", default=BASELINE_COMMIT)
    parser.add_argument("--candidate", default="HEAD")
    parser.add_argument("--baseline-plugin", type=Path, required=True)
    parser.add_argument("--candidate-plugin", type=Path, required=True)
    parser.add_argument("--candidate-build-receipt", type=Path, required=True)
    parser.add_argument("--matlab-root", type=Path, required=True)
    parser.add_argument("--matlab", type=Path,
                        default=Path("/Applications/MATLAB_R2024b.app/bin/matlab"))
    parser.add_argument("--stata", type=Path,
                        default=Path("/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp"))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo = Path(__file__).resolve().parents[3]
    require(not git(repo, "status", "--porcelain"), "benchmark tool worktree must be clean")
    tool_commit = git(repo, "rev-parse", "HEAD^{commit}")
    baseline = git(repo, "rev-parse", f"{args.baseline}^{{commit}}")
    candidate = git(repo, "rev-parse", f"{args.candidate}^{{commit}}")
    require(baseline == BASELINE_COMMIT, "baseline commit is not the registered checkpoint")
    require(all(len(value) == 40 for value in (tool_commit, baseline, candidate)),
            "source commits must be full SHA-1 values")
    require(not args.output_dir.exists() or not any(args.output_dir.iterdir()),
            "output directory must be new or empty")
    for executable in (args.stata, args.matlab):
        require(executable.is_file(), f"missing executable: {executable}")
    for plugin in (args.baseline_plugin, args.candidate_plugin):
        require(plugin.is_file(), f"missing plugin: {plugin}")
    build = read_key_values(args.candidate_build_receipt)
    require(build.get("schema") == "CMG_FULL_SPIKE_BUILD_V1", "candidate build schema changed")
    require(build.get("vckss_commit") == candidate and build.get("vckss_dirty") == "0",
            "candidate build is not clean and source-bound")
    require(build.get("cmg_commit") == CMG_COMMIT, "standalone CMG source changed")
    require(build.get("plugin_sha256") == sha256(args.candidate_plugin),
            "candidate plugin hash does not match its build receipt")

    args.output_dir.mkdir(parents=True, exist_ok=True)
    sources = args.output_dir / "sources"
    sources.mkdir()
    roots = {role: sources / role for role in ("baseline", "candidate")}
    archive_package(repo, baseline, roots["baseline"])
    archive_package(repo, candidate, roots["candidate"])
    plugin_hashes = {
        "baseline": sha256(args.baseline_plugin),
        "candidate": sha256(args.candidate_plugin),
    }
    for role, plugin in (
        ("baseline", args.baseline_plugin),
        ("candidate", args.candidate_plugin),
    ):
        shutil.copy2(plugin, roots[role] / "vckss/vckss_rust_macos_arm64.plugin")

    driver_dir = Path(__file__).resolve().parent
    input_dir = args.output_dir / "input"
    input_dir.mkdir()
    input_csv = input_dir / "input.csv"
    input_receipt = input_dir / "input_receipt.csv"
    input_log = input_dir / "generate.log"
    input_console = input_dir / "generate.console.log"
    with input_console.open("wb") as log_handle:
        input_command = [
            str(args.stata), "-q", "-b", "do",
            str(driver_dir.parent / "paper_matlab_scaling/generate_input.do"),
            str(input_csv), str(input_receipt), "strong_d6", "strong",
            str(ROWS), str(DEGREE),
        ]
        completed = subprocess.run(input_command, cwd=input_dir, stdout=log_handle,
                                   stderr=subprocess.STDOUT, check=False)
    collect_stata_log(input_dir, input_console, input_log)
    require(completed.returncode == 0 and input_csv.is_file(), "input generation failed")
    require("PAPER_MATLAB_SCALING_INPUT_PASS" in input_log.read_text(encoding="utf-8"),
            "input generation PASS marker is missing")
    input_sha = sha256(input_csv)

    matlab_runtime = args.output_dir / "matlab_runtime"
    matlab_runtime.mkdir()
    for relative in ("codes", "CMG"):
        shutil.copytree(
            args.matlab_root / relative,
            matlab_runtime / relative,
            ignore=shutil.ignore_patterns(".DS_Store"),
        )
    source_identity = args.output_dir / "matlab_source_identity.json"
    contract = driver_dir.parent / "matlab_scale/source_contract.json"
    subprocess.run(
        [
            "python3", str(repo / "vckss/benchmarks/scc/verify_numopt2_matlab_source.py"),
            "--matlab-root", str(matlab_runtime), "--contract", str(contract),
            "--contract-sha256", sha256(contract), "--output", str(source_identity),
        ],
        cwd=repo,
        check=True,
    )

    task_payload = {
        "schema": "VCKSS-FULL-CMG-SPIKE-LOCAL-CASE-V1",
        "tool_commit": tool_commit,
        "baseline_commit": baseline,
        "candidate_commit": candidate,
        "cmg_commit": CMG_COMMIT,
        "baseline_plugin_sha256": plugin_hashes["baseline"],
        "candidate_plugin_sha256": plugin_hashes["candidate"],
        "input_sha256": input_sha,
        "structure": "strong_d6",
        "rows": ROWS,
        "workers": WORKERS,
        "firms": FIRMS,
        "degree": DEGREE,
        "probes": PROBES,
        "seed": SEED,
        "threads": THREADS,
        "cold_repetitions": 1,
        "warm_repetitions": WARM_REPETITIONS,
        "orders": RUN_ORDERS,
    }
    task_path = args.output_dir / "case.json"
    task_path.write_text(json.dumps(task_payload, indent=2, sort_keys=True) + "\n",
                         encoding="utf-8")
    task_sha = sha256(task_path)
    source_binding_sha = hashlib.sha256(
        (candidate + plugin_hashes["candidate"] + sha256(Path(__file__)) + input_sha).encode()
    ).hexdigest()

    rows: dict[str, list[dict[str, object]]] = {role: [] for role in ("baseline", "candidate", "matlab")}
    evidence: list[dict[str, object]] = []
    for run_index, order in enumerate(RUN_ORDERS):
        temperature = "cold" if run_index == 0 else "warm"
        for role in order:
            run_dir = args.output_dir / "runs" / f"{run_index:02d}-{temperature}" / role
            run_dir.mkdir(parents=True)
            log_path = run_dir / "application.log"
            resources = run_dir / "resources.txt"
            if role in ("baseline", "candidate"):
                output_csv = run_dir / "stata.csv"
                command = [
                    "/usr/bin/time", "-l", "-o", str(resources), str(args.stata),
                    "-q", "-b", "do", str(driver_dir / "stata_run.do"), str(roots[role]),
                    str(input_csv), str(output_csv), role,
                    baseline if role == "baseline" else candidate,
                    task_sha, input_sha, "strong_d6", "strong", str(ROWS),
                    str(DEGREE), str(PROBES), str(SEED),
                ]
                environment = os.environ.copy()
                for name in (
                    "VCKSS_PRIVATE_CMG_FULL_V1",
                    "VCKSS_PRIVATE_CMG_THREADS",
                    "VCKSS_PRIVATE_CMG_DIAGNOSTICS",
                ):
                    environment.pop(name, None)
                if role == "candidate":
                    environment.update({
                        "VCKSS_PRIVATE_CMG_FULL_V1": "1",
                        "VCKSS_PRIVATE_CMG_THREADS": str(THREADS),
                        "VCKSS_PRIVATE_CMG_DIAGNOSTICS": "1",
                    })
                console_path = run_dir / "console.log"
                with console_path.open("wb") as log_handle:
                    completed = subprocess.run(command, cwd=run_dir, env=environment,
                                               stdout=log_handle, stderr=subprocess.STDOUT,
                                               check=False)
                collect_stata_log(run_dir, console_path, log_path)
                commit = baseline if role == "baseline" else candidate
                log_text = log_path.read_text(encoding="utf-8")
                require(completed.returncode == 0 and
                        f"{STATA_MARKER} {role}" in log_text,
                        f"{role} run {run_index} did not pass")
                row = read_one_csv(output_csv)
                validate_stata(row, role, commit, task_sha, input_sha)
                record: dict[str, object] = dict(row)
                record.update({
                    "run_index": run_index,
                    "temperature": temperature,
                    "launch_position": order.index(role) + 1,
                    "process_peak_rss_bytes": parse_peak_rss(resources),
                })
                if role == "candidate":
                    record["full_cmg"] = parse_diagnostics(log_path)
                rows[role].append(record)
            else:
                output_dir = run_dir / "output"
                scratch_dir = run_dir / "scratch"
                output_dir.mkdir()
                scratch_dir.mkdir()
                identity_path = output_dir / "matlab_process_identity.json"
                matlab_driver = repo / "vckss/benchmarks/paper_matlab_scaling"
                environment = os.environ.copy()
                environment.update({
                    "PMS_OUTPUT_DIR": str(output_dir),
                    "PMS_SCRATCH_DIR": str(scratch_dir),
                    "PMS_INPUT_CSV": str(input_csv),
                    "PMS_INPUT_SHA256": input_sha,
                    "PMS_MATLAB_ROOT": str(matlab_runtime),
                    "PMS_TASK_SHA256": task_sha,
                    "PMS_SOURCE_IDENTITY": str(source_identity),
                    "PMS_PROCESS_IDENTITY": str(identity_path),
                    "PMS_EXPERIMENT_ID": f"full_cmg_local_{run_index:02d}",
                    "PMS_SOURCE_COMMIT": candidate,
                    "PMS_BUNDLE_SHA256": source_binding_sha,
                    "PMS_STRUCTURE": "strong_d6",
                    "PMS_CONNECTIVITY": "strong",
                    "PMS_ROWS": str(ROWS),
                    "PMS_WORKERS": str(WORKERS),
                    "PMS_FIRMS": str(FIRMS),
                    "PMS_DEGREE": str(DEGREE),
                    "PMS_PROBES": str(PROBES),
                    "PMS_SEED": str(SEED),
                    "MATLAB_NUM_THREADS": "1",
                    "OMP_NUM_THREADS": "1",
                    "MKL_NUM_THREADS": "1",
                    "OPENBLAS_NUM_THREADS": "1",
                })
                command = [
                    "/usr/bin/time", "-l", "-o", str(resources), str(args.matlab),
                    "-batch", f"addpath('{matlab_driver}'); paper_matlab_scaling_run",
                ]
                with log_path.open("wb") as log_handle:
                    process = subprocess.Popen(command, cwd=run_dir, env=environment,
                                               stdout=log_handle, stderr=subprocess.STDOUT)
                    process_tree = monitor_macos_tree(
                        process, identity_path, run_dir / "process_tree.json"
                    )
                    returncode = process.wait()
                log_text = log_path.read_text(encoding="utf-8")
                require(returncode == 0 and MATLAB_MARKER in log_text,
                        f"MATLAB run {run_index} did not pass")
                aggregate_path = output_dir / "matlab_aggregate.json"
                aggregate = json.loads(aggregate_path.read_text(encoding="utf-8"))
                require(aggregate.get("status") == "PASS" and
                        aggregate.get("input_sha256") == input_sha,
                        "MATLAB aggregate binding failed")
                aggregate.update({
                    "run_index": run_index,
                    "temperature": temperature,
                    "launch_position": order.index(role) + 1,
                    "process_peak_rss_bytes": int(process_tree["peak_rss_kib"]) * 1024,
                })
                rows[role].append(aggregate)
            evidence.append({
                "role": role,
                "run_index": run_index,
                "temperature": temperature,
                "launch_position": order.index(role) + 1,
                "log": str(log_path.relative_to(args.output_dir)),
                "log_sha256": sha256(log_path),
                "resources": str(resources.relative_to(args.output_dir)),
                "resources_sha256": sha256(resources),
            })
            command_time = float(rows[role][-1]["command_seconds"])
            print(f"run={run_index} role={role} command_seconds={command_time:.3f}", flush=True)

    for role in rows:
        rows[role].sort(key=lambda value: int(value["run_index"]))
        require(len(rows[role]) == 1 + WARM_REPETITIONS, f"incomplete {role} run set")
    for role in ("baseline", "candidate"):
        reference = rows[role][0]
        for row in rows[role][1:]:
            for column in range(1, 5):
                difference, limit, _ = common_draw_acceptance(reference, row, column)
                require(
                    difference <= limit,
                    f"{role} repeated corrected target {column} exceeds common-draw limit",
                )
    for field in STRUCTURAL_FIELDS:
        require(rows["baseline"][0][field] == rows["candidate"][0][field],
                f"A/C structural mismatch: {field}")
    primary_acceptance: list[dict[str, float | int]] = []
    for column in range(1, 5):
        difference, limit, ratio = common_draw_acceptance(
            rows["baseline"][0], rows["candidate"][0], column
        )
        primary_acceptance.append(
            {
                "column": column,
                "absolute_difference": difference,
                "acceptance_limit": limit,
                "acceptance_ratio": ratio,
            }
        )
        require(ratio <= 1, f"A/C corrected target {column} exceeds common-draw limit")
    secondary_differences = {
        field: abs(float(rows["baseline"][0][field]) - float(rows["candidate"][0][field]))
        for field in RESULT_FIELDS
        if field not in PRIMARY_RESULT_FIELDS
    }

    medians = {
        role: statistics.median(float(row["command_seconds"]) for row in values[1:])
        for role, values in rows.items()
    }
    candidate_diagnostics = [row["full_cmg"] for row in rows["candidate"][1:]]
    diagnostic_medians = {
        field: statistics.median(float(receipt[field]) for receipt in candidate_diagnostics)
        for field in (
            "rhs_seconds", "solve_seconds", "extraction_seconds", "total_iterations",
            "total_operator_applications", "total_preconditioner_applications",
        )
    }
    phase_medians = {
        field: statistics.median(float(row[field]) for row in rows["candidate"][1:])
        for field in PHASE_FIELDS
    }
    phase_medians.update({
        "full_cmg_rhs_seconds": diagnostic_medians["rhs_seconds"],
        "full_cmg_solve_seconds": diagnostic_medians["solve_seconds"],
        "full_cmg_extraction_seconds": diagnostic_medians["extraction_seconds"],
    })
    dominant_phase = max(phase_medians, key=phase_medians.get)
    summary = {
        "schema": "VCKSS-FULL-CMG-SPIKE-LOCAL-V1",
        "status": "PASS",
        "generated_at_utc": datetime.now(UTC).isoformat(),
        "tool_commit": tool_commit,
        "baseline_commit": baseline,
        "candidate_commit": candidate,
        "cmg_commit": CMG_COMMIT,
        "task_sha256": task_sha,
        "input_sha256": input_sha,
        "host": socket.gethostname(),
        "platform": platform.platform(),
        "stata_executable": str(args.stata),
        "matlab_executable": str(args.matlab),
        "matlab_maintained_source_root": str(args.matlab_root),
        "matlab_isolated_runtime_root": str(matlab_runtime),
        "matlab_isolated_view_exclusions": [".DS_Store"],
        "matlab_source_identity_sha256": sha256(source_identity),
        "plugin_sha256": plugin_hashes,
        "cold_repetitions": 1,
        "warm_repetitions": WARM_REPETITIONS,
        "position_balanced_orders": RUN_ORDERS,
        "warm_median_command_seconds": medians,
        "candidate_over_baseline": medians["candidate"] / medians["baseline"],
        "candidate_over_matlab": medians["candidate"] / medians["matlab"],
        "baseline_over_matlab": medians["baseline"] / medians["matlab"],
        "two_x_matlab_target_met": medians["candidate"] / medians["matlab"] <= 0.5,
        "hardening_gate_met": medians["candidate"] / medians["matlab"] <= 0.75,
        "development_acceptance_schema": POLICY["schema"],
        "a_c_primary_corrected_acceptance": primary_acceptance,
        "a_c_maximum_equivalence_limit_ratio": max(
            row["acceptance_ratio"] for row in primary_acceptance
        ),
        "a_c_secondary_differences": secondary_differences,
        "a_c_scientific_gate": "PASS_CORRECTED_TARGETS_COMMON_DRAW",
        "matlab_equality_gate": "NONE_REGISTERED_DESCRIPTIVE_ONLY",
        "matlab_solver_tolerance_comparable": False,
        "candidate_full_cmg_warm_medians": diagnostic_medians,
        "candidate_phase_warm_medians": phase_medians,
        "candidate_dominant_measured_phase": dominant_phase,
        "candidate_dominant_measured_phase_seconds": phase_medians[dominant_phase],
        "rows": rows,
        "evidence": evidence,
    }
    summary_path = args.output_dir / "summary.json"
    summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n",
                            encoding="utf-8")
    print(
        "VCKSS_FULL_CMG_SPIKE_LOCAL_PASS "
        f"candidate_over_matlab={summary['candidate_over_matlab']:.6f} "
        f"candidate_over_baseline={summary['candidate_over_baseline']:.6f}",
        flush=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
