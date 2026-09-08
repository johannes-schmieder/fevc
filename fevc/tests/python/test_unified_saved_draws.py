"""Adversarial checks for the unified fitter's saved-draw comparison harness."""
import copy
import importlib.util
import gzip
import json
from pathlib import Path
import sys

import pytest

ROOT = Path(__file__).resolve().parents[3]
HERE = ROOT / "rust/experiments/unified_residual_moments"
sys.path.insert(0, str(HERE))
SPEC = importlib.util.spec_from_file_location("unified_replay", HERE / "run.py")
RUN = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUN)
sys.path.pop(0)
V = sys.modules["validation"]


def fixture(family="match_q0", arm="unified"):
    task = next(t for t in V.OLD.tasks("tiny") if t["family"] == family)
    residual = family == "observation" or arm == "unified"
    design = {"kind": "design", "truth": [1, 1, 0, 2], "n": 144, "min_variance": 1}
    if family == "observation":
        design.update(p=23, max_h=.2, leading_share=[.1]*4, remainder_share=[.1]*4)
    records = [{"kind": "task", "schema": "individual-v1",
                **{k: task[k] for k in ("family", "profile", "cell", "k", "start", "reps", "master", "numseed")}}, design]
    for rep in range(2):
        spectrum = [.1]*60
        for target in range(4):
            spectrum[15*target+12:15*target+15] = [128, 512, .002]
        atoms = (1000+128+2+(512 if residual else 0))*144
        records.append({"kind": "call", "replication": rep,
                        "seed": V.OLD.seed(task["master"], task["cell"], task["k"], rep),
                        "seconds": .1, "status": "success", "point": [1, 1, 0, 2], "point_mcse": [.01]*4,
                        "targets": [1, 0]*3+[6, 0], "q1": [None]*80, "spectrum": spectrum,
                        "primitive": [1, 0, 0, 0, 1, 0, 0, 0, 1],
                        "covariance": [1, 0, 0, 1, 0, 1, 0, 1, 0, 0, 1, 2, 1, 1, 2, 6],
                        "fit": 2 if residual else 1, "joint": 0,
                        "gram_probes": 512 if residual else 0,
                        "gram_rcond": [.1 if residual else None],
                        "ordering": (2 if family == "observation" else 3) if residual else 0,
                        "gram_inverse_relres": [1e-12 if residual else None],
                        "fit_relres": [1e-12 if residual else None],
                        "positivity_floor": [1e-8 if residual else None], "nonpositive": 0,
                        "max_residual": 1e-12, "residual_gate": 1e-9,
                        "units": 144, "solver_columns": 512, "counter_atoms": atoms,
                        "counter_words": 2*atoms, "peak": 10000, "floored": 0,
                        "computed": 4, "critical_draws": 0})
    return task, records


@pytest.mark.parametrize("family,arm", [(f, a) for f in ("observation", "match_q0") for a in RUN.BUILD.ARMS])
def test_real_metadata_validated_without_rewriting_raw_records(family, arm):
    task, records = fixture(family, arm)
    before = copy.deepcopy(records)
    assert len(V.validate(records, task, arm)[1]) == 2
    assert records == before


@pytest.mark.parametrize("fault", ["missing", "duplicate", "seed", "fitter", "ordering", "gram",
                                  "floor", "counter", "spectrum", "joint", "point", "covariance",
                                  "nonfinite", "schema", "units", "moment_residual", "failure"])
def test_malformed_candidate_is_rejected(fault):
    task, records = fixture()
    row = records[2]
    if fault == "missing": records.pop()
    elif fault == "duplicate": records[-1] = copy.deepcopy(row)
    elif fault == "seed": row["seed"] += 1
    elif fault == "fitter": row["fit"] = 1
    elif fault == "ordering": row["ordering"] = 2
    elif fault == "gram": row["gram_probes"] = 0
    elif fault == "floor": row["nonpositive"] = 1
    elif fault == "counter": row["counter_atoms"] += 1
    elif fault == "spectrum": row["spectrum"][13] = 128
    elif fault == "joint": row["joint"] = 2
    elif fault == "point": row["point"][3] = 3
    elif fault == "covariance": row["covariance"][3] = 2
    elif fault == "nonfinite": row["point"][0] = float("nan")
    elif fault == "schema": records[0]["schema"] = "bad"
    elif fault == "units": row["units"] = 0
    elif fault == "moment_residual": row["fit_relres"] = [.1]
    elif fault == "failure": row.update(status="shared_failure", detail="unclassified")
    with pytest.raises(ValueError):
        V.validate(records, task, "unified")


def test_inventory_reuses_only_registered_saved_draws():
    parent = {"cells": V.OLD.CELLS, "tasks": list(V.OLD.tasks("development"))}
    tiny = list(RUN.tasks(parent, "tiny"))
    full = list(RUN.tasks(parent, "comparison"))
    assert len(tiny) == 48 and 2*sum(t["reps"] for t in tiny) == 192
    assert len(full) == 192 and len({t["id"] for t in full}) == 192
    assert 2*sum(t["reps"] for t in full if t["family"] != "observation") == 22400
    obs = [t for t in full if t["family"] == "observation"]
    assert len(obs) == 80 and {t["start"] for t in obs} == {0, 133, 266, 399}
    assert all(t["reps"] == 1 for t in obs)
    assert all(t["profile"] == "development" and t["start"]+t["reps"] <= 400 for t in full)


