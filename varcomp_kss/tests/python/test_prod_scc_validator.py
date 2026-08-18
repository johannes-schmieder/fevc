from __future__ import annotations

import csv
import subprocess
import sys
from pathlib import Path

import pytest

from varcomp_kss.benchmarks.scc.select_prod_calibration import (
    CALIBRATION_REPETITIONS,
    MAXIMUM_TIMEOUT_SECONDS,
    SCC_HARD_RUNTIME_CEILING_SECONDS,
    WRAPPER_RUNTIME_MARGIN_SECONDS,
    candidate_sort_key,
    conservative_candidate_sort_key,
    conservative_projection_from_repetitions,
    node_class_multiset,
    projection_from_measurements,
    same_host_coincidence_diagnostic,
    select_public_automatic_candidate,
    timing_identity_after_csv,
    timing_inversion_status,
)
from varcomp_kss.benchmarks.scc.select_prod_calibration import (
    main as selector_main,
)
from varcomp_kss.benchmarks.scc.validate_stress_projection import (
    main as stress_projection_main,
)
from varcomp_kss.benchmarks.scc.validate_stress_projection import (
    stress_projection_from_calibrations,
)
from varcomp_kss.benchmarks.validate_prod_scc import (
    expected_timeout,
    identity,
    load_plan,
    recompute_calibration_projection,
    recompute_conservative_calibration_projection,
    recompute_same_host_coincidences,
    recompute_timing_inversion,
    validate_correction_timing,
    validate_fixedpoint_graph_certificate,
    validate_node_characteristics,
    validate_selection,
    validate_selector_route,
)


def observed_inverted_pair() -> list[dict[str, float | str]]:
    return [
        {"probes": 20, "temperature": "cold", "command_seconds": 1496.0,
         "correction_seconds": 200.0, "qacct_wall_seconds": 1626.0},
        {"probes": 20, "temperature": "warm", "command_seconds": 773.0,
         "correction_seconds": 180.0, "qacct_wall_seconds": 1678.0},
        {"probes": 40, "temperature": "cold", "command_seconds": 1050.0,
         "correction_seconds": 400.0, "qacct_wall_seconds": 1090.0},
        {"probes": 40, "temperature": "warm", "command_seconds": 1000.0,
         "correction_seconds": 380.0, "qacct_wall_seconds": 1919.0},
    ]


def test_projection_rejects_zero_slope_from_inverted_noisy_pair() -> None:
    observed = observed_inverted_pair()
    selected = projection_from_measurements(observed)
    independent = recompute_calibration_projection(observed)
    assert selected == independent
    alpha, beta, headroom, projected, timeout = selected
    assert alpha == pytest.approx(1269.0)
    assert beta == pytest.approx(11.35)
    assert headroom == pytest.approx(250.0)
    assert projected == pytest.approx(5241.25)
    assert timeout == 5242
    for row in observed:
        probes = int(row["probes"])
        assert alpha + probes * beta >= float(row["command_seconds"])
        assert probes * beta >= float(row["correction_seconds"])


def test_projection_correction_rate_is_a_marginal_cost_floor() -> None:
    observed = observed_inverted_pair()
    observed[0]["correction_seconds"] = 300.0
    alpha, beta, _, _, _ = projection_from_measurements(observed)
    assert beta == pytest.approx(15.0)
    assert alpha == pytest.approx(1196.0)


@pytest.mark.parametrize(
    ("field", "value"),
    [("command_seconds", float("nan")),
     ("correction_seconds", -1.0),
     ("qacct_wall_seconds", -1.0)],
)
def test_projection_rejects_invalid_timing(field: str, value: float) -> None:
    observed = observed_inverted_pair()
    observed[0][field] = value
    with pytest.raises(ValueError, match="invalid projection timing"):
        projection_from_measurements(observed)


def independent_repetition_measurements() -> list[dict[str, float | str]]:
    measurements: list[dict[str, float | str]] = []
    cells = {
        (20, "cold"): ([100.0, 110.0, 120.0], ["node-a", "node-b", "node-c"]),
        (40, "cold"): ([130.0, 140.0, 150.0], ["node-a", "node-d", "node-e"]),
        (20, "warm"): ([90.0, 100.0, 110.0], ["node-f", "node-g", "node-h"]),
        (40, "warm"): ([120.0, 130.0, 140.0], ["node-i", "node-j", "node-k"]),
    }
    for (probes, temperature), (commands, hosts) in cells.items():
        for repetition, (command, host) in enumerate(zip(commands, hosts, strict=False), 1):
            measurements.append({
                "probes": probes,
                "temperature": temperature,
                "repetition": repetition,
                "command_seconds": command,
                "correction_seconds": float(probes),
                "qacct_wall_seconds": command + 10.0,
                "hostname": host,
            })
    return measurements


