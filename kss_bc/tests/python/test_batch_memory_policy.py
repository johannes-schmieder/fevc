from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def test_batch_memory_gate_accounts_for_physical_observation_matrix() -> None:
    ado = (ROOT / "kss_bc.ado").read_text(encoding="utf-8")
    forecast = ado.index("local batch_physical_column_bytes = 0")
    gate = ado.index('quietly _kss_bc_post_failure "BATCH_MEMORY_LIMIT"')
    routing = ado.index("kssbc__stata_jla_routed")
    assert forecast < gate < routing
    assert "8*scalar(`retained_physical_total')" in ado
    assert "`batch_scratch_forecast_bytes' > `batch_memory_budget_bytes'" in ado
    assert "ereturn scalar batch_physical_column_bytes" in ado


def test_direct_cmg_guard_binds_api_and_design() -> None:
    ado = (ROOT / "kss_bc.ado").read_text(encoding="utf-8")
    expected = "clean-room-cmg-inspired-degree3-hybrid-v5-robust-hierarchy"
    assert ado.count("kssbc_cmg__api_level() == 5") == 2
    assert ado.count("kssbc_cmg__design_label()") == 2
    assert ado.count(expected) == 1


def test_help_documents_hard_memory_gate_and_disjoint_timers() -> None:
    help_text = (ROOT / "kss_bc.sthlp").read_text(encoding="utf-8")
    assert "{cmd:BATCH_MEMORY_LIMIT}" in help_text
    assert "before routing or random" in help_text
    assert "setup and fit are disjoint" in help_text
    assert "Setup is included within fit time" not in help_text