def test_adaptation_changes_policy_not_generators_or_common_budgets():
    raw = (RUN.BUILD.LEGACY / "public_api.rs").read_text()
    old = RUN.BUILD.adapt(raw, "baseline")
    new = RUN.BUILD.adapt(raw, "unified")
    for name in ("component", "match_component"):
        new = new.replace(f"augment_{name}_inference_interrupt_v3", f"augment_{name}_inference_interrupt_v2")
    assert new == old
    assert "if observation || !q1 {512}else{128}" in old
    assert "rust/crates/vckss-core/src/cmg_impl.rs.in" in RUN.BUILD.source_files()
    with pytest.raises(ValueError): RUN.BUILD.adapt(raw.replace("if observation{512}else{128}", "changed"), "unified")


def test_conservatism_not_reclassified_as_historical_calibration_pass():
    row = dict(arm="unified", family="match_q0", cell="test", target="worker", gate="correct",
               success_rate=1., successes=400, se_denominator=400, coverage=1., coverage_mcse=0.,
               se_ratio=.7, bias=0., bias_mcse=.01)
    assert V.readiness([row]) == []
    assert V.OLD.Q1._gate_correct("test", row)
    for changed, expected in (({"coverage": .8}, "undercoverage"),
                              ({"success_rate": .9}, "availability"),
                              ({"se_ratio": 1.3}, "underestimated SE"),
                              ({"se_denominator": 399}, "incomplete SE")):
        assert any(expected in error for error in V.readiness([{**row, **changed}]))


def test_target_failure_keeps_its_point_and_shared_failure_stays_explicit():
    task, records = fixture()
    row = records[2]
    row.update(joint=2, primitive=[None]*9, covariance=[None]*16, computed=3, targets=[-1, 1]+[1, 0]*2+[6, 0])
    assert len(V.validate(records, task, "unified")[1]) == 2
    target = V.target_row(row, records[1], V.OLD.spec_for(task), 0)
    assert target["status"] == "target_1" and target["point_error"] == 0
    assert "covered" not in target
    row.update(status="shared_failure", detail="solve: status 2: test rank failure")
    assert len(V.validate(records, task, "unified")[1]) == 2
    assert V.target_row(row, records[1], V.OLD.spec_for(task), 0)["point_error"] is None


def test_point_replay_mismatch_is_a_blocker():
    _, records = fixture()
    saved = copy.deepcopy(records[2])
    assert V.compare_points(saved, records[2], saved) == []
    records[2]["point"][0] += .01
    assert "unified changed point" in V.compare_points(saved, records[2], saved)


@pytest.fixture
def stored_task(tmp_path):
    task, records = fixture()
    manifest = {"binaries": {"unified": {task["family"]: {"sha256": "test-binary"}}}}
    RUN.write(tmp_path / "manifest.json", manifest)
    folder = tmp_path / "tasks" / task["id"] / "unified"
    folder.mkdir(parents=True)
    path = folder / "rows.jsonl.gz"
    path.write_bytes(gzip.compress("\n".join(json.dumps(r) for r in records).encode()))
    receipt = {"status": "VALIDATED", "task": task, "arm": "unified",
               "manifest_sha256": RUN.sha(tmp_path / "manifest.json"), "binary_sha256": "test-binary",
               "output_sha256": RUN.sha(path), "calls": 2, "targets": 8,
               "shared_failures": 0, "design": records[1]}
    RUN.write(folder / "receipt.json", receipt)
    return tmp_path, manifest, task, folder, receipt


@pytest.mark.parametrize("fault", ["missing", "hash", "count", "binary", "manifest", "status", "shared"])
def test_task_receipt_integrity(stored_task, fault):
    output, manifest, task, folder, receipt = stored_task
    assert len(RUN.read_task(output, manifest, task, "unified")[1]) == 2
    if fault == "missing": (folder / "rows.jsonl.gz").unlink()
    elif fault == "hash": (folder / "rows.jsonl.gz").write_bytes(b"corrupt")
    elif fault == "count": receipt["calls"] = 1
    elif fault == "binary": receipt["binary_sha256"] = "wrong"
    elif fault == "manifest": receipt["manifest_sha256"] = "wrong"
    elif fault == "status": receipt["status"] = "EXECUTION_FAILURE"
    elif fault == "shared": receipt["shared_failures"] = 1
    (folder / "receipt.json").write_text(json.dumps(receipt))
    with pytest.raises((ValueError, OSError)):
        RUN.read_task(output, manifest, task, "unified")


def test_process_failure_gets_a_failure_receipt_not_a_success(tmp_path):
    task, _ = fixture()
    receipt = RUN.execute(Path(sys.executable), tmp_path / "failure", task, "unified", "test-manifest")
    assert receipt["status"] == "EXECUTION_FAILURE"
    assert "process exit" in receipt["detail"]
    assert (tmp_path / "failure/rows.jsonl.partial").is_file()
    assert not (tmp_path / "failure/rows.jsonl.gz").exists()
