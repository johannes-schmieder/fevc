from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import sys

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location("paired_offset_campaign", ROOT/"fevc/tools/run_fixed_offset_paired.py")
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


@pytest.fixture(scope="module")
def design():
    return MODULE.prepare_design()


def fake_binary(path, fail_process=False):
    path.write_text("#!/usr/bin/env python3\n"+f"""
import json, math, sys
if {fail_process!r}:
    print('deliberate process failure', file=sys.stderr)
    raise SystemExit(9)
tokens=open(sys.argv[1]).read().split()
seed=int(tokens[3])
truth=list(map(float,tokens[5:9]))
for arm in {MODULE.ARMS!r}:
 for t,value in zip({MODULE.TARGETS!r},truth):
  point=value+(seed%19)*1e-6
  v,c,r,l=.2,.01,.3,.1
  row=dict(schema='fevc-offset-paired-row-v1', arm=arm,target=t,seed=seed,truth=value,
   point=point,sd=.1,q0_lower=point-.196,q0_upper=point+.196,maximum_complete_residual=1e-12,
   mean_variance=.26,fold_hash='1234567890abcdef' if arm.endswith('fitted') else None,
   status='success',q1_status=0,lower=point-.2,upper=point+.2,critical=2,
   leading_variance=v,remainder_variance=r,cross_covariance=c,leading_eigenvalue=l,
   curvature=2*abs(l)*v/math.sqrt(r-c*c/v),leading_score=1.1,
   remainder_identity_error=0,point_identity_error=0)
  print(json.dumps(row))
""")
    path.chmod(0o755)
    return path


@pytest.fixture
def manifest(tmp_path, monkeypatch):
    monkeypatch.setattr(MODULE.BASE, "_git_identity", lambda _root: {"commit": "a"*40, "dirty": False})
    binary = fake_binary(tmp_path/"fake")
    path = tmp_path/"manifest.json"
    value = MODULE.create_manifest(path, binary, "tiny")
    return path, binary, value


def rows_for_input(manifest, design, tmp_path):
    path, binary, value = manifest
    contents, outcome, seed, _ = MODULE.make_input(value, 0, design[0])
    input_path = tmp_path/"input.txt"
    input_path.write_bytes(contents)
    result = MODULE.subprocess.run([str(binary), str(input_path)], capture_output=True, check=True)
    rows = [MODULE.strict_json(line) for line in result.stdout.splitlines()]
    return rows, outcome, seed


def test_exact_preflight_and_frozen_semantics(design):
    registration = json.loads((ROOT/MODULE.REGISTRATION).read_text())
    result = MODULE.preflight(registration, *design)
    assert result["status"] == "PASS"
    assert min(result["minimum_covariance_eigenvalues"].values()) > 0
    assert registration["expected_diagnostic_target_attempts"] == 16000
    assert registration["numerics"]["production_changes"] is False
    invalid = copy.deepcopy(design[1])
    invalid[0]["remainder_concentration"] = .9
    with pytest.raises(ValueError, match="preflight failed"):
        MODULE.preflight(registration, design[0], invalid)


def test_input_rng_order_and_profile_invariance(manifest, design):
    _, _, value = manifest
    a = MODULE.make_input(value, 0, design[0])
    b = MODULE.make_input(value, 1, design[0])
    assert MODULE.make_input(value, 0, design[0])[0] == a[0]
    assert a[0] != b[0] and a[2:] != b[2:]
    changed = copy.deepcopy(value)
    changed["profile"] = "diagnostic"
    assert MODULE.make_input(changed, 0, design[0])[0] != a[0]
    fields = a[0].decode().split()
    assert fields[:3] == ["FEVC_OFFSET_INPUT_V1", "1199", "400"]
    assert len(fields) == 9+8*1199+400
    physical = np.array(fields[9:9+8*1199], dtype=float).reshape(1199, 8)
    assert physical[:, 3].sum() == 8387
    assert len(set(physical[:, 2])) == 400
    assert np.array_equal(physical[:, 6:], design[0]["controls"])
    assert np.array_equal(physical[:, 5], a[1])
    with pytest.raises(ValueError, match="replication"):
        MODULE.make_input(value, 2, design[0])


