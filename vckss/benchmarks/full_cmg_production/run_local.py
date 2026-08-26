#!/usr/bin/env python3
"""Run the registered macOS VCkss CMG_FULL_V2/MATLAB comparison."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import platform
import re
import shutil
import signal
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
INPUT_SHA256 = "19744b8418ffff82461527d7426cdc976dce18bf36597555d19bf847dec5afbe"
CMG_COMMIT = "dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10"
PRIVATE_WINNER_COMMIT = "598a08d5c0792519b3d87d6f56f743cacbf93a24"
PRIVATE_WINNER_SECONDS = 81.145
MATLAB_WALL_LIMIT_SECONDS = 1_800
RUN_ORDERS = (
    ("vckss", "matlab"),
    ("matlab", "vckss"),
    ("vckss", "matlab"),
    ("matlab", "vckss"),
    ("vckss", "matlab"),
    ("matlab", "vckss"),
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
        ["git", *arguments], cwd=repo, check=True, text=True, capture_output=True
    ).stdout.strip()


def archive_package(repo: Path, commit: str, destination: Path) -> None:
    archive_path = destination.with_suffix(".tar")
    destination.mkdir()
    with archive_path.open("wb") as handle:
        subprocess.run(
            ["git", "archive", commit, "vckss"],
            cwd=repo,
            check=True,
            stdout=handle,
        )
    with tarfile.open(archive_path, "r:") as archive:
        archive.extractall(destination, filter="data")
    archive_path.unlink()


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
    logs = [path for path in directory.glob("*.log") if path not in (console, destination)]
    require(len(logs) == 1, f"expected one Stata batch log in {directory}")
    destination.write_bytes(logs[0].read_bytes() + console.read_bytes())
    logs[0].unlink()
    console.unlink()


def process_snapshot() -> dict[int, tuple[int, int]]:
    completed = subprocess.run(
        ["ps", "-axo", "pid=,ppid=,rss="], check=True, text=True, capture_output=True
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


def monitor_matlab(
    process: subprocess.Popen[bytes], identity_path: Path, receipt_path: Path
) -> dict[str, object]:
    started = time.monotonic()
    peak_rss_kib = 0
    peak_processes = 0
    samples = 0
    identity_observations = 0
    identity: dict[str, object] | None = None
    while process.poll() is None:
        if time.monotonic() - started > MATLAB_WALL_LIMIT_SECONDS:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=15)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=15)
            raise RuntimeError("MATLAB exceeded the registered wall limit")
        snapshot = process_snapshot()
        selected = descendants(snapshot, process.pid)
        if selected:
            samples += 1
            peak_rss_kib = max(peak_rss_kib, sum(snapshot[pid][1] for pid in selected))
            peak_processes = max(peak_processes, len(selected))
        if identity is None and identity_path.is_file():
            identity = json.loads(identity_path.read_text(encoding="utf-8"))
        if identity is not None:
            named = {int(identity["client_pid"]), *(int(pid) for pid in identity["worker_pids"])}
            if named.issubset(selected):
                identity_observations += 1
        time.sleep(0.25)
    require(process.returncode == 0, f"MATLAB exited {process.returncode}")
    require(samples > 0 and peak_rss_kib > 0, "MATLAB process monitor saw no RSS")
    require(identity is not None and identity_observations > 0, "MATLAB worker identity missing")
    receipt = {
        "schema": "VCKSS-FULL-CMG-PRODUCTION-MATLAB-PROCESS-V1",
        "status": "PASS",
        "root_pid": process.pid,
        "samples": samples,
        "peak_rss_bytes": peak_rss_kib * 1024,
        "peak_processes": peak_processes,
        "identity_observations": identity_observations,
        "identity_sha256": sha256(identity_path),
    }
    receipt_path.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    return receipt


def build_plugin(repo: Path, build_dir: Path, source_commit: str) -> tuple[Path, dict[str, object]]:
    build_dir.mkdir()
    cargo = subprocess.run(
        ["rustup", "which", "--toolchain", "1.85.1", "cargo"],
        check=True,
        text=True,
        capture_output=True,
    ).stdout.strip()
    rustc = subprocess.run(
        ["rustup", "which", "--toolchain", "1.85.1", "rustc"],
        check=True,
        text=True,
        capture_output=True,
    ).stdout.strip()
    rustc_version = subprocess.run(
        [rustc, "--version", "--verbose"], check=True, text=True, capture_output=True
    ).stdout
    require("release: 1.85.1" in rustc_version, "Rust 1.85.1 is not active")
    environment = os.environ.copy()
    environment["PATH"] = f"{Path(rustc).parent}:{environment['PATH']}"
    environment["RUSTC"] = rustc
    environment["CARGO_TARGET_DIR"] = str(build_dir / "target")
    log = build_dir / "build.log"
    with log.open("wb") as handle:
        completed = subprocess.run(
            [
                cargo,
                "build",
                "--manifest-path",
                str(repo / "rust/stata_backend/Cargo.toml"),
                "--locked",
                "--release",
            ],
            cwd=repo,
            env=environment,
            stdout=handle,
            stderr=subprocess.STDOUT,
            check=False,
        )
    require(completed.returncode == 0, "normal production plugin build failed")
    built = build_dir / "target/release/libvckss_stata.dylib"
    require(built.is_file(), "normal production plugin artifact is missing")
    plugin = build_dir / "vckss_rust_macos_arm64.plugin"
    shutil.copy2(built, plugin)
    receipt: dict[str, object] = {
        "schema": "VCKSS-FULL-CMG-PRODUCTION-BUILD-V1",
        "status": "PASS",
        "source_commit": source_commit,
        "rustc": rustc_version.strip(),
        "cargo": cargo,
        "cmg_commit": CMG_COMMIT,
        "plugin_sha256": sha256(plugin),
        "build_log_sha256": sha256(log),
    }
    (build_dir / "receipt.json").write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return plugin, receipt


def validate_vckss(row: dict[str, str], source_commit: str, task_sha: str) -> None:
    require(row["schema"] == "VCKSS-FULL-CMG-PRODUCTION-STATA-V1", "Stata schema changed")
    require(row["application_status"] == "PASS", "VCkss application failed")
    require(row["source_commit"] == source_commit, "VCkss source binding failed")
    require(row["task_sha256"] == task_sha and row["input_sha256"] == INPUT_SHA256,
            "VCkss task/input binding failed")
    require(row["cmg_backend"] == "CMG_FULL_V2", "production backend identity changed")
    require(row["cmg_source_commit"] == CMG_COMMIT, "vendored CMG identity changed")
    require(int(float(row["processors"])) == THREADS, "VCkss processor count changed")
    require(int(float(row["cmg_rhs_count"])) == 1 + 3 * PROBES, "CMG RHS count changed")
    require(float(row["cmg_max_complete_residual"]) <= float(row["residual_acceptance"]),
            "complete residual gate failed")
    require(float(row["cmg_admitted_peak_bytes"]) <= float(row["cmg_pre_rng_forecast_bytes"]),
            "CMG memory receipt did not reconcile")
    for field in ("data_restored", "rng_restored", "sort_rng_restored"):
        require(int(float(row[field])) == 1, f"VCkss failed {field}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--input-csv", type=Path, required=True)
    parser.add_argument("--matlab-root", type=Path, required=True)
    parser.add_argument(
        "--stata",
        type=Path,
        default=Path("/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp"),
    )
    parser.add_argument(
        "--matlab",
        type=Path,
        default=Path("/Applications/MATLAB_R2024b.app/bin/matlab"),
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo = Path(__file__).resolve().parents[3]
    require(not git(repo, "status", "--porcelain"), "benchmark worktree must be clean")
    source_commit = git(repo, "rev-parse", "HEAD^{commit}")
    require(sha256(args.input_csv) == INPUT_SHA256, "registered input hash changed")
    require(args.stata.is_file() and args.matlab.is_file(), "Stata or MATLAB executable missing")
    require(not args.output_dir.exists() or not any(args.output_dir.iterdir()),
            "output directory must be new or empty")
    args.output_dir.mkdir(parents=True, exist_ok=True)

    plugin, build_receipt = build_plugin(args.output_dir / "build", source_commit)
    package_root = args.output_dir / "source"
    archive_package(repo, source_commit, package_root)
    shutil.copy2(plugin, package_root / "vckss/vckss_rust_macos_arm64.plugin")

    matlab_runtime = args.output_dir / "matlab_runtime"
    matlab_runtime.mkdir()
    for relative in ("codes", "CMG"):
        shutil.copytree(
            args.matlab_root / relative,
            matlab_runtime / relative,
            ignore=shutil.ignore_patterns(".DS_Store"),
        )
    source_identity = args.output_dir / "matlab_source_identity.json"
    source_contract = repo / "vckss/benchmarks/matlab_scale/source_contract.json"
    subprocess.run(
        [
            os.sys.executable,
            str(repo / "vckss/benchmarks/scc/verify_numopt2_matlab_source.py"),
            "--matlab-root",
            str(matlab_runtime),
            "--contract",
            str(source_contract),
            "--contract-sha256",
            sha256(source_contract),
            "--output",
            str(source_identity),
        ],
        cwd=repo,
        check=True,
    )

    task = {
        "schema": "VCKSS-FULL-CMG-PRODUCTION-LOCAL-CASE-V1",
        "source_commit": source_commit,
        "plugin_sha256": build_receipt["plugin_sha256"],
        "input_sha256": INPUT_SHA256,
        "matlab_source_identity_sha256": sha256(source_identity),
        "cmg_commit": CMG_COMMIT,
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
    task_path.write_text(json.dumps(task, indent=2, sort_keys=True) + "\n")
    task_sha = sha256(task_path)

    rows: dict[str, list[dict[str, object]]] = {"vckss": [], "matlab": []}
    evidence: list[dict[str, object]] = []
    driver_dir = Path(__file__).resolve().parent
    matlab_driver = repo / "vckss/benchmarks/paper_matlab_scaling"
    for run_index, order in enumerate(RUN_ORDERS):
        temperature = "cold" if run_index == 0 else "warm"
        for role in order:
            run_dir = args.output_dir / "runs" / f"{run_index:02d}-{temperature}" / role
            run_dir.mkdir(parents=True)
            log_path = run_dir / "application.log"
            resources = run_dir / "resources.txt"
            if role == "vckss":
                output_csv = run_dir / "stata.csv"
                command = [
                    "/usr/bin/time", "-l", "-o", str(resources), str(args.stata),
                    "-q", "-b", "do", str(driver_dir / "stata_run.do"),
                    str(package_root), str(args.input_csv), str(output_csv), role,
                    source_commit, task_sha, INPUT_SHA256, "strong_d6", "strong",
                    str(ROWS), str(DEGREE), str(PROBES), str(SEED),
                ]
                console = run_dir / "console.log"
                with console.open("wb") as handle:
                    completed = subprocess.run(
                        command, cwd=run_dir, stdout=handle, stderr=subprocess.STDOUT, check=False
                    )
                collect_stata_log(run_dir, console, log_path)
                require(completed.returncode == 0, f"VCkss run {run_index} failed")
                require("VCKSS_FULL_CMG_PRODUCTION_STATA_PASS" in log_path.read_text(),
                        "VCkss PASS marker missing")
                row: dict[str, object] = dict(read_one_csv(output_csv))
                validate_vckss(row, source_commit, task_sha)
                row.update({
                    "run_index": run_index,
                    "temperature": temperature,
                    "launch_position": order.index(role) + 1,
                    "process_peak_rss_bytes": parse_peak_rss(resources),
                })
            else:
                output_dir = run_dir / "output"
                scratch_dir = run_dir / "scratch"
                output_dir.mkdir()
                scratch_dir.mkdir()
                identity_path = output_dir / "matlab_process_identity.json"
                environment = os.environ.copy()
                environment.update({
                    "PMS_OUTPUT_DIR": str(output_dir),
                    "PMS_SCRATCH_DIR": str(scratch_dir),
                    "PMS_INPUT_CSV": str(args.input_csv),
                    "PMS_INPUT_SHA256": INPUT_SHA256,
                    "PMS_MATLAB_ROOT": str(matlab_runtime),
                    "PMS_TASK_SHA256": task_sha,
                    "PMS_SOURCE_IDENTITY": str(source_identity),
                    "PMS_PROCESS_IDENTITY": str(identity_path),
                    "PMS_EXPERIMENT_ID": f"full_cmg_production_{run_index:02d}",
                    "PMS_SOURCE_COMMIT": source_commit,
                    "PMS_BUNDLE_SHA256": task_sha,
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
                    "-nodisplay", "-nosplash", "-nodesktop", "-batch",
                    f"addpath('{matlab_driver}'); paper_matlab_scaling_run",
                ]
                with log_path.open("wb") as handle:
                    process = subprocess.Popen(
                        command,
                        cwd=run_dir,
                        env=environment,
                        stdin=subprocess.DEVNULL,
                        stdout=handle,
                        stderr=subprocess.STDOUT,
                        start_new_session=True,
                    )
                    process_receipt = monitor_matlab(
                        process, identity_path, run_dir / "process_tree.json"
                    )
                require("PAPER MATLAB SCALING MATLAB PASS" in log_path.read_text(),
                        "MATLAB PASS marker missing")
                row = json.loads((output_dir / "matlab_aggregate.json").read_text())
                require(row["status"] == "PASS" and row["input_sha256"] == INPUT_SHA256,
                        "MATLAB aggregate binding failed")
                row.update({
                    "run_index": run_index,
                    "temperature": temperature,
                    "launch_position": order.index(role) + 1,
                    "process_peak_rss_bytes": process_receipt["peak_rss_bytes"],
                })
            rows[role].append(row)
            evidence.append({
                "role": role,
                "run_index": run_index,
                "temperature": temperature,
                "log": str(log_path.relative_to(args.output_dir)),
                "log_sha256": sha256(log_path),
                "resources": str(resources.relative_to(args.output_dir)),
                "resources_sha256": sha256(resources),
            })
            print(
                f"run={run_index} role={role} command_seconds={float(row['command_seconds']):.3f}",
                flush=True,
            )

    medians = {
        role: statistics.median(float(row["command_seconds"]) for row in values[1:])
        for role, values in rows.items()
    }
    rss_medians = {
        role: statistics.median(float(row["process_peak_rss_bytes"]) for row in values[1:])
        for role, values in rows.items()
    }
    vckss_reference = rows["vckss"][0]
    matlab_reference = rows["matlab"][0]
    target_pairs = (
        ("worker", "corrected1", "corrected_worker"),
        ("firm", "corrected2", "corrected_firm"),
        ("covariance", "corrected3", "corrected_covariance"),
        ("total", "corrected4", "corrected_total"),
    )
    target_differences = []
    for name, vckss_field, matlab_field in target_pairs:
        vckss_value = float(vckss_reference[vckss_field])
        matlab_value = float(matlab_reference[matlab_field])
        difference = abs(vckss_value - matlab_value)
        descriptive_limit = 1.0e-3 * max(1.0, abs(vckss_value), abs(matlab_value))
        target_differences.append({
            "target": name,
            "vckss": vckss_value,
            "matlab": matlab_value,
            "absolute_difference": difference,
            "descriptive_scale_limit": descriptive_limit,
            "descriptive_scale_check": "PASS" if difference <= descriptive_limit else "FAIL",
        })
    require(all(item["descriptive_scale_check"] == "PASS" for item in target_differences),
            "VCkss and MATLAB corrected targets are not statistically comparable")
    production_over_matlab = medians["vckss"] / medians["matlab"]
    production_over_private = medians["vckss"] / PRIVATE_WINNER_SECONDS
    summary = {
        "schema": "VCKSS-FULL-CMG-PRODUCTION-LOCAL-V1",
        "status": "PASS",
        "generated_at_utc": datetime.now(UTC).isoformat(),
        "source_commit": source_commit,
        "task_sha256": task_sha,
        "input_sha256": INPUT_SHA256,
        "cmg_commit": CMG_COMMIT,
        "private_winner_commit": PRIVATE_WINNER_COMMIT,
        "private_winner_seconds": PRIVATE_WINNER_SECONDS,
        "host": socket.gethostname(),
        "platform": platform.platform(),
        "stata_executable": str(args.stata),
        "matlab_executable": str(args.matlab),
        "matlab_source_root": str(args.matlab_root),
        "build_receipt": build_receipt,
        "warm_median_command_seconds": medians,
        "warm_median_process_peak_rss_bytes": rss_medians,
        "production_over_matlab": production_over_matlab,
        "production_over_private_winner": production_over_private,
        "faster_than_matlab_gate": production_over_matlab < 1.0,
        "within_five_percent_private_winner_gate": production_over_private <= 1.05,
        "two_x_matlab_objective": production_over_matlab <= 0.5,
        "matlab_target_comparison": "DESCRIPTIVE_SCALE_CHECK_ONLY_INDEPENDENT_RNG_AND_SOLVER",
        "target_differences": target_differences,
        "rows": rows,
        "evidence": evidence,
    }
    require(summary["faster_than_matlab_gate"], "production VCkss is not faster than MATLAB")
    require(summary["within_five_percent_private_winner_gate"],
            "production VCkss regressed more than 5% from the private winner")
    (args.output_dir / "summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(
        "VCKSS_FULL_CMG_PRODUCTION_LOCAL_PASS "
        f"production_over_matlab={production_over_matlab:.6f} "
        f"production_over_private={production_over_private:.6f}",
        flush=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
