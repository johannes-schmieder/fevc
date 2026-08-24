import argparse
import csv
import hashlib
import importlib.util
import json
import tarfile
from pathlib import Path

import pytest
from build_prepare_receipt import build as build_prepare_receipt
from build_reference_receipt import build, experiment_paths, load_kss_validator
from common import BenchmarkError, sha256_file
from conftest import write_csv, write_json, write_kv, write_scc_acceptance
from test_prepare_receipt import arguments as preparation_arguments

ROOT = Path(__file__).resolve().parents[4]
SOURCE_COMMIT = "4" * 40
EXPERIMENT = "well_connected_1x_p2"


def load_module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def deployed_bundle(tmp_path):
    builder = load_module(
        ROOT / "vckss/benchmarks/build_scale_bundle.py", "reference_test_bundle_builder"
    )
    payload, manifest_text = builder.build(
        ROOT,
        ROOT / "vckss/benchmarks/scale_bundle_allowlist.txt",
        SOURCE_COMMIT,
    )
    bundle_sha = hashlib.sha256(payload).hexdigest()
    bundle_dir = tmp_path / "bundles" / bundle_sha
    source_dir = bundle_dir / "source"
    source_dir.mkdir(parents=True)
    archive = bundle_dir / f"{bundle_sha}.tar.gz"
    manifest = bundle_dir / f"{bundle_sha}.files.sha256"
    archive.write_bytes(payload)
    manifest.write_text(manifest_text, encoding="utf-8")
    with tarfile.open(archive, "r:gz") as handle:
        handle.extractall(source_dir, filter="data")
    return source_dir, bundle_sha


