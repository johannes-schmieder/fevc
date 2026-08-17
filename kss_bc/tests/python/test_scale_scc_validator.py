from __future__ import annotations

import csv
import importlib.util
import os
import re
import subprocess
from pathlib import Path

import pytest

KSS_ROOT = Path(__file__).resolve().parents[2]
VALIDATOR_PATH = KSS_ROOT / "benchmarks/scc/validate_kss_scale.py"
SPEC = importlib.util.spec_from_file_location("validate_kss_scale", VALIDATOR_PATH)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

SOURCE = "4" * 40
BUNDLE = "b" * 64
INPUT = "d" * 64
EXPERIMENT = "well_connected_1x_p2"
FREQUENCY_VAR = "-"
TARGET_VAR = "-"
DELETION_VAR = "-"
DELETION_MODE = "match"


def write_csv(path: Path, rows: list[dict[str, object]]) -> None:
    assert rows
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def write_kv(path: Path, values: dict[str, object]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(["key", "value"])
        writer.writerows(values.items())


def fixture(tmp_path: Path) -> dict[str, Path | str | int]:
    output = tmp_path / "output"
    output.mkdir()
    fixed_overhead = 4 * MODULE.GIB
    reservation = output / "reservation.tsv"
    write_kv(reservation, {
        "experiment_id": EXPERIMENT,
        "source_commit": SOURCE,
        "bundle_sha256": BUNDLE,
        "input_sha256": INPUT,
        "option_contract": MODULE.OPTION_CONTRACT,
        "frequency_var": FREQUENCY_VAR,
        "target_var": TARGET_VAR,
        "deletion_var": DELETION_VAR,
        "deletion_mode": DELETION_MODE,
        "requested_slots": 14,
        "mem_per_core_gib": 4,
        "total_reserved_gib": 56,
        "stata_processors": 4,
        "hard_wall_seconds": 43200,
        "estimator_hard_wall_seconds": 43080,
        "fixture": "well_connected",
        "scale_factor": 1,
        "prior_admission_receipt": "-",
        "prior_admission_sha256": "-",
        "prior_experiment_id": "-",
        "prior_scale_factor": "-",
    })
    job_id = output / "job_id.txt"
    job_id.write_text("7199001\n", encoding="utf-8")
    qacct = output / "qacct.txt"
    qacct.write_text(
        "==============================================================\n"
        "qname econ-pub.q\n"
        "hostname scc-test.scc.bu.edu\n"
        "project welfgr\n"
        "jobnumber 7199001\n"
        "taskid undefined\n"
        "granted_pe omp\n"
        "slots 14\n"
        "failed 0\n"
        "exit_status 0\n"
        "ru_wallclock 120\n"
        "ru_maxrss 8388608\n"
        "cpu 301.5\n"
        "maxvmem 8.25G\n",
        encoding="utf-8",
    )
    submissions = tmp_path / "submissions"
    submissions.mkdir()
    scheduler_request = submissions / f"{EXPERIMENT}.scheduler_request.txt"
    remote_run = "/projectnb/welfgr/kss-bc/runs/test"
    remote_bundle = f"/projectnb/welfgr/kss-bc/bundles/{BUNDLE}"
    remote_source = f"{remote_bundle}/source"
    scheduler_environment = {
        "PATH": "/usr/local/bin:/usr/bin",
        "KSS_RUN_DIR": remote_run,
        "KSS_SOURCE_DIR": remote_source,
        "KSS_SOURCE_COMMIT": SOURCE,
        "KSS_BUNDLE_ARCHIVE":
            f"{remote_bundle}/{BUNDLE}.tar.gz",
        "KSS_BUNDLE_SHA256": BUNDLE,
        "KSS_SOURCE_MANIFEST":
            f"{remote_bundle}/{BUNDLE}.files.sha256",
        "KSS_INPUT_DATASET": "/projectnb/welfgr/private/input.dta",
        "KSS_INPUT_SHA256": INPUT,
        "KSS_EXPERIMENT_ID": EXPERIMENT,
        "KSS_FIXTURE": "well_connected",
        "KSS_SCALE_FACTOR": "1",
        "KSS_PROBES": "2",
        "KSS_SEED": "8675309",
        "KSS_BATCH": "auto",
        "KSS_FREQUENCY_VAR": FREQUENCY_VAR,
        "KSS_TARGET_VAR": TARGET_VAR,
        "KSS_DELETION_VAR": DELETION_VAR,
        "KSS_REQUESTED_SLOTS": "14",
        "KSS_STATA_PROCESSORS": "4",
        "KSS_MEMORY_GIB": "56",
        "KSS_HARD_WALL_SECONDS": "43200",
        "KSS_OUTPUT_DIR": f"{remote_run}/experiments/{EXPERIMENT}",
        "KSS_PRIOR_ADMISSION_RECEIPT": "-",
        "KSS_PRIOR_ADMISSION_SHA256": "-",
        "KSS_PRIOR_EXPERIMENT_ID": "-",
    }
    env_list = ",".join(
        f"{key}={value}" for key, value in scheduler_environment.items())
    scheduler_request.write_text(
        f"cwd: {remote_source}\n"
        "hard resource_list: mem_per_core=4G,h_rt=12:00:00\n"
        "parallel environment:  omp range: 14\n"
        "project:                    welfgr\n"
        f"stdout_path_list: NONE:NONE:{remote_run}/logs/"
        f"{EXPERIMENT}.stdout.txt\n"
        f"env_list: {env_list}\n"
        f"script_file: {remote_source}/kss_bc/benchmarks/scc/"
        "run_kss_scale.sge\n",
        encoding="utf-8",
    )
    node_receipt = output / "node_receipt.tsv"
    write_kv(node_receipt, {
        "receipt_version": MODULE.NODE_RECEIPT_VERSION,
        "experiment_id": EXPERIMENT,
        "job_id": "7199001",
        "hostname": "scc-test",
        "requested_slots": 14,
        "actual_slots": 14,
        "requested_stata_processors": 4,
        "mem_per_core_gib": 4,
        "reserved_memory_gib": 56,
        "scheduler_hard_wall_seconds": 43200,
        "estimator_hard_wall_seconds": 43080,
        "statatmp": "/tmp/kss-scale-7199001",
        "source_commit": SOURCE,
        "bundle_sha256": BUNDLE,
        "input_sha256": INPUT,
        "fixture": "well_connected",
        "scale_factor": 1,
        "option_contract": MODULE.OPTION_CONTRACT,
        "frequency_var": FREQUENCY_VAR,
        "target_var": TARGET_VAR,
        "deletion_var": DELETION_VAR,
        "deletion_mode": DELETION_MODE,
        "tmp_required_bytes": 7000 + fixed_overhead,
        "tmp_available_bytes": 64 * MODULE.GIB,
        "prior_admission_receipt": "-",
        "prior_admission_sha256": "-",
        "prior_experiment_id": "-",
    })
    wrapper_pass = output / "wrapper.pass"
    wrapper_pass.write_text(
        f"KSS_SCALE_WRAPPER_PASS {EXPERIMENT} {BUNDLE} {SOURCE} {INPUT}\n",
        encoding="utf-8",
    )
    stata_pass = output / "stata.pass"
    stata_pass.write_text(
        f"KSS_SCALE_ESTIMATOR_PASS {EXPERIMENT} {BUNDLE} {SOURCE} {INPUT}\n",
        encoding="utf-8",
    )
    application = output / "application.log"
    application.write_text(
        f"KSS-SCALE ESTIMATOR PASS: {EXPERIMENT}\n", encoding="utf-8")
    wrapper_metrics = output / "wrapper_metrics.tsv"
    write_kv(wrapper_metrics, {
        "source_commit": SOURCE,
        "input_sha256": INPUT,
        "input_staging_seconds": 2,
        "stata_process_seconds": 100,
        "output_validation_seconds": 1,
        "total_wrapper_seconds": 105,
    })
    process_resources = output / "process_resources.txt"
    process_resources.write_text(
        'Command being timed: "stata-mp -q do kss_scale_driver.do"\n'
        "User time (seconds): 250.25\n"
        "System time (seconds): 5.25\n"
        "Elapsed (wall clock) time (h:mm:ss or m:ss): 1:40.00\n"
        "Maximum resident set size (kbytes): 8388608\n"
        "Exit status: 0\n",
        encoding="utf-8",
    )
    summary = output / "summary.csv"
    write_csv(summary, [{
        "experiment_id": EXPERIMENT,
        "fixture": "well_connected",
        "scale_factor": 1,
        "source_commit": SOURCE,
        "bundle_sha256": BUNDLE,
        "input_sha256": INPUT,
        "option_contract": MODULE.OPTION_CONTRACT,
        "frequency_var": FREQUENCY_VAR,
        "target_var": TARGET_VAR,
        "deletion_var": DELETION_VAR,
        "deletion_mode": DELETION_MODE,
        "requested_slots": 14,
        "actual_slots": 14,
        "mem_per_core_gib": 4,
        "requested_stata_processors": 4,
        "actual_stata_processors": 4,
        "stata_version": "19",
        "stata_flavor": "MP",
        "stata_mp": 1,
        "input_rows": 100,
        "requested_probes": 2,
        "seed": 8675309,
        "tolerance": 1e-10,
        "declared_memory_gib": 56,
        "batch_requested": "auto",
        "command_rc": 0,
        "load_seconds": 1,
        "fixture_construction_seconds": 0,
        "import_selection_seconds": 1,
        "command_seconds": 90,
        "rng_seconds": 5,
        "correction_seconds": 10,
        "timing_attribution_contract":
            "rng_and_correction_nested_nonadditive",
        "estimator_status": "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES",
        "engine_requested": "auto",
        "engine_selected": "compressed",
        "fallback_status": "NOT_NEEDED",
        "fastpath_status": "ELIGIBLE",
        "resource_status": "ADMITTED",
        "rng_contract": "KSS-MT64S-DOMAIN-CURSOR-V2-STATA18-19",
        "rng_implementation": "per_domain_stream_cursor",
        "rng_runtime": "19",
        "rng_master_seed": 8675309,
        "rng_leverage_domain": "leverage",
        "rng_target_domain": "target",
        "rng_leverage_probe_first": 1,
        "rng_leverage_probe_last": 2,
        "rng_target_probe_first": 1,
        "rng_target_probe_last": 2,
        "N_stored": 100,
        "N_retained": 96,
        "N_physical": 96,
        "worker_levels": 20,
        "firm_levels": 5,
        "deletion_units": 30,
        "coefficient_cells": 25,
        "target_strata": 25,
        "diagnostic_coefficient_cells": 25,
        "diagnostic_deletion_units": 30,
        "sample_semantics_valid": 1,
        "selected_batch": 8,
        "solver_iterations": 4,
        "solver_max_residual": 5e-10,
        "rhs_max_residual": 5e-10,
        "residual_acceptance_tolerance": 1e-9,
        "residual_normalization": "l2_rhs_or_absolute_zero_rhs",
        "residual_equation_contract": "original_rhs_worker_firm_v1",
        "quotient_convention": "full_firm_zero_sum",
        "grounding_convention":
            "last_firm_zero_after_quotient_with_grounded_equation_checked",
        "grounded_coordinate_handling": "included_in_full_residual",
        "plugin_worker": 1,
        "plugin_firm": 2,
        "plugin_covariance": 0.5,
        "plugin_total": 4,
        "correction_worker": 0.1,
        "correction_firm": -0.1,
        "correction_covariance": -0.1,
        "correction_total": -0.2,
        "corrected_worker": 0.9,
        "corrected_firm": 2.1,
        "corrected_covariance": 0.6,
        "corrected_total": 4.2,
        "lifecycle_method": "PRESERVE_DISK",
        "life_sample_restored": 1,
        "resource_selection_peak_bytes": 200,
        "resource_transition_peak_bytes": 300,
        "resource_numerical_peak_bytes": 610,
        "resource_restoration_peak_bytes": 220,
        "resource_peak_bytes": 610,
        "resource_peak_phase": "numerical",
        "resource_mem_headroom": 0.30,
        "resource_mem_admit_bytes": 793,
        "resource_hard_mem_bytes": 56 * 1024**3,
        "resource_wall_upper_seconds": 1000,
        "resource_wall_headroom": 0.50,
        "resource_wall_admit_seconds": 1500,
        "resource_hard_wall_seconds": 43080,
        "resource_raw_stata_bytes": 100,
        "resource_cell_bytes": 20,
        "resource_deletion_unit_bytes": 30,
        "resource_target_stratum_bytes": 10,
        "resource_cmg_hierarchy_bytes": 100,
        "resource_phase_scratch_bytes": 190,
        "resource_sort_compress_bytes": 20,
        "resource_solve_ahead_bytes": 20,
        "resource_output_cert_bytes": 30,
        "resource_preserve_bytes": 40,
        "resource_runtime_resident_bytes": 50,
    }])
    rhs = output / "rhs.csv"
    rows = [{
        "experiment_id": EXPERIMENT,
        "stage": 1,
        "batch_start": 0,
        "rhs": 1,
        "iterations": 4,
        "relative_residual": 5e-10,
        "converged": 1,
    }]
    for probe in (1, 2):
        rows.append({
            "experiment_id": EXPERIMENT,
            "stage": 4,
            "batch_start": probe,
            "rhs": probe,
            "iterations": 3,
            "relative_residual": 4e-10,
            "converged": 1,
        })
        for local_rhs in (1, 2):
            rows.append({
                "experiment_id": EXPERIMENT,
                "stage": 5,
                "batch_start": probe,
                "rhs": 2 * (probe - 1) + local_rhs,
                "iterations": 3,
                "relative_residual": 4e-10,
                "converged": 1,
            })
    write_csv(rhs, rows)
    stage_memory = output / "stage_memory.csv"
    stage_forecasts = (200, 300, 610, 220)
    stage_observed = (80, 160, 300, 140)
    write_csv(stage_memory, [{
        "stage": stage,
        "elapsed_seconds": 1 + index,
        "forecast_peak_bytes": stage_forecasts[index],
        "observed_allocation_bytes": stage_observed[index],
        "measurement_kind": "stata_allocated_endpoint_not_peak",
    } for index, stage in enumerate(MODULE.EXPECTED_STAGES)])
    phase_rss_samples = output / "phase_rss_samples.csv"
    phase_tokens = MODULE.PHASE_TOKENS
    write_csv(phase_rss_samples, [{
        "timestamp_utc_seconds": 1700000000 + index,
        "phase": phase,
        "rss_bytes": stage_observed[index],
    } for index, phase in enumerate(phase_tokens)])
    phase_rss_peaks = output / "phase_rss_peaks.csv"
    write_csv(phase_rss_peaks, [{
        "phase": phase,
        "peak_rss_bytes": stage_observed[index],
        "sample_count": 1,
        "first_timestamp_utc_seconds": 1700000000 + index,
        "last_timestamp_utc_seconds": 1700000000 + index,
    } for index, phase in enumerate(phase_tokens)])
    phase_rss_sampler = output / "phase_rss_sampler.tsv"
    write_kv(phase_rss_sampler, {
        "receipt_version": MODULE.RSS_SAMPLER_RECEIPT_VERSION,
        "poll_count": 4,
        "valid_sample_count": 4,
        "invalid_marker_read_count": 1,
        "zero_rss_read_count": 0,
        "sampler_interval_seconds": 0.25,
    })
    tmp_capacity = output / "tmp_capacity.tsv"
    write_kv(tmp_capacity, {
        "receipt_version": MODULE.TMP_CAPACITY_RECEIPT_VERSION,
        "source_commit": SOURCE,
        "bundle_sha256": BUNDLE,
        "input_sha256": INPUT,
        "fixture": "well_connected",
        "scale_factor": 1,
        "staged_input_bytes": 1000,
        "fixture_expansion_factor": 1,
        "expanded_caller_forecast_bytes": 2000,
        "preserve_spool_forecast_bytes": 2000,
        "sort_temp_forecast_bytes": 2000,
        "fixed_output_temp_overhead_bytes": fixed_overhead,
        "required_bytes": 7000 + fixed_overhead,
        "available_bytes": 64 * MODULE.GIB,
        "status": "ADMITTED",
    })
    return {
        "summary": summary,
        "rhs": rhs,
        "stage_memory": stage_memory,
        "phase_rss_samples": phase_rss_samples,
        "phase_rss_peaks": phase_rss_peaks,
        "phase_rss_sampler": phase_rss_sampler,
        "tmp_capacity": tmp_capacity,
        "qacct": qacct,
        "job_id_file": job_id,
        "scheduler_request": scheduler_request,
        "node_receipt": node_receipt,
        "wrapper_pass": wrapper_pass,
        "stata_pass": stata_pass,
        "application_log": application,
        "reservation_path": reservation,
        "wrapper_metrics": wrapper_metrics,
        "process_resources": process_resources,
        "experiment_id": EXPERIMENT,
        "source_commit": SOURCE,
        "bundle_sha": BUNDLE,
        "input_sha": INPUT,
        "probes": 2,
    }


def test_three_layer_validator_accepts_14_slots_and_four_stata_processors(
    tmp_path: Path,
) -> None:
    report = MODULE.validate_run(**fixture(tmp_path))
    assert report["status"] == "KSS_SCALE_VALIDATION_PASS"
    assert report["scheduler"]["slots"] == 14
    assert report["output"]["engine"] == "compressed"
    assert report["options"] == {
        "option_contract": MODULE.OPTION_CONTRACT,
        "frequency_var": "-",
        "target_var": "-",
        "deletion_var": "-",
        "deletion_mode": "match",
    }
    assert report["output"]["input_preparation_timing"] == {
        "load_seconds": 1,
        "fixture_construction_seconds": 0,
        "sample_selection_seconds_inferred": 0,
        "import_selection_seconds": 1,
    }
    assert report["output"]["rhs"]["count"] == 7
    assert report["output"]["resource_reconciliation"]["status"] == \
        "FORECAST_UNDERESTIMATED"
    assert not report["output"]["resource_reconciliation"][
        "next_scale_allowed"]


def test_scheduler_failure_is_not_hidden_by_application_success(
    tmp_path: Path,
) -> None:
    files = fixture(tmp_path)
    qacct = files["qacct"]
    assert isinstance(qacct, Path)
    qacct.write_text(qacct.read_text(encoding="utf-8").replace(
        "exit_status 0", "exit_status 137"), encoding="utf-8")
    with pytest.raises(ValueError, match="exit_status"):
        MODULE.validate_run(**files)


def test_consistent_8x7_resource_rewrite_is_not_self_validating(
    tmp_path: Path,
) -> None:
    files = fixture(tmp_path)
    reservation = files["reservation_path"]
    node = files["node_receipt"]
    summary = files["summary"]
    qacct = files["qacct"]
    scheduler_request = files["scheduler_request"]
    assert all(isinstance(path, Path)
               for path in (
                   reservation, node, summary, qacct, scheduler_request))
    assert isinstance(reservation, Path)
    assert isinstance(node, Path)
    assert isinstance(summary, Path)
    assert isinstance(qacct, Path)
    assert isinstance(scheduler_request, Path)

    reservation_values = MODULE.read_key_values(reservation, "reservation")
    reservation_values.update({
        "requested_slots": "8",
        "mem_per_core_gib": "7",
        "total_reserved_gib": "56",
    })
    write_kv(reservation, reservation_values)
    node_values = MODULE.read_key_values(node, "node")
    node_values.update({
        "requested_slots": "8",
        "actual_slots": "8",
        "mem_per_core_gib": "7",
        "reserved_memory_gib": "56",
    })
    write_kv(node, node_values)
    with summary.open(newline="", encoding="utf-8") as handle:
        row = next(csv.DictReader(handle))
    row.update({
        "requested_slots": "8",
        "actual_slots": "8",
        "mem_per_core_gib": "7",
        "declared_memory_gib": "56",
    })
    write_csv(summary, [row])
    qacct.write_text(
        qacct.read_text(encoding="utf-8")
        .replace("slots 14", "slots 8"),
        encoding="utf-8",
    )
    scheduler_request.write_text(
        scheduler_request.read_text(encoding="utf-8")
        .replace("mem_per_core=4G", "mem_per_core=7G")
        .replace("omp range: 14", "omp range: 8"),
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="registered 14x4-GiB policy"):
        MODULE.validate_run(**files)


@pytest.mark.parametrize(
    ("original", "tampered", "message"),
    (
        ("taskid undefined", "taskid 7", "scalar-job"),
        ("project welfgr", "project other", "project mismatch"),
        ("granted_pe omp", "granted_pe smp", "parallel environment"),
    ),
)
def test_qacct_policy_tampering_fails_closed(
    tmp_path: Path, original: str, tampered: str, message: str,
) -> None:
    files = fixture(tmp_path)
    qacct = files["qacct"]
    assert isinstance(qacct, Path)
    text = qacct.read_text(encoding="utf-8")
    assert original in text
    qacct.write_text(text.replace(original, tampered), encoding="utf-8")
    with pytest.raises(ValueError, match=message):
        MODULE.validate_run(**files)


@pytest.mark.parametrize(
    ("original", "tampered", "message"),
    (
        ("project:                    welfgr",
         "project:                    other", "project changed"),
        ("omp range: 14", "smp range: 14", "parallel environment changed"),
        ("omp range: 14", "omp range: 8", "slot range changed"),
        ("mem_per_core=4G", "mem_per_core=5G", "memory binding changed"),
        ("h_rt=12:00:00", "h_rt=11:59:59", "hard-wall binding changed"),
    ),
)
def test_scheduler_request_policy_tampering_fails_closed(
    tmp_path: Path, original: str, tampered: str, message: str,
) -> None:
    files = fixture(tmp_path)
    request = files["scheduler_request"]
    assert isinstance(request, Path)
    contents = request.read_text(encoding="utf-8")
    assert original in contents
    request.write_text(contents.replace(original, tampered), encoding="utf-8")
    with pytest.raises(ValueError, match=message):
        MODULE.validate_run(**files)


@pytest.mark.parametrize(
    ("original", "tampered", "message"),
    (
        ("run_kss_scale.sge", "other.sge", "script changed"),
        (".stdout.txt", ".different.txt", "stdout path changed"),
    ),
)
def test_scheduler_request_execution_paths_are_bound(
    tmp_path: Path, original: str, tampered: str, message: str,
) -> None:
    files = fixture(tmp_path)
    request = files["scheduler_request"]
    assert isinstance(request, Path)
    contents = request.read_text(encoding="utf-8")
    request.write_text(contents.replace(original, tampered), encoding="utf-8")
    with pytest.raises(ValueError, match=message):
        MODULE.validate_run(**files)


@pytest.mark.parametrize(
    ("original", "tampered", "message"),
    (
        (f"KSS_SOURCE_COMMIT={SOURCE}",
         f"KSS_SOURCE_COMMIT={'5' * 40}", "KSS_SOURCE_COMMIT"),
        ("KSS_STATA_PROCESSORS=4", "KSS_STATA_PROCESSORS=14",
         "KSS_STATA_PROCESSORS"),
        (f"KSS_EXPERIMENT_ID={EXPERIMENT}",
         "KSS_EXPERIMENT_ID=other", "KSS_EXPERIMENT_ID"),
        ("KSS_DELETION_VAR=-", "KSS_DELETION_VAR=match_id",
         "KSS_DELETION_VAR"),
        ("KSS_SEED=8675309", "KSS_SEED=8675310", "seed differ"),
        ("KSS_BATCH=auto", "KSS_BATCH=8", "batch differ"),
    ),
)
def test_scheduler_request_environment_is_source_bound(
    tmp_path: Path, original: str, tampered: str, message: str,
) -> None:
    files = fixture(tmp_path)
    request = files["scheduler_request"]
    assert isinstance(request, Path)
    contents = request.read_text(encoding="utf-8")
    assert original in contents
    request.write_text(contents.replace(original, tampered), encoding="utf-8")
    with pytest.raises(ValueError, match=message):
        MODULE.validate_run(**files)


def test_node_hostname_must_match_qacct(tmp_path: Path) -> None:
    files = fixture(tmp_path)
    node = files["node_receipt"]
    assert isinstance(node, Path)
    values = MODULE.read_key_values(node, "node")
    values["hostname"] = "different-node"
    write_kv(node, values)
    with pytest.raises(ValueError, match="node receipt mismatch: hostname"):
        MODULE.validate_run(**files)


def test_compressed_engine_requires_stage_memory_diagnostics(tmp_path: Path) -> None:
    files = fixture(tmp_path)
    stage_memory = files["stage_memory"]
    assert isinstance(stage_memory, Path)
    text = stage_memory.read_text(encoding="utf-8")
    stage_memory.write_text(text.replace("200,80", "200,."), encoding="utf-8")
    with pytest.raises(ValueError, match="memory endpoint diagnostic missing"):
        MODULE.validate_run(**files)


def test_every_rhs_certificate_is_required(tmp_path: Path) -> None:
    files = fixture(tmp_path)
    rhs = files["rhs"]
    assert isinstance(rhs, Path)
    with rhs.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    write_csv(rhs, rows[:-1])
    with pytest.raises(ValueError, match="RHS certificate count"):
        MODULE.validate_run(**files)


def test_rhs_receipt_binds_to_dedicated_max_not_generic_solver_max(
    tmp_path: Path,
) -> None:
    files = fixture(tmp_path)
    summary = files["summary"]
    assert isinstance(summary, Path)
    with summary.open(newline="", encoding="utf-8") as handle:
        row = next(csv.DictReader(handle))
    # The generic route's aggregate also includes reduced-maker residuals,
    # which have no rows in rhs.csv.  Its aggregate may therefore be larger.
    row["solver_max_residual"] = "8e-10"
    row["rhs_max_residual"] = "5e-10"
    write_csv(summary, [row])
    report = MODULE.validate_run(**files)
    assert report["output"]["rhs"]["maximum_relative_residual"] == 5e-10

    row["rhs_max_residual"] = "4e-10"
    write_csv(summary, [row])
    with pytest.raises(ValueError, match="summary and per-RHS"):
        MODULE.validate_run(**files)

    row["rhs_max_residual"] = "9e-10"
    write_csv(summary, [row])
    with pytest.raises(ValueError, match="exceeds aggregate solver"):
        MODULE.validate_run(**files)

    row["solver_max_residual"] = "2e-9"
    row["rhs_max_residual"] = "5e-10"
    write_csv(summary, [row])
    with pytest.raises(ValueError, match="aggregate complete residual"):
        MODULE.validate_run(**files)


def test_real_generic_command_rhs_receipt_validates_when_stata_is_available(
    tmp_path: Path,
) -> None:
    configured = os.environ.get("KSS_STATA_MP")
    stata = Path(configured) if configured else Path(
        "/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp")
    if not stata.is_file():
        pytest.skip("Stata/MP is unavailable for the optional receipt test")

    rhs_path = tmp_path / "rhs.csv"
    residual_path = tmp_path / "residuals.tsv"
    marker_path = tmp_path / "stata.pass"
    do_path = tmp_path / "generic_receipt.do"
    do_path.write_text(
        f'''version 18.0
clear all
set more off
set varabbrev off
adopath ++ "{KSS_ROOT.as_posix()}"

set obs 27
generate long observation_key = _n
generate byte cell = ceil(_n/3)
generate byte within_cell = mod(_n-1,3)+1
generate long worker = floor((cell-1)/3)+1
generate long firm = mod(cell-1,3)+1
generate long match = 1000+cell
generate long frequency = 1+mod(_n,3)
generate double target = frequency*(1+mod(_n,3)/4)
generate double y = 2+.7*worker-.3*firm + ///
    .11*within_cell+.03*worker*firm+.007*observation_key^2

quietly kss_bc y [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(match) targetweight(target) ///
    probeorder(observation_key) algorithm(jla) engine(generic) ///
    preconditioner(diagonal) memory_gib(4) wallseconds(300) ///
    probes(2) batch(1) seed(8675309) tolerance(1e-10) nodisplay
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
assert "`e(engine_selected)'" == "generic"

tempname evidence receipt marker
matrix `evidence' = e(solver_rhs_diagnostics)
local solver_max : display %21.17g e(solver_max_residual)
mata: st_local("rhs_max", ///
    strofreal(max(st_matrix("`evidence'")[,5]), "%21.17g"))
file open `receipt' using "{residual_path.as_posix()}", ///
    write text replace
file write `receipt' "key" _tab "value" _n
file write `receipt' "solver_max_residual" _tab "`solver_max'" _n
file write `receipt' "rhs_max_residual" _tab "`rhs_max'" _n
file close `receipt'

preserve
clear
quietly set obs `=rowsof(`evidence')'
svmat double `evidence', names(col)
generate str64 experiment_id = "generic_real"
order experiment_id stage batch_start rhs iterations ///
    relative_residual converged
export delimited using "{rhs_path.as_posix()}", replace
restore
file open `marker' using "{marker_path.as_posix()}", write text replace
file write `marker' "PASS" _n
file close `marker'
exit 0
''',
        encoding="utf-8",
    )
    completed = subprocess.run(
        [str(stata), "-q", "do", str(do_path)],
        cwd=KSS_ROOT.parent,
        capture_output=True,
        text=True,
        timeout=120,
        check=False,
    )
    assert marker_path.read_text(encoding="utf-8").strip() == "PASS", (
        completed.stdout + completed.stderr
    )
    residuals = MODULE.read_key_values(residual_path, "real residual receipt")
    solver_max = float(residuals["solver_max_residual"])
    rhs_max = float(residuals["rhs_max_residual"])
    assert solver_max >= rhs_max
    report = MODULE.validate_rhs(
        rhs_path, experiment_id="generic_real", probes=2,
        tolerance=1e-10, rhs_max_residual=rhs_max)
    assert report["count"] == 7
    assert MODULE.close(report["maximum_relative_residual"], rhs_max, 1e-12)


def test_source_binding_rejects_stale_summary(tmp_path: Path) -> None:
    files = fixture(tmp_path)
    summary = files["summary"]
    assert isinstance(summary, Path)
    text = summary.read_text(encoding="utf-8")
    summary.write_text(text.replace(SOURCE, "5" * 40), encoding="utf-8")
    with pytest.raises(ValueError, match="wrong source commit"):
        MODULE.validate_run(**files)


@pytest.mark.parametrize(
    ("artifact", "field", "tampered", "message"),
    (
        ("reservation_path", "frequency_var", "frequency", "option binding"),
        ("node_receipt", "target_var", "target", "option binding"),
        ("summary", "deletion_var", "match_id", "option binding"),
        ("summary", "deletion_mode", "observation", "deletion mode"),
        ("node_receipt", "option_contract", "OLD", "option contract"),
    ),
)
def test_option_chain_rejects_tampering(
    tmp_path: Path, artifact: str, field: str, tampered: str, message: str,
) -> None:
    files = fixture(tmp_path)
    path = files[artifact]
    assert isinstance(path, Path)
    if artifact == "summary":
        with path.open(newline="", encoding="utf-8") as handle:
            values = next(csv.DictReader(handle))
        values[field] = tampered
        write_csv(path, [values])
    else:
        values = MODULE.read_key_values(path, artifact)
        values[field] = tampered
        write_kv(path, values)
    with pytest.raises(ValueError, match=message):
        MODULE.validate_run(**files)


def test_explicit_option_variables_are_supported_when_bound_end_to_end(
    tmp_path: Path,
) -> None:
    files = fixture(tmp_path)
    replacements = {
        "frequency_var": "frequency",
        "target_var": "target_weight",
        "deletion_var": "match_id",
    }
    for artifact in ("reservation_path", "node_receipt"):
        path = files[artifact]
        assert isinstance(path, Path)
        values = MODULE.read_key_values(path, artifact)
        values.update(replacements)
        write_kv(path, values)
    summary = files["summary"]
    assert isinstance(summary, Path)
    with summary.open(newline="", encoding="utf-8") as handle:
        row = next(csv.DictReader(handle))
    row.update(replacements)
    write_csv(summary, [row])
    scheduler_request = files["scheduler_request"]
    assert isinstance(scheduler_request, Path)
    request_text = scheduler_request.read_text(encoding="utf-8")
    for field, replacement in replacements.items():
        env_name = f"KSS_{field.upper()}"
        request_text = request_text.replace(
            f"{env_name}=-", f"{env_name}={replacement}")
    scheduler_request.write_text(request_text, encoding="utf-8")
    report = MODULE.validate_run(**files)
    assert report["options"]["frequency_var"] == "frequency"
    assert report["options"]["target_var"] == "target_weight"
    assert report["options"]["deletion_var"] == "match_id"


def test_driver_preserves_submitted_deletion_before_fixture_rewrite() -> None:
    driver = (KSS_ROOT / "benchmarks/scc/kss_scale_driver.do").read_text(
        encoding="utf-8")
    capture = "local submitted_deletion_var \"`deletion_var'\""
    rewrite = "local deletion_var \"`fixture_deletion'\""
    summary = "generate str32 deletion_var = \"`submitted_deletion_var'\""
    assert capture in driver and rewrite in driver and summary in driver
    assert driver.index(capture) < driver.index(rewrite) < driver.index(summary)


def test_rng_logical_probe_range_is_mandatory(tmp_path: Path) -> None:
    files = fixture(tmp_path)
    summary = files["summary"]
    assert isinstance(summary, Path)
    with summary.open(newline="", encoding="utf-8") as handle:
        row = next(csv.DictReader(handle))
    row["rng_target_probe_last"] = "3"
    write_csv(summary, [row])
    with pytest.raises(ValueError, match="logical probe ranges"):
        MODULE.validate_run(**files)


def test_compressed_status_and_k1_cursor_are_engine_specific(
    tmp_path: Path,
) -> None:
    files = fixture(tmp_path)
    summary = files["summary"]
    assert isinstance(summary, Path)
    with summary.open(newline="", encoding="utf-8") as handle:
        row = next(csv.DictReader(handle))
    row["estimator_status"] = "KSS_POINT_ESTIMATES_ONLY"
    write_csv(summary, [row])
    with pytest.raises(ValueError, match="status does not match"):
        MODULE.validate_run(**files)

    row["estimator_status"] = "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES"
    row["rng_implementation"] = "per_domain_stream_fixed_order"
    write_csv(summary, [row])
    with pytest.raises(ValueError, match="implementation for selected engine"):
        MODULE.validate_run(**files)


def test_phase_rss_peaks_are_measured_separately_from_stata_endpoints(
    tmp_path: Path,
) -> None:
    report = MODULE.validate_run(**fixture(tmp_path))
    reconciliation = report["output"]["resource_reconciliation"]
    assert reconciliation["phase_peak_measurement_available"]
    assert reconciliation["phase_endpoint_complete"]
    assert reconciliation["phase_peak_measurement_kind"] == \
        "sampled_process_tree_rss_peak"
    assert "phase_observed_allocation_endpoint_bytes" in reconciliation
    assert reconciliation["phase_observed_peak_rss_bytes"] == [80, 160, 300, 140]
    assert report["output"]["phase_rss"][
        "invalid_marker_read_count"] == 1


def test_phase_rss_uses_allocation_high_water_envelope(tmp_path: Path) -> None:
    files = fixture(tmp_path)
    samples = files["phase_rss_samples"]
    peaks = files["phase_rss_peaks"]
    assert isinstance(samples, Path)
    assert isinstance(peaks, Path)
    with samples.open(newline="", encoding="utf-8") as handle:
        sample_rows = list(csv.DictReader(handle))
    sample_rows[-1]["rss_bytes"] = "350"
    write_csv(samples, sample_rows)
    with peaks.open(newline="", encoding="utf-8") as handle:
        peak_rows = list(csv.DictReader(handle))
    peak_rows[-1]["peak_rss_bytes"] = "350"
    write_csv(peaks, peak_rows)
    reconciliation = MODULE.validate_run(
        **files)["output"]["resource_reconciliation"]
    assert reconciliation["phase_forecast_bytes"] == [200, 300, 610, 220]
    assert reconciliation["phase_rss_forecast_envelope_bytes"] == [
        200, 300, 610, 610,
    ]
    assert reconciliation["within_phase_peak_forecasts"]


def test_missing_phase_peak_forbids_next_scale(tmp_path: Path) -> None:
    files = fixture(tmp_path)
    samples = files["phase_rss_samples"]
    peaks = files["phase_rss_peaks"]
    sampler = files["phase_rss_sampler"]
    assert isinstance(samples, Path)
    assert isinstance(peaks, Path)
    assert isinstance(sampler, Path)
    with samples.open(newline="", encoding="utf-8") as handle:
        sample_rows = list(csv.DictReader(handle))
    write_csv(samples, [
        row for row in sample_rows if row["phase"] != "restoration"
    ])
    with peaks.open(newline="", encoding="utf-8") as handle:
        peak_rows = list(csv.DictReader(handle))
    restoration = peak_rows[-1]
    restoration.update({
        "peak_rss_bytes": "",
        "sample_count": 0,
        "first_timestamp_utc_seconds": "",
        "last_timestamp_utc_seconds": "",
    })
    write_csv(peaks, peak_rows)
    sampler_values = MODULE.read_key_values(sampler, "sampler")
    sampler_values["poll_count"] = "3"
    sampler_values["valid_sample_count"] = "3"
    write_kv(sampler, sampler_values)
    report = MODULE.validate_run(**files)
    reconciliation = report["output"]["resource_reconciliation"]
    assert reconciliation["status"] == "PHASE_PEAK_EVIDENCE_INCOMPLETE"
    assert not reconciliation["phase_peak_measurement_available"]
    assert not reconciliation["next_scale_allowed"]


def test_validator_writes_source_bound_rung_admission_receipt(
    tmp_path: Path,
) -> None:
    report = MODULE.validate_run(**fixture(tmp_path))
    receipt = tmp_path / "admission_receipt.tsv"
    MODULE.write_admission_receipt(receipt, report)
    values = MODULE.read_key_values(receipt, "admission receipt")
    assert values["receipt_version"] == MODULE.ADMISSION_RECEIPT_VERSION
    assert values["source_commit"] == SOURCE
    assert values["bundle_sha256"] == BUNDLE
    assert values["input_sha256"] == INPUT
    assert values["option_contract"] == MODULE.OPTION_CONTRACT
    assert values["frequency_var"] == "-"
    assert values["target_var"] == "-"
    assert values["deletion_var"] == "-"
    assert values["deletion_mode"] == "match"
    assert values["fixture"] == "well_connected"
    assert values["scale_factor"] == "1"
    assert values["phase_peak_complete"] == "1"
    assert values["next_scale_allowed"] == "0"


def test_4x_receipt_cannot_unlock_unprojected_8x() -> None:
    row = {
        "engine_selected": "compressed",
        "fixture": "well_connected",
        "scale_factor": "4",
        "resource_peak_bytes": "400",
        "resource_hard_mem_bytes": str(56 * MODULE.GIB),
        "resource_wall_upper_seconds": "200",
        "resource_hard_wall_seconds": "43200",
    }
    stages = [{
        "forecast_peak_bytes": "400",
        "observed_allocation_bytes": "100",
    } for _ in MODULE.PHASE_TOKENS]
    phase_rss = {
        "phase_peaks_bytes": {phase: 100 for phase in MODULE.PHASE_TOKENS},
        "phase_sample_counts": {phase: 1 for phase in MODULE.PHASE_TOKENS},
        "phase_peak_complete": True,
        "measurement_kind": "sampled_process_tree_rss_peak",
    }
    reconciliation = MODULE.reconcile_resources(
        row, stages, phase_rss,
        {"ru_maxrss_bytes": 100, "maxvmem_bytes": 100,
         "wall_seconds": 100},
        {"peak_rss_bytes": 100},
    )
    assert reconciliation["status"] == "SCALE_PROJECTION_REQUIRED"
    assert reconciliation["next_rung_projection_required"]
    assert not reconciliation["registered_successor"]
    assert not reconciliation["next_scale_allowed"]


def test_virtual_memory_does_not_inflate_physical_rss_reconciliation(
    tmp_path: Path,
) -> None:
    files = fixture(tmp_path)
    qacct = files["qacct"]
    assert isinstance(qacct, Path)
    text = qacct.read_text(encoding="utf-8")
    qacct.write_text(
        text.replace("maxvmem 8.25G", "maxvmem 40G"),
        encoding="utf-8",
    )
    report = MODULE.validate_run(**files)
    reconciliation = report["output"]["resource_reconciliation"]
    assert reconciliation["qacct_ru_maxrss_bytes"] == 8 * MODULE.GIB
    assert reconciliation["qacct_maxvmem_bytes"] == 40 * MODULE.GIB
    assert reconciliation["overall_observed_peak_rss_bytes"] == 8 * MODULE.GIB
    assert reconciliation["comparison_peak_bytes"] == 8 * MODULE.GIB


def test_qacct_and_gnu_time_rss_must_agree(tmp_path: Path) -> None:
    files = fixture(tmp_path)
    qacct = files["qacct"]
    assert isinstance(qacct, Path)
    text = qacct.read_text(encoding="utf-8")
    qacct.write_text(
        text.replace("ru_maxrss 8388608", "ru_maxrss 8388607"),
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="GNU-time maximum RSS disagree"):
        MODULE.validate_run(**files)


def test_tmp_capacity_counts_scaled_caller_preserve_and_sort_copies(
    tmp_path: Path,
) -> None:
    files = fixture(tmp_path)
    capacity = files["tmp_capacity"]
    assert isinstance(capacity, Path)
    values = MODULE.read_key_values(capacity, "TMPDIR capacity receipt")
    values["required_bytes"] = str(int(values["required_bytes"]) - 2000)
    write_kv(capacity, values)
    with pytest.raises(ValueError, match="capacity arithmetic"):
        MODULE.validate_run(**files)


def test_next_rung_requires_bound_p200_predecessor(tmp_path: Path) -> None:
    files = fixture(tmp_path)
    reservation = files["reservation_path"]
    assert isinstance(reservation, Path)
    prior_id = "cz18_1x_p200"
    prior = tmp_path / "experiments" / prior_id / "admission_receipt.tsv"
    prior.parent.mkdir(parents=True)
    prior_values = {
        "receipt_version": MODULE.ADMISSION_RECEIPT_VERSION,
        "validation_status": "KSS_SCALE_VALIDATION_PASS",
        "experiment_id": prior_id,
        "fixture": "cz18",
        "scale_factor": 1,
        "source_commit": SOURCE,
        "bundle_sha256": BUNDLE,
        "input_sha256": INPUT,
        "option_contract": MODULE.OPTION_CONTRACT,
        "frequency_var": FREQUENCY_VAR,
        "target_var": TARGET_VAR,
        "deletion_var": DELETION_VAR,
        "deletion_mode": DELETION_MODE,
        "requested_probes": 200,
        "engine": "compressed",
        "phase_peak_complete": 1,
        "next_scale_allowed": 1,
    }
    write_kv(prior, prior_values)
    values = MODULE.read_key_values(reservation, "reservation")
    values.update({
        "fixture": "well_connected",
        "scale_factor": "2",
        "prior_admission_receipt": str(prior),
        "prior_admission_sha256": MODULE.sha256(prior),
        "prior_experiment_id": prior_id,
        "prior_scale_factor": "1",
    })
    write_kv(reservation, values)
    MODULE.validate_reservation(
        reservation, experiment_id=EXPERIMENT, source_commit=SOURCE,
        bundle_sha=BUNDLE, input_sha=INPUT)

    prior_values["deletion_var"] = "match_id"
    write_kv(prior, prior_values)
    values["prior_admission_sha256"] = MODULE.sha256(prior)
    write_kv(reservation, values)
    with pytest.raises(ValueError, match="does not unlock"):
        MODULE.validate_reservation(
            reservation, experiment_id=EXPERIMENT, source_commit=SOURCE,
            bundle_sha=BUNDLE, input_sha=INPUT)

    prior_values["deletion_var"] = DELETION_VAR
    prior_values["requested_probes"] = 20
    write_kv(prior, prior_values)
    values["prior_admission_sha256"] = MODULE.sha256(prior)
    write_kv(reservation, values)
    with pytest.raises(ValueError, match="does not unlock"):
        MODULE.validate_reservation(
            reservation, experiment_id=EXPERIMENT, source_commit=SOURCE,
            bundle_sha=BUNDLE, input_sha=INPUT)

    prior_values.update({
        "fixture": "ring",
        "requested_probes": 200,
    })
    write_kv(prior, prior_values)
    values["prior_admission_sha256"] = MODULE.sha256(prior)
    write_kv(reservation, values)
    with pytest.raises(ValueError, match="does not unlock"):
        MODULE.validate_reservation(
            reservation, experiment_id=EXPERIMENT, source_commit=SOURCE,
            bundle_sha=BUNDLE, input_sha=INPUT)


def test_generic_fallback_requires_typed_resource_admission(
    tmp_path: Path,
) -> None:
    files = fixture(tmp_path)
    summary = files["summary"]
    stage_memory = files["stage_memory"]
    assert isinstance(summary, Path)
    assert isinstance(stage_memory, Path)
    with summary.open(newline="", encoding="utf-8") as handle:
        row = next(csv.DictReader(handle))
    row.update({
        "engine_selected": "generic",
        "estimator_status": "KSS_POINT_ESTIMATES_ONLY",
        "rng_implementation": "per_domain_stream_physical_copies",
        "fastpath_status": "FASTPATH_CONTROLS",
        "lifecycle_method": "",
        "life_sample_restored": ".",
        "solver_max_residual": 8e-10,
        "rhs_max_residual": 5e-10,
        "resource_numerical_peak_bytes": 550,
        "resource_peak_bytes": 550,
        "resource_mem_admit_bytes": 715,
    })
    write_csv(summary, [row])
    with stage_memory.open(newline="", encoding="utf-8") as handle:
        stages = list(csv.DictReader(handle))
    stages[2]["forecast_peak_bytes"] = "550"
    for stage in stages:
        stage["observed_allocation_bytes"] = "."
    write_csv(stage_memory, stages)
    report = MODULE.validate_run(**files)
    assert report["status"] == "KSS_SCALE_GENERIC_EVIDENCE_ONLY"
    assert report["output"]["resource_reconciliation"]["route"] == "generic"
    assert report["output"]["resource_reconciliation"]["status"] == \
        "GENERIC_ROUTE_NOT_SCALE_QUALIFYING"
    assert not report["output"]["resource_reconciliation"][
        "next_scale_allowed"]
    assert not report["output"]["resource_reconciliation"][
        "phase_endpoint_complete"]

    row["resource_status"] = "GENERIC_RESOURCE_ADMISSION_FAILED"
    write_csv(summary, [row])
    with pytest.raises(ValueError, match="pre-RNG resource admission"):
        MODULE.validate_run(**files)


def test_scaled_fixture_requires_independent_connectivity_certificate(
    tmp_path: Path,
) -> None:
    files = fixture(tmp_path)
    summary = files["summary"]
    reservation = files["reservation_path"]
    node_receipt = files["node_receipt"]
    capacity = files["tmp_capacity"]
    stage_memory = files["stage_memory"]
    scheduler_request = files["scheduler_request"]
    assert isinstance(summary, Path)
    assert isinstance(reservation, Path)
    assert isinstance(node_receipt, Path)
    assert isinstance(capacity, Path)
    assert isinstance(stage_memory, Path)
    assert isinstance(scheduler_request, Path)
    with summary.open(newline="", encoding="utf-8") as handle:
        row = next(csv.DictReader(handle))
    row.update({
        "scale_factor": 2,
        "fixture_construction_seconds": 0.25,
        "import_selection_seconds": 1.25,
        "input_rows": 204,
        "N_stored": 204,
        "N_retained": 204,
        "N_physical": 204,
        "worker_levels": 42,
        "firm_levels": 10,
        "deletion_units": 64,
        "coefficient_cells": 54,
        "diagnostic_coefficient_cells": 54,
        "diagnostic_deletion_units": 64,
        "fixture_base_rows": 100,
        "fixture_base_physical": 100,
        "fixture_base_workers": 20,
        "fixture_base_firms": 5,
        "fixture_base_cells": 25,
        "fixture_base_units": 30,
        "fixture_connector_rows": 4,
        "fixture_expected_rows": 204,
        "fixture_expected_physical": 204,
        "fixture_expected_workers": 42,
        "fixture_expected_firms": 10,
        "fixture_expected_cells": 54,
        "fixture_expected_units": 64,
        "fixture_conductance": 1,
        "fixture_lambda2": 2,
        "fixture_lambda_max": 2,
        "fixture_condition_proxy": 1,
        "fixture_min_degree": 2,
        "fixture_max_degree": 18,
    })
    write_csv(summary, [row])
    reservation_values = MODULE.read_key_values(reservation, "reservation")
    prior_id = "cz18_1x_p200"
    prior = tmp_path / "experiments" / prior_id / "admission_receipt.tsv"
    prior.parent.mkdir(parents=True)
    write_kv(prior, {
        "receipt_version": MODULE.ADMISSION_RECEIPT_VERSION,
        "validation_status": "KSS_SCALE_VALIDATION_PASS",
        "experiment_id": prior_id,
        "fixture": "cz18",
        "scale_factor": 1,
        "source_commit": SOURCE,
        "bundle_sha256": BUNDLE,
        "input_sha256": INPUT,
        "option_contract": MODULE.OPTION_CONTRACT,
        "frequency_var": FREQUENCY_VAR,
        "target_var": TARGET_VAR,
        "deletion_var": DELETION_VAR,
        "deletion_mode": DELETION_MODE,
        "requested_probes": 200,
        "engine": "compressed",
        "phase_peak_complete": 1,
        "next_scale_allowed": 1,
    })
    reservation_values.update({
        "scale_factor": "2",
        "prior_admission_receipt": str(prior),
        "prior_admission_sha256": MODULE.sha256(prior),
        "prior_experiment_id": prior_id,
        "prior_scale_factor": "1",
    })
    write_kv(reservation, reservation_values)
    node_values = MODULE.read_key_values(node_receipt, "node receipt")
    node_values.update({
        "scale_factor": "2",
        "tmp_required_bytes": str(13000 + 4 * MODULE.GIB),
        "prior_admission_receipt": str(prior),
        "prior_admission_sha256": reservation_values[
            "prior_admission_sha256"],
        "prior_experiment_id": prior_id,
    })
    write_kv(node_receipt, node_values)
    request_text = scheduler_request.read_text(encoding="utf-8")
    request_text = request_text.replace("KSS_SCALE_FACTOR=1", "KSS_SCALE_FACTOR=2")
    request_text = request_text.replace(
        "KSS_PRIOR_ADMISSION_RECEIPT=-",
        f"KSS_PRIOR_ADMISSION_RECEIPT={prior}")
    request_text = request_text.replace(
        "KSS_PRIOR_ADMISSION_SHA256=-",
        "KSS_PRIOR_ADMISSION_SHA256=" +
        reservation_values["prior_admission_sha256"])
    request_text = request_text.replace(
        "KSS_PRIOR_EXPERIMENT_ID=-",
        f"KSS_PRIOR_EXPERIMENT_ID={prior_id}")
    scheduler_request.write_text(request_text, encoding="utf-8")
    capacity_values = MODULE.read_key_values(capacity, "capacity")
    capacity_values.update({
        "scale_factor": "2",
        "fixture_expansion_factor": "2",
        "expanded_caller_forecast_bytes": "4000",
        "preserve_spool_forecast_bytes": "4000",
        "sort_temp_forecast_bytes": "4000",
        "required_bytes": str(13000 + 4 * MODULE.GIB),
    })
    write_kv(capacity, capacity_values)
    with stage_memory.open(newline="", encoding="utf-8") as handle:
        stage_rows = list(csv.DictReader(handle))
    stage_rows[0]["elapsed_seconds"] = "1.25"
    write_csv(stage_memory, stage_rows)
    report = MODULE.validate_run(**files)
    assert report["status"] == "KSS_SCALE_VALIDATION_PASS"
    assert report["output"]["input_preparation_timing"][
        "fixture_construction_seconds"] == 0.25

    row["import_selection_seconds"] = 1
    write_csv(summary, [row])
    with pytest.raises(ValueError, match="import-selection timing accounting"):
        MODULE.validate_run(**files)

    row["import_selection_seconds"] = 1.25
    row["fixture_lambda2"] = 0
    write_csv(summary, [row])
    with pytest.raises(ValueError, match="spectrum"):
        MODULE.validate_run(**files)


def test_nonreplication_fixture_cannot_claim_a_scaled_input() -> None:
    with pytest.raises(ValueError, match="false scale label"):
        MODULE.validate_scale_fixture({
            "fixture": "cz18",
            "scale_factor": "4",
        })


@pytest.mark.parametrize("scale", [1, 4, 8, 16])
def test_ring_fixture_is_restricted_to_the_registered_2x_stress(
    scale: int,
) -> None:
    with pytest.raises(ValueError, match="ring fixture must use scale factor 2"):
        MODULE.validate_scale_fixture({
            "fixture": "ring",
            "scale_factor": str(scale),
        })


def test_submitter_reservation_schema_has_unique_keys() -> None:
    submitter = (KSS_ROOT / "benchmarks/scc/submit_kss_scale.sh").read_text(
        encoding="utf-8")
    block = submitter.split('reservation="$output_dir/reservation.tsv"', 1)[1]
    block = block.split('} > "$reservation"', 1)[0]
    emitted = re.findall(r"printf '([a-z0-9_]+)\\t", block)
    assert emitted[0] == "key"
    keys = emitted[1:]
    assert len(keys) == len(set(keys))
    assert keys == [
        "experiment_id", "source_commit", "bundle_sha256", "input_sha256",
        "option_contract", "frequency_var", "target_var", "deletion_var",
        "deletion_mode",
        "requested_slots", "mem_per_core_gib", "total_reserved_gib",
        "stata_processors", "hard_wall_seconds",
        "estimator_hard_wall_seconds", "fixture", "scale_factor",
        "prior_admission_receipt", "prior_admission_sha256",
        "prior_experiment_id", "prior_scale_factor",
    ]


def test_ring_reservation_cannot_bypass_the_2x_restriction(
    tmp_path: Path,
) -> None:
    files = fixture(tmp_path)
    reservation = files["reservation_path"]
    assert isinstance(reservation, Path)
    values = MODULE.read_key_values(reservation, "reservation")
    values.update({"fixture": "ring", "scale_factor": "4"})
    write_kv(reservation, values)
    with pytest.raises(ValueError, match="ring reservation must use scale factor 2"):
        MODULE.validate_reservation(
            reservation, experiment_id=EXPERIMENT, source_commit=SOURCE,
            bundle_sha=BUNDLE, input_sha=INPUT)


@pytest.mark.parametrize("scale", [8, 16])
def test_large_rung_rejects_even_a_bound_p200_predecessor(
    tmp_path: Path, scale: int,
) -> None:
    files = fixture(tmp_path)
    reservation = files["reservation_path"]
    assert isinstance(reservation, Path)
    prior_scale = scale // 2
    prior_id = f"well_connected_{prior_scale}x_p200"
    prior = tmp_path / "experiments" / prior_id / "admission_receipt.tsv"
    prior.parent.mkdir(parents=True)
    write_kv(prior, {
        "receipt_version": MODULE.ADMISSION_RECEIPT_VERSION,
        "validation_status": "KSS_SCALE_VALIDATION_PASS",
        "experiment_id": prior_id,
        "fixture": "well_connected",
        "scale_factor": prior_scale,
        "source_commit": SOURCE,
        "bundle_sha256": BUNDLE,
        "input_sha256": INPUT,
        "option_contract": MODULE.OPTION_CONTRACT,
        "frequency_var": FREQUENCY_VAR,
        "target_var": TARGET_VAR,
        "deletion_var": DELETION_VAR,
        "deletion_mode": DELETION_MODE,
        "requested_probes": 200,
        "engine": "compressed",
        "phase_peak_complete": 1,
        "next_scale_allowed": 1,
    })
    values = MODULE.read_key_values(reservation, "reservation")
    values.update({
        "fixture": "well_connected",
        "scale_factor": str(scale),
        "prior_admission_receipt": str(prior),
        "prior_admission_sha256": MODULE.sha256(prior),
        "prior_experiment_id": prior_id,
        "prior_scale_factor": str(prior_scale),
    })
    write_kv(reservation, values)
    with pytest.raises(ValueError, match="SCALE_PROJECTION_REQUIRED"):
        MODULE.validate_reservation(
            reservation, experiment_id=EXPERIMENT, source_commit=SOURCE,
            bundle_sha=BUNDLE, input_sha=INPUT)


@pytest.mark.parametrize("scale", [8, 16])
def test_submitter_rejects_large_rung_before_receipt_or_qsub(
    scale: int,
) -> None:
    submitter = KSS_ROOT / "benchmarks/scc/submit_kss_scale.sh"
    run_dir = "/projectnb/welfgr/kss-bc/runs/admission-test"
    bundle_dir = f"/projectnb/welfgr/kss-bc/bundles/{BUNDLE}"
    completed = subprocess.run([
        "bash", str(submitter), run_dir, f"{bundle_dir}/source", SOURCE,
        f"{bundle_dir}/{BUNDLE}.tar.gz", BUNDLE,
        f"{bundle_dir}/{BUNDLE}.files.sha256",
        "/projectnb/welfgr/private/input.dta", INPUT,
        f"well_connected_{scale}x_p200", "well_connected", str(scale),
        "200", "8675309", "auto", "43200", "-", "-", "-",
        f"{run_dir}/experiments/prior/admission_receipt.tsv", "a" * 64,
    ], capture_output=True, text=True, check=False)
    assert completed.returncode == 75
    assert completed.stderr.strip() == "SCALE_PROJECTION_REQUIRED"


@pytest.mark.parametrize("scale", [8, 16])
def test_wrapper_rejects_large_rung_before_staging_or_stata(
    tmp_path: Path, scale: int,
) -> None:
    wrapper = KSS_ROOT / "benchmarks/scc/run_kss_scale.sge"
    run_dir = "/projectnb/welfgr/kss-bc/runs/admission-test"
    bundle_dir = f"/projectnb/welfgr/kss-bc/bundles/{BUNDLE}"
    environment = os.environ | {
        "KSS_RUN_DIR": run_dir,
        "KSS_SOURCE_DIR": f"{bundle_dir}/source",
        "KSS_SOURCE_COMMIT": SOURCE,
        "KSS_BUNDLE_ARCHIVE": f"{bundle_dir}/{BUNDLE}.tar.gz",
        "KSS_BUNDLE_SHA256": BUNDLE,
        "KSS_SOURCE_MANIFEST": f"{bundle_dir}/{BUNDLE}.files.sha256",
        "KSS_INPUT_DATASET": "/projectnb/welfgr/private/input.dta",
        "KSS_INPUT_SHA256": INPUT,
        "KSS_EXPERIMENT_ID": f"well_connected_{scale}x_p200",
        "KSS_FIXTURE": "well_connected",
        "KSS_SCALE_FACTOR": str(scale),
        "KSS_PROBES": "200",
        "KSS_SEED": "8675309",
        "KSS_BATCH": "auto",
        "KSS_FREQUENCY_VAR": "-",
        "KSS_TARGET_VAR": "-",
        "KSS_DELETION_VAR": "-",
        "KSS_REQUESTED_SLOTS": "14",
        "KSS_STATA_PROCESSORS": "4",
        "KSS_MEMORY_GIB": "56",
        "KSS_HARD_WALL_SECONDS": "43200",
        "KSS_OUTPUT_DIR": f"{run_dir}/experiments/blocked",
        "KSS_PRIOR_ADMISSION_RECEIPT":
            f"{run_dir}/experiments/prior/admission_receipt.tsv",
        "KSS_PRIOR_ADMISSION_SHA256": "a" * 64,
        "KSS_PRIOR_EXPERIMENT_ID": "prior",
        "TMPDIR": str(tmp_path),
        "NSLOTS": "14",
        "JOB_ID": "7199001",
        "HOSTNAME": "scc-test",
    }
    completed = subprocess.run(
        ["bash", str(wrapper)], env=environment,
        capture_output=True, text=True, check=False)
    assert completed.returncode == 75
    assert completed.stderr.strip() == "SCALE_PROJECTION_REQUIRED"

    environment["SGE_TASK_ID"] = "7"
    completed = subprocess.run(
        ["bash", str(wrapper)], env=environment,
        capture_output=True, text=True, check=False)
    assert completed.returncode == 198
    assert completed.stderr.strip() == \
        "KSS-SCALE must be one scalar SGE job"


def test_submission_is_scalar_and_separates_slots_from_stata_processors() -> None:
    submitter = (KSS_ROOT / "benchmarks/scc/submit_kss_scale.sh").read_text(
        encoding="utf-8")
    wrapper = (KSS_ROOT / "benchmarks/scc/run_kss_scale.sge").read_text(
        encoding="utf-8")
    driver = (KSS_ROOT / "benchmarks/scc/kss_scale_driver.do").read_text(
        encoding="utf-8")
    assert "if (( $# != 20 ))" in submitter
    assert "frequency_var=${16}" in submitter
    assert "target_var=${17}" in submitter
    assert "deletion_var=${18}" in submitter
    assert "-pe omp 14" in submitter
    assert "mem_per_core=4G" in submitter
    assert "KSS_STATA_PROCESSORS=4" in submitter
    assert " -t " not in submitter
    assert "#$ -pe omp 14" in wrapper
    assert "module load stata-mp/19" in wrapper
    assert 'export STATATMP="$TMPDIR/' in wrapper
    assert 'export KSS_PHASE_FILE="$STATATMP/' in wrapper
    assert "process_tree_rss_bytes" in wrapper
    assert "process_tree_rss.awk" in wrapper
    assert "!included[pid]" not in wrapper
    assert "phase_rss_samples.csv" in wrapper
    assert "phase_rss_peaks.csv" in wrapper
    assert "invalid_marker_read_count" in wrapper
    assert "*) exit 65" not in wrapper
    assert 'cd "$KSS_SOURCE_DIR"' in wrapper
    assert 'case "${SGE_TASK_ID:-undefined}"' in wrapper
    assert "KSS-SCALE must be one scalar SGE job" in wrapper
    assert "job_id=${raw_job_id%%.*}" not in submitter
    assert '[[ "$raw_job_id" =~ ^[0-9]+$ ]]' in submitter
    assert ".scheduler_request.txt" in submitter
    assert 'qsub -verify "${qsub_args[@]}"' in submitter
    assert 'qsub -terse "${qsub_args[@]}"' in submitter
    assert submitter.count('"${qsub_args[@]}"') == 2
    assert submitter.index("qsub -verify") < submitter.index("qsub -terse")
    assert "PRIOR_VALIDATOR_RECEIPT PRIOR_VALIDATOR_SHA256" in submitter
    assert "KSS-SCALE-ADMISSION-V1" in submitter
    assert "phase_peak_complete" in submitter
    assert "next_scale_allowed" in submitter
    assert "3 * input_bytes" not in wrapper
    assert "expanded_caller_forecast_bytes" in wrapper
    assert "preserve_spool_forecast_bytes" in wrapper
    assert 'test "$KSS_SCALE_FACTOR" = 1' in wrapper
    assert 'test "$scale_factor" = 1' in submitter
    assert 'test "$KSS_SCALE_FACTOR" = 2' in wrapper
    assert 'test "$scale_factor" = 2' in submitter
    assert "SCALE_PROJECTION_REQUIRED" in wrapper
    assert "SCALE_PROJECTION_REQUIRED" in submitter
    driver_receipts = {
        "option_contract": "generate str32 option_contract",
        "frequency_var": "generate str32 frequency_var",
        "target_var": "generate str32 target_var",
        "deletion_var": "generate str32 deletion_var",
        "deletion_mode": "generate str16 deletion_mode",
    }
    for field, driver_receipt in driver_receipts.items():
        assert f"printf '{field}\\t" in submitter
        assert f"printf '{field}\\t" in wrapper
        assert driver_receipt in driver