def test_conservative_projection_never_pairs_incidental_same_host_jobs() -> None:
    observed = independent_repetition_measurements()
    selected = conservative_projection_from_repetitions(observed)
    independent = recompute_conservative_calibration_projection(observed)
    assert selected == independent
    alpha, beta, headroom, projected, timeout, policy = selected
    assert beta == pytest.approx(2.5)
    assert alpha == pytest.approx(70.0)
    assert headroom == pytest.approx(130.0)
    assert projected == pytest.approx(967.5)
    assert timeout == 968
    assert policy == ("cold=cross_host_upper_envelope|"
                      "warm=cross_host_upper_envelope")
    assert same_host_coincidence_diagnostic(observed) == \
        recompute_same_host_coincidences(observed) == "cold=node-a|warm=-"
    for item in observed:
        probes = int(item["probes"])
        assert alpha + probes * beta >= float(item["command_seconds"])
        assert probes * beta >= float(item["correction_seconds"])


def test_cell_medians_rank_typical_time_but_outlier_sets_timeout_envelope() -> None:
    observed = independent_repetition_measurements()
    typical = []
    for probes in (20, 40):
        for temperature in ("cold", "warm"):
            cell = [item for item in observed
                    if item["probes"] == probes and
                    item["temperature"] == temperature]
            typical.append({
                "probes": probes,
                "temperature": temperature,
                "command_seconds": sorted(float(item["command_seconds"])
                                          for item in cell)[1],
                "correction_seconds": sorted(float(item["correction_seconds"])
                                             for item in cell)[1],
                "qacct_wall_seconds": sorted(float(item["qacct_wall_seconds"])
                                             for item in cell)[1],
            })
    typical_projection = projection_from_measurements(typical)
    high_projection = conservative_projection_from_repetitions(observed)
    assert typical_projection[3] == pytest.approx(680.0)
    assert high_projection[3] == pytest.approx(967.5)

    outlier = [dict(item) for item in observed]
    next(item for item in outlier
         if item["probes"] == 40 and item["temperature"] == "cold" and
         item["repetition"] == 3)["command_seconds"] = 500.0
    outlier_high = conservative_projection_from_repetitions(outlier)
    assert projection_from_measurements(typical) == typical_projection
    assert outlier_high[3] > high_projection[3]
    assert outlier_high[4] > high_projection[4]


def test_conservative_projection_is_permutation_invariant() -> None:
    observed = independent_repetition_measurements()
    assert conservative_projection_from_repetitions(observed) == \
        conservative_projection_from_repetitions(list(reversed(observed)))


def test_missing_or_failed_replica_invalidates_projection() -> None:
    observed = independent_repetition_measurements()
    with pytest.raises(ValueError, match="requires 12 measurements"):
        conservative_projection_from_repetitions(observed[:-1])
    observed[0]["command_seconds"] = -1.0
    with pytest.raises(ValueError, match="invalid conservative projection"):
        conservative_projection_from_repetitions(observed)


def test_selection_is_restricted_to_public_auto_candidates() -> None:
    forced = {
        "preconditioner": "cmg", "batch": 8,
        "typical_timeout_seconds": 300, "typical_projected_seconds": 200.0,
        "timeout_seconds": 400, "projected_seconds": 399.0,
        "batch_request": "8", "node_class_multiset": "forced",
    }
    automatic = {
        "preconditioner": "auto", "batch": 16,
        "typical_timeout_seconds": 500, "typical_projected_seconds": 499.0,
        "timeout_seconds": 700, "projected_seconds": 699.0,
        "batch_request": "16", "node_class_multiset": "controlled",
    }
    assert candidate_sort_key(forced) < candidate_sort_key(automatic)
    selected, policy, identical = select_public_automatic_candidate([forced, automatic])
    assert selected is automatic
    assert policy == "MEDIAN_TYPICAL_IDENTICAL_NODE_CLASS_MULTISET"
    assert identical


