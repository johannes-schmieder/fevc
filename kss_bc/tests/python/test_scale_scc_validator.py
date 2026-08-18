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
EXPERIMENT = "replicated_blocks_3x_p17"


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
