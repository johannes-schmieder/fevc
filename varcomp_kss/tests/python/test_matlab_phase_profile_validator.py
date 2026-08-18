from __future__ import annotations

import csv
import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[3]
VALIDATOR_PATH = ROOT / "varcomp_kss" / "benchmarks" / "validate_matlab_phase_profile.py"
SPEC = importlib.util.spec_from_file_location("matlab_phase_validator", VALIDATOR_PATH)
assert SPEC is not None and SPEC.loader is not None
VALIDATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VALIDATOR)

SOURCE = "a" * 40
BUNDLE = "b" * 64
INPUT = "c" * 64
UPSTREAM = VALIDATOR.EXPECTED_UPSTREAM_COMMIT
CORE = VALIDATOR.EXPECTED_CORE_SHA256
RUNTIME_TREE = VALIDATOR.EXPECTED_RUNTIME_TREE_SHA256
CMG = "e" * 64
CMG_MEX = "f" * 64
CMG_SOLVER = "0" * 64
PROFILER = "1" * 64
TARGET_HASH = "2" * 64
DETAIL_HASH = "3" * 64
LABEL = "cz24_profile"
JOB_ID = "7200001"


def base_record() -> dict[str, object]:
    record: dict[str, object] = {field: "" for field in VALIDATOR.CSV_FIELDS}
    record.update(
        status="PASS",
        failure_code="NONE",
        failure_explanation="NONE",
        label=LABEL,
        source_commit=SOURCE,
        bundle_sha256=BUNDLE,
        input_sha256=INPUT,
        matlab_upstream_commit=UPSTREAM,
        matlab_core_sha256=CORE,
        matlab_cmg_sha256=CMG,
        matlab_cmg_mex_sha256=CMG_MEX,
        matlab_cmg_solver_sha256=CMG_SOLVER,
        profiler_sha256=PROFILER,
        matlab_version="25.2.0.2998904 (R2025b)",
        algorithm="JLA",
        deletion_level="matches",
        profile_timer="real",
        profile_function="leave_out_KSS",
        rng_protocol="client_and_worker_state_restored_parfor_schedule_not_fixed",
        projection_basis="three_call_cz24_calibration",
        process_start_utc="2026-08-16T12:00:00.000000000Z",
        first_matlab_utc="2026-08-16T12:00:10.000000000Z",
        final_matlab_utc="2026-08-16T12:00:40.000000000Z",
        cold_target_sha256=TARGET_HASH,
        profiled_target_sha256=TARGET_HASH,
        warm_target_sha256=TARGET_HASH,
        cold_detail_sha256=DETAIL_HASH,
        profiled_detail_sha256=DETAIL_HASH,
        warm_detail_sha256=DETAIL_HASH,
        cold_retained_key_sha256="4" * 64,
        profiled_retained_key_sha256="4" * 64,
        warm_retained_key_sha256="4" * 64,
        matlab_core_newline_count=632,
        matlab_core_profile_max_line=633,
        processors=4,
        seed=8675309,
        probes=200,
        input_rows=256472,
        input_workers=4063,
        input_firms=1285,
        profile_function_calls=1,
        profile_executed_lines=0,
        profile_line_calls=0,
        targets_identical=1,
        details_identical=1,
        retained_keys_identical=1,
        target_replay_within_gate=1,
        rng_state_restore_verified=1,
        cold_detail_rows=10343,
        profiled_detail_rows=10343,
        warm_detail_rows=10343,
        timeout_seconds=330,
        wrapper_seconds=30.0,
        mex_seconds=2.0,
        import_seconds=1.0,
        pool_seconds=4.0,
        cold_call_seconds=6.0,
        warm_profiled_call_seconds=6.0,
        warm_unprofiled_call_seconds=5.0,
        serialization_seconds=0.02,
        pool_teardown_seconds=1.0,
        profile_top_level_seconds=4.0,
        projected_seconds=100.0,
        cold_target_worker=0.08,
        cold_target_firm=0.03,
        cold_target_covariance=0.01,
        cold_target_total=0.13,
        profiled_target_worker=0.08,
        profiled_target_firm=0.03,
        profiled_target_covariance=0.01,
        profiled_target_total=0.13,
        target_worker=0.08,
        target_firm=0.03,
        target_covariance=0.01,
        target_total=0.13,
        target_replay_max_scaled_diff=0.0,
    )
    total_lines = 0
    total_calls = 0
    phase_seconds = 4.0 / len(VALIDATOR.PHASE_RANGES)
    for phase, first, last in VALIDATOR.PHASE_RANGES:
        lines = min(3, last - first + 1)
        calls = lines + 2
        record[f"phase_{phase}_line_first"] = first
        record[f"phase_{phase}_line_last"] = last
        record[f"phase_{phase}_executed_lines"] = lines
        record[f"phase_{phase}_line_calls"] = calls
        record[f"phase_{phase}_seconds"] = phase_seconds
        total_lines += lines
        total_calls += calls
    record["profile_executed_lines"] = total_lines
    record["profile_line_calls"] = total_calls
    assert set(record) == set(VALIDATOR.CSV_FIELDS)
    return record


