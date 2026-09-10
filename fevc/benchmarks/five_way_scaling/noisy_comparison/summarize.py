"""Validate and report all twenty noisy-comparison calls, including disagreements."""
from __future__ import annotations
import argparse
import csv
import hashlib
import json
import math
import statistics
from pathlib import Path

ROLES = ("fevc", "matlab", "julia", "r", "pytwoway")
TARGETS = ("worker", "firm", "covariance", "total")
LABELS = dict(fevc="FEVC", matlab="KSS MATLAB", julia="Julia", r="R", pytwoway="PyTwoWay")


def read_json(path):
    return json.loads(path.read_text())


def read_csv(path, delimiter=","):
    with path.open(newline="") as handle:
        return list(csv.DictReader(handle, delimiter=delimiter))


def write_csv(path, rows):
    with path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]), lineterminator="\n")
        writer.writeheader(); writer.writerows(rows)


def load_calls(run):
    tasks = read_csv(run/"input/tasks.tsv", "\t")
    if len(tasks) != 4 or [int(t["task_id"]) for t in tasks] != [1, 2, 3, 4]:
        raise ValueError("missing or duplicate task keys")
    if sorted(p.name for p in (run/"output/smoke").iterdir() if p.is_dir()) != [f"task-{i:03}" for i in range(1, 5)]:
        raise ValueError("task inventory changed")
    expected_hash = read_json(run/"input/fixture.json")["sha256"]
    if hashlib.sha256((run/"input/fixture.csv").read_bytes()).hexdigest() != expected_hash:
        raise ValueError("input identity changed")
    oracle = read_json(run/"input/oracle.json")
    calls, checks = [], []
    for task in tasks:
        folder = run/"output/smoke"/f"task-{int(task['task_id']):03}"
        if not (folder/"wrapper.pass").is_file() or (folder/"wrapper.fail").exists():
            raise ValueError("task wrapper did not pass")
        actual_task = read_json(folder/"task.json")
        if any(str(actual_task[k]) != str(v) for k, v in task.items()):
            raise ValueError("task metadata changed")
        if read_json(folder/"input.json")["sha256"] != expected_hash:
            raise ValueError("task input changed")
        for role in ROLES:
            directory = folder/role
            status = read_json(directory/"status.json")
            monitor = read_json(directory/"process_tree.json")
            if role == "fevc":
                results = read_csv(directory/"result.csv")
                if len(results) != 1:
                    raise ValueError("duplicate or missing FEVC result")
                result = results[0]
            else:
                result = read_json(directory/"result.json")
            if any(v.get("status") != "PASS" for v in (status, monitor, result)):
                raise ValueError("failed role output")
            if result["role"] != role or result["algorithm"] != task["algorithm"]:
                raise ValueError("role or algorithm mismatch")
            for key in ("rows", "cores", "seed", "probes"):
                if int(float(result[key])) != int(task[key]):
                    raise ValueError(f"{key} mismatch")
            if int(result["retained_rows"]) != int(task["rows"]):
                raise ValueError("retained sample changed")
            if monitor["phase_sample_count"] < 1 or not monitor["phase_start_observed"] or not monitor["phase_end_observed"]:
                raise ValueError("incomplete phase memory measurements")
            factor = (int(task["rows"])-1)/int(task["rows"]) if role in ("matlab", "julia", "r") else 1.0
            if abs(float(result["normalization_factor"])-factor) > 1e-15:
                raise ValueError("normalization changed")
            values = {k: float(result[f"normalized_{k}"]) for k in TARGETS}
            if not all(map(math.isfinite, values.values())):
                raise ValueError("nonfinite estimates")
            if abs(values["total"]-values["worker"]-values["firm"]-2*values["covariance"]) > 1e-9:
                raise ValueError("variance accounting identity failed")
            primary = float(result["primary_seconds"])
            if not math.isfinite(primary) or primary <= 0:
                raise ValueError("invalid time")
            row = dict(task_id=int(task["task_id"]), algorithm=task["algorithm"],
                repeat=int(task["repeat"]), seed=int(task["seed"]), role=role,
                rows=int(task["rows"]), cores=int(task["cores"]),
                primary_seconds=primary, phase_peak_rss_mib=monitor["phase_peak_rss_kib"]/1024,
                **values)
            calls.append(row)
            for target in TARGETS:
                expected = oracle["targets"][target]; gap = values[target]-expected
                tolerance = max(1e-8, 1e-5*max(1, abs(expected)))
                correction = oracle["correction"][target]
                checks.append(dict(task_id=row["task_id"], algorithm=row["algorithm"],
                    repeat=row["repeat"], role=role, target=target, estimate=values[target],
                    oracle=expected, plugin=oracle["plugin"][target], signed_gap=gap,
                    implied_correction=oracle["plugin"][target]-values[target],
                    oracle_correction=correction, gap_fraction_of_correction=gap/abs(correction),
                    absolute_gap=abs(gap), tolerance=tolerance, within_tolerance=abs(gap)<=tolerance))
    return calls, checks, oracle


