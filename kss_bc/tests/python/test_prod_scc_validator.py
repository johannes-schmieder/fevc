from __future__ import annotations

import pytest

from kss_bc.benchmarks.validate_prod_scc import (
    expected_timeout,
    identity,
    recompute_calibration_projection,
    validate_fixedpoint_graph_certificate,
    validate_selector_route,
)
from kss_bc.benchmarks.scc.select_prod_calibration import (
    projection_from_measurements,
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
