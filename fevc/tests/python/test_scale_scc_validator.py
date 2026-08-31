from __future__ import annotations

import csv
import importlib.util
import subprocess
from pathlib import Path

import pytest

KSS_ROOT = Path(__file__).resolve().parents[2]
VALIDATOR = KSS_ROOT / "benchmarks" / "scc" / "validate_kss_scale.py"
SPEC = importlib.util.spec_from_file_location("validate_kss_scale", VALIDATOR)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

SOURCE = "4" * 40
BUNDLE = "b" * 64
INPUT = "c" * 64
EXPERIMENT = "well_connected_1x_p2"
FREQUENCY_VAR = "-"
TARGET_VAR = "-"
DELETION_VAR = "-"
DELETION_MODE = "match"


def _write_kv(path: Path, values: dict[str, object]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(("key", "value"))
        writer.writerows(values.items())


def _write_one_csv(path: Path, row: dict[str, object]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(row))
        writer.writeheader()
        writer.writerow(row)


def _reservation_values(**overrides: object) -> dict[str, object]:
    values: dict[str, object] = {
        "experiment_id": EXPERIMENT,
        "source_commit": SOURCE,
        "bundle_sha256": BUNDLE,
        "input_sha256": INPUT,
        "option_contract": MODULE.OPTION_CONTRACT,
        "frequency_var": "-",
        "target_var": "-",
        "deletion_var": "-",
        "deletion_mode": "match",
        "requested_slots": 3,
        "mem_per_core_gib": 7,
        "total_reserved_gib": 21,
        "stata_processors": 2,
        "hard_wall_seconds": 1800,
        "estimator_hard_wall_seconds": 1680,
        "fixture": "replicated_blocks",
        "scale_factor": 3,
    }
    values.update(overrides)
    return values


def _validated_reservation(tmp_path: Path, **overrides: object) -> dict[str, str]:
    path = tmp_path / "reservation.tsv"
    _write_kv(path, _reservation_values(**overrides))
    return MODULE.validate_reservation(
        path,
        experiment_id=EXPERIMENT,
        source_commit=SOURCE,
        bundle_sha=BUNDLE,
        input_sha=INPUT,
    )


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
    remote_run = "/projectnb/welfgr/fevc/runs/test"
    remote_bundle = f"/projectnb/welfgr/fevc/bundles/{BUNDLE}"
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
        "KSS_MEM_PER_CORE_GIB": "4",
        "KSS_MEMORY_GIB": "56",
        "KSS_HARD_WALL_SECONDS": "43200",
        "KSS_OUTPUT_DIR": f"{remote_run}/experiments/{EXPERIMENT}",
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
        f"script_file: {remote_source}/fevc/benchmarks/scc/"
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
    })
    wrapper_pass = output / "wrapper.pass"
    wrapper_pass.write_text(
        f"KSS_STREAMLINE_WRAPPER_PASS {EXPERIMENT} {BUNDLE} {SOURCE} {INPUT}\n",
        encoding="utf-8",
    )
    stata_pass = output / "stata.pass"
    stata_pass.write_text(
        f"KSS_STREAMLINE_ESTIMATOR_PASS {EXPERIMENT} {BUNDLE} {SOURCE} {INPUT}\n",
        encoding="utf-8",
    )
    application = output / "application.log"
    application.write_text(
        f"KSS-STREAMLINE ESTIMATOR PASS: {EXPERIMENT}\n", encoding="utf-8")
    wrapper_metrics = output / "wrapper_metrics.tsv"
    write_kv(wrapper_metrics, {
        "run_outcome": "PASS",
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
        "route_api": "KSS-ROUTE-STRUCTURAL-V1",
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
        "rng_contract": "KSS-MT64S-DOMAIN-CURSOR-V3-STATA19",
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
    expected_stages = (
        "import_selection",
        "compression_transition",
        "numerical_computation",
        "restoration",
    )
    write_csv(stage_memory, [{
        "stage": stage,
        "elapsed_seconds": 1 + index,
        "forecast_peak_bytes": stage_forecasts[index],
        "observed_allocation_bytes": stage_observed[index],
        "measurement_kind": "stata_allocated_endpoint_not_peak",
    } for index, stage in enumerate(expected_stages)])
    phase_rss_samples = output / "phase_rss_samples.csv"
    phase_tokens = (
        "import_selection",
        "compression_transition",
        "numerical",
        "restoration",
    )
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




def test_run_spec_accepts_arbitrary_positive_resources_and_scale(
    tmp_path: Path,
) -> None:
    values = _validated_reservation(tmp_path)
    assert values["requested_slots"] == "3"
    assert values["mem_per_core_gib"] == "7"
    assert values["total_reserved_gib"] == "21"
    assert values["stata_processors"] == "2"
    assert values["scale_factor"] == "3"


def test_no_predecessor_field_is_accepted(tmp_path: Path) -> None:
    path = tmp_path / "reservation.tsv"
    values = _reservation_values(prior_admission_receipt="old.tsv")
    _write_kv(path, values)
    with pytest.raises(ValueError, match="predecessor"):
        MODULE.validate_reservation(
            path,
            experiment_id=EXPERIMENT,
            source_commit=SOURCE,
            bundle_sha=BUNDLE,
            input_sha=INPUT,
        )


def test_replicated_fixture_labels_meta_graph_and_connector_volume() -> None:
    row = {
        "fixture": "replicated_blocks",
        "scale_factor": "3",
        "fixture_base_rows": "10",
        "fixture_base_physical": "12",
        "fixture_base_workers": "4",
        "fixture_base_firms": "3",
        "fixture_base_cells": "6",
        "fixture_base_units": "7",
        "fixture_expected_rows": "42",
        "fixture_expected_physical": "48",
        "fixture_expected_workers": "18",
        "fixture_expected_firms": "9",
        "fixture_expected_cells": "30",
        "fixture_expected_units": "33",
        "fixture_connector_rows": "12",
        "fixture_connector_volume_ratio": ".25",
        "fixture_meta_conductance": "1",
        "fixture_meta_lambda2": "1.5",
        "fixture_meta_lambda_max": "1.5",
        "fixture_meta_cond_proxy": "1",
    }
    MODULE.validate_fixture(row)
    row["fixture_connector_volume_ratio"] = ".5"
    with pytest.raises(ValueError, match="connector-volume"):
        MODULE.validate_fixture(row)


def _summary_base(reservation: dict[str, str]) -> dict[str, object]:
    return {
        "experiment_id": EXPERIMENT,
        "source_commit": SOURCE,
        "bundle_sha256": BUNDLE,
        "input_sha256": INPUT,
        "fixture": reservation["fixture"],
        "scale_factor": reservation["scale_factor"],
        "option_contract": MODULE.OPTION_CONTRACT,
        "frequency_var": "-",
        "target_var": "-",
        "deletion_var": "-",
        "deletion_mode": "match",
        "requested_probes": 17,
        "requested_slots": 3,
        "actual_slots": 3,
        "requested_stata_processors": 2,
        "actual_stata_processors": 2,
        "declared_memory_gib": 21,
    }


def test_typed_estimator_failure_is_a_diagnostic_outcome(tmp_path: Path) -> None:
    reservation = _validated_reservation(tmp_path, fixture="local", scale_factor=1)
    summary = tmp_path / "summary.csv"
    row = _summary_base(reservation)
    row.update(command_rc=498, estimator_status="PCG_NONCONVERGENCE")
    _write_one_csv(summary, row)
    parsed, success, resource = MODULE.validate_summary(
        summary,
        experiment_id=EXPERIMENT,
        source_commit=SOURCE,
        bundle_sha=BUNDLE,
        input_sha=INPUT,
        probes=17,
        reservation=reservation,
    )
    assert parsed["estimator_status"] == "PCG_NONCONVERGENCE"
    assert success is False
    assert resource is None


def test_scientific_pass_allows_advisory_headroom_and_wall_miss(
    tmp_path: Path,
) -> None:
    reservation = _validated_reservation(tmp_path, fixture="local", scale_factor=1)
    summary = tmp_path / "summary.csv"
    row = _summary_base(reservation)
    row.update(
        command_rc=0,
        estimator_status="KSS_POINT_ESTIMATES_ONLY",
        route_api="KSS-ROUTE-STRUCTURAL-V1",
        sample_semantics_valid=1,
        rng_runtime="19",
        rng_contract="KSS-MT64S-DOMAIN-CURSOR-V3-STATA19",
        seed=8675309,
        rng_master_seed=8675309,
        tolerance=1e-10,
        solver_max_residual=5e-10,
        rhs_max_residual=5e-10,
        plugin_worker=1,
        plugin_firm=2,
        plugin_covariance=.5,
        plugin_total=4,
        correction_worker=.1,
        correction_firm=.2,
        correction_covariance=.05,
        correction_total=.4,
        corrected_worker=.9,
        corrected_firm=1.8,
        corrected_covariance=.45,
        corrected_total=3.6,
        resource_status="ADMITTED",
        resource_peak_bytes=10 * MODULE.GIB,
        resource_hard_mem_bytes=12 * MODULE.GIB,
        resource_mem_headroom=.3,
        resource_mem_admit_bytes=13 * MODULE.GIB,
        resource_wall_upper_seconds=1000,
        resource_wall_admit_seconds=1500,
        resource_hard_wall_seconds=1200,
    )
    _write_one_csv(summary, row)
    _, success, resource = MODULE.validate_summary(
        summary,
        experiment_id=EXPERIMENT,
        source_commit=SOURCE,
        bundle_sha=BUNDLE,
        input_sha=INPUT,
        probes=17,
        reservation=reservation,
    )
    assert success is True
    assert resource is not None
    assert resource["headroom_fits"] is False
    assert resource["wall_advisory_fits"] is False


def test_reconciliation_never_authorizes_a_future_run() -> None:
    reservation = {key: str(value) for key, value in _reservation_values().items()}
    scheduler = {
        "maxvmem_bytes": 11 * MODULE.GIB,
        "ru_maxrss_bytes": 10 * MODULE.GIB,
        "wall_seconds": 900,
    }
    resource = {"direct_peak_bytes": 10 * MODULE.GIB}
    result = MODULE.reconcile_resources(
        {}, scheduler, reservation, resource
    )
    assert result["direct_memory_safe"] is True
    assert result["forecast_ratio"] == pytest.approx(1.1)
    assert result["future_run_authorized"] is False


def test_run_receipt_is_self_contained_and_non_authorizing(tmp_path: Path) -> None:
    path = tmp_path / "run.tsv"
    report = {
        "status": "KSS_STREAMLINE_DIAGNOSTIC_COLLECTED",
        "experiment_id": EXPERIMENT,
        "fixture": "replicated_blocks",
        "scale_factor": 3,
        "source_commit": SOURCE,
        "bundle_sha256": BUNDLE,
        "input_sha256": INPUT,
        "requested_probes": 17,
        "options": {
            "option_contract": MODULE.OPTION_CONTRACT,
            "frequency_var": "-",
            "target_var": "-",
            "deletion_var": "-",
            "deletion_mode": "match",
        },
        "output": {"engine": "compressed"},
    }
    MODULE.write_admission_receipt(path, report)
    receipt = MODULE.read_key_values(path, "receipt")
    assert receipt["receipt_version"] == "KSS-STREAMLINE-RUN-V1"
    assert receipt["future_run_authorized"] == "0"
    assert not any(key.startswith("prior_") for key in receipt)


def test_submitter_and_wrapper_take_run_specific_resources() -> None:
    scc = KSS_ROOT / "benchmarks" / "scc"
    submitter = (scc / "submit_kss_scale.sh").read_text(encoding="utf-8")
    wrapper = (scc / "run_kss_scale.sge").read_text(encoding="utf-8")
    driver = (scc / "kss_scale_driver.do").read_text(encoding="utf-8")
    assert "if (( $# != 21 ))" in submitter
    assert 'if [[ "$fixture" == well_connected ]]; then fixture=replicated_blocks; fi' in submitter
    assert '-pe omp "$KSS_REQUESTED_SLOTS"' in submitter
    assert 'mem_per_core=${mem_per_core_gib}G' in submitter
    assert 'h_rt=$hard_wall_hms' in submitter
    assert "KSS_MEM_PER_CORE_GIB" in submitter + wrapper
    assert "stata_processors <= requested_slots" in submitter
    assert "KSS_STATA_PROCESSORS <= KSS_REQUESTED_SLOTS" in wrapper
    assert '"$batch" =~ ^[1-9][0-9]*$' in submitter
    assert '"$KSS_BATCH" =~ ^[1-9][0-9]*$' in wrapper
    assert '"^[1-9][0-9]*$"' in driver
    assert "KSS_PRIOR_" not in submitter + wrapper + driver
    assert "route_pilot_diagnostics" not in driver
    assert "KSS-ROUTE-STRUCTURAL-V1" in driver or "route_api" in driver
    assert "#$ -pe omp 14" not in wrapper
    assert "#$ -l mem_per_core=4G" not in wrapper
    assert "#$ -l h_rt=12:00:00" not in wrapper


def test_shell_entrypoints_are_syntactically_valid() -> None:
    scc = KSS_ROOT / "benchmarks" / "scc"
    for script in ("submit_kss_scale.sh", "run_kss_scale.sge"):
        completed = subprocess.run(
            ["bash", "-n", str(scc / script)],
            capture_output=True,
            text=True,
            check=False,
        )
        assert completed.returncode == 0, completed.stderr


def test_driver_serializes_typed_diagnostic_before_clean_exit() -> None:
    driver = (
        KSS_ROOT / "benchmarks" / "scc" / "kss_scale_driver.do"
    ).read_text(encoding="utf-8")
    summary = driver.index("export delimited using `\"`output_dir'/summary.csv\"'")
    marker = driver.index("KSS_STREAMLINE_ESTIMATOR_DIAGNOSTIC", summary)
    clean_exit = driver.index("exit 0", marker)
    assert summary < marker < clean_exit