def figures(out, calls, oracle):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    fig, axes = plt.subplots(2, 2, figsize=(11.5, 7.5))
    for ax, target in zip(axes.flat, TARGETS):
        for index, role in enumerate(ROLES):
            exact = next(row[target] for row in calls if row["role"] == role and row["algorithm"] == "exact")
            jla = [row[target] for row in calls if row["role"] == role and row["algorithm"] == "jla"]
            median = statistics.median(jla)
            ax.plot(exact, index-0.12, marker="o", color="#0072B2", label="Exact" if index == 0 else None)
            ax.errorbar(median, index+0.12, xerr=[[median-min(jla)], [max(jla)-median]],
                        fmt="s", color="#D55E00", capsize=3, label="JLA median / range" if index == 0 else None)
        ax.axvline(oracle["targets"][target], color="#009E73", label="Dense oracle", linestyle="--")
        ax.axvline(oracle["plugin"][target], color="#666666", label="Uncorrected", linestyle=":")
        ax.set_yticks(range(5), [LABELS[r] for r in ROLES]); ax.invert_yaxis()
        ax.set_title(target.capitalize()); ax.grid(axis="x", alpha=.2)
    axes[0, 0].legend(fontsize=8, loc="best")
    fig.suptitle("3,840 rows, 2 cores: large bias correction; one outcome draw")
    fig.tight_layout()
    for ext in ("png", "svg"):
        fig.savefig(out/f"estimates.{ext}", dpi=220, bbox_inches="tight")
    plt.close(fig)
    fig, axes = plt.subplots(1, 2, figsize=(11.5, 4))
    for ax, key, label in zip(axes, ("primary_seconds", "phase_peak_rss_mib"), ("Primary runtime (seconds)", "Primary peak process-tree RSS (MiB)")):
        for shift, algorithm, color in ((-.18, "exact", "#0072B2"), (.18, "jla", "#D55E00")):
            vals = [statistics.median(row[key] for row in calls if row["role"] == role and row["algorithm"] == algorithm) for role in ROLES]
            ax.bar([i+shift for i in range(5)], vals, width=.34, label=algorithm.upper(), color=color)
        ax.set_xticks(range(5), [LABELS[r] for r in ROLES], rotation=15)
        ax.set_yscale("log"); ax.set_ylabel(label); ax.legend(); ax.grid(axis="y", alpha=.2)
    fig.suptitle("Single-size diagnostic timings: exact once; JLA median of three calls")
    fig.tight_layout()
    for ext in ("png", "svg"):
        fig.savefig(out/f"performance.{ext}", dpi=220, bbox_inches="tight")
    plt.close(fig)


