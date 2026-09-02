from __future__ import annotations

import json
from pathlib import Path

import pytest

from fevc.benchmarks.fevc_matlab_2026 import monitor_process_tree as monitor_module
from fevc.benchmarks.fevc_matlab_2026.build_benchmark_ado import build as build_ado
from fevc.benchmarks.fevc_matlab_2026.build_bundles import build_rows as build_bundles
from fevc.benchmarks.fevc_matlab_2026.build_manifest import build_rows
from fevc.benchmarks.fevc_matlab_2026.collect_generation import cell_list, task_range
from fevc.benchmarks.fevc_matlab_2026.common import (
    CORE_GRID,
    MAX_ITERATIONS,
    REPLICATES,
    ROW_GRID,
    SMOKE_ESTIMATOR_TIMEOUT_SECONDS,
    SMOKE_HARD_WALL_SECONDS,
    SMOKE_REQUESTED_SLOTS,
    SMOKE_TASK_SCHEMA,
    STRUCTURES,
    TASK_FIELDS,
    EvidenceError,
    parse_qacct,
    sha256,
    validate_task,
)
from fevc.benchmarks.fevc_matlab_2026.merge_generations import merge
from fevc.benchmarks.fevc_matlab_2026.prepare_retry import prepare
from fevc.benchmarks.fevc_matlab_2026.validate_task import (
    RUST_LEGACY_PHASES,
    RUST_NATIVE_PHASES,
    rust_phase_receipt,
)


def cells() -> list[dict[str, str]]:
    rows = build_rows("a" * 40, "b" * 64,
                      mem_per_core_gib=8, command_memory_gib=192)
    return [{field: str(row[field]) for field in TASK_FIELDS} for row in rows]


def test_main_manifest_is_registered_two_slice_matrix() -> None:
    rows = cells()
    assert MAX_ITERATIONS == 10_000
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


def test_smoke_task_is_small_but_uses_the_registered_cell_contract() -> None:
    smoke = dict(cells()[0])
    smoke.update({
        "task_schema": SMOKE_TASK_SCHEMA,
        "experiment_id": "smoke_strong_d2_n7680_c4_r1",
        "active_cores": str(SMOKE_REQUESTED_SLOTS),
        "stata_processors": str(SMOKE_REQUESTED_SLOTS),
        "rust_threads": str(SMOKE_REQUESTED_SLOTS),
        "matlab_workers": str(SMOKE_REQUESTED_SLOTS),
        "requested_slots": str(SMOKE_REQUESTED_SLOTS),
        "command_memory_gib": "32",
        "hard_wall_seconds": str(SMOKE_HARD_WALL_SECONDS),
        "estimator_timeout_seconds": str(SMOKE_ESTIMATOR_TIMEOUT_SECONDS),
    })
    assert validate_task(smoke)["rows"] == "7680"


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
    smoke = (harness / "run_smoke.sge").read_text(encoding="utf-8")
    submit = (harness / "submit_scc.sh").read_text(encoding="utf-8")
    assert "#$ -pe omp 28" in main
    assert "#$ -l cpu_type=E5-2680v4" in main
    assert "#$ -l h_rt=08:00:00" in main
    assert "module load matlab/2026a" in cell
    assert "module load matlab/2026a" in prepare
    assert "#$ -pe omp 4" in smoke
    assert "#$ -l h_rt=00:20:00" in smoke
    assert "cpu_type=" not in smoke
    assert '-hold_jid "$smoke_job"' in submit
    assert '-hold_jid "$pilot_job"' in submit
    assert "VCS_BUNDLE_CELL_FILTER=DUAL_BOUNDARY" in submit
    assert "VCS_BUNDLE_CELL_FILTER=5,235" not in submit
    assert 'test "$VCS_BUNDLE_CELL_FILTER" = DUAL_BOUNDARY' in main
    assert "cell_ids=5,235" in main
    combined = main + cell + prepare + smoke + submit
    assert "/projectnb/welfgr/vckss/runs/" in combined
    assert "cz18" not in combined.lower()


