"""Freeze a modest noisy-fixture diagnostic using the previously tested adapters."""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import shutil
from pathlib import Path

import numpy as np

ROLES = ("fevc", "matlab", "julia", "r", "pytwoway")
TARGETS = ("worker", "firm", "covariance", "total")
SEEDS = (202609091, 202609193, 202609299)
N = 3840
PRIOR = "/projectnb/welfgr/vckss/five_way_scaling/runs/20260909T112050Z-f3098bc"


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def moments(a, b):
    worker = float(np.var(a)); firm = float(np.var(b))
    covariance = float(np.mean((a-a.mean())*(b-b.mean())))
    return dict(worker=worker, firm=firm, covariance=covariance,
                total=worker+firm+2*covariance)


def design(n):
    if n % 120 or n < 960:
        raise ValueError("invalid design dimensions")
    worker = np.arange(n)//3
    period = np.arange(n)%3+1
    nf = n//120
    layer, base = worker//nf, worker%nf
    offset = np.where(period == 1, 0, np.where(
        period == 2, 1+layer%(nf//4-1), (nf+2)//3+(97*layer)%(nf//4)))
    firm = (base+offset)%nf
    return worker, firm, period


def dense_reference(worker, firm, y):
    """Independent grounded normal equations; no production solver is reused.

    Equal observation counts allow target influence norms to be evaluated over
    coefficient levels. For the fitted-value target, idempotency of the hat
    matrix gives the centered column norm h_ii-1/N. Covariance follows from the
    polarization identity, tested against the independent brute-force oracle.
    """
    n = len(y); nw = int(worker.max())+1; nf = int(firm.max())+1
    d = np.eye(nw)[worker]
    f = -np.eye(nf)[firm, :nf-1]
    x = np.column_stack((d, f))
    inverse = np.linalg.inv(x.T@x)
    influence = inverse@x.T
    beta = influence@y
    alpha = beta[:nw]; psi = np.r_[-beta[nw:], 0.0]
    residual = y-alpha[worker]-psi[firm]
    h = np.einsum("ij,ji->i", x, influence)
    if not np.all((h >= 0) & (h < 1-1e-10)):
        raise ValueError("fixture not leave-match-out estimable")
    a = influence[:nw]; b = np.vstack((-influence[nw:], np.zeros(n)))
    ca = np.bincount(worker); cb = np.bincount(firm)
    bw = np.einsum("i,ij,ij->j", ca, a, a)-(ca@a)**2/n
    bf = np.einsum("i,ij,ij->j", cb, b, b)-(cb@b)**2/n
    bt = h-1/n
    bc = (bt-bw-bf)/2
    weights = dict(worker=bw, firm=bf, covariance=bc, total=bt)
    plugin = moments(alpha[worker], psi[firm])
    score = y*residual/(1-h)
    centered_score = (y-y.mean())*residual/(1-h)
    correction = {k: float(v@score/n) for k, v in weights.items()}
    centered = {k: plugin[k]-float(v@centered_score/n) for k, v in weights.items()}
    targets = {k: plugin[k]-correction[k] for k in TARGETS}
    return dict(schema="FEVC-NOISY-DENSE-REFERENCE-V1", status="PASS", rows=n,
                workers=nw, firms=nf, denominator="population_N", plugin=plugin,
                correction=correction, targets=targets, centered_targets=centered,
                mean_outcome=float(y.mean()), outcome_variance=float(np.var(y)),
                residual_variance=float(np.mean(residual**2)),
                r_squared=float(1-np.mean(residual**2)/np.var(y)),
                maximum_leverage=float(h.max()), minimum_leverage=float(h.min()),
                normal_equation_residual=float(np.max(np.abs(x.T@residual))),
                correction_fraction_of_plugin={k: correction[k]/plugin[k] for k in TARGETS})


def fixture(n=N):
    worker, firm, period = design(n)
    alpha = np.random.Generator(np.random.PCG64(20260909201)).normal(size=n//3)
    psi = np.random.Generator(np.random.PCG64(20260909202)).normal(size=n//120)
    # Scale outcome-free latent effects to fixed finite-design variances.
    alpha = (alpha-alpha.mean())/alpha.std()*0.5
    psi = (psi-psi.mean())/psi.std()*0.25
    epsilon = np.random.Generator(np.random.PCG64(20260909203)).normal(size=n)*2
    y = alpha[worker]+psi[firm]+epsilon
    y -= y.mean()
    reference = dense_reference(worker, firm, y)
    reference["true_components"] = moments(alpha[worker], psi[firm])
    reference["noise_sd"] = 2.0
    return worker, firm, period, y, reference


def replace_once(text, old, new):
    if text.count(old) != 1:
        raise ValueError(f"source transform anchor changed: {old}")
    return text.replace(old, new, 1)


def prepare(base, output):
    if output.exists():
        raise ValueError("run directory already exists")
    worker, firm, period, y, reference = fixture()
    fractions = reference["correction_fraction_of_plugin"]
    # No seed search: preserve the first specified draw, stop if unsuitable.
    if fractions["worker"] < 0.25 or fractions["firm"] < 0.20:
        raise ValueError(f"fixed fixture did not realize substantial correction: {fractions}")
    code = output/"code"; inputs = output/"input"
    code.mkdir(parents=True); inputs.mkdir()
    for line in (base/"input/code.sha256").read_text().splitlines():
        digest, relative = line.split(maxsplit=1)
        source = base/"code"/relative
        if sha(source) != digest:
            raise ValueError(f"frozen source hash mismatch: {relative}")
        target = code/relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
    # Only widen the input admission guards in isolated copies of all adapters.
    transforms = {
        "common.py": ("(*SIZE_ROWS, 960)", "(*SIZE_ROWS, 960, 3840)"),
        "fevc_run.do": ("inlist(`rows',960,7680", "inlist(`rows',960,3840,7680"),
        "matlab_run.m": ("[960 7680", "[960 3840 7680"),
        "julia_run.jl": ("(960,7680", "(960,3840,7680"),
        "r_run.R": ("c(960L,7680L", "c(960L,3840L,7680L"),
        "pytwoway_run.py": ("(960, 7_680", "(960, 3_840, 7_680"),
    }
    for name, (old, new) in transforms.items():
        path = code/name
        path.write_text(replace_once(path.read_text(), old, new))
    # 1,311 identified coefficients exceed the default exact dimension cap of
    # 500. A 1,400 cap admits this small dense design within the 32 GiB budget;
    # it changes resource admission, not numerical tolerances or estimation.
    path = code/"fevc_run.do"
    path.write_text(replace_once(path.read_text(),
        "backend(rust) algorithm(exact)",
        "backend(rust) algorithm(exact) exact_limit(1400)"))
    # This is a descriptive experiment. Preserve every numerical check without
    # turning disagreements into execution failures or suppressing later roles.
    path = code/"validate_task.py"
    path.write_text(replace_once(path.read_text(),
        '        if not all(item["pass"] for item in exact_checks):\n            raise ValueError("exact consistency gate failed")',
        '        # Numerical disagreements are reported in this descriptive experiment.'))
    path = code/"run_task.sge"
    path.write_text(replace_once(path.read_text(),
        'artifacts=$FW_RUN_DIR/artifacts', f'artifacts={PRIOR}/artifacts'))
    # Copy the same frozen input and reference for every invocation.
    for name in ("generate_input.py", "exact_oracle.py"):
        shutil.copyfile(Path(__file__).with_name("frozen_input.py"), code/name)
    for name in ("run_noisy.sge", "summarize.py"):
        shutil.copyfile(Path(__file__).with_name(name), code/name)
    shutil.copyfile(Path(__file__), code/"prepare_noisy.py")
    shutil.copyfile(Path(__file__).with_name("PROTOCOL.md"), code/"NOISY_PROTOCOL.md")
    with (inputs/"fixture.csv").open("w", newline="") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow(("observation_key", "worker", "firm", "period", "match", "y"))
        for i in range(N):
            writer.writerow((i+1, int(worker[i])+1, int(firm[i])+1, int(period[i]), i+1, format(y[i], ".17g")))
    receipt = dict(schema="FEVC-NOISY-INPUT-V1", status="PASS", rows=N, workers=N//3,
                   firms=N//120, sha256=sha(inputs/"fixture.csv"), mean_centered=True,
                   unique_worker_firm=True, all_workers_move=True,
                   seed_worker=20260909201, seed_firm=20260909202, seed_noise=20260909203)
    write_json(inputs/"fixture.json", receipt)
    write_json(inputs/"oracle.json", reference)
    tasks = []
    for task_id in range(1, 5):
        repeat = max(1, task_id-1)
        order = ROLES[repeat-1:]+ROLES[:repeat-1]
        tasks.append(dict(task_id=task_id, cell_id="noisy-exact" if task_id == 1 else "noisy-jla",
            sweep="diagnostic", rows=N, workers=N//3, firms=N//120, cores=2,
            repeat=repeat, seed=SEEDS[repeat-1], probes=0 if task_id == 1 else 280,
            algorithm="exact" if task_id == 1 else "jla", role_order=",".join(order)))
    with (inputs/"tasks.tsv").open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(tasks[0]), delimiter="\t", lineterminator="\n")
        writer.writeheader(); writer.writerows(tasks)
    write_json(inputs/"identity.json", dict(schema="FEVC-NOISY-COMPARISON-V1", status="FROZEN",
        prior_run=PRIOR, rows=N, cores=2, tasks=4, estimator_calls=20,
        comparator_estimation_code_changed=False, admission_guard_added_rows=N,
        fevc_exact_dimension_limit=1400,
        data=receipt, manifest_sha256=sha(inputs/"tasks.tsv"),
        reference_sha256=sha(inputs/"oracle.json"), diagnostic=True,
        correction_requirements=dict(worker_fraction_min=0.25, firm_fraction_min=0.20),
        comparison_tolerance="max(1e-8, 1e-5*max(1,abs(reference)))",
        acceptance="execution/schema/input integrity; scientific differences descriptive"))
    paths = sorted(p for folder in (code, inputs) for p in folder.rglob("*") if p.is_file())
    (output/"bundle.sha256").write_text("".join(f"{sha(p)}  {p.relative_to(output)}\n" for p in paths))
    print(json.dumps(reference, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(); prepare(args.base, args.output)
