from __future__ import annotations

from pathlib import Path

from fevc.benchmarks.fevc_matlab_2026.build_benchmark_ado import build as build_ado
from fevc.benchmarks.fevc_matlab_2026.build_bundles import build_rows as build_bundles
from fevc.benchmarks.fevc_matlab_2026.build_manifest import build_rows
from fevc.benchmarks.fevc_matlab_2026.common import (
    CORE_GRID,
    REPLICATES,
    ROW_GRID,
    STRUCTURES,
    TASK_FIELDS,
    validate_task,
)


def cells() -> list[dict[str, str]]:
    rows = build_rows("a" * 40, "b" * 64,
                      mem_per_core_gib=8, command_memory_gib=192)
    return [{field: str(row[field]) for field in TASK_FIELDS} for row in rows]


def test_main_manifest_is_registered_two_slice_matrix() -> None:
    rows = cells()
    assert len(rows) == 240
    expected = {(n, 28) for n in ROW_GRID}
    expected |= {(491_520, cores) for cores in CORE_GRID}
    assert len(expected) == 10
    for row in rows:
        validate_task(row)
    for structure in STRUCTURES:
        for replicate, _, order in REPLICATES:
            selected = [row for row in rows
                        if row["structure"] == structure
                        and int(row["replicate"]) == replicate]
            assert len(selected) == 10
            assert {(int(row["rows"]), int(row["active_cores"]))
                    for row in selected} == expected
            assert {row["execution_order"] for row in selected} == {order}


def test_bundles_partition_cells_and_balance_order() -> None:
    rows = cells()
    bundles = build_bundles(rows)
    assert len(bundles) == 24
    ids = [int(value) for bundle in bundles
           for value in str(bundle["cell_task_ids"]).split(",")]
    assert sorted(ids) == list(range(1, 241))
    assert all(int(bundle["cell_count"]) == 10 for bundle in bundles)
    assert sum(bundle["execution_order"] == "rust,matlab" for bundle in bundles) == 12
    assert sum(bundle["execution_order"] == "matlab,rust" for bundle in bundles) == 12


def test_native_adapter_allows_registered_whole_node_grid(tmp_path: Path) -> None:
    root = Path(__file__).parents[3]
    output = tmp_path / "fevc.ado"
    receipt = tmp_path / "receipt.json"
    value = build_ado(root / "fevc.ado", output, receipt)
    assert value["allowed_native_threads"] == [1, 2, 4, 8, 14, 28]
    source = output.read_text(encoding="utf-8")
    assert "inlist(`benchmark_threads',1,2,4,8,14,28)" in source


def test_scc_scripts_pin_registered_platform_without_restricted_data() -> None:
    harness = Path(__file__).parents[1]
    main = (harness / "run_task.sge").read_text(encoding="utf-8")
    cell = (harness / "run_cell.sh").read_text(encoding="utf-8")
    prepare = (harness / "prepare_artifacts.sge").read_text(encoding="utf-8")
    assert "#$ -pe omp 28" in main
    assert "#$ -l cpu_type=E5-2680v4" in main
    assert "#$ -l h_rt=08:00:00" in main
    assert "module load matlab/2026a" in cell
    assert "module load matlab/2026a" in prepare
    combined = main + cell + prepare
    assert "/projectnb/welfgr/vckss/runs/" in combined
    assert "cz18" not in combined.lower()


def test_memory_phase_contract_is_wired_end_to_end() -> None:
    harness = Path(__file__).parents[1]
    cell = (harness / "run_cell.sh").read_text(encoding="utf-8")
    monitor = (harness / "monitor_process_tree.py").read_text(encoding="utf-8")
    stata = (harness / "stata_run.do").read_text(encoding="utf-8")
    matlab = (harness / "matlab_run.m").read_text(encoding="utf-8")
    for marker in ("empty.ready", "data.ready", "phase.start", "phase.end"):
        assert marker in cell
    for field in ("empty_rss_kib_median", "data_rss_kib_median",
                  "phase_peak_rss_kib", "empty_pss_kib_last",
                  "data_pss_kib_last"):
        assert field in monitor
    assert "EMPTY_READY rust" in stata and "DATA_READY rust" in stata
    assert "EMPTY_READY matlab" in matlab and "DATA_READY matlab" in matlab


def test_preparation_and_runtime_share_thread_contract() -> None:
    harness = Path(__file__).parents[1]
    preparation = (harness / "prepare_artifacts.sge").read_text(encoding="utf-8")
    runner = (harness / "run_cell.sh").read_text(encoding="utf-8")
    adapter = (harness / "build_benchmark_ado.py").read_text(encoding="utf-8")
    contract = "FEVC-BENCHMARK-THREADS-V1"
    assert f"benchmark_thread_contract\\t{contract}" in preparation
    assert f"VCKSS_BENCHMARK_THREAD_CONTRACT={contract}" in runner
    assert f'THREAD_CONTRACT = "{contract}"' in adapter
    assert "VCKSS-BENCHMARK-THREADS-V1" not in preparation


def test_role_launchers_isolate_python_loader_from_stata_and_matlab() -> None:
    harness = Path(__file__).parents[1]
    runner = (harness / "run_cell.sh").read_text(encoding="utf-8")
    stata_block = runner.split("run_stata() {", 1)[1].split("run_matlab() {", 1)[0]
    matlab_block = runner.split("run_matlab() {", 1)[1].split(
        "failure_stage=applications", 1)[0]
    for block, module in ((stata_block, "stata-mp/19"),
                          (matlab_block, "matlab/2026a")):
        assert "python_current=$python_bin" in block
        assert "module purge" in block and f"module load {module}" in block
        assert "module load python3/3.12.4" not in block
        assert '$python_current --version' in block