def prepare_kss_evidence(tmp_path, source_dir, bundle_sha, input_sha):
    scale_tests = load_module(
        ROOT / "vckss/tests/python/test_scale_scc_validator.py",
        "reference_test_scale_fixture",
    )
    staging = tmp_path / "kss-fixture"
    staging.mkdir()
    scale_tests.fixture(staging)
    run_dir = tmp_path / "runs" / "reference-run"
    experiment = run_dir / "experiments" / EXPERIMENT
    experiment.parent.mkdir(parents=True)
    (staging / "output").rename(experiment)
    (run_dir / "submissions").mkdir()
    (run_dir / "qacct").mkdir()
    (experiment / "job_id.txt").rename(run_dir / "submissions" / f"{EXPERIMENT}.job_id")
    scheduler_request = run_dir / "submissions" / f"{EXPERIMENT}.scheduler_request.txt"
    (staging / "submissions" / f"{scale_tests.EXPERIMENT}.scheduler_request.txt").rename(
        scheduler_request
    )
    (experiment / "qacct.txt").rename(run_dir / "qacct" / f"{EXPERIMENT}.txt")
    (run_dir / "source_commit.txt").write_text(SOURCE_COMMIT + "\n", encoding="utf-8")
    (run_dir / "bundle.sha256").write_text(bundle_sha + "\n", encoding="utf-8")

    for path in experiment.iterdir():
        if path.is_file():
            text = path.read_text(encoding="utf-8")
            path.write_text(
                text.replace(scale_tests.BUNDLE, bundle_sha).replace(scale_tests.INPUT, input_sha),
                encoding="utf-8",
            )
    scheduler_request.write_text(
        scheduler_request.read_text(encoding="utf-8")
        .replace(scale_tests.BUNDLE, bundle_sha)
        .replace(scale_tests.INPUT, input_sha)
        .replace(
            "/projectnb/welfgr/vckss/bundles/source",
            f"/projectnb/welfgr/vckss/bundles/{bundle_sha}/source",
        )
        .replace(
            f"/projectnb/welfgr/vckss/bundles/{bundle_sha}.tar.gz",
            f"/projectnb/welfgr/vckss/bundles/{bundle_sha}/{bundle_sha}.tar.gz",
        )
        .replace(
            f"/projectnb/welfgr/vckss/bundles/{bundle_sha}.files.sha256",
            f"/projectnb/welfgr/vckss/bundles/{bundle_sha}/{bundle_sha}.files.sha256",
        )
        .replace("KSS_FIXTURE=well_connected", "KSS_FIXTURE=cz18")
        .replace("KSS_PROBES=2", "KSS_PROBES=200"),
        encoding="utf-8",
    )

    validator = load_kss_validator(
        source_dir / "vckss/benchmarks/scc/validate_kss_scale.py"
    )
    for name in ("reservation.tsv", "node_receipt.tsv", "tmp_capacity.tsv"):
        path = experiment / name
        values = validator.read_key_values(path, name)
        values["fixture"] = "cz18"
        write_kv(path, values)
    summary_path = experiment / "summary.csv"
    with summary_path.open(newline="", encoding="utf-8") as handle:
        summary = next(csv.DictReader(handle))
    summary.update(
        {
            "fixture": "cz18",
            "input_rows": 5,
            "requested_probes": 200,
            "rng_leverage_probe_last": 200,
            "rng_target_probe_last": 200,
            "N_stored": 5,
            "N_retained": 5,
            "N_physical": 5,
            "worker_levels": 3,
            "firm_levels": 2,
            "deletion_units": 4,
            "coefficient_cells": 4,
            "target_strata": 4,
            "diagnostic_coefficient_cells": 4,
            "diagnostic_deletion_units": 4,
        }
    )
    write_csv(summary_path, [summary])
    rhs_rows = [
        {
            "experiment_id": EXPERIMENT,
            "stage": 1,
            "batch_start": 0,
            "rhs": 1,
            "iterations": 4,
            "relative_residual": 5e-10,
            "converged": 1,
        }
    ]
    for probe in range(1, 201):
        rhs_rows.append(
            {
                "experiment_id": EXPERIMENT,
                "stage": 4,
                "batch_start": probe,
                "rhs": probe,
                "iterations": 3,
                "relative_residual": 4e-10,
                "converged": 1,
            }
        )
        for local_rhs in (1, 2):
            rhs_rows.append(
                {
                    "experiment_id": EXPERIMENT,
                    "stage": 5,
                    "batch_start": probe,
                    "rhs": 2 * (probe - 1) + local_rhs,
                    "iterations": 3,
                    "relative_residual": 4e-10,
                    "converged": 1,
                }
            )
    write_csv(experiment / "rhs.csv", rhs_rows)
    paths = experiment_paths(run_dir, EXPERIMENT)
    report = validator.validate_run(
        summary=paths["summary"],
        rhs=paths["rhs"],
        stage_memory=paths["stage_memory"],
        qacct=paths["qacct"],
        phase_rss_samples=paths["phase_rss_samples"],
        phase_rss_peaks=paths["phase_rss_peaks"],
        phase_rss_sampler=paths["phase_rss_sampler"],
        tmp_capacity=paths["tmp_capacity"],
        job_id_file=paths["job_id_file"],
        scheduler_request=paths["scheduler_request"],
        node_receipt=paths["node_receipt"],
        wrapper_pass=paths["wrapper_pass"],
        stata_pass=paths["stata_pass"],
        application_log=paths["application_log"],
        reservation_path=paths["reservation"],
        wrapper_metrics=paths["wrapper_metrics"],
        process_resources=paths["process_resources"],
        experiment_id=EXPERIMENT,
        source_commit=SOURCE_COMMIT,
        bundle_sha=bundle_sha,
        input_sha=input_sha,
        probes=200,
    )
    certificate = experiment / "validation.json"
    write_json(certificate, report)
    validator.write_admission_receipt(paths["admission"], report)
    return run_dir, certificate, paths["admission"]