def test_qacct_accepts_the_registered_28_core_parallel_environment(
    tmp_path: Path,
) -> None:
    qacct = tmp_path / "qacct.txt"
    qacct.write_text(
        "jobnumber 123\ntaskid 24\nproject welfgr\ngranted_pe omp28\n"
        "slots 28\nfailed 0\nexit_status 0\nru_wallclock 1\ncpu 1\n"
        "maxvmem 1G\nhostname scc-wh1.scc.bu.edu\n",
        encoding="utf-8",
    )
    assert parse_qacct(qacct, expected_slots=28)["granted_pe"] == "omp28"

    qacct.write_text(qacct.read_text().replace("omp28", "serial"),
                     encoding="utf-8")
    with pytest.raises(EvidenceError, match="scheduler slot contract changed"):
        parse_qacct(qacct, expected_slots=28)


def test_memory_phase_contract_is_wired_end_to_end() -> None:
    harness = Path(__file__).parents[1]
    cell = (harness / "run_cell.sh").read_text(encoding="utf-8")
    monitor = (harness / "monitor_process_tree.py").read_text(encoding="utf-8")
    stata = (harness / "stata_run.do").read_text(encoding="utf-8")
    matlab = (harness / "matlab_run.m").read_text(encoding="utf-8")
    for marker in ("empty.ready", "data.ready", "empty.sampled", "data.sampled",
                   "phase.start", "phase.end"):
        assert marker in cell
    for field in ("empty_rss_kib_median", "data_rss_kib_median",
                  "phase_peak_rss_kib", "empty_pss_kib_last",
                  "data_pss_kib_last"):
        assert field in monitor
    assert "EMPTY_READY rust" in stata and "DATA_READY rust" in stata
    assert "EMPTY_READY matlab" in matlab and "DATA_READY matlab" in matlab
    data_marker = "write_marker(data_ready_file,['DATA_READY matlab ' experiment]);"
    assert data_marker in matlab
    assert "wait_for_marker(data_ack_file,30.0,'DataBaseline');" in matlab
    assert "pause(5.0);" not in matlab
    assert "sleep 1000" not in stata
    assert "empty_ack_written" in monitor and "data_ack_written" in monitor
    assert "nuisance(joint) stayers(movers)" in stata
    assert "maxiter(`maxiter')" in stata
    assert "maxiter(1000)" not in stata
    assert '''"`e(nuisance)'"=="joint" & "`e(stayers)'"=="movers"''' in stata


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


def test_stata_launch_uses_environment_contract_not_long_argv() -> None:
    harness = Path(__file__).parents[1]
    runner = (harness / "run_cell.sh").read_text(encoding="utf-8")
    stata = (harness / "stata_run.do").read_text(encoding="utf-8")
    stata_block = runner.split("run_stata() {", 1)[1].split("run_matlab() {", 1)[0]
    assert 'stata-mp -q do "$harness/stata_run.do"' in stata_block
    assert '"$package" "$scratch/input/input.csv"' not in stata_block
    required = {
        "PACKAGE_ROOT", "INPUT_CSV", "OUTPUT_CSV", "EMPTY_READY", "DATA_READY",
        "PHASE_START", "PHASE_END", "ROLE", "SOURCE_COMMIT", "TASK_SHA",
        "INPUT_SHA", "STRUCTURE", "CONNECTIVITY", "ROWS", "DEGREE", "PROBES",
        "SEED", "CORES", "PROCESSORS", "RUST_THREADS", "MEMORY", "TIMEOUT",
        "EMPTY_ACK", "DATA_ACK", "REQUESTED_SLOTS", "MAXITER",
    }
    for field in required:
        name = f"VCS_STATA_{field}"
        assert name in stata_block
        assert f": environment {name}" in stata
    assert not any(line.startswith("args ") for line in stata.splitlines())


