from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOLVER = (ROOT / "varcomp_kss_solver.mata").read_text(encoding="utf-8")


def _between(start: str, stop: str) -> str:
    left = SOLVER.index(start)
    right = SOLVER.index(stop, left)
    return SOLVER[left:right]


def test_routed_production_prepares_cells_once_and_reuses_them() -> None:
    routed = _between(
        "struct vckss_route_result scalar vckss_solver__jla_routed(",
        "void vckss__stata_jla_routed(",
    )
    assert routed.count("vckss_cmg__cells_prepare(") == 1
    assert "vckss_solver__hierarchy_cells(\n                cells," in routed
    assert "hierarchy = vckss_solver__hierarchy(\n                base," not in routed


def test_legacy_hierarchy_wrapper_preserves_preflight_and_delegates() -> None:
    wrapper = _between(
        "struct vckss_cmg__hierarchy scalar vckss_solver__hierarchy(\n",
        "real matrix vckss_solver__pilot_rhs(",
    )
    assert wrapper.count("vckss_cmg__cells_prepare(") == 1
    assert wrapper.count("vckss_cmg__preflight(") == 1
    assert "return(vckss_solver__hierarchy_cells(" in wrapper


def test_slot_22_records_single_structural_setup_pass() -> None:
    routed = _between(
        "struct vckss_route_result scalar vckss_solver__jla_routed(",
        "void vckss__stata_jla_routed(",
    )
    assert "timer_on(90)" in routed
    assert routed.count("timer_on(90)") == 1
    assert routed.count("timer_off(90)") == 1
    assert routed.count("cells = vckss_cmg__cells_prepare(") == 1
    assert routed.count("vckss_solver__hierarchy_cells(") == 1
    assert "hierarchy_seconds = vckss__timer_seconds(90)" in routed
