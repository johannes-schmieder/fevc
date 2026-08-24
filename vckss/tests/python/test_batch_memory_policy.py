from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def test_batch_forecast_accounts_for_physical_observation_matrix() -> None:
    ado = (ROOT / "vckss.ado").read_text(encoding="utf-8")
    forecast = ado.index("local batch_physical_column_bytes =")
    routing = ado.index("vckss__stata_jla_routed")
    assert forecast < routing
    assert "8*scalar(`retained_physical_total')" in ado
    assert 'quietly _vckss_post_failure "BATCH_MEMORY_LIMIT"' not in ado
    assert "Percentage batch budgets select a practical automatic width" in ado
    assert "ereturn scalar batch_physical_column_bytes" in ado


def test_automatic_batch_caps_are_evidence_backed() -> None:
    ado = (ROOT / "vckss.ado").read_text(encoding="utf-8")
    assert "cond(`active_processors'>=8,64,32)" in ado
    assert "foreach candidate in 16 32 64" in ado
    assert "foreach candidate in 16 32 64 128" not in ado
    assert "capture confirm integer number `batch_requested'" in ado
    assert "batch() must be auto or a positive integer" in ado


def test_direct_cmg_guard_binds_api_and_design() -> None:
    ado = (ROOT / "vckss.ado").read_text(encoding="utf-8")
    expected = "gpl-cmg-mata-degree3-hybrid-v8-vckss-component"
    assert ado.count("vckss_cmg__api_level() == 8") == 2
    assert ado.count("vckss_cmg__design_label()") == 2
    assert ado.count(expected) == 1


def test_help_documents_direct_memory_gate_and_disjoint_timers() -> None:
    help_text = (ROOT / "vckss.sthlp").read_text(encoding="utf-8")
    assert "complete direct-peak forecast is the memory" in help_text
    assert "percentage and processor rules are" in help_text
    assert "routing trial solves" in help_text
    assert "setup and fit are disjoint" in help_text
    assert "Setup is included within fit time" not in help_text