def summarize(run, plot=True):
    calls, checks, oracle = load_calls(run)
    out = run/"diagnostic"; out.mkdir(exist_ok=True)
    write_csv(out/"measurements.csv", calls)
    write_csv(out/"consistency.csv", checks)
    exact_table = "\n".join("| "+LABELS[role]+" | "+" | ".join(
        f"{next(row[k] for row in calls if row['role']==role and row['algorithm']=='exact'):.8f}" for k in TARGETS)+" |" for role in ROLES)
    jla_table = "\n".join("| "+LABELS[role]+" | "+" | ".join(
        f"{statistics.median(row[k] for row in calls if row['role']==role and row['algorithm']=='jla'):.8f}" for k in TARGETS)+" |" for role in ROLES)
    reference_table = "\n".join("| "+name+" | "+" | ".join(f"{oracle[key][k]:.8f}" for k in TARGETS)+" |" for name, key in (("Latent finite-design truth", "true_components"), ("Uncorrected OLS", "plugin"), ("Dense corrected oracle", "targets"), ("Oracle correction (subtracted)", "correction")))
    counts = {role: sum(c["within_tolerance"] for c in checks if c["role"]==role and c["algorithm"]=="exact") for role in ROLES}
    report = f"""# Noisy-fixture comparison

3,840 observations, 1,280 workers, 32 firms, three observations per worker,
two active cores, one fixed outcome draw. All 20 estimator calls completed.
Source identities and the DGP are frozen under `input/identity.json` and
`code/NOISY_PROTOCOL.md`. These are diagnostic results, not a scale benchmark.

The corrected worker variance subtracts {oracle['correction_fraction_of_plugin']['worker']:.1%}
of its plug-in value; firm variance subtracts
{oracle['correction_fraction_of_plugin']['firm']:.1%}. R-squared is
{oracle['r_squared']:.4f}. The maximum deletion leverage is
{oracle['maximum_leverage']:.6f}; the normal-equation residual is
{oracle['normal_equation_residual']:.3g}.

The outcome was centered identically before every fit, so original and
centered-score references differ by at most
{max(abs(oracle['targets'][k]-oracle['centered_targets'][k]) for k in TARGETS):.3g}.
Known latent moments are shown as context. The corrected estimates need not
equal those moments in a single outcome realization.

| Reference | Worker | Firm | Covariance | Total |
|---|---:|---:|---:|---:|
{reference_table}

## Exact implementations

| Implementation | Worker | Firm | Covariance | Total |
|---|---:|---:|---:|---:|
{exact_table}

Original tolerance checks passed out of four: {json.dumps(counts)}.
`consistency.csv` records all absolute gaps and gaps relative to the oracle
correction, including every failed check.

## 280-projection approximation

Each entry is the median across three fixed projection seeds on the same
outcome. The plot shows their full range. These ranges describe algorithmic
randomness; they are not confidence intervals or sampling uncertainty.

| Implementation | Worker | Firm | Covariance | Total |
|---|---:|---:|---:|---:|
{jla_table}

![Exact and randomized estimates](estimates.png)

![Runtime and memory](performance.png)

Timing includes package-specific cleaning, pool setup, estimation and target
extraction, excludes generic CSV import, and uses the same primary phase as
the prior smoke. RSS is sampled over the entire process tree every 100 ms.
The run reserves four scheduler slots for memory/headroom, with two active
cores per role. Hardware is shared; timings are descriptive.

Comparator formulas and solver settings were not repaired. R exact and JLA
paths are evaluated separately because the previously diagnosed extra
normalization occurs in its exact correction weights. The prior failed run
remains unchanged.
"""
    (out/"report.md").write_text(report)
    result = dict(schema="FEVC-NOISY-SUMMARY-V1", status="OUTPUTS_COMPLETE",
                  calls=len(calls), comparisons=len(checks), exact_checks_passed=counts,
                  correction_fraction_of_plugin=oracle["correction_fraction_of_plugin"],
                  scheduler_accounting="validate after job completion")
    (out/"summary.json").write_text(json.dumps(result, indent=2)+"\n")
    if plot:
        figures(out, calls, oracle)
    print(json.dumps(result))
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(); parser.add_argument("--run", type=Path, required=True)
    args = parser.parse_args(); summarize(args.run)