def test_heterogeneous_auto_node_classes_force_high_envelope_ranking() -> None:
    typical_fast = {
        "preconditioner": "auto", "batch": 8, "batch_request": "8",
        "typical_timeout_seconds": 300, "typical_projected_seconds": 250.0,
        "timeout_seconds": 800, "projected_seconds": 799.0,
        "node_class_multiset": "class-a",
    }
    high_fast = {
        "preconditioner": "auto", "batch": 16, "batch_request": "16",
        "typical_timeout_seconds": 500, "typical_projected_seconds": 450.0,
        "timeout_seconds": 600, "projected_seconds": 599.0,
        "node_class_multiset": "class-b",
    }
    assert candidate_sort_key(typical_fast) < candidate_sort_key(high_fast)
    assert conservative_candidate_sort_key(high_fast) < \
        conservative_candidate_sort_key(typical_fast)
    selected, policy, identical = select_public_automatic_candidate(
        [typical_fast, high_fast])
    assert selected is high_fast
    assert policy == "CONSERVATIVE_HIGH_HETEROGENEOUS_NODE_CLASS_MULTISET"
    assert not identical


def test_node_class_multiset_is_sorted_and_multiplicity_preserving() -> None:
    rows = [
        {"uname_machine": "x86_64", "cpu_model": model, "logical_cpus": cpus}
        for model, cpus in (["B", 32], ["A", 64], ["A", 64]) * 4
    ]
    observed = node_class_multiset(list(reversed(rows)))
    assert observed == node_class_multiset(rows)
    assert observed.count('"A",64') == 8
    assert observed.count('"B",32') == 4


def test_inverted_cell_medians_are_classified_not_rejected() -> None:
    summaries = [
        {"probes": 20, "temperature": "cold", "command_seconds": 120.0},
        {"probes": 40, "temperature": "cold", "command_seconds": 110.0},
        {"probes": 20, "temperature": "warm", "command_seconds": 90.0},
        {"probes": 40, "temperature": "warm", "command_seconds": 100.0},
    ]
    expected = ("UNEXPLAINED_VARIABILITY", "cold")
    assert timing_inversion_status(summaries) == expected
    assert recompute_timing_inversion(summaries) == expected


def test_timing_identity_accepts_observed_stata_csv_rounding() -> None:
    assert timing_identity_after_csv(1083.4611, 451.64999, 631.81097)


def test_timing_identity_rejects_material_accounting_discrepancy() -> None:
    assert not timing_identity_after_csv(1083.4621, 451.64999, 631.81097)


def test_production_validator_checks_jla_timing_with_context() -> None:
    validate_correction_timing({
        "algorithm_selected": "jla", "correction_seconds": "1083.4611",
        "leverage_seconds": "451.64999", "target_seconds": "631.81097",
    }, "cal_auto_b8_p40_cold_r1")
    with pytest.raises(
        ValueError,
        match=(r"correction timing identity failed.*material_mismatch: "
               r"correction=.*leverage=.*target=.*difference=.*bound="),
    ):
        validate_correction_timing({
            "algorithm_selected": "jla", "correction_seconds": "1083.4621",
            "leverage_seconds": "451.64999", "target_seconds": "631.81097",
        }, "material_mismatch")


def test_production_validator_preserves_exact_timer_semantics() -> None:
    validate_correction_timing({
        "algorithm_selected": "exact", "correction_seconds": "0.125",
        "leverage_seconds": "0", "target_seconds": "0",
    }, "install_auto19")


def test_static_calibration_plan_has_three_independent_jobs_per_cell() -> None:
    root = Path(__file__).resolve().parents[2]
    plan = load_plan(root / "benchmarks/prod_experiments.tsv")
    calibration = [row for row in plan if row["stage"] == "calibration"]
    assert len(plan) == 102
    assert len(calibration) == 72
    assert all(row["depends"] == "cz18_preflight" for row in calibration)
    assert all(row["repetitions"] == "1" for row in calibration)
    assert {int(row["experiment_id"].rsplit("_r", 1)[1])
            for row in calibration} == set(CALIBRATION_REPETITIONS)
    selector = next(row for row in plan if row["stage"] == "calibration_selector")
    assert set(selector["depends"].split(",")) == {
        row["experiment_id"] for row in calibration
    }
    stress_calibrations = [
        row for row in plan
        if row["experiment_id"].startswith("cz18_stress2x_cal20_r")
    ]
    assert len(stress_calibrations) == 3
    assert all(row["phase"] == "production" and
               row["depends"] == "cz18_full200" and
               row["repetitions"] == "1"
               for row in stress_calibrations)
    stress_full = next(
        row for row in plan if row["experiment_id"] == "cz18_stress2x_full200"
    )
    assert stress_full["phase"] == "stress"
    assert set(stress_full["depends"].split(",")) == {
        row["experiment_id"] for row in stress_calibrations
    }


