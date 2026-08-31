from __future__ import annotations

from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
MANUAL = ROOT / "tests" / "manual"


def test_manual_referee_files_are_complete() -> None:
    expected = {
        "README.md",
        "fevc_matlab.ado",
        "fevc_manual_matlab.m",
        "manual_benchmarks.do",
        "manual_validation.do",
    }
    assert {path.name for path in MANUAL.iterdir() if path.is_file()} == expected


def test_manual_matlab_bridge_is_external_and_bounded() -> None:
    ado = (MANUAL / "fevc_matlab.ado").read_text(encoding="utf-8")
    matlab = (MANUAL / "fevc_manual_matlab.m").read_text(encoding="utf-8")
    assert "preserve" in ado and "restore" in ado
    assert "quietly sort `worker' `order' `firm'" in ado
    assert "matlab -batch" not in ado
    assert ' -batch "eval(fileread(' in ado
    assert "codes/leave_out_KSS.m" in ado
    assert "return matrix kss" in ado
    assert "target_identity_scaled_error" in ado
    assert "leave_out_KSS(" in matlab
    assert "canonical_path(core_file)" in matlab
    assert "fullfile(matlab_root, 'codes', 'leave_out_KSS.m')" in matlab
    assert "mex('-silent', '-largeArrayDims'" in matlab
    assert "writetable(result, output_file" in matlab
    assert "FEVC MANUAL MATLAB PASS" in matlab
    assert "copyfile" not in matlab


def test_manual_suites_label_matlab_comparison_honestly() -> None:
    validation = (MANUAL / "manual_validation.do").read_text(encoding="utf-8")
    benchmark = (MANUAL / "manual_benchmarks.do").read_text(encoding="utf-8")
    assert "maintained correction formula is not an equality oracle" in validation
    assert "matlab_exact_descriptive" in validation
    assert "FEVC_REFEREE_SUITE|PASS" in validation
    assert "formulas and random draws are not equalized" in benchmark
    assert "matlab_wrapper_seconds" in benchmark
    assert "FEVC_MANUAL_BENCHMARK|PASS" in benchmark
    for text in (validation, benchmark):
        assert "deletionid(match)" in text
        assert "stayers(movers)" in text
        assert "algorithm(jla)" in text