@pytest.mark.parametrize("kind", ["missing", "duplicate", "seed", "truth", "nan", "status", "withheld_bounds", "curvature", "fold", "point"])
def test_reject_invalid_replication(kind, manifest, design, tmp_path):
    rows, _, seed = rows_for_input(manifest, design, tmp_path)
    MODULE.validate_replication(rows, seed, design[1])
    if kind == "missing": rows.pop()
    elif kind == "duplicate": rows.append(copy.deepcopy(rows[0]))
    elif kind == "seed": rows[0]["seed"] += 1
    elif kind == "truth": rows[0]["truth"] += .01
    elif kind == "nan": rows[0]["sd"] = float("nan")
    elif kind == "status": rows[0]["status"] = "unknown"
    elif kind == "withheld_bounds": rows[0].update(status="target_unavailable", q1_status=1)
    elif kind == "curvature": rows[0]["curvature"] *= 2
    elif kind == "fold": rows[4]["fold_hash"] = "0000000000000000"
    elif kind == "point": rows[4]["point"] += 1
    with pytest.raises(ValueError):
        MODULE.validate_replication(rows, seed, design[1])


def test_known_and_estimated_exact_points_agree_with_physical_joint_wls(manifest, design, tmp_path):
    rows, outcome, _ = rows_for_input(manifest, design, tmp_path)
    enriched = MODULE.enrich(rows, outcome, 0, design[0])
    d = design[0]
    z = d["z"][d["group"]]
    full = np.column_stack((z, d["controls"]))
    w = d["frequency"]
    beta = np.linalg.solve(full.T@(w[:, None]*full), full.T@(w*outcome))
    adjusted = outcome-d["controls"]@beta[-2:]
    y = np.bincount(d["group"], weights=w*adjusted)/np.sqrt(d["mass"])
    for index, target in enumerate(MODULE.TARGETS):
        value = next(r for r in enriched if r["arm"] == "estimated_known" and r["target"] == target)
        assert value["exact_point"] == pytest.approx(y@d["kernels"][index].apply(y), abs=1e-10)


def test_failures_are_retained_in_paired_denominators(manifest, design, tmp_path):
    rows, outcome, seed = rows_for_input(manifest, design, tmp_path)
    original = rows[0]
    rows[0] = {k: original[k] for k in ("schema", "arm", "target", "seed", "truth")}
    rows[0].update(status="backend_failure", error_code="JLA_CONSTRAINT_FAILED", error_phase="component_inference_psd")
    MODULE.validate_replication(rows, seed, design[1])
    rows = MODULE.enrich(rows, outcome, 0, design[0])
    summary, pairs, _ = MODULE.summarize(rows, manifest[2]["registration"], "tiny")
    affected = next(r for r in summary if r["arm"] == "known_known" and r["target"] == "worker")
    assert affected["attempts"] == 1 and affected["availability"] == 0
    assert affected["coverage_all_attempts"] == 0 and affected["coverage"] is None
    contrast = next(r for r in pairs if r["target"] == "worker" and r["contrast"] == "offset_at_known_variance")
    assert contrast["attempts"] == 1 and contrast["jointly_available"] == 0
    assert contrast["coverage_difference_all_attempts"] == 1


def test_dense_export_oracle_and_corruptions(manifest, design, tmp_path):
    rows, outcome, _ = rows_for_input(manifest, design, tmp_path)
    d, reference = design
    MODULE.enrich(rows, outcome, 0, d)
    diagonal = [np.sum((d["q"]@(d["root"].T@a@d["root"]))*d["q"], axis=1).tolist()
                for a in d["targets"][:3]]
    maker = (1/(1-np.sum(d["q"]**2, axis=1))).tolist()
    states = [dict(kind="state", arm=arm, target_diagonal=diagonal, maker_inverse=maker,
                   variance=d["tau"].tolist(), folds=([i%5 for i in range(400)] if arm.endswith("fitted") else []))
              for arm in MODULE.ARMS]
    for row in rows:
        row["point"] = row["exact_point"]
    records = rows+states
    result = MODULE.verify_export(records, outcome, d, reference)
    assert result["status"] == "PASS"
    assert max(result["point_differences"].values()) < 1e-10
    assert max(result["moment_differences"].values()) < 1e-10
    for kind in ("missing", "point", "variance", "fold", "moment"):
        changed, ref = copy.deepcopy(records), copy.deepcopy(reference)
        if kind == "missing": changed.pop()
        elif kind == "point": changed[0]["point"] += .01
        elif kind == "variance": changed[16]["variance"][0] += .01
        elif kind == "fold": changed[17]["folds"][0] = 1
        elif kind == "moment": ref[0]["known"]["point"]["variance"] += .01
        with pytest.raises(ValueError):
            MODULE.verify_export(changed, outcome, d, ref)


