import argparse
import csv
import json
from pathlib import Path

from build_prepare_receipt import build as build_prepare_receipt
from common import sha256_file
from conftest import write_csv, write_json, write_scc_acceptance
from test_common import CONTRACT_PATH, valid_case
from test_prepare_receipt import arguments as preparation_arguments
from verify_submission import verify


def setup_gate(tmp_path):
    tmp_path.mkdir(parents=True, exist_ok=True)
    label = "cz18-fixed"
    preparation_dir = tmp_path / "matlab_scale" / label / "prepare"
    preparation_dir.mkdir(parents=True)
    preparation_args = preparation_arguments(preparation_dir)
    Path(preparation_args.application_log).write_text(
        f"KSS MATLAB SCALE PREPARE PASS: {label}\n", encoding="utf-8"
    )
    assert build_prepare_receipt(preparation_args) == 0
    preparation_path = Path(preparation_args.output)
    preparation = json.loads(preparation_path.read_text(encoding="utf-8"))
    (preparation_dir / "wrapper.pass").write_text(
        "KSS_MATLAB_SCALE_PREPARE_WRAPPER_PASS "
        f"{label} {preparation['source_commit']} {preparation['bundle_sha256']} "
        f"{preparation['prepared_input_sha256']} {preparation['retained_key_sha256']}\n",
        encoding="utf-8",
    )
    input_path = preparation_dir / "input.csv"
    input_sha = sha256_file(input_path)
    preparation_sha = sha256_file(preparation_path)
    acceptance_path = write_scc_acceptance(
        tmp_path,
        stage="prepare",
        label=label,
        source_commit=preparation["source_commit"],
        bundle_sha256=preparation["bundle_sha256"],
        input_sha256=preparation["source_input_sha256"],
        job_dir=preparation_dir,
    )

    case = valid_case()
    case["label"] = label
    case["scale"] = 1
    case["topology"] = "well"
    case["input"]["sha256"] = input_sha
    case["input"].update(preparation["dimensions"])
    case["preparation"] = {
        "receipt_sha256": preparation_sha,
        "acceptance_sha256": sha256_file(acceptance_path),
        **{
            field: preparation[field]
            for field in (
                "label",
                "scale",
                "topology",
                "source_commit",
                "bundle_sha256",
                "source_input_sha256",
                "prepared_input_sha256",
                "retained_key_sha256",
                "dimensions",
                "source_dimensions",
            )
        },
        "receipt_schema": preparation["schema"],
    }
    case["reference_sample"].update(
        {
            "label": label,
            "input_bindings": {"prepared_input_sha256": input_sha},
            "retained_key_sha256": preparation["retained_key_sha256"],
            **preparation["dimensions"],
        }
    )
    case["reference_sample"]["provenance"].update(
        {
            "preparation_receipt_sha256": preparation_sha,
            "preparation_acceptance_sha256": sha256_file(acceptance_path),
            "source_input_sha256": preparation["source_input_sha256"],
            "prepared_input_sha256": input_sha,
            "retained_key_sha256": preparation["retained_key_sha256"],
        }
    )
    case["reference_sample"]["input_bindings"] = {"prepared_input_sha256": input_sha}
    case_path = tmp_path / "matlab_scale" / label / "case.json"
    write_json(case_path, case)
    output = tmp_path / "submission_gate.json"
    args = argparse.Namespace(
        case=str(case_path),
        case_sha256=sha256_file(case_path),
        contract=str(CONTRACT_PATH),
        contract_sha256=sha256_file(CONTRACT_PATH),
        input=str(input_path),
        input_sha256=input_sha,
        preparation_receipt=str(preparation_path),
        preparation_acceptance=str(acceptance_path),
        executing_source_commit=case["source"]["source_commit"],
        executing_bundle_sha256=case["source"]["bundle_sha256"],
        label=case["label"],
        output=str(output),
    )
    return args, case


def failure_message(args):
    assert verify(args) == 2
    return json.loads(Path(args.output).read_text(encoding="utf-8"))["failure_message"]


