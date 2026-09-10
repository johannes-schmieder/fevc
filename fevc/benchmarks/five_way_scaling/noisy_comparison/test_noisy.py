import importlib.util
import csv
import hashlib
import json
from pathlib import Path
import numpy as np
import pytest

path = Path(__file__).with_name("prepare.py")
spec = importlib.util.spec_from_file_location("noisy_prepare", path)
module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)


def test_noisy_reference_matches_independent_deletion_oracle():
    from fevc.tests.python.oracle import exact_kss
    worker = np.repeat(np.arange(12), 3)
    firm = (np.arange(36)%3+worker)%5
    y = np.random.default_rng(81).normal(size=36)
    y -= y.mean()
    observed = module.dense_reference(worker, firm, y)
    reference = exact_kss(y.tolist(), (worker+1).tolist(), (firm+1).tolist(), deletion="match")
    for i, target in enumerate(module.TARGETS):
        assert observed["targets"][target] == pytest.approx(reference.corrected[i], abs=1e-10)
        assert observed["targets"][target] == pytest.approx(observed["centered_targets"][target], abs=1e-12)


def test_frozen_source_transform_refuses_missing_or_duplicate_anchor():
    for text in ("none", "old old"):
        with pytest.raises(ValueError):
            module.replace_once(text, "old", "new")


def test_complete_collector_reports_disagreements_and_rejects_corruption(tmp_path):
    spec = importlib.util.spec_from_file_location("noisy_summary", Path(__file__).with_name("summarize.py"))
    summary = importlib.util.module_from_spec(spec); spec.loader.exec_module(summary)
    inputs = tmp_path/"input"; inputs.mkdir()
    (inputs/"fixture.csv").write_text("fixture\n")
    digest = hashlib.sha256((inputs/"fixture.csv").read_bytes()).hexdigest()
    module.write_json(inputs/"fixture.json", dict(sha256=digest))
    targets = dict(worker=.3, firm=.1, covariance=.02, total=.44)
    oracle = dict(targets=targets, plugin={k: 2*v for k, v in targets.items()}, correction=targets)
    module.write_json(inputs/"oracle.json", oracle)
    tasks = []
    for i in range(1, 5):
        task = dict(task_id=i, cell_id=f"cell-{i}", rows=3840, cores=2,
                    seed=202609091+i, probes=0 if i == 1 else 280,
                    algorithm="exact" if i == 1 else "jla", repeat=max(1, i-1))
        tasks.append(task)
        folder = tmp_path/"output/smoke"/f"task-{i:03}"; folder.mkdir(parents=True)
        (folder/"wrapper.pass").write_text("PASS\n")
        module.write_json(folder/"task.json", task)
        module.write_json(folder/"input.json", dict(sha256=digest))
        for role in module.ROLES:
            directory = folder/role; directory.mkdir()
            module.write_json(directory/"status.json", dict(status="PASS"))
            module.write_json(directory/"process_tree.json", dict(status="PASS", phase_sample_count=3,
                phase_start_observed=True, phase_end_observed=True, phase_peak_rss_kib=2048))
            factor = 3839/3840 if role in ("matlab", "julia", "r") else 1.
            result = dict(status="PASS", role=role, rows=3840, cores=2, seed=task["seed"],
                          probes=task["probes"], algorithm=task["algorithm"], retained_rows=3840,
                          primary_seconds=1., normalization_factor=factor)
            result.update({f"normalized_{k}": v*(2 if role == "r" else 1) for k, v in targets.items()})
            if role == "fevc":
                with (directory/"result.csv").open("w", newline="") as handle:
                    writer = csv.DictWriter(handle, fieldnames=list(result)); writer.writeheader(); writer.writerow(result)
            else:
                module.write_json(directory/"result.json", result)
    with (inputs/"tasks.tsv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(tasks[0]), delimiter="\t")
        writer.writeheader(); writer.writerows(tasks)
    calls, checks, _ = summary.load_calls(tmp_path)
    assert len(calls) == 20 and len(checks) == 80
    assert sum(not c["within_tolerance"] for c in checks) == 16
    result_path = tmp_path/"output/smoke/task-004/r/result.json"
    saved = json.loads(result_path.read_text())
    for key, value in (("seed", 19), ("normalized_worker", float("nan")), ("retained_rows", 3)):
        module.write_json(result_path, dict(saved, **{key: value}))
        with pytest.raises(ValueError):
            summary.load_calls(tmp_path)
    module.write_json(result_path, saved)
    result_path.unlink()
    with pytest.raises(FileNotFoundError):
        summary.load_calls(tmp_path)