def write_aggregate(output_dir: Path, record: dict[str, object]) -> None:
    with (output_dir / "aggregate.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=VALIDATOR.CSV_FIELDS)
        writer.writeheader()
        writer.writerow(record)
    (output_dir / "aggregate.json").write_text(
        json.dumps(record, sort_keys=True) + "\n", encoding="utf-8"
    )
    (output_dir / "wrapper.pass").write_text(
        f"VARCOMP_KSS_MATLAB_PHASE_PROFILE_PASS {LABEL} {BUNDLE} {SOURCE} "
        f"{INPUT} {CORE} {TARGET_HASH} {DETAIL_HASH}\n",
        encoding="utf-8",
    )


@pytest.fixture()
def evidence(tmp_path: Path) -> tuple[Path, dict[str, object]]:
    run_dir = tmp_path / "run"
    output_dir = run_dir / "matlab_phase_profile" / LABEL / "output"
    output_dir.mkdir(parents=True)
    (run_dir / "source_commit.txt").write_text(SOURCE + "\n", encoding="utf-8")
    (run_dir / "bundle.sha256").write_text(BUNDLE + "\n", encoding="utf-8")
    (run_dir / "submissions").mkdir()
    (run_dir / "qacct").mkdir()
    (run_dir / "submissions" / f"matlab_phase_profile_{LABEL}.job_id").write_text(
        JOB_ID + "\n", encoding="utf-8"
    )
    job_dir = output_dir.parent
    submission = {
        "label": LABEL,
        "source_commit": SOURCE,
        "bundle_sha256": BUNDLE,
        "input_sha256": INPUT,
        "matlab_upstream_commit": UPSTREAM,
        "matlab_runtime_tree_sha256": RUNTIME_TREE,
        "matlab_core_sha256": CORE,
        "matlab_cmg_sha256": CMG,
        "matlab_cmg_mex_sha256": CMG_MEX,
        "matlab_cmg_solver_sha256": CMG_SOLVER,
        "profiler_sha256": PROFILER,
        "seed": "8675309",
        "probes": "200",
        "processors": "4",
        "projected_seconds": "100.0",
        "projection_basis": "three_call_cz24_calibration",
        "timeout_seconds": "330",
    }
    (job_dir / "submission.tsv").write_text(
        "".join(f"{key}\t{value}\n" for key, value in submission.items()),
        encoding="utf-8",
    )
    (job_dir / "application.txt").write_text(
        f"VARCOMP_KSS MATLAB PHASE PROFILE PASS: {LABEL}\n", encoding="utf-8"
    )
    (job_dir / "resources.txt").write_text(
        "\tMaximum resident set size (kbytes): 1700000\n", encoding="utf-8"
    )
    (run_dir / "qacct" / f"matlab_phase_profile_{LABEL}.txt").write_text(
        "\n".join(
            (
                f"jobnumber        {JOB_ID}",
                "failed           0",
                "exit_status      0",
                "ru_wallclock     60.000",
                "maxvmem          44.000G",
                "slots            4",
            )
        ) + "\n",
        encoding="utf-8",
    )
    record = base_record()
    write_aggregate(output_dir, record)
    return run_dir, record


def command(run_dir: Path) -> list[str]:
    return [
        sys.executable,
        str(VALIDATOR_PATH),
        "--run-dir", str(run_dir),
        "--label", LABEL,
        "--expected-source-commit", SOURCE,
        "--expected-bundle-sha256", BUNDLE,
        "--expected-input-sha256", INPUT,
        "--expected-upstream-commit", UPSTREAM,
        "--expected-core-sha256", CORE,
        "--expected-cmg-sha256", CMG,
        "--expected-cmg-mex-sha256", CMG_MEX,
        "--expected-cmg-solver-sha256", CMG_SOLVER,
        "--expected-profiler-sha256", PROFILER,
        "--expected-projected-seconds", "100.0",
        "--expected-projection-basis", "three_call_cz24_calibration",
    ]


def run_validator(run_dir: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command(run_dir), text=True, capture_output=True, check=False)


def rewrite(run_dir: Path, record: dict[str, object]) -> None:
    write_aggregate(run_dir / "matlab_phase_profile" / LABEL / "output", record)


def test_valid_profile_evidence_passes(evidence: tuple[Path, dict[str, object]]) -> None:
    run_dir, _ = evidence
    result = run_validator(run_dir)
    assert result.returncode == 0, result.stderr
    assert "MATLAB PHASE PROFILE EVIDENCE PASS" in result.stdout


def test_csv_json_tampering_is_rejected(evidence: tuple[Path, dict[str, object]]) -> None:
    run_dir, _ = evidence
    json_path = run_dir / "matlab_phase_profile" / LABEL / "output" / "aggregate.json"
    record = json.loads(json_path.read_text(encoding="utf-8"))
    record["bundle_sha256"] = "9" * 64
    json_path.write_text(json.dumps(record) + "\n", encoding="utf-8")
    result = run_validator(run_dir)
    assert result.returncode != 0
    assert "CSV/JSON text mismatch" in result.stderr


def test_registered_phase_bounds_are_exact(evidence: tuple[Path, dict[str, object]]) -> None:
    run_dir, record = evidence
    record["phase_leverage_line_first"] = 476
    rewrite(run_dir, record)
    result = run_validator(run_dir)
    assert result.returncode != 0
    assert "registered phase bounds changed" in result.stderr


def test_phase_aggregate_accounting_is_bounded(evidence: tuple[Path, dict[str, object]]) -> None:
    run_dir, record = evidence
    record["phase_leverage_seconds"] = 20.0
    rewrite(run_dir, record)
    result = run_validator(run_dir)
    assert result.returncode != 0
    assert "phase time exceeds profiled call" in result.stderr


def test_direct_timing_accounting_is_bounded(evidence: tuple[Path, dict[str, object]]) -> None:
    run_dir, record = evidence
    record["wrapper_seconds"] = 10.0
    rewrite(run_dir, record)
    result = run_validator(run_dir)
    assert result.returncode != 0
    assert "direct stage timers exceed wrapper time" in result.stderr


def test_reproducibility_tampering_is_rejected(evidence: tuple[Path, dict[str, object]]) -> None:
    run_dir, record = evidence
    record["profiled_target_sha256"] = "4" * 64
    rewrite(run_dir, record)
    result = run_validator(run_dir)
    assert result.returncode != 0
    assert "target exact-replay flag is inconsistent" in result.stderr


def test_bounded_parfor_drift_with_identical_retained_keys_passes(
    evidence: tuple[Path, dict[str, object]],
) -> None:
    run_dir, record = evidence
    record["cold_target_sha256"] = "5" * 64
    record["profiled_target_sha256"] = "6" * 64
    record["cold_detail_sha256"] = "7" * 64
    record["profiled_detail_sha256"] = "8" * 64
    record["targets_identical"] = 0
    record["details_identical"] = 0
    record["cold_target_worker"] = 0.080001
    record["cold_target_total"] = 0.130001
    record["profiled_target_worker"] = 0.079999
    record["profiled_target_total"] = 0.129999
    record["target_replay_max_scaled_diff"] = 0.000002 / 1.080001
    rewrite(run_dir, record)
    result = run_validator(run_dir)
    assert result.returncode == 0, result.stderr


def test_parfor_drift_above_legacy_threshold_is_diagnostic(
    evidence: tuple[Path, dict[str, object]],
) -> None:
    run_dir, record = evidence
    record["cold_target_sha256"] = "5" * 64
    record["targets_identical"] = 0
    record["cold_target_worker"] = 0.0743426087641489
    record["cold_target_firm"] = 0.023885457082427397
    record["cold_target_covariance"] = 0.011345164496846215
    record["cold_target_total"] = 0.12091839484026873
    for prefix in ("profiled_target_", "target_"):
        record[f"{prefix}worker"] = 0.07434833920601823
        record[f"{prefix}firm"] = 0.023879173523817219
        record[f"{prefix}covariance"] = 0.011336085884280967
        record[f"{prefix}total"] = 0.12089968449839739
    record["target_replay_max_scaled_diff"] = 1.6691975042490168e-5
    record["target_replay_within_gate"] = 0
    rewrite(run_dir, record)
    result = run_validator(run_dir)
    assert result.returncode == 0, result.stderr


def test_parfor_drift_diagnostic_must_be_consistent(
    evidence: tuple[Path, dict[str, object]],
) -> None:
    run_dir, record = evidence
    record["target_replay_within_gate"] = 0
    rewrite(run_dir, record)
    result = run_validator(run_dir)
    assert result.returncode != 0
    assert "target replay diagnostic is inconsistent" in result.stderr


def test_unregistered_upstream_commit_is_rejected(
    evidence: tuple[Path, dict[str, object]],
) -> None:
    run_dir, _ = evidence
    invocation = command(run_dir)
    invocation[invocation.index("--expected-upstream-commit") + 1] = "9" * 40
    result = subprocess.run(invocation, text=True, capture_output=True, check=False)
    assert result.returncode != 0
    assert "registered maintained MATLAB commit" in result.stderr


def test_non_r2025b_evidence_is_rejected(
    evidence: tuple[Path, dict[str, object]],
) -> None:
    run_dir, record = evidence
    record["matlab_version"] = "25.1.0.0000000 (R2025a)"
    rewrite(run_dir, record)
    result = run_validator(run_dir)
    assert result.returncode != 0
    assert "MATLAB release is not R2025b" in result.stderr


def test_out_of_range_seed_is_rejected(
    evidence: tuple[Path, dict[str, object]],
) -> None:
    run_dir, _ = evidence
    invocation = command(run_dir)
    invocation.extend(("--expected-seed", str(2**32)))
    result = subprocess.run(invocation, text=True, capture_output=True, check=False)
    assert result.returncode != 0
    assert "invalid expected RNG settings" in result.stderr


def test_array_job_receipt_is_rejected(
    evidence: tuple[Path, dict[str, object]],
) -> None:
    run_dir, _ = evidence
    receipt = run_dir / "submissions" / f"matlab_phase_profile_{LABEL}.job_id"
    receipt.write_text(JOB_ID + ".1-4:1\n", encoding="utf-8")
    result = run_validator(run_dir)
    assert result.returncode != 0
    assert "invalid recorded job ID" in result.stderr


@pytest.mark.parametrize(
    ("relative_path", "replacement", "message"),
    (
        (f"qacct/matlab_phase_profile_{LABEL}.txt",
         "jobnumber 7200001\nfailed 0\nexit_status 1\nru_wallclock 60\nmaxvmem 44G\nslots 4\n",
         "nonzero exit"),
        (f"matlab_phase_profile/{LABEL}/resources.txt",
         "Maximum resident set size (kbytes): 70000000\n",
         "RSS exceeds"),
    ),
)
def test_scheduler_and_rss_tampering_is_rejected(
    evidence: tuple[Path, dict[str, object]],
    relative_path: str,
    replacement: str,
    message: str,
) -> None:
    run_dir, _ = evidence
    (run_dir / relative_path).write_text(replacement, encoding="utf-8")
    result = run_validator(run_dir)
    assert result.returncode != 0
    assert message in result.stderr


def test_matlab_profiler_has_bounded_top_level_real_time_contract() -> None:
    source = (ROOT / "varcomp_kss/benchmarks/separations_matlab_phase_profile.m").read_text(
        encoding="utf-8"
    )
    assert CORE in source
    assert "profile on -timer real -nohistory" in source
    assert "entry.ExecutedLines" in source
    assert "entry.NumCalls == 1" in source
    assert "first_lines = [1, 334, 429, 477, 519, 574, 617, 622]" in source
    assert "last_lines = [333, 428, 476, 518, 573, 616, 621, 633]" in source
    assert "'FileType', 'text', 'Delimiter', ','" in source
    assert "worker_call_state = worker_original" in source
    assert "worker_seed" not in source
    assert "local_cluster.JobStorageLocation = parallel_scratch" in source
    assert "canonical_path(resolved_cmg), canonical_path(cmg_file)" in source
    assert "matlab_upstream_tree" not in source
    assert "profsave" not in source.lower()
    assert "copyfile" not in source.lower()


def test_scc_wrapper_binds_and_rechecks_immutable_inputs() -> None:
    source = (ROOT / "varcomp_kss/benchmarks/scc/run_matlab_phase_profile.sge").read_text(
        encoding="utf-8"
    )
    assert source.count('sha256sum "$core"') == 2
    assert source.count('cmg_mex_hash)" = "$KSS_MATLAB_CMG_MEX_SHA256"') == 2
    assert source.count('cmg_solver_hash)" = "$KSS_MATLAB_CMG_SOLVER_SHA256"') == 2
    assert source.count('sha256sum "$KSS_INPUT_CSV"') == 2
    assert source.count('sha256sum -c "$bundle_manifest"') == 2
    assert source.count('test "$runtime_tree_actual" = "$registered_runtime_tree_sha"') == 2
    assert "LC_ALL=C sort -z" in source
    assert "MATLAB PHASE PROFILE TYPED FAILURE" in source
    assert "module load matlab/2025b" in source
    assert "mem_per_core=14G" in source
    assert "KSS_TIMEOUT_SECONDS <= 3600" in source

    submit = (ROOT / "varcomp_kss/benchmarks/scc/submit_matlab_phase_profile.sh").read_text(
        encoding="utf-8"
    )
    collect = (ROOT / "varcomp_kss/benchmarks/scc/collect_matlab_phase_profile.sh").read_text(
        encoding="utf-8"
    )
    assert '[[ "$job_id" =~ ^[0-9]+$ ]]' in submit
    assert '[[ "$job_id" =~ ^[0-9]+$ ]]' in collect
    assert "raw=1.5*value+180" in submit


def test_new_shell_entrypoints_pass_bash_syntax() -> None:
    scripts = [
        ROOT / "varcomp_kss/benchmarks/scc/run_matlab_phase_profile.sge",
        ROOT / "varcomp_kss/benchmarks/scc/submit_matlab_phase_profile.sh",
        ROOT / "varcomp_kss/benchmarks/scc/collect_matlab_phase_profile.sh",
    ]
    result = subprocess.run(
        ["bash", "-n", *(str(path) for path in scripts)],
        text=True,
        capture_output=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