def test_monitor_acknowledges_two_complete_baseline_samples(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch,
) -> None:
    empty_ready = tmp_path / "empty.ready"
    data_ready = tmp_path / "data.ready"
    empty_ack = tmp_path / "empty.sampled"
    data_ack = tmp_path / "data.sampled"
    phase_start = tmp_path / "phase.start"
    phase_end = tmp_path / "phase.end"
    output = tmp_path / "process_tree.json"
    empty_ready.write_text("READY\n", encoding="utf-8")

    def snapshot(_root: Path) -> dict[int, tuple[int, int]]:
        return {} if phase_end.exists() else {100: (1, 2048)}

    def advance(_seconds: float) -> None:
        if empty_ack.exists() and not data_ready.exists():
            data_ready.write_text("READY\n", encoding="utf-8")
        elif data_ack.exists() and not phase_start.exists():
            phase_start.write_text("START\n", encoding="utf-8")
        elif phase_start.exists() and not phase_end.exists():
            phase_end.write_text("END\n", encoding="utf-8")

    monkeypatch.setattr(monitor_module, "process_snapshot", snapshot)
    monkeypatch.setattr(monitor_module, "process_tree_pss_kib",
                        lambda _root, _pids: 1024)
    monkeypatch.setattr(monitor_module.time, "sleep", advance)
    rc = monitor_module.monitor(
        100, output, interval=0.05, empty_ready=empty_ready,
        data_ready=data_ready, empty_ack=empty_ack, data_ack=data_ack,
        phase_start=phase_start, phase_end=phase_end,
        proc_root=tmp_path / "proc",
    )
    receipt = json.loads(output.read_text(encoding="utf-8"))
    assert rc == 0
    assert receipt["status"] == "PASS"
    assert receipt["empty_sample_count"] >= 2
    assert receipt["data_sample_count"] >= 2
    assert empty_ack.read_text().strip() == "EMPTY_BASELINE_SAMPLED"
    assert data_ack.read_text().strip() == "DATA_BASELINE_SAMPLED"


def test_campaign_has_one_artifact_lineage_and_no_heartbeat_contract() -> None:
    harness = Path(__file__).parents[1]
    sources = "\n".join(
        (harness / name).read_text(encoding="utf-8")
        for name in ("build_run.py", "prepare_artifacts.sge", "submit_scc.sh")
    )
    assert "artifact_source_run_id" not in sources
    assert "pilot_small_run_id" not in sources
    assert "heartbeat" not in sources.lower()


def test_collection_can_validate_a_frozen_run_with_a_later_collector() -> None:
    harness = Path(__file__).parents[1]
    generation = (harness / "collect_generation.py").read_text(encoding="utf-8")
    collection = (harness / "collect_campaign.sh").read_text(encoding="utf-8")
    aggregate = (harness / "aggregate.py").read_text(encoding="utf-8")
    assert "Path(__file__).resolve().parent" in generation
    assert 'harness=$script_dir' in collection
    assert "collector_source_commit" in generation
    assert "collector_source_commit" in collection
    assert "collector_source_commit" in aggregate


def test_smoke_and_pilot_gate_on_application_validation_before_release() -> None:
    harness = Path(__file__).parents[1]
    smoke = (harness / "run_smoke.sge").read_text(encoding="utf-8")
    pilot = (harness / "run_task.sge").read_text(encoding="utf-8")
    for source in (smoke, pilot):
        assert "validate_task.py" in source
        assert "--application-only --require-rankable" in source
    assert "application.pass.json" in smoke
    assert "application.$pilot_task_id.pass.json" in pilot
    assert 'cell_task_ids\\t5,235' in pilot
    receipt_block = smoke.split("failure_stage=receipt", 1)[1]
    assert "module load python3/3.12.4" in receipt_block
    assert "python_bin=$(command -v python3)" in receipt_block


def test_validator_accepts_the_documented_compressed_engine_receipt() -> None:
    harness = Path(__file__).parents[1]
    validator = (harness / "validate_task.py").read_text(encoding="utf-8")
    assert '"KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES"' in validator
    assert 'value.get("estimator_status") == "OK"' not in validator
    assert 'value.get("engine") == "compressed"' in validator
    assert 'value.get("engine") == "rust"' not in validator


def test_validator_allows_blank_legacy_but_requires_native_rust_phases() -> None:
    receipt = {name: "" for name in RUST_LEGACY_PHASES}
    receipt.update({name: "0" for name in RUST_NATIVE_PHASES})
    phases = rust_phase_receipt(receipt)
    assert all(phases[name] is None for name in RUST_LEGACY_PHASES)
    assert all(phases[name] == 0.0 for name in RUST_NATIVE_PHASES)

    incomplete = dict(receipt)
    incomplete["rust_solve_seconds"] = ""
    with pytest.raises(EvidenceError, match="native phase receipt incomplete"):
        rust_phase_receipt(incomplete)

    nonfinite = dict(receipt)
    nonfinite["selection_seconds"] = "not-a-number"
    with pytest.raises(EvidenceError, match="nonfinite Rust selection_seconds"):
        rust_phase_receipt(nonfinite)