def test_submitter_revalidates_and_hash_checks_prior_phase_evidence() -> None:
    root = Path(__file__).resolve().parents[2]
    submitter = (root / "benchmarks/scc/submit_prod_dag.sh").read_text(
        encoding="utf-8")
    assert 'validate_prior_phase preflight' in submitter
    assert 'validate_prior_phase calibration' in submitter
    assert '--phase "$prior"' in submitter
    assert 'actual_evidence_sha=$(sha256sum "$evidence_manifest"' in submitter
    assert 'test "$recorded_evidence_sha" = "$actual_evidence_sha"' in submitter
    assert 'sha256sum -c "validation/$prior.evidence.sha256"' in submitter
    prior_validator = submitter.split("validate_prior_phase()", 1)[1].split("}\n", 1)[0]
    assert "--write-pass" not in prior_validator


def test_submitter_uses_measured_bounded_calibration_timeout() -> None:
    root = Path(__file__).resolve().parents[2]
    submitter = (root / "benchmarks/scc/submit_prod_dag.sh").read_text(
        encoding="utf-8")
    assert "prepare|fixed) timeout=3600 ;;" in submitter
    assert "calibration) timeout=5400 ;;" in submitter
    assert "300 <= int(timeout) <= 42600" in submitter
    assert "stress_calibration_timeout_seconds" in submitter
    assert "timeout=$stress_calibration_timeout" in submitter
    assert "timeout=${stress_full_timeout:?missing measured stress admission timeout}" \
        in submitter
    assert 'stress_timeout_argument=auto' in submitter
    assert 'validate_prior_phase production' in submitter
    assert "hard_seconds=$(( timeout + 600 ))" in submitter
    assert "(( hard_seconds <= 43200 ))" in submitter


def test_privacy_safe_collector_does_not_preserve_remote_modes() -> None:
    root = Path(__file__).resolve().parents[2]
    collector = (root / "benchmarks/scc/collect_prod_summary.sh").read_text(
        encoding="utf-8")
    assert "rsync -rOv --prune-empty-dirs" in collector
    assert "rsync -av" not in collector
    assert "rsync -rtv" not in collector
    assert "--include='/validation/*.pass'" in collector
    assert "--exclude='*'" in collector


def embedded_submitter_selection_parser() -> str:
    root = Path(__file__).resolve().parents[2]
    submitter = (root / "benchmarks/scc/submit_prod_dag.sh").read_text(
        encoding="utf-8")
    section = submitter.split(
        "# KSS_SELECTION_CSV_PARSER_BEGIN", 1
    )[1].split("# KSS_SELECTION_CSV_PARSER_END", 1)[0]
    return section.split("<<'PY'\n", 1)[1].rsplit("\nPY", 1)[0]


def run_embedded_submitter_selection_parser(
    selection: Path,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, "-", str(selection)],
        input=embedded_submitter_selection_parser(),
        text=True,
        capture_output=True,
        check=False,
    )


def selection_admission_row() -> dict[str, str]:
    return {
        "node_class_multiset":
            '[["x86_64","Intel, Xeon",64],["x86_64","AMD EPYC",64]]',
        "auto_node_class_multisets":
            '{"8":[["x86_64","Intel, Xeon",64]],'
            '"16":[["x86_64","AMD EPYC",64]]}',
        "preconditioner": "auto",
        "batch": "16",
        "processors": "4",
        "memory_gib": "56",
        "timeout_seconds": "4321",
        "stress_calibration_timeout_seconds": "8642",
    }


def test_submitter_selection_parser_honors_quoted_embedded_commas(
    tmp_path: Path,
) -> None:
    selection = tmp_path / "calibration_selection.csv"
    write_csv(selection, [selection_admission_row()])
    raw = selection.read_text(encoding="utf-8")
    assert '"Intel, Xeon"' in raw

    result = run_embedded_submitter_selection_parser(selection)
    assert result.returncode == 0, result.stderr
    assert result.stdout == "auto\t16\t4\t56\t4321\t8642\n"

    root = Path(__file__).resolve().parents[2]
    submitter = (root / "benchmarks/scc/submit_prod_dag.sh").read_text(
        encoding="utf-8")
    selection_block = submitter.split(
        "selection=", 1)[1].split("# The full larger-than-CZ18 job", 1)[0]
    assert "awk -F," not in selection_block
    assert "csv.DictReader" in selection_block


@pytest.mark.parametrize(
    ("mutation", "message"),
    [
        ("fractional_timeout", "timeout_seconds must be an integer"),
        ("second_row", "must contain exactly one data row"),
        ("missing_header", "missing required headers: memory_gib"),
    ],
)
def test_submitter_selection_parser_rejects_malformed_admission(
    tmp_path: Path, mutation: str, message: str,
) -> None:
    selection = tmp_path / "calibration_selection.csv"
    row = selection_admission_row()
    rows = [row]
    if mutation == "fractional_timeout":
        row["timeout_seconds"] = "2057.0"
    elif mutation == "second_row":
        rows.append(dict(row))
    elif mutation == "missing_header":
        del row["memory_gib"]
    write_csv(selection, rows)

    result = run_embedded_submitter_selection_parser(selection)
    assert result.returncode != 0
    assert message in result.stderr