def prepare_matlab_sample(tmp_path, bundle_sha, input_sha):
    label = "cz18-fixed"
    job_dir = tmp_path / "runs" / "reference-run" / "matlab_scale" / label / "prepare"
    job_dir.mkdir(parents=True)
    args = preparation_arguments(job_dir)
    args.source_commit = SOURCE_COMMIT
    args.bundle_sha256 = bundle_sha
    args.source_input_sha256 = input_sha
    with Path(args.summary).open(newline="", encoding="utf-8") as handle:
        summary = next(csv.DictReader(handle))
    summary.update(
        {
            "source_commit": SOURCE_COMMIT,
            "bundle_sha256": bundle_sha,
            "source_input_sha256": input_sha,
        }
    )
    write_csv(args.summary, [summary])
    Path(args.pass_marker).write_text(
        f"KSS_MATLAB_SCALE_PREPARE_PASS {label} {SOURCE_COMMIT} {bundle_sha} {input_sha}\n",
        encoding="utf-8",
    )
    Path(args.application_log).write_text(
        f"KSS MATLAB SCALE PREPARE PASS: {label}\n", encoding="utf-8"
    )
    assert build_prepare_receipt(args) == 0
    preparation = json.loads(Path(args.output).read_text(encoding="utf-8"))
    (job_dir / "wrapper.pass").write_text(
        "KSS_MATLAB_SCALE_PREPARE_WRAPPER_PASS "
        f"{label} {SOURCE_COMMIT} {bundle_sha} "
        f"{preparation['prepared_input_sha256']} {preparation['retained_key_sha256']}\n",
        encoding="utf-8",
    )
    acceptance = write_scc_acceptance(
        tmp_path / "runs" / "reference-run",
        stage="prepare",
        label=label,
        source_commit=SOURCE_COMMIT,
        bundle_sha256=bundle_sha,
        input_sha256=input_sha,
        job_dir=job_dir,
    )
    return Path(args.output), acceptance


@pytest.fixture
def evidence(tmp_path):
    source_input = tmp_path / "source-input.dta"
    source_input.write_bytes(b"source-bound synthetic DTA bytes\n")
    input_sha = sha256_file(source_input)
    source_dir, bundle_sha = deployed_bundle(tmp_path)
    run_dir, certificate, admission = prepare_kss_evidence(
        tmp_path, source_dir, bundle_sha, input_sha
    )
    preparation, acceptance = prepare_matlab_sample(tmp_path, bundle_sha, input_sha)
    args = argparse.Namespace(
        kss_run_dir=str(run_dir),
        kss_source_dir=str(source_dir),
        experiment_id=EXPERIMENT,
        kss_certificate=str(certificate),
        kss_admission_receipt=str(admission),
        source_input_dta=str(source_input),
        preparation_receipt=str(preparation),
        preparation_acceptance=str(acceptance),
        output=str(tmp_path / "reference.json"),
    )
    return args


def test_converter_builds_a_source_and_scheduler_bound_kss_reference(evidence):
    assert build(evidence) == 0
    record = json.loads(Path(evidence.output).read_text(encoding="utf-8"))
    assert record["kind"] == "stata_vckss"
    assert record["requested_probes"] == 200
    assert record["N_retained"] == 5
    assert record["provenance"]["experiment_id"] == EXPERIMENT
    assert record["provenance"]["kss_qacct_sha256"]
    assert record["provenance"]["kss_scheduler_request_sha256"]
    assert record["provenance"]["preparation_acceptance_sha256"]


def test_converter_rejects_rehashed_noneligible_kss_options(evidence):
    admission = Path(evidence.kss_admission_receipt)
    text = admission.read_text(encoding="utf-8").replace("target_var\t-", "target_var\ttarget")
    admission.write_text(text, encoding="utf-8")
    with pytest.raises(BenchmarkError, match="admission receipt changed"):
        build(evidence)


def test_converter_replays_actual_kss_qacct_bytes(evidence):
    qacct = experiment_paths(
        Path(evidence.kss_run_dir), evidence.experiment_id
    )["qacct"]
    qacct.write_text(
        qacct.read_text(encoding="utf-8").replace("exit_status 0", "exit_status 137"),
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="scheduler or wrapper failed"):
        build(evidence)
