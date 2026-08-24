from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCC = ROOT / "vckss/benchmarks/scc"


def test_synthetic_generator_has_exact_registered_dimensions() -> None:
    source = (SCC / "numopt2_generate.do").read_text(encoding="utf-8")
    compact = "".join(source.split())
    assert "`workers'!=40*`firms'" in compact
    assert "localcells=`workers'*`cells_per_worker'" in compact
    assert "localrows=`cells'*`rows_per_cell'" in compact
    assert "byworkerfirm,sort:assert_N==`rows_per_cell'" in compact
    assert "egen group" not in source
    assert "KSS_NUMOPT2_INPUT_PASS" in source


def test_scc_wrapper_keeps_each_estimate_in_one_stata_process() -> None:
    source = (SCC / "run_numopt2_scale.sge").read_text(encoding="utf-8")
    assert source.count("kss_scale_driver.do") == 1
    assert "stata-mp -q do vckss/benchmarks/scc/kss_scale_driver.do" in source
    assert "KSS_STATA_PROCESSORS" in source
    assert "KSS_NUMOPT2_WRAPPER_PASS" in source
    assert "/usr/bin/time -v" in source


def test_submission_is_scalar_source_bound_and_explicitly_resourced() -> None:
    source = (SCC / "submit_numopt2_scale.sh").read_text(encoding="utf-8")
    assert "qsub -verify" in source
    assert "qsub -terse" in source
    assert "-t " not in source
    assert "-P welfgr" in source
    assert "-pe omp" in source
    assert "mem_per_core=" in source
    assert "KSS_TASK_SHA256" in source


def test_external_validator_requires_all_three_evidence_layers() -> None:
    source = (SCC / "validate_numopt2_scale.py").read_text(encoding="utf-8")
    compact = "".join(source.split())
    assert 'accounting["failed"]=="0"' in compact
    assert 'accounting["exit_status"]=="0"' in compact
    assert 're.fullmatch(r"omp([0-9]+)",granted_pe)' in compact
    assert 'int(queue_pe.group(1))==task_slots' in compact
    assert 'output/"wrapper.pass"' in compact
    assert 'output/"stata.pass"' in compact
    assert 'summary["engine_selected"]=="compressed"' in compact
    assert 'integer(summary,"target_strata")==cells' in compact
    assert 'num(summary,"solver_max_residual")<=' in compact
