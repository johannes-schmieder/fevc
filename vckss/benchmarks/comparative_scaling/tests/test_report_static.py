from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "report"


def test_report_build_is_headless_and_source_bound() -> None:
    compiler = (REPORT / "compile_report.sh").read_text(encoding="utf-8")
    analysis = (REPORT / "analyze_report.py").read_text(encoding="utf-8")
    assert "MPLBACKEND=Agg" in compiler
    assert "MPLCONFIGDIR" in compiler
    assert 'collection.get("validated_tasks") == 297' in analysis
    assert 'collection.get("estimator_calls") == 891' in analysis
    assert 'collection.get("censored_matlab_attempts") == 3' in analysis
    assert 'collection.get("runtime_identity")' in analysis
    assert "to_latex" not in analysis


def test_report_covers_registered_time_memory_and_failure_outputs() -> None:
    analysis = (REPORT / "analyze_report.py").read_text(encoding="utf-8")
    template = (REPORT / "report.tex").read_text(encoding="utf-8")
    for artifact in (
        "time_by_rows_4cores.pdf", "speedup_efficiency.pdf",
        "rust_matlab_time_ratio.pdf", "rust_mata_time_ratio.pdf",
        "memory_by_rows_4cores.pdf", "process_memory_by_rows_4cores.pdf",
        "rust_matlab_memory_ratio.pdf", "rust_mata_memory_ratio.pdf",
        "time_memory_pareto.pdf", "failure_map.pdf",
        "timing_components.tsv", "rust_phase_timing.tsv",
        "memory_budget_guidance.tsv", "core_guidance.tsv",
        "censored_matlab.tsv",
    ):
        assert artifact in analysis or artifact in template
    assert "Exact invocation contract" in template
    assert "leave_out_KSS" in template
    assert "effective_role_cores" in analysis
    assert "if 1 in by_core and maximum in by_core" in analysis
    assert "fit_width=True" in analysis
    assert "axes[0, 1].get_legend_handles_labels()" in analysis
    assert 'f"At least {value:,} s"' in analysis
    assert "capped four-core" in template
    assert "target/effective core counts" in template