def test_retry_ranges_are_sparse_bounded_and_unambiguous() -> None:
    assert task_range("1,3-4,24") == [1, 3, 4, 24]
    assert cell_list("5,235") == [5, 235]
    for bad in ("1,1", "0", "25", "3-2"):
        with pytest.raises(ValueError):
            task_range(bad)
    for bad in ("5,5", "0", "241", "5,,235"):
        with pytest.raises(ValueError):
            cell_list(bad)


def test_retry_contract_is_single_use_and_scheduler_failure_only() -> None:
    harness = Path(__file__).parents[1]
    retry = (harness / "retry_scc.sh").read_text(encoding="utf-8")
    gate = (harness / "prepare_retry.py").read_text(encoding="utf-8")
    collection = (harness / "collect_campaign.sh").read_text(encoding="utf-8")
    assert 'test ! -e "$run_dir/submissions/retry.tsv"' in retry
    assert 'fields["failed"] != "0"' in gate
    assert "merge_generations.py" in collection
    assert "production.original.generation.json" in collection


def test_retry_accepts_only_failed_scheduler_bundles(tmp_path: Path) -> None:
    run_dir = tmp_path / "run"
    (run_dir / "submissions").mkdir(parents=True)
    run_identity = {
        "schema": "FEVC-MATLAB-2026-CAMPAIGN-V2",
        "source_mode": "CLEAN_COMMIT",
        "run_id": "run",
        "source_commit": "a" * 40,
    }
    (run_dir / "run_identity.json").write_text(json.dumps(run_identity))
    (run_dir / "submissions" / "campaign.tsv").write_text(
        "key\tvalue\nproduction_job_id\t123\n", encoding="utf-8",
    )
    qacct = tmp_path / "qacct.txt"
    qacct.write_text(
        "=" * 20 + "\nqname all.q\njobnumber 123\ntaskid 24\n"
        "project welfgr\nslots 28\nfailed 100\nexit_status 0\n",
        encoding="utf-8",
    )
    plan = prepare(run_dir, "24", qacct)
    assert plan["retry_bundle_task_ids"] == [24]
    assert plan["retry_bundle_range"] == "24"
    with pytest.raises(ValueError, match="contiguous SGE range"):
        prepare(run_dir, "22,24", qacct)
    qacct.write_text(qacct.read_text().replace("failed 100", "failed 0"))
    with pytest.raises(ValueError, match="scheduler or execution-host"):
        prepare(run_dir, "24", qacct)


def test_retry_merge_preserves_both_attempts_and_covers_all_cells(
    tmp_path: Path,
) -> None:
    run_dir = tmp_path / "run"
    (run_dir / "attempts" / "production" / "validations").mkdir(parents=True)
    (run_dir / "attempts" / "retry" / "validations").mkdir(parents=True)
    identity = {
        "schema": "FEVC-MATLAB-2026-CAMPAIGN-V2",
        "source_mode": "CLEAN_COMMIT",
        "run_id": "run",
    }
    (run_dir / "run_identity.json").write_text(json.dumps(identity))
    inventory_paths = {}
    for attempt_id, selected, bundles, skipped, job_id in (
        ("production", range(1, 231), list(range(1, 24)), [24], "123"),
        ("retry", range(231, 241), [24], [], "124"),
    ):
        hashes = {}
        for cell_id in selected:
            path = run_dir / "attempts" / attempt_id / "validations" / f"{cell_id}.json"
            path.write_text(json.dumps({"task_id": cell_id}), encoding="utf-8")
            hashes[str(cell_id)] = sha256(path)
        inventory = {
            "schema": "FEVC-MATLAB-2026-GENERATION-INVENTORY-V1",
            "status": "PASS",
            "run_id": "run",
            "attempt_id": attempt_id,
            "job_id": job_id,
            "expected_bundle_ids": bundles,
            "skipped_bundle_ids": skipped,
            "validated_cell_ids": list(selected),
            "validation_sha256": hashes,
        }
        path = tmp_path / f"{attempt_id}.json"
        path.write_text(json.dumps(inventory), encoding="utf-8")
        inventory_paths[attempt_id] = path
    target = tmp_path / "merged"
    result = merge(
        run_dir, inventory_paths["production"], inventory_paths["retry"], target,
    )
    assert result["validated_cells"] == 240
    assert result["retry_bundle_ids"] == [24]
    assert len(list(target.glob("*.json"))) == 240
