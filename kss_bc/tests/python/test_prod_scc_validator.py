from __future__ import annotations

import pytest

from kss_bc.benchmarks.validate_prod_scc import (
    expected_timeout,
    identity,
    validate_fixedpoint_graph_certificate,
    validate_selector_route,
)


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
