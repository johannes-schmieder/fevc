from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]
MANUAL = ROOT / "tests" / "manual"


def test_manual_referee_files_are_complete() -> None:
    expected = {
        "README.md",
        "fevc_matlab.ado",
        "fevc_manual_matlab.m",
        "fevc_manual_build_ado.py",
        "manual_benchmarks.do",
        "manual_validation.do",
    }
    assert {
        path.name
        for path in MANUAL.iterdir()
        if path.is_file() and not path.name.startswith(".")
    } == expected


def test_manual_machine_paths_use_ignored_stata_config() -> None:
    validation = (MANUAL / "manual_validation.do").read_text(encoding="utf-8")
    benchmark = (MANUAL / "manual_benchmarks.do").read_text(encoding="utf-8")
    example = (MANUAL / ".fevc_manual_local.do.example").read_text(
        encoding="utf-8"
    )
    gitignore = (ROOT.parent / ".gitignore").read_text(encoding="utf-8")
    local_name = ".fevc_manual_local.do"
    for text in (validation, benchmark):
        assert f"/tests/manual/{local_name}" not in text
        assert f"`manual_directory'/{local_name}" in text
        assert "if !_rc include" in text
        assert "/Applications/" not in text
        assert "/Users/" not in text
    assert "local matlab_root" in example
    assert "local matlab_binary" in example
    assert f"fevc/tests/manual/{local_name}" in gitignore


def test_manual_matlab_bridge_is_external_and_bounded() -> None:
    ado = (MANUAL / "fevc_matlab.ado").read_text(encoding="utf-8")
    matlab = (MANUAL / "fevc_manual_matlab.m").read_text(encoding="utf-8")
    assert "preserve" in ado and "restore" in ado
    assert "quietly sort `worker' `order' `firm'" in ado
    assert "matlab -batch" not in ado
    assert ' -batch "eval(fileread(' in ado
    assert "codes/leave_out_KSS.m" in ado
    assert "return matrix kss" in ado
    assert 'local algorithm "default"' in ado
    assert "selected_algorithm" in ado
    assert "target_identity_scaled_error" in ado
    assert "leave_out_KSS(" in matlab
    assert "strcmp(algorithm, 'default')" in matlab
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


def test_manual_benchmark_has_two_requested_three_panel_slices() -> None:
    benchmark = (MANUAL / "manual_benchmarks.do").read_text(encoding="utf-8")
    readme = (MANUAL / "README.md").read_text(encoding="utf-8")
    assert (
        'local problem_types       '
        '"two_way_dense two_way_sparse two_way_bottleneck"'
    ) in benchmark
    assert "three_fe_sparse" not in benchmark
    assert "four independent cross-community matches" in benchmark
    assert '3 "Two-way bottleneck"' in benchmark
    assert "local fixed_threads       8" in benchmark
    assert 'local thread_counts       "1 2 4 8 16"' in benchmark
    assert 'local algorithm_settings  "default"' in benchmark
    assert '"default", "harmonized"' in benchmark
    assert "if `rows' == `medium_dataset_size'" in benchmark
    assert 'local mata_status = cond(`mata_capped\', "capped"' in benchmark
    assert 'local rust_status = cond(`rust_rc\' == 0, "pass", "fail")' in benchmark
    assert 'if `mata_capped\' local arms "rust matlab"' in benchmark
    assert "FEVC-MANUAL-BENCHMARK-THREADS-V1" in benchmark
    assert 'backend(auto) rng(auto)' in benchmark
    assert "e(cmg_threads_requested)" in benchmark
    assert "e(cmg_threads_used)" in benchmark
    assert "Only fevc Mata observations" in benchmark
    assert "backend(mata) rng(stata)" in benchmark
    assert benchmark.count("connected mata_seconds") == 2
    assert benchmark.count("connected rust_seconds") == 2
    assert benchmark.count("connected matlab_seconds") == 2
    assert '1 "fevc (Mata)" 2 "fevc (Rust)"' in benchmark
    assert benchmark.count("by(panel, cols(3)") == 2
    assert "manual_benchmark_time_by_size.png" in benchmark
    assert "manual_benchmark_time_by_cores.png" in benchmark
    assert "exactly two combined timing figures" in readme
    assert "Every panel has three" in readme


def test_manual_benchmark_adapter_is_source_bound(tmp_path: Path) -> None:
    builder = MANUAL / "fevc_manual_build_ado.py"
    source = ROOT / "fevc.ado"
    output = tmp_path / "fevc.ado"
    receipt = tmp_path / "receipt.json"
    result = subprocess.run(
        [
            sys.executable,
            str(builder),
            "--source",
            str(source),
            "--output",
            str(output),
            "--receipt",
            str(receipt),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    value = json.loads(receipt.read_text(encoding="utf-8"))
    adapted = output.read_text(encoding="utf-8")
    assert value["schema"] == "FEVC-MANUAL-BENCHMARK-ADO-V1"
    assert value["transformations"] == 4
    assert "FEVC_MANUAL_BENCHMARK_ADAPTER|PASS" in result.stdout
    assert adapted.count("FEVC_MANUAL_THREAD_CONTRACT") == 1
    assert "threads(`manual_rust_threads')" in adapted
    assert "`cmg_threads_requested'==`manual_rust_threads'" in adapted
    assert "`cmg_threads_used'==`manual_rust_threads'" in adapted
