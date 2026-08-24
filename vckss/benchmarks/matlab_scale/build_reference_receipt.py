#!/usr/bin/env python3
"""Convert one accepted KSS-SCALE run into a fixed MATLAB reference receipt."""

import argparse
import importlib.util
import re
import sys
from pathlib import Path, PurePosixPath

from common import (
    REFERENCE_KSS_OPTIONS,
    REFERENCE_SCHEMA,
    BenchmarkError,
    atomic_write_json,
    integer,
    load_json,
    require,
    sha256_file,
    target_identity,
    validate_preparation_acceptance,
    validate_preparation_receipt,
    validate_reference_record,
)
from validate_scc_job import revalidate_acceptance_sources

MANIFEST_ROW = re.compile(r"^([0-9a-f]{64})  ([A-Za-z0-9._/-]+)$")
TARGETS = ("worker", "firm", "covariance", "total")


def regular_file(path, label):
    path = Path(path).absolute()
    require(path.is_file() and not path.is_symlink(), f"invalid {label}: {path}")
    return path


def verify_source_bundle(source_dir, source_commit, bundle_sha256):
    """Verify the immutable deployed source tree and its external manifest."""
    source_dir = Path(source_dir).absolute()
    require(
        source_dir.is_dir()
        and not source_dir.is_symlink()
        and source_dir.resolve(strict=True) == source_dir,
        "KSS source directory is not a real absolute directory",
    )
    bundle_dir = source_dir.parent
    require(
        source_dir.name == "source" and bundle_dir.name == bundle_sha256,
        "KSS source directory is not content-addressed by its bundle",
    )
    archive = regular_file(bundle_dir / f"{bundle_sha256}.tar.gz", "KSS bundle archive")
    manifest = regular_file(
        bundle_dir / f"{bundle_sha256}.files.sha256", "KSS source manifest"
    )
    internal_manifest = regular_file(source_dir / "BUNDLE_FILES.sha256", "internal manifest")
    require(sha256_file(archive) == bundle_sha256, "KSS bundle archive checksum changed")
    require(manifest.read_bytes() == internal_manifest.read_bytes(), "KSS manifests differ")
    require(
        regular_file(source_dir / "SOURCE_COMMIT.txt", "source commit marker")
        .read_text(encoding="utf-8")
        .strip()
        == source_commit,
        "KSS source commit marker changed",
    )

    seen = set()
    for raw in manifest.read_text(encoding="utf-8").splitlines():
        match = MANIFEST_ROW.fullmatch(raw)
        require(match is not None, "malformed KSS source manifest row")
        digest, spelling = match.groups()
        relative = PurePosixPath(spelling)
        require(
            not relative.is_absolute()
            and str(relative) == spelling
            and ".." not in relative.parts
            and "." not in relative.parts
            and spelling not in seen,
            "unsafe or duplicate KSS source manifest path",
        )
        seen.add(spelling)
        item = regular_file(source_dir / Path(relative), f"manifest source {spelling}")
        require(sha256_file(item) == digest, f"KSS source checksum changed: {spelling}")
    validator = source_dir / "vckss/benchmarks/scc/validate_kss_scale.py"
    driver = source_dir / "vckss/benchmarks/scc/kss_scale_driver.do"
    require(
        validator.relative_to(source_dir).as_posix() in seen
        and driver.relative_to(source_dir).as_posix() in seen,
        "KSS validator or driver is absent from the source manifest",
    )
    return {
        "archive": archive,
        "manifest": manifest,
        "validator": regular_file(validator, "KSS scale validator"),
        "driver": regular_file(driver, "KSS scale driver"),
    }