def test_selector_and_stress_admission_pin_supported_scc_python() -> None:
    root = Path(__file__).resolve().parents[2]
    runner = (root / "benchmarks/scc/run_prod_stage.sge").read_text(
        encoding="utf-8")
    assert runner.count("module load python3/3.12.4") == 2
    assert runner.count("command -v python3 >/dev/null") == 2
    assert 'python3 "$source_dir/varcomp_kss/benchmarks/scc/' in runner
    assert "module load miniconda/25.3.1" not in runner
    assert 'if [[ "$KSS_STAGE" =~ ^(full|stress2x)$ ]]; then' in runner
    assert "KSS_TIMEOUT_SECONDS <= 42600" in runner
    assert "KSS_TIMEOUT_SECONDS <= 5400" in runner
    assert 'if [[ "$KSS_STAGE" =~ ^(calibration|full|stress2x)$ ]]; then' \
        in runner
    assert 'cmp -s "$output_dir/stress_projection.recomputed.txt"' in runner


def test_submitter_rejects_comma_before_composing_qsub_environment() -> None:
    root = Path(__file__).resolve().parents[2]
    submitter = root / "benchmarks/scc/submit_prod_dag.sh"
    result = subprocess.run(
        ["bash", str(submitter),
         "/projectnb/welfgr/varcomp-kss/runs/bad,run",
         "/projectnb/welfgr/varcomp-kss/bundles/test", "a" * 64,
         "/private/tmp/manifest.tsv", "preflight"],
        text=True, capture_output=True, check=False,
    )
    assert result.returncode == 198
    assert "unsafe comma/newline in qsub value: run_dir" in result.stderr


def test_stress_driver_uses_exact_signed_identifier_spans() -> None:
    root = Path(__file__).resolve().parents[2]
    driver = (root / "benchmarks/scc/kss_prod_driver.do").read_text(
        encoding="utf-8"
    )
    assert "recast double worker firm" in driver
    assert "local worker_shift = `worker_max'-`worker_min'+1" in driver
    assert "local firm_shift = `firm_max'-`firm_min'+1" in driver
    assert "assert abs(worker) < 2^52-2 & abs(firm) < 2^52" in driver


def test_node_characteristics_bind_wrapper_host_to_qacct(tmp_path: Path) -> None:
    evidence = tmp_path / "node_characteristics.txt"
    evidence.write_text(
        "schema=kss_prod_node_v1\n"
        "hostname=node-a.example\n"
        "uname_machine=x86_64\n"
        "cpu_model=Example CPU\n"
        "logical_cpus=64\n",
        encoding="utf-8",
    )
    observed = validate_node_characteristics(evidence, "node-a")
    assert observed["cpu_model"] == "Example CPU"
    with pytest.raises(ValueError, match="node and qacct host disagree"):
        validate_node_characteristics(evidence, "node-b")


