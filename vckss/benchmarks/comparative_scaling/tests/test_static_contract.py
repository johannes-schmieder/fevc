from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def source(name: str) -> str:
    return (ROOT / name).read_text(encoding="utf-8")


def test_sge_resource_and_paired_host_contract() -> None:
    wrapper = source("run_task.sge")
    for token in (
        "#$ -clear", "#$ -P welfgr", "#$ -pe omp 16", "#$ -binding linear:16",
        "taskset -c", "VCKSS_COMPARATIVE_SCALING_TASK_CAPTURED",
        "VCKSS_BENCHMARK_RUST_THREADS", "mata_cpu_list", "flock -n",
        "assigned_cpu_affinity", "HARNESS_TASKSET_FLOCK_V1",
    ):
        assert token in wrapper
    for token in ("#$ -q econ", "cpu_type=Gold-6242", "exclusive=TRUE"):
        assert token not in wrapper
        assert token not in source("prepare_artifacts.sge")
    assert "#$ -clear" in source("prepare_artifacts.sge")
    submit = source("submit_scc.sh")
    assert "-t 1-300 -l" in submit
    assert "-tc" not in submit
    assert "array_concurrency\\tSCHEDULER_MANAGED" in submit
    assert "client_task_throttle\\tNONE" in submit
    assert "retry TASK_IDS ATTEMPT_ID" in submit
    assert "verify_pilots.py" in submit
    assert "verify_retry.py" in submit
    assert "PAIRED_WITHIN_TASK_HOST" in submit
    for token in ("qsub -terse -h", "verify_sge_submission.py", "qrls",
                  "buyin_requested_by_harness\\tFALSE",
                  "soft_buyin_injection\\tSCC_GLOBAL_JSV_MANDATORY"):
        assert token in submit
    verifier = source("verify_sge_submission.py")
    for token in ("SCC_SLOT_SPECIFIC_ALIAS", "binding_script_sha256",
                  "SOURCE_DIRECTIVE_RUNTIME_AFFINITY_REQUIRED"):
        assert token in verifier
    for token in ("task_start_epoch", "task_end_epoch", "VCS_ATTEMPT_ID"):
        assert token in wrapper
    assert wrapper.count("monitor_rc != 0") == 2


def test_every_scc_python_entry_point_pins_supported_runtime() -> None:
    for name in (
        "submit_scc.sh",
        "collect_preparation_qacct.sh",
        "collect_qacct.sh",
        "prepare_artifacts.sge",
        "run_task.sge",
    ):
        script = source(name)
        assert "python3/3.12.4" in script
        assert "module load" in script
        assert 'Python 3.12.4' in script
        assert "command -v python3" in script


def test_strict_backend_and_numerical_contract() -> None:
    driver = source("stata_run.do")
    for token in (
        "backend(rust) rng(counter_v1)",
        "algorithm(jla) engine(auto) preconditioner(auto) batch(auto)",
        "backend(mata) rng(stata)",
        'e(cmg_backend)', "CMG_FULL_V2", "e(sample)",
        "complete_residual_max", "rng_restored", "sort_rng_restored",
        "peak_bytes'==`forecast_bytes", "peak_bytes'>=`cmg_admitted",
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
        "stata_processor_capability.tsv", "VCS_REQUIRED_STATA_PROCESSORS",
        "build_benchmark_ado.py", "benchmark_ado_adapter.json",
    ):
        assert token in driver
    capability = (ROOT.parent / "stata_processor_capability.do").read_text(
        encoding="utf-8")
    for token in ("c(processors_lic)", "required_processors",
                  "VCKSS-STATA-PROCESSOR-CAPABILITY-V1"):
        assert token in capability


def test_collection_carries_compact_source_and_binary_provenance() -> None:
    aggregator = source("aggregate.py")
    for token in (
        "task_manifest_300.tsv", "source.files.sha256",
        "binary_manifest.sha256", "matlab_source_identity.json",
        "preparation_qacct.txt", "input_hashes_20.tsv", "runtime_identity",
        "cpu_model_strata.tsv", "overlap_sensitivity.tsv",
    ):
        assert token in aggregator
    for token in ("python_module", "python_executable", "python_version"):
        assert token in aggregator
        assert token in source("validate_task.py")


def test_pilots_are_distinct_source_and_binary_bound_run_gates() -> None:
    builder = source("build_run.py")
    collector = source("collect_qacct.sh")
    verifier = source("verify_pilots.py")
    for token in ("pilot-small", "pilot-worst", "production"):
        assert token in builder
    assert "validate_pilot.py" in collector
    assert "qacct.pass.json" in source("submit_scc.sh")
    for token in ("source_manifest_sha256", "task_manifest_sha256",
                  "binary_manifest_sha256"):
        assert token in verifier