def load_kss_validator(path):
    spec = importlib.util.spec_from_file_location(
        "kss_scale_reference_source_validator", path
    )
    require(spec is not None and spec.loader is not None, "cannot load KSS scale validator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def experiment_paths(run_dir, experiment_id):
    run_dir = Path(run_dir).absolute()
    experiment = run_dir / "experiments" / experiment_id
    return {
        "experiment": experiment,
        "summary": experiment / "summary.csv",
        "rhs": experiment / "rhs.csv",
        "stage_memory": experiment / "stage_memory.csv",
        "phase_rss_samples": experiment / "phase_rss_samples.csv",
        "phase_rss_peaks": experiment / "phase_rss_peaks.csv",
        "phase_rss_sampler": experiment / "phase_rss_sampler.tsv",
        "tmp_capacity": experiment / "tmp_capacity.tsv",
        "node_receipt": experiment / "node_receipt.tsv",
        "wrapper_pass": experiment / "wrapper.pass",
        "stata_pass": experiment / "stata.pass",
        "application_log": experiment / "application.log",
        "reservation": experiment / "reservation.tsv",
        "wrapper_metrics": experiment / "wrapper_metrics.tsv",
        "process_resources": experiment / "process_resources.txt",
        "job_id_file": run_dir / "submissions" / f"{experiment_id}.job_id",
        "scheduler_request": run_dir
        / "submissions"
        / f"{experiment_id}.scheduler_request.txt",
        "qacct": run_dir / "qacct" / f"{experiment_id}.txt",
        "admission": experiment / "admission_receipt.tsv",
    }


def expected_admission(report):
    return {
        "receipt_version": "KSS-STREAMLINE-RUN-V1",
        "validation_status": report["status"],
        "experiment_id": report["experiment_id"],
        "fixture": report["fixture"],
        "scale_factor": str(report["scale_factor"]),
        "source_commit": report["source_commit"],
        "bundle_sha256": report["bundle_sha256"],
        "input_sha256": report["input_sha256"],
        "option_contract": report["options"]["option_contract"],
        "frequency_var": report["options"]["frequency_var"],
        "target_var": report["options"]["target_var"],
        "deletion_var": report["options"]["deletion_var"],
        "deletion_mode": report["options"]["deletion_mode"],
        "requested_probes": str(report["requested_probes"]),
        "engine": report["output"]["engine"],
        "future_run_authorized": "0",
    }


def validate_topology(preparation, report):
    scale = preparation["scale"]
    topology = preparation["topology"]
    require(report["scale_factor"] == scale, "KSS/preparation scale changed")
    if scale == 1 and topology == "well":
        require(
            report["fixture"] in {"local", "cz24", "cz25", "cz18"},
            "unreplicated KSS fixture is not a fixed retained benchmark",
        )
    else:
        require(report["fixture"] == topology, "KSS/preparation topology changed")


def build(args):
    output = Path(args.output).absolute()
    require(not output.exists(), "reference output already exists")
    run_dir = Path(args.kss_run_dir).absolute()
    require(run_dir.is_dir() and not run_dir.is_symlink(), "invalid KSS run directory")
    experiment_id = args.experiment_id
    require(
        isinstance(experiment_id, str)
        and re.fullmatch(r"[A-Za-z0-9._-]+", experiment_id) is not None,
        "invalid KSS experiment ID",
    )
    paths = experiment_paths(run_dir, experiment_id)
    require(paths["experiment"].is_dir(), "KSS experiment directory is missing")
    for name, path in paths.items():
        if name != "experiment":
            regular_file(path, f"KSS {name}")

    certificate_path = regular_file(args.kss_certificate, "KSS certificate")
    require(certificate_path.parent == paths["experiment"], "KSS certificate path changed")
    require(
        Path(args.kss_admission_receipt).absolute() == paths["admission"],
        "KSS admission receipt path changed",
    )
    certificate = load_json(certificate_path)

    # Derive identities only from source-bound KSS evidence, then execute the
    # validator from the exact immutable bundle named by that evidence.
    reservation_probe = {}
    for raw in paths["reservation"].read_text(encoding="utf-8").splitlines()[1:]:
        fields = raw.split("\t")
        require(len(fields) == 2 and fields[0] not in reservation_probe, "bad reservation")
        reservation_probe[fields[0]] = fields[1]
    source_commit = reservation_probe.get("source_commit", "")
    bundle_sha = reservation_probe.get("bundle_sha256", "")
    input_sha = reservation_probe.get("input_sha256", "")
    require(re.fullmatch(r"[0-9a-f]{40}", source_commit) is not None, "bad KSS source commit")
    require(re.fullmatch(r"[0-9a-f]{64}", bundle_sha) is not None, "bad KSS bundle hash")
    require(re.fullmatch(r"[0-9a-f]{64}", input_sha) is not None, "bad KSS input hash")
    require(
        regular_file(run_dir / "source_commit.txt", "run source marker")
        .read_text(encoding="utf-8")
        .strip()
        == source_commit
        and regular_file(run_dir / "bundle.sha256", "run bundle marker")
        .read_text(encoding="utf-8")
        .strip()
        == bundle_sha,
        "KSS run source markers changed",
    )
    source = verify_source_bundle(args.kss_source_dir, source_commit, bundle_sha)
    validator = load_kss_validator(source["validator"])
    summary_probe = validator.read_one_csv(paths["summary"], "scale summary")
    probes = integer(float(summary_probe.get("requested_probes", "")), "KSS probes", 2)
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
        experiment_id=experiment_id,
        source_commit=source_commit,
        bundle_sha=bundle_sha,
        input_sha=input_sha,
        probes=probes,
    )
    require(
        report["status"] == "KSS_STREAMLINE_SCIENTIFIC_PASS",
        "KSS run was not accepted",
    )
    require(report["output"]["engine"] == "compressed", "KSS reference is not compressed")
    require(report["requested_probes"] == 200, "KSS reference does not use 200 probes")
    require(
        report["options"] == REFERENCE_KSS_OPTIONS,
        "KSS options are not MATLAB-reference eligible",
    )
    require(certificate == report, "KSS certificate differs from replayed validation")
    admission = validator.read_key_values(paths["admission"], "KSS admission receipt")
    require(admission == expected_admission(report), "KSS admission receipt changed")

    source_input = regular_file(args.source_input_dta, "KSS source input")
    require(sha256_file(source_input) == input_sha, "KSS source input bytes changed")
    preparation_path = regular_file(args.preparation_receipt, "preparation receipt")
    acceptance_path = regular_file(args.preparation_acceptance, "preparation acceptance")
    require(
        preparation_path.name == "wrapper.json"
        and acceptance_path == preparation_path.parent / "acceptance.json",
        "preparation evidence path changed",
    )
    preparation_sha = sha256_file(preparation_path)
    acceptance_sha = sha256_file(acceptance_path)
    preparation_record = load_json(preparation_path)
    preparation = validate_preparation_receipt(preparation_record)
    acceptance = load_json(acceptance_path)
    validate_preparation_acceptance(acceptance, preparation_sha, preparation)
    revalidate_acceptance_sources(
        acceptance_path,
        stage="prepare",
        job_dir=preparation_path.parent,
        expected_acceptance_sha256=acceptance_sha,
    )
    require(
        preparation["source_commit"] == source_commit
        and preparation["bundle_sha256"] == bundle_sha
        and preparation["source_input_sha256"] == input_sha,
        "KSS and preparation source binding changed",
    )
    prepared_input = regular_file(
        preparation_record["artifacts"]["input_csv"]["path"], "prepared MATLAB input"
    )
    retained_key = regular_file(
        preparation_record["artifacts"]["retained_keys"]["path"], "retained-key file"
    )
    require(
        prepared_input == preparation_path.parent / "input.csv"
        and retained_key == preparation_path.parent / "retained_keys.canonical.txt",
        "preparation artifact path changed",
    )
    require(
        sha256_file(prepared_input) == preparation["prepared_input_sha256"]
        and sha256_file(retained_key) == preparation["retained_key_sha256"],
        "prepared input or retained-key bytes changed",
    )
    validate_topology(preparation, report)
    dimensions = {
        "rows": integer(float(summary_probe["N_retained"]), "KSS retained rows", 1),
        "workers": integer(float(summary_probe["worker_levels"]), "KSS workers", 1),
        "firms": integer(float(summary_probe["firm_levels"]), "KSS firms", 1),
        "matches": integer(float(summary_probe["deletion_units"]), "KSS matches", 1),
    }
    require(dimensions == preparation["dimensions"], "KSS/preparation dimensions changed")
    require(
        integer(float(summary_probe["N_physical"]), "KSS physical rows", 1)
        == dimensions["rows"],
        "unweighted KSS physical-row count changed",
    )
    seed = integer(float(summary_probe["seed"]), "KSS seed", 0)
    plugin = {target: float(summary_probe[f"plugin_{target}"]) for target in TARGETS}
    require(target_identity(plugin) <= 1e-12, "KSS plug-in accounting identity failed")

    provenance = {
        "producer": "kss_matlab_scale_reference_converter_v1",
        "experiment_id": experiment_id,
        "fixture": report["fixture"],
        "scale_factor": report["scale_factor"],
        "job_id": report["scheduler"]["job_id"],
        **report["options"],
        "kss_admission_receipt_sha256": sha256_file(paths["admission"]),
        "kss_certificate_sha256": sha256_file(certificate_path),
        "kss_driver_sha256": sha256_file(source["driver"]),
        "kss_job_id_file_sha256": sha256_file(paths["job_id_file"]),
        "kss_node_receipt_sha256": sha256_file(paths["node_receipt"]),
        "kss_qacct_sha256": sha256_file(paths["qacct"]),
        "kss_reservation_sha256": sha256_file(paths["reservation"]),
        "kss_scheduler_request_sha256": sha256_file(paths["scheduler_request"]),
        "kss_source_manifest_sha256": sha256_file(source["manifest"]),
        "kss_summary_sha256": sha256_file(paths["summary"]),
        "kss_validator_sha256": sha256_file(source["validator"]),
        "preparation_acceptance_sha256": acceptance_sha,
        "preparation_receipt_sha256": preparation_sha,
        "prepared_input_sha256": preparation["prepared_input_sha256"],
        "retained_key_sha256": preparation["retained_key_sha256"],
        "source_input_sha256": input_sha,
    }
    record = {
        "schema": REFERENCE_SCHEMA,
        "status": "PASS",
        "kind": "stata_vckss",
        "label": preparation["label"],
        "sample_mode": "fixed_retained",
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "source_input_sha256": input_sha,
        "prepared_input_sha256": preparation["prepared_input_sha256"],
        "retained_key_sha256": preparation["retained_key_sha256"],
        "N_retained": dimensions["rows"],
        "worker_levels": dimensions["workers"],
        "firm_levels": dimensions["firms"],
        "deletion_units": dimensions["matches"],
        "algorithm_requested": "jla",
        "algorithm_selected": "jla",
        "deletion": "match",
        "controls": "none",
        "frequency_semantics": "literal_physical_rows_v1",
        "target_weight_semantics": "uniform_stored_rows_v1",
        "requested_probes": probes,
        "seed": seed,
        **{f"plugin_{target}": plugin[target] for target in TARGETS},
        "provenance": provenance,
    }
    validate_reference_record(record, preparation, probes=probes, seed=seed)
    atomic_write_json(output, record)
    print(sha256_file(output))
    return 0


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--kss-run-dir", required=True)
    parser.add_argument("--kss-source-dir", required=True)
    parser.add_argument("--experiment-id", required=True)
    parser.add_argument("--kss-certificate", required=True)
    parser.add_argument("--kss-admission-receipt", required=True)
    parser.add_argument("--source-input-dta", required=True)
    parser.add_argument("--preparation-receipt", required=True)
    parser.add_argument("--preparation-acceptance", required=True)
    parser.add_argument("--output", required=True)
    return parser.parse_args(argv)


if __name__ == "__main__":
    try:
        sys.exit(build(parse_args()))
    except (BenchmarkError, KeyError, OSError, TypeError, ValueError) as exc:
        print(f"KSS MATLAB SCALE REFERENCE CONVERSION FAIL: {exc}", file=sys.stderr)
        sys.exit(2)
