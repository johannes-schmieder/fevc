from __future__ import annotations

import csv
import hashlib
import importlib.util
import re
import subprocess
from pathlib import Path

import pytest

KSS_ROOT = Path(__file__).resolve().parents[2]
REPO_ROOT = KSS_ROOT.parent
DRIVER = KSS_ROOT / "benchmarks/scc/rng_k1_driver.do"
WRAPPER = KSS_ROOT / "benchmarks/scc/run_rng_k1.sge"
VALIDATOR = KSS_ROOT / "benchmarks/scc/validate_rng_k1.py"
RNG_MODULE = KSS_ROOT / "vckss_rng.mata"
SPEC = importlib.util.spec_from_file_location("validate_rng_k1", VALIDATOR)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

SOURCE = "4" * 40
BUNDLE = "b" * 64
EXPERIMENT = "rng_k1_stata19"
JOB_ID = "7200001"


def write_kv(path: Path, values: dict[str, object]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(["key", "value"])
        writer.writerows(values.items())


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_goldens(path: Path) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow(MODULE.GOLDEN_FIELDS)
        for candidate in ("per_probe_stream", "per_domain_stream"):
            for domain in ("leverage", "target"):
                values = MODULE.EXPECTED_GOLDENS[(candidate, domain)]
                for key, probes in zip(
                    MODULE.SEMANTIC_KEYS, values, strict=True
                ):
                    writer.writerow([candidate, domain, key, *probes])


def make_fixture(tmp_path: Path) -> dict[str, Path]:
    output = tmp_path / "output"
    output.mkdir()
    job_id_file = tmp_path / "job_id.txt"
    job_id_file.write_text(JOB_ID + "\n", encoding="utf-8")
    qacct = tmp_path / "qacct.txt"
    qacct.write_text(
        "==============================================================\n"
        "qname econ-pub.q\n"
        "hostname scc-test\n"
        "project welfgr\n"
        f"jobnumber {JOB_ID}\n"
        "slots 14\n"
        "failed 0\n"
        "exit_status 0\n"
        "ru_wallclock 120\n"
        "cpu 250.5\n"
        "maxvmem 1.5G\n",
        encoding="utf-8",
    )
    write_goldens(output / "golden_vectors.csv")
    snapshot = (
        "status\tOK\nactive_algorithm\tkiss32\nactive_stream\t177\n"
        "active_state\tX\nsort_state\tY\nmt64s_stream_1\tZ\n"
    )
    (output / "caller_rng_before.tsv").write_text(
        snapshot, encoding="utf-8")
    (output / "caller_rng_after.tsv").write_text(
        snapshot, encoding="utf-8")
    (output / "application.log").write_text(
        f"KSS-RNG-K1 STATA19 COMPATIBILITY PASS: {EXPERIMENT}\n",
        encoding="utf-8",
    )
    (output / "process_resources.txt").write_text(
        'Command being timed: "stata-mp -q do rng_k1_driver.do"\n'
        "User time (seconds): 220.25\n"
        "System time (seconds): 4.25\n"
        "Elapsed (wall clock) time (h:mm:ss or m:ss): 1:30.00\n"
        "Maximum resident set size (kbytes): 1048576\n"
        "Exit status: 0\n",
        encoding="utf-8",
    )
    (output / "stata.pass").write_text(
        f"KSS_RNG_K1_STATA_PASS {EXPERIMENT} {BUNDLE} {SOURCE} {JOB_ID}\n",
        encoding="utf-8",
    )
    (output / "wrapper.pass").write_text(
        f"KSS_RNG_K1_WRAPPER_PASS {EXPERIMENT} {BUNDLE} {SOURCE} {JOB_ID}\n",
        encoding="utf-8",
    )
    stata = {
        "receipt_version": "KSS-RNG-K1-STATA19-V1",
        "experiment_id": EXPERIMENT,
        "source_commit": SOURCE,
        "bundle_sha256": BUNDLE,
        "job_id": JOB_ID,
        "stata_version": "19",
        "stata_flavor": "IC",
        "stata_mp": 1,
        "requested_slots": 14,
        "actual_slots": 14,
        "requested_stata_processors": 4,
        "actual_stata_processors": 4,
        "rng_api_level": 2,
        "rng_build_id": "vckss-rng-k1-mt64s-complete-guard-v2",
        "rng_invariant_version": "KSS-RNG-K1-INVARIANT-V1",
        "stata18_reference_contract":
            "KSS-MT64S-DOMAIN-CURSOR-V2-STATA18",
        "production_contract_at_runtime": "",
        "per_probe_leverage_status": "OK",
        "per_probe_target_status": "OK",
        "per_domain_leverage_status": "OK",
        "per_domain_target_status": "OK",
        "per_probe_contract": "KSS-MT64S-PER-PROBE-CANDIDATE-V2",
        "per_domain_contract": "KSS-MT64S-PER-DOMAIN-CANDIDATE-V2",
        "leverage_timing_status": "OK",
        "target_timing_status": "OK",
        "leverage_recommendation": "per_probe_stream",
        "target_recommendation": "per_probe_stream",
        "production_scalar_seconds": 0.1,
        "production_vector_seconds": 0.05,
        "leverage_per_probe_seconds": 0.01,
        "leverage_per_domain_seconds": 0.02,
        "target_per_probe_seconds": 0.01,
        "target_per_domain_seconds": 0.02,
        "tiny_timing_role": "SMOKE_ONLY_NOT_SELECTION",
        "selection_timing_contract":
            "50000_ATOMS_P40_BOTH_DOMAINS_3_PAIRED_REPS_V1",
        "production_candidate_atoms": 50000,
        "production_candidate_probes": 40,
        "production_candidate_repetitions": 3,
        "production_per_probe_min_seconds": 0.9,
        "production_per_probe_median_seconds": 1.0,
        "production_per_probe_max_seconds": 1.1,
        "production_per_domain_min_seconds": 0.4,
        "production_per_domain_median_seconds": 0.5,
        "production_per_domain_max_seconds": 0.6,
        "production_domain_wins": 3,
        "per_probe_streams_tested": "1,2,3,16384,16385,16386",
        "per_probe_changed_stream_count": 0,
        "nonselected_probe_range": "7:8",
        "nonselected_selected_stream": 9,
        "nonselected_leverage_streams": "7,8",
        "nonselected_target_streams": "16390,16391",
        "per_probe_candidate_result": "QUALIFIED_STATA19_CANDIDATE",
        "per_domain_streams_tested": "1,2",
        "per_domain_candidate_result":
            "QUALIFIED_STATA19_CANDIDATE_NOT_REGISTERED",
        "selected_candidate": "per_domain_stream_cursor",
        "chunked_calls": 2,
        "boundary_atom": 2,
        "chunked_atom": 3,
        "core_rc": 0,
        "processor_restore_rc": 0,
        "guard_restore_rc": 0,
        "all_stream_restore_rc": 0,
        "overall_status": "KSS_RNG_K1_STATA19_COMPATIBLE_FAIL_CLOSED",
    }
    stata.update({key: 1 for key in MODULE.TRUE_FIELDS})
    write_kv(output / "stata_receipt.tsv", stata)
    write_kv(output / "node_receipt.tsv", {
        "receipt_version": "KSS-RNG-K1-NODE-V1",
        "experiment_id": EXPERIMENT,
        "job_id": JOB_ID,
        "source_commit": SOURCE,
        "bundle_sha256": BUNDLE,
        "requested_slots": 14,
        "actual_slots": 14,
        "requested_stata_processors": 4,
        "mem_per_core_gib": 4,
        "total_reserved_gib": 56,
        "hard_wall_seconds": 5400,
        "timeout_basis": "scc_stata19_job7200951_censored_3480s_x1.5",
        "timeout_projected_k1_seconds": 5220,
        "scalar_job": 1,
        "stata_module": "stata-mp/19",
    })
    write_kv(output / "wrapper_receipt.tsv", {
        "receipt_version": "KSS-RNG-K1-WRAPPER-V1",
        "experiment_id": EXPERIMENT,
        "job_id": JOB_ID,
        "source_commit": SOURCE,
        "bundle_sha256": BUNDLE,
        "requested_slots": 14,
        "actual_slots": 14,
        "mem_per_core_gib": 4,
        "total_reserved_gib": 56,
        "requested_stata_processors": 4,
        "actual_stata_processors": 4,
        "hard_wall_seconds": 5400,
        "timeout_basis": "scc_stata19_job7200951_censored_3480s_x1.5",
        "timeout_projected_k1_seconds": 5220,
        "application_timeout_seconds": 5280,
        "stata_process_seconds": 90,
        "total_wrapper_seconds": 95,
        "golden_vectors_sha256": digest(output / "golden_vectors.csv"),
        "caller_rng_snapshot_sha256":
            digest(output / "caller_rng_before.tsv"),
        "stata_receipt_sha256": digest(output / "stata_receipt.tsv"),
        "process_resources_sha256":
            digest(output / "process_resources.txt"),
        "application_log_sha256": digest(output / "application.log"),
        "qacct_jobnumber_binding": JOB_ID,
        "qacct_status": "PENDING_POST_EXIT_VALIDATION",
        "execution_boundary": "one_scalar_job_one_stata_process_no_data",
    })
    return {"output": output, "job_id": job_id_file, "qacct": qacct}


def validate(paths: dict[str, Path]) -> dict[str, object]:
    return MODULE.validate_run(
        output_dir=paths["output"], qacct_path=paths["qacct"],
        job_id_file=paths["job_id"], experiment_id=EXPERIMENT,
        source_commit=SOURCE, bundle_sha=BUNDLE,
    )


def test_scalar_wrapper_separates_reservation_from_stata_processors() -> None:
    text = WRAPPER.read_text(encoding="utf-8")
    assert "#$ -pe omp 14" in text
    assert "#$ -l mem_per_core=4G" in text
    assert "#$ -l h_rt=01:30:00" in text
    assert "module load stata-mp/19" in text
    assert 'export OMP_NUM_THREADS="$KSS_STATA_PROCESSORS"' in text
    assert '[[ "$KSS_STATA_PROCESSORS" == 4 ]]' in text
    assert "actual_stata_processors" in text
    assert "scc_stata19_job7200951_censored_3480s_x1.5" in text
    assert "one_scalar_job_one_stata_process_no_data" in text
    assert "#$ -t" not in text
    assert "qsub" not in text


def test_stata_k1_receipts_use_real_tab_bytes() -> None:
    text = DRIVER.read_text(encoding="utf-8")
    assert 'tab = char(9)' in text
    assert 'fput(file,key+char(9)+value)' in text
    assert 'fput(file,"key"+char(9)+"value")' in text
    writer = text.split("void rngk1__write_snapshot", 1)[1].split(
        "void rngk1__write_golden", 1
    )[0]
    assert '"\\t"' not in writer


def test_driver_contract_is_data_free_and_mata_identifiers_fit() -> None:
    text = DRIVER.read_text(encoding="utf-8")
    assert "Stata/MP 19" in text
    assert "c(processors)" in text and "set processors" in text
    assert "50000" in text and "production_candidate_probes = 40" in text
    assert "production_candidate_repetitions = 3" in text
    assert "SMOKE_ONLY_NOT_SELECTION" in text
    assert '"7:8"' in text and '"16390,16391"' in text
    assert "vckss_rng__capture_streams" in text
    assert not re.search(r"(?m)^\s*(use|merge|append|fevc)\b", text)
    without_strings = re.sub(r'"(?:[^"]|"")*"', "", text)
    identifiers = re.findall(r"\b[A-Za-z_][A-Za-z0-9_]*\b", without_strings)
    assert not [identifier for identifier in identifiers
                if len(identifier) > 32]


def test_driver_mata_definitions_compile_in_local_stata(tmp_path: Path) -> None:
    stata = Path("/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp")
    if not stata.is_file():
        pytest.skip("local Stata/MP is unavailable")
    text = DRIVER.read_text(encoding="utf-8")
    start = text.index("mata:\n") + len("mata:\n")
    stop = text.index("\nRNGK1_EVIDENCE =", start)
    compile_do = tmp_path / "rng_k1_compile.do"
    compile_do.write_text(
        "version 18.0\nset more off\n"
        f'do "{RNG_MODULE}"\n'
        "mata:\n" + text[start:stop] + "\nend\n"
        'display "RNG_K1_MATA_COMPILE_PASS"\nexit, clear\n',
        encoding="utf-8",
    )
    result = subprocess.run(
        [str(stata), "-q", "do", str(compile_do)], cwd=tmp_path,
        capture_output=True, text=True, timeout=120, check=False,
    )
    transcript = result.stdout + result.stderr
    assert result.returncode == 0, transcript
    assert "RNG_K1_MATA_COMPILE_PASS" in transcript
    assert "found where name expected" not in transcript
    assert re.search(r"(?m)^r\([0-9]+\);$", transcript) is None


def test_validator_accepts_qacct_bound_fixture(tmp_path: Path) -> None:
    paths = make_fixture(tmp_path)
    report = validate(paths)
    assert report["status"] == "KSS_RNG_K1_SCC_VALIDATION_PASS"
    receipt = tmp_path / "validation.tsv"
    MODULE.write_validation_receipt(receipt, report)
    assert "KSS-RNG-K1-QACCT-BOUND-V1" in receipt.read_text()


@pytest.mark.parametrize(
    ("mutation", "message"),
    [
        ("golden", "golden vector"),
        ("latent_stream", "per_probe_hidden_streams_restored"),
        ("processors", "Stata processor receipt"),
        ("flavor", "Stata receipt mismatch: stata_flavor"),
        ("snapshot", "snapshots differ"),
        ("qacct", "SGE failed"),
    ],
)
def test_validator_fails_closed(
    tmp_path: Path, mutation: str, message: str,
) -> None:
    paths = make_fixture(tmp_path)
    output = paths["output"]
    if mutation == "golden":
        golden = output / "golden_vectors.csv"
        golden.write_text(
            golden.read_text(encoding="utf-8").replace(
                "per_probe_stream,leverage,a,1,-1,1",
                "per_probe_stream,leverage,a,9,-1,1"),
            encoding="utf-8",
        )
    elif mutation in {"latent_stream", "processors", "flavor"}:
        receipt = output / "stata_receipt.tsv"
        values = MODULE.read_key_values(receipt, "Stata receipt")
        if mutation == "latent_stream":
            values["per_probe_hidden_streams_restored"] = "0"
        elif mutation == "processors":
            values["actual_stata_processors"] = "3"
        else:
            values["stata_flavor"] = "MP"
        write_kv(receipt, values)
    elif mutation == "snapshot":
        with (output / "caller_rng_after.tsv").open(
            "a", encoding="utf-8") as handle:
            handle.write("changed\t1\n")
    else:
        qacct = paths["qacct"]
        qacct.write_text(
            qacct.read_text(encoding="utf-8").replace(
                "failed 0", "failed 1"), encoding="utf-8")
    with pytest.raises(ValueError, match=message):
        validate(paths)
