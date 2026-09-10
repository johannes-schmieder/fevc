from __future__ import annotations

import csv
import json
from pathlib import Path

from fevc.benchmarks.five_way_scaling.aggregate import aggregate
from fevc.benchmarks.five_way_scaling.build_benchmark_ado import build
from fevc.benchmarks.five_way_scaling.build_manifest import build_rows
from fevc.benchmarks.five_way_scaling.common import ROLES, TASK_FIELDS, atomic_json
from fevc.benchmarks.five_way_scaling.exact_oracle import oracle
from fevc.benchmarks.five_way_scaling.generate_input import generate
from fevc.benchmarks.five_way_scaling.validate_task import validate


def test_exact_generator_validator_aggregate_path(tmp_path: Path) -> None:
    task_dir = tmp_path / "tasks" / "task-001"
    task_dir.mkdir(parents=True)
    task = build_rows("exact")[0]
    atomic_json(task_dir / "task.json", task)
    generate(tmp_path / "input.csv", tmp_path / "input.json", 960)
    exact = oracle(tmp_path / "input.csv")
    atomic_json(task_dir / "oracle.json", exact)
    for role in ROLES:
        role_dir = task_dir / role
        role_dir.mkdir()
        factor = (959 / 960) if role in {"matlab", "julia", "r"} else 1.0
        result = {"schema":"FEVC-FIVE-WAY-ROLE-V1","status":"PASS","role":role,
                  "algorithm":"exact","rows":960,"cores":1,"probes":0,"seed":202609091,
                  "primary_seconds":1.0,"estimator_seconds":0.9,"normalization_factor":factor,
                  "retained_rows":960}
        for target, value in exact["targets"].items():
            result[f"normalized_{target}"] = value
            result[f"raw_{target}"] = value / factor
        if role == "fevc":
            with (role_dir / "result.csv").open("w", encoding="utf-8", newline="") as handle:
                writer = csv.DictWriter(handle, fieldnames=list(result)); writer.writeheader(); writer.writerow(result)
        else:
            atomic_json(role_dir / "result.json", result)
        atomic_json(role_dir / "status.json", {"status":"PASS","role":role})
        atomic_json(role_dir / "process_tree.json", {"status":"PASS","phase_sample_count":2,
                    "phase_peak_rss_kib":2048,"whole_peak_rss_kib":3072})
        (role_dir / "resources.txt").write_text(
            "Elapsed (wall clock) time (h:mm:ss or m:ss): 0:01.20\n", encoding="utf-8")
    value = validate(task_dir)
    atomic_json(task_dir / "validation.json", value)
    assert len(value["exact_checks"]) == 20
    receipt = aggregate(tmp_path / "tasks", tmp_path / "aggregate", 1)
    assert receipt["estimator_calls"] == 5


def test_benchmark_ado_adapter_is_source_bound(tmp_path: Path) -> None:
    source = Path(__file__).parents[4] / "fevc" / "fevc.ado"
    value = build(source, tmp_path / "fevc.ado", tmp_path / "receipt.json")
    assert value["allowed_native_threads"] == [1, 2, 4, 8, 14, 28]
    assert value["transformations"] == 4
