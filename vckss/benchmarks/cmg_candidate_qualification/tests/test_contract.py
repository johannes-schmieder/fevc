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
from validate_task import (  # noqa: E402
    parse_cpu_set,
    validate_cpu_block,
    validate_rust_phases,
)


def source(name: str) -> str:
    return (ROOT / name).read_text(encoding="utf-8")


def test_scheduler_contract_is_flexible_bound_and_unthrottled() -> None:
    wrapper = source("run_task.sge")
    submit = source("submit_scc.sh")
    for token in ("#$ -clear", "#$ -P welfgr", "#$ -pe omp 16", "#$ -binding linear:16",
                  "#$ -l h_rt=12:00:00", "taskset -c", "flock -n",
                  "assigned_cpu_affinity", "HARNESS_TASKSET_FLOCK_V1"):
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
    assert "monitor_rc != 0" in wrapper
    preparation = source("prepare_artifacts.sge")
    for token in ("stata_processor_capability.tsv",
                  "VCS_REQUIRED_STATA_PROCESSORS", "c(processors_lic)",
                  "build_benchmark_ado.py"):
        assert token in preparation or token in (
            ROOT.parent / "stata_processor_capability.do").read_text(
                encoding="utf-8")


def test_every_scc_python_entry_point_pins_supported_runtime() -> None:
    for name in (
        "submit_scc.sh",
        "collect_preparation_qacct.sh",
        "collect_qacct.sh",
        "run_task.sge",
    ):
        script = source(name)
        assert "python3/3.12.4" in script
        assert "module load" in script
        assert 'Python 3.12.4' in script
        assert "command -v python3" in script


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


def test_effective_sge_validator_accepts_site_pe_alias_and_source_binding(
    tmp_path: Path,
) -> None:
    from verify_sge_submission import validate

    wrapper = tmp_path / "run_task.sge"
    wrapper.write_text("#!/bin/bash -l\n#$ -binding linear:16\n", encoding="utf-8")
    qstat = tmp_path / "qstat.txt"
    qstat.write_text(
        "hard resource_list: no_gpu=TRUE,h_rt=43200,mem_per_core=8G\n"
        "soft resource_list: buyin=TRUE\n"
        "parallel environment: omp16 range: 16\n"
        f"script_file: {wrapper}\n",
        encoding="utf-8",
    )

    receipt = validate(qstat, 16, True, wrapper)

    assert receipt["parallel_environment_resolution"] == "SCC_SLOT_SPECIFIC_ALIAS"
    assert receipt["binding_request"] == "linear:16"
    assert receipt["binding_evidence"] == "SOURCE_DIRECTIVE_RUNTIME_AFFINITY_REQUIRED"
    assert receipt["runtime_affinity_gate_required"] is True


def test_cpu_set_parser_covers_compact_scc_affinity() -> None:
    assert parse_cpu_set("0-7,16-23") == set(range(8)) | set(range(16, 24))
    with pytest.raises(EvidenceError, match="affinity range"):
        parse_cpu_set("8-3")


def test_runtime_block_accepts_second_half_of_unrestricted_32_core_host() -> None:
    scheduler, assigned, index, capacity = validate_cpu_block({
        "scheduler_cpu_affinity": "0-31",
        "assigned_cpu_affinity": "16-31",
        "cpu_block_index": "1",
        "cpu_block_capacity": "2",
        "binding_enforcement": "HARNESS_TASKSET_FLOCK_V1",
    })
    assert scheduler == set(range(32))
    assert assigned == set(range(16, 32))
    assert (index, capacity) == (1, 2)


def test_runtime_block_rejects_more_than_16_assigned_cpus() -> None:
    with pytest.raises(EvidenceError, match="runtime CPU block"):
        validate_cpu_block({
            "scheduler_cpu_affinity": "0-31",
            "assigned_cpu_affinity": "0-31",
            "cpu_block_index": "0",
            "cpu_block_capacity": "2",
            "binding_enforcement": "HARNESS_TASKSET_FLOCK_V1",
        })


def test_source_and_scientific_contract_is_explicit() -> None:
    driver = source("stata_run.do")
    validator = source("validate_task.py")
    contract = source("common.py")
    for token in ("CMG_FULL_V2", CANDIDATE_CMG_COMMIT, COMPARISON_CMG_COMMIT,
                  "complete_residual_max", "e(sample)", "rng_restored",
                  "sort_rng_restored", "VCKSS_BENCHMARK_RUST_THREADS",
                  "cmg_threads_used", "cmg[1,42]",
                  "resource_peak'==`memory_forecast"):
        assert token in driver or token in validator or token in contract
    for token in ("python_module", "Python 3.12.4", "/share/pkg.8/python3"):
        assert token in validator
    for token in ("VCKSS-COMPARATIVE-SCALING-INPUT-V5",
                  "six_hub_leaf_panel_vector_v1",
                  "weak_hub_firms", "weak_leaf_firms",
                  "weak_panel_layers", "weak_hub_leaf_edges",
                  "weak_hub_tree_edges", "weak_canonical_edges",
                  "131_072", "786_437"):
        assert token in validator


def test_native_rust_phase_profile_replaces_inapplicable_legacy_scalars() -> None:
    driver = source("stata_run.do")
    validator = source("validate_task.py")
    for token in (
        "VCKSS-CMG-CANDIDATE-QUALIFICATION-STATA-V3",
        "e(rust_phase_profile)",
        "VCKSS-NATIVE-PHASE-PERF-V1",
        "rust_native_total_seconds",
    ):
        assert token in driver or token in validator
    assert "generate double selection_seconds" not in driver
    phases = validate_rust_phases({
        "rust_ingest_seconds": "0.1",
        "rust_canonicalize_seconds": "0.2",
        "rust_graph_seconds": "0.3",
        "rust_compress_seconds": "0.4",
        "rust_plan_seconds": "0.5",
        "rust_stayer_augmentation_seconds": "0.6",
        "rust_solve_seconds": "1.8",
        "rust_native_total_seconds": "2.0",
    }, "candidate")
    assert phases["rust_native_total_seconds"] == 2.0


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
                             "stata_processors": str(min(cores, 4)),
                             "rust_threads": str(cores),
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