def test_fixed_submission_gate_binds_preparation_and_executing_source(tmp_path):
    args, _ = setup_gate(tmp_path)
    assert verify(args) == 0
    receipt = json.loads(Path(args.output).read_text(encoding="utf-8"))
    assert receipt["status"] == "PASS"
    assert receipt["sample_mode"] == "fixed"
    assert receipt["preparation_receipt_sha256"] == sha256_file(args.preparation_receipt)


def test_submission_gate_rejects_selection_case(tmp_path):
    args, case = setup_gate(tmp_path)
    case["sample_mode"] = "selection"
    write_json(args.case, case)
    args.case_sha256 = sha256_file(args.case)
    assert "fixed samples only" in failure_message(args)


def test_submission_gate_rejects_preparation_digest_drift(tmp_path):
    args, _ = setup_gate(tmp_path)
    preparation = json.loads(Path(args.preparation_receipt).read_text(encoding="utf-8"))
    preparation["label"] = "tampered"
    write_json(args.preparation_receipt, preparation)
    assert "preparation acceptance label changed" in failure_message(args)


def test_submission_gate_rejects_executing_source_or_bundle_drift(tmp_path):
    args, _ = setup_gate(tmp_path)
    args.executing_source_commit = "0" * 40
    assert "executing source commit differs" in failure_message(args)
    args, _ = setup_gate(tmp_path / "bundle")
    args.executing_bundle_sha256 = "0" * 64
    assert "executing bundle differs" in failure_message(args)


def test_submission_gate_replays_preparation_qacct_bytes(tmp_path):
    args, _ = setup_gate(tmp_path)
    acceptance = json.loads(Path(args.preparation_acceptance).read_text(encoding="utf-8"))
    qacct = Path(acceptance["evidence"]["qacct"]["path"])
    qacct.write_text(
        qacct.read_text(encoding="utf-8").replace("failed 0", "failed 37"),
        encoding="utf-8",
    )
    assert "acceptance evidence bytes changed: qacct" in failure_message(args)


def test_submission_gate_rejects_missing_preparation_qacct_hash(tmp_path):
    args, _ = setup_gate(tmp_path)
    acceptance = json.loads(Path(args.preparation_acceptance).read_text(encoding="utf-8"))
    acceptance["evidence"]["qacct"].pop("sha256")
    acceptance.pop("qacct_sha256")
    write_json(args.preparation_acceptance, acceptance)
    assert "evidence qacct SHA-256" in failure_message(args)


def test_submission_gate_rejects_tampered_scheduler_summary(tmp_path):
    args, _ = setup_gate(tmp_path)
    acceptance = json.loads(Path(args.preparation_acceptance).read_text(encoding="utf-8"))
    acceptance["scheduler"]["slots"] = 13
    write_json(args.preparation_acceptance, acceptance)
    assert "replayed source-byte validation" in failure_message(args)


def test_submission_gate_replays_prepare_csv_after_wrapper_rehash(tmp_path):
    args, case = setup_gate(tmp_path)
    preparation_path = Path(args.preparation_receipt)
    summary_path = preparation_path.parent / "prepare.csv"
    with summary_path.open(newline="", encoding="utf-8") as handle:
        summary = next(csv.DictReader(handle))
    summary["actual_processors"] = "3"
    write_csv(summary_path, [summary])

    preparation = json.loads(preparation_path.read_text(encoding="utf-8"))
    preparation["artifacts"]["summary"]["sha256"] = sha256_file(summary_path)
    write_json(preparation_path, preparation)
    preparation_sha = sha256_file(preparation_path)

    acceptance_path = Path(args.preparation_acceptance)
    acceptance = json.loads(acceptance_path.read_text(encoding="utf-8"))
    acceptance["wrapper_sha256"] = preparation_sha
    acceptance["evidence"]["wrapper"]["sha256"] = preparation_sha
    write_json(acceptance_path, acceptance)
    acceptance_sha = sha256_file(acceptance_path)

    case["preparation"]["receipt_sha256"] = preparation_sha
    case["preparation"]["acceptance_sha256"] = acceptance_sha
    case["reference_sample"]["provenance"][
        "preparation_receipt_sha256"
    ] = preparation_sha
    case["reference_sample"]["provenance"][
        "preparation_acceptance_sha256"
    ] = acceptance_sha
    write_json(args.case, case)
    args.case_sha256 = sha256_file(args.case)

    assert "four Stata processors" in failure_message(args)
