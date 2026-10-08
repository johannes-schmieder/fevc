"""Publication-style views of the accepted October 7 centering experiment.

This presentation overlay reads accepted evidence without regenerating or
modifying it. Kernel estimates retain every draw and use one common absolute
bandwidth for the four estimators within each panel.
"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

os.environ.setdefault("MPLCONFIGDIR", "/tmp/fevc-centering-report-matplotlib")
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.ticker import MaxNLocator
import numpy as np
import pandas as pd


TARGETS = (
    "Worker variance", "Firm variance", "Worker–firm covariance", "Variance of sum"
)
CASES = ("homo", "firm", "interaction")
CASE_LABELS = {"homo": "Homoskedastic", "firm": "Firm only", "interaction": "Interaction"}
MODES = ("none", "mean", "corrected")
COLORS = {"plugin": "#ca792a", "none": "#92999c", "mean": "#26689a", "corrected": "#297b57"}
LABELS = {"plugin": "Plug-in", "none": "None", "mean": "Mean", "corrected": "Corrected"}
STYLE = {
    "font.family": "serif", "font.serif": ["DejaVu Serif"],
    "mathtext.fontset": "dejavuserif", "font.size": 8,
    "axes.titlesize": 8.5, "axes.labelsize": 8, "xtick.labelsize": 7,
    "ytick.labelsize": 7, "axes.linewidth": .55,
    "axes.spines.top": False, "axes.spines.right": False,
    "axes.edgecolor": "#747474", "axes.axisbelow": True,
    "grid.color": "#e8e8e8", "grid.linewidth": .45,
    "figure.facecolor": "white", "axes.facecolor": "white",
    "savefig.facecolor": "white", "lines.antialiased": True,
}


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _read_inputs(root: Path):
    inventory = json.loads((root / "main.artifact-hashes.json").read_text())["files"]
    paths = sorted((root / "output/main").glob("*/task-*/results.csv"))
    paths.extend(root / "analysis/main" / f"{name}.csv" for name in
                 ("sampling", "centering-contrast", "calibration-pooled"))
    hashes = {}
    for path in paths:
        relative = str(path.relative_to(root))
        actual = _sha256(path)
        if inventory.get(relative) != actual:
            raise ValueError(f"Accepted evidence hash mismatch: {relative}")
        hashes[relative] = actual
    raw = pd.concat([pd.read_csv(p, na_values=["."]) for p in paths[:-3]], ignore_index=True)
    if len(raw) != 52227 or len(paths[:-3]) != 48 or (raw.rc != 0).any():
        raise ValueError("Expected all 52,227 successful fits in 48 accepted task files")
    sampling = raw[raw.phase == "sampling"].copy()
    key = ["case", "algorithm", "centering", "rep"]
    if sampling.duplicated(key).any() or len(sampling) != 36000:
        raise ValueError("Sampling inventory contains duplicate or missing fits")
    for case in CASES:
        for algorithm in ("exact", "jla"):
            for mode in MODES:
                subset = sampling[(sampling.case == case) & (sampling.algorithm == algorithm)
                                  & (sampling.centering == mode)]
                if set(subset.rep) != set(range(1, 2001)):
                    raise ValueError(f"Incomplete sampling draws: {case}/{algorithm}/{mode}")
                if algorithm == "jla" and not (subset.probes == 256).all():
                    raise ValueError("Sampling figure requires JLA with 256 probes")
                if not np.isfinite(subset[[f"point{i}" for i in range(1, 5)]].to_numpy()).all():
                    raise ValueError("Nonfinite estimates in sampling figure")
    return sampling, *(pd.read_csv(p) for p in paths[-3:]), hashes


def _density(values: np.ndarray, grid: np.ndarray, bandwidth: float) -> np.ndarray:
    # Direct Gaussian evaluation avoids estimator-specific Scott factors.
    result = np.zeros(len(grid))
    for start in range(0, len(values), 500):
        z = (grid[:, None] - values[None, start:start + 500]) / bandwidth
        result += np.exp(-.5 * z * z).sum(axis=1)
    return result / (len(values) * bandwidth * np.sqrt(2 * np.pi))


def _distribution_figure(case, raw, summary, destination, metadata):
    fig, axes = plt.subplots(4, 2, figsize=(7, 7.8))
    fig.subplots_adjust(left=.085, right=.985, bottom=.069, top=.907, hspace=.56, wspace=.27)
    handles = [Line2D([], [], color=COLORS[mode], lw=1.65,
                      linestyle=":" if mode == "corrected" else "-", label=LABELS[mode])
               for mode in ("plugin", *MODES)]
    handles.extend([Line2D([], [], color="black", lw=.85, label="Truth / zero"),
                    Line2D([], [], color="#555555", lw=.8, linestyle="--", label="Estimator mean")])
    fig.legend(handles=handles, loc="upper center", bbox_to_anchor=(.52, .993),
               ncol=3, frameon=False, fontsize=8, columnspacing=1.6, handlelength=2.4)
    metadata[case] = {"panels": [], "figure_inches": [7, 7.8]}
    for algorithm_index, algorithm in enumerate(("exact", "jla")):
        group = raw[(raw.case == case) & (raw.algorithm == algorithm)]
        for target_index, target in enumerate(TARGETS, 1):
            row = algorithm_index * 2 + (target_index - 1) // 2
            col = (target_index - 1) % 2
            ax = axes[row, col]
            truth_rows = summary[(summary.case == case) & (summary.target == target)]
            if not np.allclose(truth_rows.truth, truth_rows.truth.iloc[0], rtol=0, atol=1e-14):
                raise ValueError("Truth differs between estimators")
            truth = float(truth_rows.truth.iloc[0])
            shift = truth if algorithm == "jla" else 0.
            series = {"plugin": group[group.centering == "none"][f"plugin{target_index}"].to_numpy() - shift}
            series.update({mode: group[group.centering == mode][f"point{target_index}"].to_numpy() - shift
                           for mode in MODES})
            if not all(len(v) == 2000 and np.isfinite(v).all() for v in series.values()):
                raise ValueError("Each density must contain exactly 2,000 finite draws")
            bandwidth = float(1.06 * np.median([np.std(v, ddof=1) for v in series.values()]) * 2000 ** (-.2))
            if not bandwidth > 0:
                raise ValueError("Nonpositive common KDE bandwidth")
            left = min(float(np.min(v)) for v in series.values()) - 3.5 * bandwidth
            right = max(float(np.max(v)) for v in series.values()) + 3.5 * bandwidth
            grid = np.linspace(left, right, 700)
            reference = truth - shift
            ax.axvline(reference, color="black", lw=.85, zorder=2)
            for mode, values in series.items():
                ax.plot(grid, _density(values, grid, bandwidth), color=COLORS[mode],
                        linestyle=":" if mode == "corrected" else "-",
                        linewidth=1.9 if mode == "corrected" else 1.45, zorder=4)
                ax.axvline(np.mean(values), color=COLORS[mode], linestyle="--", lw=.7, alpha=.9, zorder=2)
            ax.set_xlim(left, right)
            ax.set_ylim(bottom=0)
            ax.grid(axis="both")
            ax.xaxis.set_major_locator(MaxNLocator(5))
            ax.yaxis.set_major_locator(MaxNLocator(4))
            letter = chr(ord("a") + algorithm_index * 4 + target_index - 1)
            prefix = "Exact" if algorithm == "exact" else "JLA(256)"
            ax.set_title(f"({letter}) {prefix}: {target}", loc="left", pad=5)
            ax.set_xlabel("Estimate" if algorithm == "exact" else "Estimate − truth", labelpad=2)
            ax.set_ylabel("Density", labelpad=2)
            metadata[case]["panels"].append({
                "algorithm": algorithm, "target": target, "n_per_estimator": 2000,
                "bandwidth": bandwidth, "bandwidth_rule": "1.06 × median estimator SD × 2000^(−1/5)",
                "xlim": [left, right], "reference": reference,
                "means": {mode: float(np.mean(v)) for mode, v in series.items()},
                "negative_draws": {mode: int(np.sum(v < 0)) for mode, v in series.items()},
            })
    fig.text(.535, .012, "Mean and Corrected overlap at this scale; no curves or means are displaced.",
             fontsize=7.3, ha="center", color="#4d4d4d")
    fig.savefig(destination, dpi=300)
    plt.close(fig)


def _contrast_figure(contrast, destination, metadata):
    fig, axes = plt.subplots(2, 2, figsize=(7, 5))
    fig.subplots_adjust(left=.16, right=.975, bottom=.17, top=.93, hspace=.52, wspace=.50)
    styles = {"exact": ("#26689a", "o", .18), "jla": ("#297b57", "s", 0.)}
    metadata["contrast"] = {"display_multiplier": 1e6, "interval": "mean ± 1.96 paired simulation SE", "panels": []}
    for ax, target in zip(axes.flat, TARGETS):
        panel = contrast[contrast.target == target]
        bounds = [0.]
        for i, case in enumerate(CASES):
            rows = panel[panel.case == case]
            for algorithm, (color, marker, offset) in styles.items():
                r = rows[rows.algorithm == algorithm]
                if len(r) != 1 or r.n.iloc[0] != 2000:
                    raise ValueError("Missing paired centering contrast")
                r = r.iloc[0]
                value = float(r.mean_minus_corrected * 1e6)
                interval = float(1.96 * r.simulation_se * 1e6)
                ax.errorbar(value, 2 - i + offset, xerr=interval, fmt=marker,
                            color=color, ms=3.6, capsize=2.5, lw=.9, zorder=4)
                bounds.extend([value - interval, value + interval])
            oracle_values = rows.exact_analytic_contrast.to_numpy()
            if len(oracle_values) != 2 or np.ptp(oracle_values) > 1e-14:
                raise ValueError("Oracle contrast differs across algorithms")
            value = float(oracle_values[0] * 1e6)
            ax.plot(value, 2 - i - .18, "D", color="#ca792a", ms=3.7, zorder=5)
            bounds.append(value)
        padding = .10 * (max(bounds) - min(bounds))
        ax.set_xlim(min(bounds) - padding, max(bounds) + padding)
        ax.set_ylim(-.5, 2.5)
        ax.axvline(0, color="black", linewidth=.7)
        ax.set_yticks([2, 1, 0], [CASE_LABELS[c] for c in CASES])
        ax.tick_params(axis="y", length=0, labelsize=7.4)
        ax.set_title(target, loc="left", pad=6)
        ax.set_xlabel(r"Mean − Corrected ($\times 10^{-6}$)", fontsize=8)
        ax.xaxis.set_major_locator(MaxNLocator(5))
        ax.grid(axis="x")
        metadata["contrast"]["panels"].append({"target": target, "xlim": list(ax.get_xlim())})
    handles = [Line2D([], [], color=color, marker=marker, linestyle="-", ms=4, lw=.8,
                      label="Exact" if algorithm == "exact" else "JLA(256)")
               for algorithm, (color, marker, _) in styles.items()]
    handles.append(Line2D([], [], color="#ca792a", marker="D", linestyle="None", ms=4, label="Exact oracle"))
    fig.legend(handles=handles, loc="lower center", bbox_to_anchor=(.55, .018), ncol=3,
               frameon=False, fontsize=8, columnspacing=1.8)
    fig.savefig(destination, dpi=300)
    plt.close(fig)


def _jla_figure(calibration, destination, metadata):
    fig, axes = plt.subplots(2, 2, figsize=(7, 4.6))
    fig.subplots_adjust(left=.11, right=.985, bottom=.15, top=.93, hspace=.45, wspace=.27)
    metadata["jla"] = {"case": "interaction", "probes": 256, "fixed_outcomes": 3,
                       "sketches_per_outcome": 300, "panels": []}
    for ax, target in zip(axes.flat, TARGETS):
        panel = calibration[(calibration.case == "interaction") & (calibration.probes == 256)
                            & (calibration.target == target)]
        if len(panel) != 3 or set(panel.centering) != set(MODES):
            raise ValueError("Incomplete pooled JLA calibration")
        panel = panel.set_index("centering").loc[list(MODES)]
        x = np.arange(3)
        ax.bar(x - .17, panel.numerical_sd, .31, color="#92999c", label="Empirical numerical SD", zorder=3)
        ax.bar(x + .17, panel.rms_mcse, .31, color="#26689a", label="RMS reported MCSE", zorder=3)
        ax.set_xticks(x, [LABELS[m] for m in MODES])
        ax.set_ylim(0, max(panel.numerical_sd.max(), panel.rms_mcse.max()) * 1.17)
        ax.set_ylabel("Numerical uncertainty")
        ax.set_title(target, loc="left", pad=6)
        ax.yaxis.set_major_locator(MaxNLocator(4))
        ax.grid(axis="y")
        metadata["jla"]["panels"].append({"target": target, "ylim": list(ax.get_ylim()),
            "numerical_sd": panel.numerical_sd.to_dict(), "rms_mcse": panel.rms_mcse.to_dict()})
    handles, labels = axes[0, 0].get_legend_handles_labels()
    fig.legend(handles, labels, loc="lower center", bbox_to_anchor=(.53, .015),
               ncol=2, frameon=False, fontsize=8.5, columnspacing=2)
    fig.savefig(destination, dpi=300)
    plt.close(fig)


def make_figures(root: Path, out_dir: Path) -> dict[str, Path]:
    """Render five PNGs from accepted inputs and record the plotting choices."""
    root, out_dir = Path(root).resolve(), Path(out_dir).resolve()
    if out_dir == root or root in out_dir.parents:
        raise ValueError("Write presentation overlays outside the accepted evidence directory")
    raw, summary, contrast, calibration, hashes = _read_inputs(root)
    out_dir.mkdir(parents=True, exist_ok=True)
    metadata = {"source_root": str(root), "input_sha256": hashes,
                "dpi": 300, "sampling_draws": 2000, "all_draws_retained": True,
                "source": "report_figures.py", "source_sha256": _sha256(Path(__file__))}
    figures = {case: out_dir / f"sampling-{case}.png" for case in CASES}
    figures.update(contrast=out_dir / "mean-corrected-contrast.png", jla=out_dir / "jla-mcse.png")
    with plt.rc_context(STYLE):
        for case in CASES:
            _distribution_figure(case, raw, summary, figures[case], metadata)
        _contrast_figure(contrast, figures["contrast"], metadata)
        _jla_figure(calibration, figures["jla"], metadata)
    metadata["output_sha256"] = {path.name: _sha256(path) for path in figures.values()}
    (out_dir / "figure-metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    return figures


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("out_dir", type=Path)
    args = parser.parse_args()
    paths = make_figures(args.root, args.out_dir)
    print(json.dumps({key: str(path) for key, path in paths.items()}, indent=2))