def write_csv(path: Path, rows: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def test_selector_and_validator_accept_parallel_three_replica_evidence(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    bundle = "a" * 64
    source = "b" * 40
    manifest = "c" * 64
    prepared = "d" * 64
    wage = "e" * 64
    plan_rows: list[dict[str, object]] = []
    job_number = 900000
    for route in ("auto", "cmg"):
        for batch in ("8", "16", "auto"):
            selected_batch = 8 if batch == "auto" else int(batch)
            route_base = 100.0 if route == "auto" else 50.0
            batch_cost = {"8": 0.0, "16": 10.0, "auto": 20.0}[batch]
            for probes in (20, 40):
                for temperature in ("cold", "warm"):
                    for repetition in CALIBRATION_REPETITIONS:
                        experiment = (
                            f"cal_{route}_b{batch}_p{probes}_{temperature}_r{repetition}"
                        )
                        plan_rows.append({
                            "experiment_id": experiment, "phase": "calibration",
                            "stage": "calibration", "dataset": "cz18",
                            "stata_version": "19", "processors": "4",
                            "memory_gib": "56", "probes": str(probes),
                            "batch": batch, "preconditioner": route,
                            "temperature": temperature, "repetitions": "1",
                            "depends": "cz18_preflight",
                        })
                        command = (route_base + batch_cost + probes + repetition +
                                   (0 if temperature == "cold" else -5))
                        root = tmp_path / "experiments" / experiment
                        result = {
                            "experiment_id": experiment, "repetition": 1,
                            "bundle_sha256": bundle, "source_commit": source,
                            "data_manifest_sha256": manifest,
                            "prepared_sha256": prepared,
                            "estimator_input_sha256": prepared,
                            "wage_input_sha256": wage, "temperature": temperature,
                            "estimator_status": "KSS_POINT_ESTIMATES_ONLY",
                            "algorithm_requested": "jla", "algorithm_selected": "jla",
                            "command_rc": 0, "command_seconds": command,
                            "leverage_seconds": probes / 2,
                            "target_seconds": probes / 2,
                            "correction_seconds": probes, "tolerance": 1e-10,
                            "solver_max_residual": 1e-12, "requested_probes": probes,
                            "seed": 8675309, "declared_memory_gib": 56,
                            "requested_processors": 4, "actual_processors": 4,
                            "batch_requested": batch, "selected_batch": selected_batch,
                            "batch_scratch_forecast_bytes": 7_447_296_000 *
                            selected_batch // 8,
                            "preconditioner_requested": route,
                            "preconditioner_selected": "cmg",
                            "rng_state_reproducible": 1,
                            "route_planned_rhs": 3 * probes + 1,
                            "N_retained": 1000, "worker_levels": 100,
                            "firm_levels": 50, "deletion_units": 200,
                            "route_hybrid_vertices": 10000,
                            "route_hybrid_edges": 20000,
                            "route_hierarchy_levels": 3,
                            "route_terminal_vertices": 1000,
                            "solver_iterations": 1,
                            "fallback_status": "NOT_NEEDED",
                            "routing_reason": "deterministic calibration fixture",
                            "plugin_worker": 1, "plugin_firm": 1,
                            "plugin_covariance": 0, "plugin_total": 2,
                            "corrected_worker": 1, "corrected_firm": 1,
                            "corrected_covariance": 0, "corrected_total": 2,
                        }
                        write_csv(root / f"prod_{experiment}.csv", [result])
                        write_csv(root / "rhs_repetition_1.csv", [{
                            "experiment_id": experiment, "repetition": 1,
                            "stage": "leverage", "batch_start": 1, "rhs": rhs,
                            "iterations": 1, "relative_residual": 1e-12,
                            "converged": 1,
                        } for rhs in range(1, 3 * probes + 2)])
                        (root / "retained_matches.csv").write_text(
                            "worker,firm\n1,1\n", encoding="utf-8")
                        (root / "resources.txt").write_text(
                            "Maximum resident set size (kbytes): 1024\n",
                            encoding="utf-8")
                        (root / "wrapper.pass").write_text(
                            f"KSS_PROD_WRAPPER_PASS {experiment} {bundle} {source} {manifest}\n",
                            encoding="utf-8")
                        host = f"node-{repetition}"
                        (root / "run.metadata.txt").write_text(
                            f"experiment={experiment} stage=calibration bundle={bundle} "
                            f"source={source} manifest={manifest} job={job_number + 1} "
                            f"host={host} slots=4\n",
                            encoding="utf-8")
                        (root / "node_characteristics.txt").write_text(
                            "schema=kss_prod_node_v1\n"
                            f"hostname={host}\n"
                            "uname_machine=x86_64\n"
                            "cpu_model=Example CPU\n"
                            "logical_cpus=64\n",
                            encoding="utf-8")
                        job_number += 1
                        submissions = tmp_path / "submissions"
                        submissions.mkdir(exist_ok=True)
                        (submissions / f"{experiment}.job_id").write_text(
                            f"{job_number}\n", encoding="utf-8")
                        qacct = tmp_path / "qacct"
                        qacct.mkdir(exist_ok=True)
                        (qacct / f"{experiment}.txt").write_text(
                            f"jobnumber {job_number}\nfailed 0\nexit_status 0\n"
                            f"slots 4\nmaxvmem 1G\nru_wallclock {command + 10}\n"
                            f"cpu {command}\nhostname {host}\nqname shared.q@{host}\n",
                            encoding="utf-8")
    plan = tmp_path / "plan.tsv"
    with plan.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(plan_rows[0]), delimiter="\t")
        writer.writeheader()
        writer.writerows(plan_rows)
    output = tmp_path / "experiments/calibration_selector/calibration_selection.csv"
    monkeypatch.setattr(sys, "argv", [
        "select_prod_calibration.py", "--run-dir", str(tmp_path),
        "--plan", str(plan), "--bundle-sha", bundle,
        "--source-commit", source, "--data-manifest-sha", manifest,
        "--prepared-sha", prepared, "--wage-sha", wage,
        "--output", str(output),
    ])
    assert selector_main() == 0
    selected = validate_selection(
        tmp_path, source, bundle, manifest, prepared, wage)
    assert selected["preconditioner"] == "auto"
    assert selected["batch_request"] == "8"
    assert selected["measured_job_count"] == "72"
    assert selected["ranking_policy"] == \
        "MEDIAN_TYPICAL_IDENTICAL_NODE_CLASS_MULTISET"
    assert selected["auto_node_characteristics_identical"] == "1"
    assert int(selected["stress_calibration_timeout_seconds"]) == min(
        MAXIMUM_TIMEOUT_SECONDS,
        max(1800, 2 * int(selected["auto_batch8_timeout_seconds"])),
    )

    for probes in (20, 40):
        for temperature in ("cold", "warm"):
            for repetition in CALIBRATION_REPETITIONS:
                node_path = (tmp_path / "experiments" /
                             f"cal_auto_b8_p{probes}_{temperature}_r{repetition}" /
                             "node_characteristics.txt")
                node_path.write_text(
                    node_path.read_text(encoding="utf-8").replace(
                        "cpu_model=Example CPU", "cpu_model=Different CPU"),
                    encoding="utf-8",
                )
    assert selector_main() == 0
    heterogeneous = validate_selection(
        tmp_path, source, bundle, manifest, prepared, wage)
    assert heterogeneous["ranking_policy"] == \
        "CONSERVATIVE_HIGH_HETEROGENEOUS_NODE_CLASS_MULTISET"
    assert heterogeneous["auto_node_characteristics_identical"] == "0"

    metadata_path = (tmp_path / "experiments/cal_auto_b8_p20_cold_r1" /
                     "run.metadata.txt")
    metadata_original = metadata_path.read_text(encoding="utf-8")
    metadata_path.write_text(
        metadata_original.replace(f"source={source}", f"source={'f' * 40}"),
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="run metadata identity changed"):
        validate_selection(tmp_path, source, bundle, manifest, prepared, wage)
    metadata_path.write_text(metadata_original, encoding="utf-8")

    forced_path = (tmp_path / "experiments/cal_cmg_b8_p20_cold_r1" /
                   "prod_cal_cmg_b8_p20_cold_r1.csv")
    with forced_path.open(newline="", encoding="utf-8") as handle:
        forced_rows = list(csv.DictReader(handle))
    forced_rows[0]["corrected_worker"] = "1.1"
    forced_rows[0]["corrected_total"] = "2.1"
    write_csv(forced_path, forced_rows)
    with pytest.raises(ValueError, match="auto/forced target changed"):
        validate_selection(tmp_path, source, bundle, manifest, prepared, wage)
    with pytest.raises(ValueError, match="auto/forced target changed"):
        selector_main()


@pytest.mark.parametrize("algorithm", ["exact", "jla"])
def test_already_fixed_graph_accepts_zero_removal_passes(algorithm: str) -> None:
    validate_fixedpoint_graph_certificate(
        {
            "algorithm_selected": algorithm,
            "graph_retained_edges": "2",
            "graph_fixedpoint_iterations": "0",
        },
        f"install_{algorithm}",
    )


@pytest.mark.parametrize("iterations", ["-1", "0.5"])
def test_invalid_fixedpoint_iteration_certificate_is_rejected(iterations: str) -> None:
    with pytest.raises(
        ValueError,
        match=(r"invalid fixed-point graph certificate: forced_cmg: "
               r"graph_retained_edges=8, graph_fixedpoint_iterations="),
    ):
        validate_fixedpoint_graph_certificate(
            {
                "algorithm_selected": "jla",
                "graph_retained_edges": "8",
                "graph_fixedpoint_iterations": iterations,
            },
            "forced_cmg",
        )


def test_exact_route_still_requires_valid_retained_graph_dimensions() -> None:
    with pytest.raises(
        ValueError,
        match=r"invalid retained graph dimensions: install_auto: graph_retained_edges=1",
    ):
        validate_fixedpoint_graph_certificate(
            {
                "algorithm_selected": "exact",
                "graph_retained_edges": "1",
                "graph_fixedpoint_iterations": "0",
            },
            "install_auto",
        )


def test_cz18_preflight_timeout_is_bound_to_measured_run() -> None:
    assert expected_timeout(
        {"stage": "cz18_preflight", "phase": "preflight"}, None
    ) == 2100
    assert expected_timeout(
        {"stage": "sample_compare", "phase": "preflight"}, None
    ) == 1800
    assert expected_timeout(
        {"stage": "calibration", "phase": "calibration"}, None
    ) == 5400


def test_production_and_stress_timeouts_use_measured_selection(
    tmp_path: Path,
) -> None:
    assert MAXIMUM_TIMEOUT_SECONDS == 42600
    assert MAXIMUM_TIMEOUT_SECONDS + WRAPPER_RUNTIME_MARGIN_SECONDS == \
        SCC_HARD_RUNTIME_CEILING_SECONDS == 43200
    selection = {
        "timeout_seconds": "11317",
        "stress_calibration_timeout_seconds": "28406",
    }
    assert expected_timeout(
        {"stage": "full", "phase": "production"}, selection
    ) == 11317
    assert expected_timeout(
        {"stage": "stress2x", "phase": "production"}, selection
    ) == 28406
    validation = tmp_path / "validation"
    validation.mkdir()
    (validation / "stress_projection.txt").write_text(
        "status=PASS\ntimeout_seconds=23519\n", encoding="utf-8"
    )
    assert expected_timeout(
        {"stage": "stress2x", "phase": "stress"}, selection, tmp_path
    ) == 23519


def test_stress_projection_rejects_timeout_outside_scc_envelope(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path,
) -> None:
    monkeypatch.setattr(sys, "argv", [
        "validate_stress_projection.py",
        "--calibration", *(str(tmp_path / f"missing-r{i}.csv") for i in (1, 2, 3)),
        "--calibration-qacct",
        *(str(tmp_path / f"missing-r{i}.qacct") for i in (1, 2, 3)),
        "--calibration-job-id-file",
        *(str(tmp_path / f"missing-r{i}.job") for i in (1, 2, 3)),
        "--calibration-rhs",
        *(str(tmp_path / f"missing-r{i}.rhs") for i in (1, 2, 3)),
        "--calibration-node-characteristics",
        *(str(tmp_path / f"missing-r{i}.node") for i in (1, 2, 3)),
        "--cz18-full-result", str(tmp_path / "missing-full.csv"),
        "--retained-sha-file", str(tmp_path / "missing.sha256"),
        "--bundle-sha", "a" * 64, "--source-commit", "b" * 40,
        "--manifest-sha", "c" * 64, "--timeout", "42601",
        "--output", str(tmp_path / "out.txt"),
    ])
    with pytest.raises(ValueError, match="invalid stress timeout"):
        stress_projection_main()


def test_stress_projection_uses_parallel_repetition_upper_envelopes() -> None:
    calibrations = [
        {"alpha_seconds": 1000.0, "beta_seconds_per_probe": 40.0},
        {"alpha_seconds": 1200.0, "beta_seconds_per_probe": 35.0},
        {"alpha_seconds": 900.0, "beta_seconds_per_probe": 45.0},
    ]
    alpha, beta, projected, timeout = stress_projection_from_calibrations(
        calibrations
    )
    assert alpha == 1200.0
    assert beta == 45.0
    assert projected == 15120
    assert timeout == projected


def test_csv_identity_accepts_observed_stata_serialization_rounding() -> None:
    identity(
        {
            "plugin_worker": ".52192515",
            "plugin_firm": ".44880891",
            "plugin_covariance": ".012490911",
            "plugin_total": ".99571592",
        },
        "plugin",
        "install_auto19",
    )


def test_csv_identity_rejects_material_discrepancy_with_context() -> None:
    with pytest.raises(
        ValueError,
        match=(r"install_auto19: plugin identity failed after CSV serialization: "
               r"total=.*parts=.*difference="),
    ):
        identity(
            {
                "plugin_worker": ".52192515",
                "plugin_firm": ".44880891",
                "plugin_covariance": ".012490911",
                "plugin_total": ".9958",
            },
            "plugin",
            "install_auto19",
        )


def test_selector_accepts_structural_direct_b1_reason_without_fallback() -> None:
    validate_selector_route(
        {
            "preconditioner_selected": "diagonal",
            "routing_reason": "fewer than 256 firm coordinates",
            "fallback_status": "NOT_NEEDED",
        },
        "selector",
    )


@pytest.mark.parametrize(
    ("route", "fallback"),
    [("cmg", "NOT_NEEDED"), ("diagonal", "CMG_SETUP_FAILED")],
)
def test_selector_rejects_nondirect_b1_route(route: str, fallback: str) -> None:
    with pytest.raises(
        ValueError,
        match=(r"selector: easy automatic graph did not route directly to B1: "
               r"preconditioner_selected=.*fallback_status="),
    ):
        validate_selector_route(
            {
                "preconditioner_selected": route,
                "routing_reason": "deterministic routing diagnostic",
                "fallback_status": fallback,
            },
            "selector",
        )
