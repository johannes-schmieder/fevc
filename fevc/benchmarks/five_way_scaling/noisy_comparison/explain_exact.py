"""Reproduce specific exact-path arithmetic from the pinned comparator sources.

This is an independent post-run diagnosis, not a patch to any comparator.
R: leverages_parallel.R divides Bii variance weights by N-1 before
kss_quadratic_form.R divides the correction by N-1 a second time.
PyTwoWay: fe.py _construct_AAinv_components_full adds vector Dwinv to a
matrix, broadcasting it across rows instead of constructing a diagonal.
The latter leaves the inverse diagonal intact in this equal-T fixture,
but changes worker target traces and covariance traces.
"""
import argparse
import csv
import json
from pathlib import Path

import numpy as np


def explain(run):
    with (run/"input/fixture.csv").open() as handle:
        data = list(csv.DictReader(handle))
    worker = np.array([int(r["worker"])-1 for r in data])
    firm = np.array([int(r["firm"])-1 for r in data])
    y = np.array([float(r["y"]) for r in data])
    n = len(y); nw = int(worker.max())+1; nf = int(firm.max())+1
    counts = np.bincount(worker)
    if not np.all(counts == 3):
        raise ValueError("diagnosis assumes exactly three observations per worker")
    d = np.eye(nw)[worker]; f = -np.eye(nf)[firm, :nf-1]
    x = np.column_stack((d, f)); influence = np.linalg.solve(x.T@x, x.T)
    beta = influence@y; residual = y-x@beta
    leverage = np.einsum("ij,ji->i", x, influence)
    score = (y-y.mean())*residual/(1-leverage)
    # In the wrong inverse block, adding 1/T across every column replaces
    # diag(1/T). Multiplying by the worker dummies gives this influence map.
    aw = influence[:nw]+1/3-d.T/3
    af = np.vstack((-influence[nw:], np.zeros(n)))
    aw -= aw.mean(axis=0); af -= af.mean(axis=0)
    bw = np.einsum("i,ij,ij->j", counts, aw, aw)
    bf = np.einsum("i,ij,ij->j", np.bincount(firm), af, af)
    bc = np.zeros(n)
    for start in range(0, n, 128):
        part = slice(start, start+128)
        bc += np.sum(aw[worker[part]]*af[firm[part]], axis=0)
    oracle = json.loads((run/"input/oracle.json").read_text())
    predictions = {"pytwoway": {k: oracle["plugin"][k]-float(weights@score/n)
                   for k, weights in (("worker", bw), ("firm", bf), ("covariance", bc))}}
    py = predictions["pytwoway"]; py["total"] = py["worker"]+py["firm"]+2*py["covariance"]
    predictions["r"] = {k: oracle["plugin"][k]-oracle["correction"][k]/(n-1)
                         for k in ("worker", "firm")}
    rows = []
    for role, targets in predictions.items():
        result = json.loads((run/f"output/smoke/task-001/{role}/result.json").read_text())
        for target, prediction in targets.items():
            observed = float(result[f"normalized_{target}"])
            rows.append(dict(role=role, target=target, predicted=prediction, observed=observed,
                             absolute_gap=abs(prediction-observed), reproduced=abs(prediction-observed)<1e-7))
    result = dict(schema="FEVC-NOISY-EXACT-DIAGNOSIS-V1", comparator_code_changed=False,
                  scope="R variances and PyTwoWay four targets, pinned exact paths only",
                  all_reproduced=all(row["reproduced"] for row in rows), checks=rows)
    (run/"diagnostic").mkdir(exist_ok=True)
    (run/"diagnostic/exact_formula_diagnosis.json").write_text(json.dumps(result, indent=2)+"\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(); parser.add_argument("--run", type=Path, required=True)
    explain(parser.parse_args().run)
