from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def source(name: str) -> str:
    return (ROOT / name).read_text(encoding="utf-8")


def test_sge_resource_and_isolation_contract() -> None:
    wrapper = source("run_task.sge")
    for token in (
        "#$ -P welfgr", "#$ -q econ", "#$ -pe omp 16",
        "#$ -binding linear:16", "#$ -l cpu_type=Gold-6242",
        "#$ -l exclusive=TRUE", "#$ -l mem_per_core=4G",
        "taskset -c", "VCKSS_COMPARATIVE_SCALING_TASK_CAPTURED",
    ):
        assert token in wrapper


def test_strict_backend_and_numerical_contract() -> None:
    driver = source("stata_run.do")
    for token in (
        "backend(rust) rng(counter_v1)",
        "algorithm(jla) engine(auto) preconditioner(auto) batch(auto)",
        "backend(mata) rng(stata)",
        'e(cmg_backend)', "CMG_FULL_V2", "e(sample)",
        "complete_residual_max", "rng_restored", "sort_rng_restored",
    ):
        assert token in driver
    assert "tolerance(" not in driver


def test_matlab_fresh_process_and_dynamic_pool_contract() -> None:
    driver = source("matlab_run.m")
    for token in (
        "gcp('nocreate')", "parpool(cluster,active_cores",
        "leave_out_KSS", "phase_start_file", "phase_end_file",
        "detail_lines==expected_rows", "maxNumCompThreads(1)",
        "MAINTAINED_UPSTREAM_INTERNAL",
        "APPLICATION_LOG_PCG_AND_OUTPUT_GATES",
    ):
        assert token in driver


def test_validation_preserves_matlab_numerical_rejections_and_phase_timers() -> None:
    validator = source("validate_task.py")
    aggregator = source("aggregate.py")
    for token in (
        "parse_matlab_pcg", "NUMERICAL_REJECTED", "cmg_solve_seconds",
        "selection_seconds", "matlab_pcg_relative_residual",
    ):
        assert token in validator or token in aggregator


def test_preparation_uses_normal_pinned_build() -> None:
    driver = source("prepare_artifacts.sge")
    for token in (
        "1.85.1-x86_64-unknown-linux-gnu", "--release --locked",
        "vckss_rust_linux_x64.plugin", "matlab/2024b",
        "binary_manifest.sha256", "verify_numopt2_matlab_source.py",
    ):
        assert token in driver


def test_collection_carries_compact_source_and_binary_provenance() -> None:
    aggregator = source("aggregate.py")
    for token in (
        "task_manifest_300.tsv", "source.files.sha256",
        "binary_manifest.sha256", "matlab_source_identity.json",
        "preparation_qacct.txt", "input_hashes_20.tsv", "runtime_identity",
    ):
        assert token in aggregator
