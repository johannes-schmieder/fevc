from __future__ import annotations

import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HARNESS = ROOT / "benchmarks/alpha"


def analyzer_module():
    path = HARNESS / "analyze.py"
    spec = importlib.util.spec_from_file_location("vckss_alpha_analyze", path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def parity_row(backend: str, delta: float = 0.0) -> dict[str, str]:
    module = analyzer_module()
    row = {
        "case_id": "smoke_compressed",
        "backend": backend,
        "n_stored": "30",
        "n_physical": "60",
        "n_retained": "30",
        "workers": "5",
        "firms": "3",
        "cells": "15",
        "units": "15",
        "strata": "18",
        "probes": "40",
        "sample_n": "30",
        "deletion": "match",
        "controls": "0",
        "processors": "4",
    }
    for field in module.RESULT_FIELDS:
        row[field] = str(1.0 + delta)
    for column in range(1, 5):
        row[f"mcse{column}"] = "0.01"
    return row


def test_driver_uses_unowned_complete_command_timer() -> None:
    source = (HARNESS / "synthetic_driver.do").read_text(encoding="utf-8")
    assert "timer clear 80" in source
    assert "timer on 80" in source
    assert "local total = r(t80)" in source
    for timer in range(91, 99):
        assert f"timer on {timer}" not in source
        assert f"timer clear {timer}" not in source


def test_receipt_width_matches_registered_column_names() -> None:
    source = (HARNESS / "synthetic_driver.do").read_text(encoding="utf-8")
    columns = (
        source.split("matrix colnames receipt =", 1)[1]
        .split("\n\nlocal rhs", 1)[0]
        .replace("///", " ")
        .split()
    )
    assert len(columns) == 62
    assert "matrix receipt = J(" + chr(96) + "reps',62,.)" in source
    assert len(set(columns)) == len(columns)


def test_driver_checks_science_and_complete_caller_state() -> None:
    source = (HARNESS / "synthetic_driver.do").read_text(encoding="utf-8")
    for contract in (
        "complete_residual_max",
        "residual_acceptance_tolerance",
        "target_identity_residual",
        "capture confirm matrix e(V)",
        "sample_ok",
        "data_ok",
        "rng_ok",
        "sort_ok",
    ):
        assert contract in source
    assert "backend(rust) rng(counter_v1)" in source
    assert "backend(mata) rng(stata)" in source


def test_runner_requires_clean_source_and_fresh_backend_processes() -> None:
    source = (HARNESS / "run_local.py").read_text(encoding="utf-8")
    assert 'git(repo, "status", "--porcelain")' in source
    assert 'for backend in ("rust", "mata")' in source
    assert '"/usr/bin/time"' in source
    assert '"-b"' in source
    assert '"-q"' not in source
    assert "application_log = log.read_text" in source
    assert '"peak_rss_bytes"' in source
    assert "VCKSS_ALPHA_LOCAL_PASS" in source
    assert "timezone.utc" in source
    assert "from datetime import UTC" not in source
    assert "KSS_POINT_ESTIMATES_ONLY" in source
    assert "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES" in source


def test_cross_backend_parity_uses_combined_mcse() -> None:
    module = analyzer_module()
    rust = [parity_row("rust")]
    mata = [parity_row("mata", delta=0.02)]
    result = module.cross_backend_parity("smoke_compressed", rust, mata)
    assert abs(result["max_combined_mcse_units"] - 2**0.5) < 1e-12
    assert result["status"] == "PASS"
    mata = [parity_row("mata", delta=0.1)]
    result = module.cross_backend_parity("smoke_compressed", rust, mata)
    assert result["max_combined_mcse_units"] > 6
    assert result["status"] == "FAIL"


def test_cross_backend_gate_uses_corrected_targets_only() -> None:
    module = analyzer_module()
    rust = parity_row("rust")
    mata = parity_row("mata")
    for field in ("r11", "r21", "r41"):
        mata[field] = "1000"
    result = module.cross_backend_parity("smoke_compressed", [rust], [mata])
    assert result["max_absolute_result_difference"] == 0
    assert result["max_equivalence_limit_ratio"] == 0
    assert result["status"] == "PASS"


def test_cross_backend_gate_has_registered_numerical_floor() -> None:
    module = analyzer_module()
    rust = parity_row("rust")
    mata = parity_row("mata")
    for column in range(1, 5):
        rust[f"mcse{column}"] = "0"
        mata[f"mcse{column}"] = "0"
        mata[f"r3{column}"] = str(1 + 0.5e-8)
    result = module.cross_backend_parity("smoke_compressed", [rust], [mata])
    assert result["max_equivalence_limit_ratio"] < 1
    assert result["status"] == "PASS"


def test_report_fragments_include_equivalence_limit(tmp_path: Path) -> None:
    module = analyzer_module()
    summary = {
        "status": "INCOMPLETE",
        "source_commit": "a" * 40,
        "generated_at_utc": "2026-08-25T00:00:00+00:00",
        "max_combined_mcse_units": 0.5,
        "max_equivalence_limit_ratio": 0.1,
        "minimum_speedup": 1.0,
    }
    timings = [
        {
            "case_id": "smoke_compressed",
            "rust_cold_seconds": 2.0,
            "rust_warm_median_seconds": 1.0,
            "mata_warm_median_seconds": 1.5,
            "speedup": 1.5,
            "rust_peak_rss_bytes": 2**20,
        }
    ]
    parity = [
        {
            "case_id": "smoke_compressed",
            "max_absolute_result_difference": 1e-9,
            "max_combined_mcse_units": 0.5,
            "max_equivalence_limit_ratio": 0.1,
            "status": "PASS",
        }
    ]
    module.write_tex(
        tmp_path,
        summary=summary,
        timings=timings,
        parity=parity,
        phases=[],
    )
    macros = (tmp_path / "summary_macros.tex").read_text(encoding="utf-8")
    table = (tmp_path / "parity_table.tex").read_text(encoding="utf-8")
    assert "AlphaMaxEquivalenceRatio" in macros
    assert "Maximum corrected gap" in table


def test_report_states_private_alpha_and_platform_boundary() -> None:
    source = (HARNESS / "report/report.tex").read_text(encoding="utf-8")
    assert "not a public release claim" in source
    assert "Windows remains deferred" in source
    assert "native macOS" in source
    assert "Linux on SCC" in source
    assert "JLA is the default" in source
    assert "200 probes" in source
    assert "Maintained MATLAB KSS is the primary performance comparator" in source
    assert "VCKSS-ALPHA-BENCH-V2" in source
