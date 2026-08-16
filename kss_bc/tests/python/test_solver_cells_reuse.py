from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SOLVER = (ROOT / "kss_bc_solver.mata").read_text(encoding="utf-8")


def _between(start: str, stop: str) -> str:
    left = SOLVER.index(start)
    right = SOLVER.index(stop, left)
    return SOLVER[left:right]


def test_routed_production_prepares_cells_once_and_reuses_them() -> None:
    routed = _between(
        "struct kssbc_route_result scalar kssbc_solver__jla_routed(",
        "void kssbc__stata_jla_routed(",
    )
    assert routed.count("kssbc_cmg__cells_prepare(") == 1
    assert "kssbc_solver__hierarchy_cells(\n                cells," in routed
    assert "hierarchy = kssbc_solver__hierarchy(\n                base," not in routed


def test_legacy_hierarchy_wrapper_preserves_preflight_and_delegates() -> None:
    wrapper = _between(
        "struct kssbc_cmg__hierarchy scalar kssbc_solver__hierarchy(\n",
        "real matrix kssbc_solver__pilot_rhs(",
    )
    assert wrapper.count("kssbc_cmg__cells_prepare(") == 1
    assert wrapper.count("kssbc_cmg__preflight(") == 1
    assert "return(kssbc_solver__hierarchy_cells(" in wrapper


def test_slot_22_accumulates_single_cells_pass_and_hierarchy_build() -> None:
    routed = _between(
        "struct kssbc_route_result scalar kssbc_solver__jla_routed(",
        "void kssbc__stata_jla_routed(",
    )
    assert "Route diagnostic slot 22 is cumulative CMG structural-preparation" in routed
    assert routed.count("timer_on(90)") == 3
    assert routed.count("cells = kssbc_cmg__cells_prepare(") == 1
    assert "hierarchy_seconds = kssbc__timer_seconds(90)" in routed