@pytest.mark.parametrize("kind", ["tasks", "rows", "preflight", "runtime", "registration", "binary", "source"])
def test_reject_modified_manifest(kind, manifest, tmp_path):
    path, binary, value = manifest
    if kind == "tasks": value["tasks"][0]["start"] = 1
    elif kind == "rows": value["expected_rows"] = 1
    elif kind == "preflight": value["preflight"]["reference"][0]["truth"] += 1
    elif kind == "runtime": value["runtime"]["numpy"] = "wrong"
    elif kind == "registration": value["registration"]["profiles"]["tiny"]["replications"] = 1
    elif kind == "binary": value["binary_sha256"] = "0"*64
    elif kind == "source": value["source"]["commit"] = "b"*40
    changed = tmp_path/"changed.json"
    changed.write_bytes(MODULE.payload(value))
    with pytest.raises(ValueError):
        MODULE.read_manifest(changed, binary, check_source=True)


def test_complete_pipeline_order_invariance_and_failed_calibration(manifest, design, tmp_path):
    path, binary, value = manifest
    first, second = tmp_path/"first", tmp_path/"second"
    MODULE.run_local(path, binary, first, 1)
    MODULE.run_local(path, binary, second, 2, reverse=True)
    assert (first/"aggregate/rows.json").read_bytes() == (second/"aggregate/rows.json").read_bytes()
    receipt = MODULE.strict_json((first/"aggregate/receipt.json").read_bytes())
    assert receipt["rows"] == 32 and receipt["status"] == "COMPLETE_TINY"
    assert len(receipt["summaries"]) == 16 and len(receipt["paired_contrasts"]) == 20
    rows = MODULE.strict_json((first/"aggregate/rows.json").read_bytes())
    _, _, failures = MODULE.summarize(rows, value["registration"], "diagnostic")
    assert failures and any("coverage" in v for v in failures)
    with pytest.raises(FileExistsError):
        MODULE.run_local(path, binary, first, 1)
    with pytest.raises(Exception, match="replace"):
        MODULE.create_manifest(path, binary, "tiny")
    with pytest.raises(ValueError, match="worker"):
        MODULE.run_local(path, binary, tmp_path/"too_many", 5)


@pytest.mark.parametrize("kind", ["missing", "extra", "hash", "receipt", "input", "rows"])
def test_aggregation_rejects_corrupted_artifacts(kind, manifest, tmp_path):
    path, binary, _ = manifest
    output = tmp_path/"output"
    MODULE.run_local(path, binary, output, 1)
    directory = output/"tasks/task-0001"
    if kind == "missing": (directory/"receipt.json").unlink()
    elif kind == "extra": (directory/"extra.txt").write_text("extra")
    elif kind == "hash": (directory/"rep-0000.jsonl").write_text("{}\n")
    elif kind == "receipt":
        receipt = MODULE.strict_json((directory/"receipt.json").read_bytes())
        receipt["source"] = {}
        (directory/"receipt.json").write_bytes(MODULE.payload(receipt))
    elif kind == "input": (directory/"rep-0000.txt").write_text("wrong input")
    elif kind == "rows": (directory/"rows.json").write_text("[]")
    with pytest.raises((ValueError, FileNotFoundError)):
        MODULE.aggregate(path, output/"tasks", tmp_path/"invalid_aggregate")
    assert not (tmp_path/"invalid_aggregate").exists()


def test_process_failure_keeps_raw_evidence_without_receipt(manifest, design, tmp_path):
    path, binary, value = manifest
    fake_binary(binary, fail_process=True)
    directory = tmp_path/"task"
    with pytest.raises(ValueError, match="exit 9"):
        MODULE.run_task(value, MODULE.sha(path.read_bytes()), value["tasks"][0], binary, directory, design[0])
    assert (directory/"rep-0000.txt").exists()
    assert "deliberate" in (directory/"rep-0000.stderr").read_text()
    assert not (directory/"receipt.json").exists()


@pytest.mark.parametrize("text", ['{"x":NaN}', '{"x":Infinity}', '{"x":1,"x":2}', 'not json'])
def test_strict_json(text):
    with pytest.raises(ValueError):
        MODULE.strict_json(text)
