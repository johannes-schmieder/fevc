from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import pytest

from aggregate import summarize  # noqa: E402
from common import (  # noqa: E402
    CANDIDATE_CMG_COMMIT,
    COMPARISON_CMG_COMMIT,
    EvidenceError,
    RESULT_SCHEMA,
    STRUCTURES,
)


def source(name: str) -> str:
    return (ROOT / name).read_text(encoding="utf-8")


def test_scheduler_contract_is_flexible_bound_and_unthrottled() -> None:
    wrapper = source("run_task.sge")
    submit = source("submit_scc.sh")
    for token in ("#$ -clear", "#$ -P welfgr", "#$ -pe omp 16", "#$ -binding linear:16",
                  "#$ -l h_rt=12:00:00", "taskset -c"):
        assert token in wrapper
    assert "-t 1-72 -l" in submit
    assert "qacct.pass.json" in submit
    for token in ("-tc", "#$ -q", "cpu_type=", "exclusive=TRUE"):
        assert token not in wrapper
        assert token not in submit
    for token in ("qsub -terse -h", "verify_sge_submission.py", "qrls",
                  "buyin_requested_by_harness\\tFALSE",
                  "soft_buyin_injection\\tSCC_GLOBAL_JSV_MANDATORY"):
        assert token in submit


def test_effective_sge_validator_rejects_hard_restrictions(tmp_path: Path) -> None:
    from verify_sge_submission import SubmissionError, validate

    qstat = tmp_path / "qstat.txt"
    qstat.write_text(
        "hard resource_list: no_gpu=TRUE,h_rt=43200,mem_per_core=8G,buyin=TRUE\n"
        "soft resource_list: buyin=TRUE\n"
        "parallel environment: omp range: 16\n"
        "binding: linear:16\n",
        encoding="utf-8",
    )
    with pytest.raises(SubmissionError, match="hard resources"):
        validate(qstat, 16, True)


def test_source_and_scientific_contract_is_explicit() -> None:
    driver = source("stata_run.do")
    validator = source("validate_task.py")
    contract = source("common.py")
    for token in ("CMG_FULL_V2", CANDIDATE_CMG_COMMIT, COMPARISON_CMG_COMMIT,
                  "complete_residual_max", "e(sample)", "rng_restored",
                  "sort_rng_restored"):
        assert token in driver or token in validator or token in contract


def payloads(ratio: float = 0.95) -> list[dict]:
    values = []
    task_id = 0
    for structure in STRUCTURES:
        for cores in (1, 8, 16):
            for replicate in range(1, 7):
                task_id += 1
                candidate_time = 100.0 * ratio
                comparison_time = 100.0
                vector = cores > 1
                roles = {
                    "candidate": {"command_seconds": candidate_time,
                                  "estimator_phase_peak_rss_bytes": 1000,
                                  "cmg_plan_bytes": 0,
                                  "cmg_planned_batches": 2 if vector else 0,
                                  "cmg_serial_batches": 1},
                    "comparison": {"command_seconds": comparison_time,
                                   "estimator_phase_peak_rss_bytes": 1000,
                                   "cmg_plan_bytes": 0,
                                   "cmg_planned_batches": 0,
                                   "cmg_serial_batches": 3},
                }
                values.append({
                    "schema": RESULT_SCHEMA, "status": "PASS",
                    "task": {"task_id": str(task_id),
                             "experiment_id": f"task-{task_id}",
                             "structure": structure, "active_cores": str(cores),
                             "replicate": str(replicate),
                             "execution_order": ("comparison,candidate" if replicate % 2 else
                                                 "candidate,comparison"),
                             "candidate_commit": "1" * 40,
                             "comparison_commit": "2" * 40,
                             "candidate_bundle_sha256": "a" * 64,
                             "comparison_bundle_sha256": "b" * 64},
                    "node": {"hostname": "host", "cpu_model": "test CPU",
                             "candidate_binary_manifest_sha256": "c" * 64,
                             "comparison_binary_manifest_sha256": "d" * 64},
                    "qacct": {"failed": "0", "exit_status": "0"},
                    "roles": roles, "statistical_gate": {"status": "PASS"},
                    "connected_vector_only": vector,
                })
    return values


def test_registered_acceptance_gate_passes_complete_improvement() -> None:
    tasks, cells, receipt = summarize(payloads())
    assert len(tasks) == 72
    assert len(cells) == 12
    assert receipt["status"] == "PASS"
    assert receipt["connected_vector_only_cells"] == {"8": 4, "16": 4}


def test_registered_acceptance_gate_rejects_parallel_regression() -> None:
    values = payloads()
    for value in values:
        if int(value["task"]["active_cores"]) > 1:
            value["roles"]["candidate"]["command_seconds"] = 101.0
    with pytest.raises(EvidenceError, match="parallel geometric-mean"):
        summarize(values)
