#!/usr/bin/env python3
"""Build figures, tables, guidance, and Markdown from compact SCC evidence."""

from __future__ import annotations

import argparse
import csv
import json
import math
import shutil
from pathlib import Path


ROLE_ORDER = ("mata", "rust", "matlab")
ROLE_LABEL = {"mata": "VCkss--Mata", "rust": "VCkss--Rust", "matlab": "MATLAB"}
COLORS = {"mata": "#0072B2", "rust": "#D55E00", "matlab": "#009E73"}
MARKERS = {"mata": "o", "rust": "s", "matlab": "^"}
GRAPH_ORDER = ("strong_d2", "strong_d3", "strong_d6", "weak_d3")
GRAPH_LABEL = {
    "strong_d2": "Strong, degree 2", "strong_d3": "Strong, degree 3",
    "strong_d6": "Strong, degree 6", "weak_d3": "Weak/bottleneck, degree 3",
}
GRAPH_TABLE_LABEL = {
    "strong_d2": "Strong d2", "strong_d3": "Strong d3",
    "strong_d6": "Strong d6", "weak_d3": "Weak d3",
}
CORE_ORDER = (1, 2, 4, 8, 16)
ROW_ORDER = (7680, 30720, 122880, 491520, 1966080)


def maximum_effective_cores(role: str) -> int:
    return 4 if role == "mata" else 16


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def prepare_style(plt) -> None:
    plt.rcParams.update({
        "font.family": "DejaVu Sans",
        "font.size": 9,
        "axes.titlesize": 10,
        "axes.labelsize": 9,
        "legend.fontsize": 8,
        "figure.dpi": 140,
        "savefig.bbox": "tight",
        "axes.spines.top": False,
        "axes.spines.right": False,
        "axes.grid": True,
        "grid.alpha": 0.20,
    })


def complete_cells(cells):
    return cells[cells["cell_status"] == "COMPLETE"].copy()


def latex_escape(value: object) -> str:
    if value is None or (isinstance(value, float) and math.isnan(value)):
        return "--"
    text = str(value)
    replacements = {
        "\\": r"\textbackslash{}", "&": r"\&", "%": r"\%", "$": r"\$",
        "#": r"\#", "_": r"\_", "{": r"\{", "}": r"\}",
        "~": r"\textasciitilde{}", "^": r"\textasciicircum{}",
    }
    return "".join(replacements.get(character, character) for character in text)


def write_latex_table(path: Path, headers: list[str], rows: list[list[object]],
                      caption: str, label: str, *, longtable: bool = False) -> None:
    columns = "l" * len(headers)
    header = " & ".join(latex_escape(item) for item in headers) + r" \\"
    body = [" & ".join(latex_escape(item) for item in row) + r" \\" for row in rows]
    if longtable:
        lines = [
            f"\\begin{{longtable}}{{{columns}}}",
            f"\\caption{{{latex_escape(caption)}}}\\label{{{label}}}\\\\",
            "\\toprule", header, "\\midrule", "\\endfirsthead",
            "\\toprule", header, "\\midrule", "\\endhead",
            *body, "\\bottomrule", "\\end{longtable}",
        ]
    else:
        lines = [
            "\\begin{table}[htbp]", "\\centering",
            f"\\caption{{{latex_escape(caption)}}}", f"\\label{{{label}}}",
            f"\\begin{{tabular}}{{{columns}}}", "\\toprule", header,
            "\\midrule", *body, "\\bottomrule", "\\end{tabular}",
            "\\end{table}",
        ]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def role_lines(ax, frame, x, y) -> None:
    for role in ROLE_ORDER:
        selected = frame[frame["role"] == role].sort_values(x)
        if selected.empty:
            continue
        ax.plot(selected[x], selected[y], color=COLORS[role], marker=MARKERS[role],
                linewidth=1.8, markersize=4.5, label=ROLE_LABEL[role])


def plot_time_rows(cells, output, plt) -> None:
    frame = complete_cells(cells)
    frame = frame[frame["active_cores"] == 4]
    fig, axes = plt.subplots(2, 2, figsize=(8.2, 6.0), sharex=True, sharey=True)
    for ax, graph in zip(axes.flat, GRAPH_ORDER):
        role_lines(ax, frame[frame["structure"] == graph], "rows",
                   "command_seconds_median")
        ax.set_title(GRAPH_LABEL[graph])
        ax.set_xscale("log", base=2)
        ax.set_yscale("log")
        ax.set_xticks(ROW_ORDER, ["7.7k", "30.7k", "123k", "492k", "1.97m"])
        ax.set_xlabel("Stored rows")
        ax.set_ylabel("Command time (seconds)")
    handles, labels = axes.flat[0].get_legend_handles_labels()
    fig.legend(handles, labels, loc="upper center", bbox_to_anchor=(0.5, 0.94),
               ncol=3, frameon=False)
    fig.suptitle("End-to-end command time at four active cores", y=0.995,
                 fontsize=12, fontweight="bold")
    fig.tight_layout(rect=(0, 0, 1, 0.89))
    fig.savefig(output / "time_by_rows_4cores.pdf")
    plt.close(fig)


def plot_scaling(cells, output, plt) -> None:
    frame = complete_cells(cells)
    frame = frame[frame["rows"] == max(ROW_ORDER)].copy()
    fig, axes = plt.subplots(2, 4, figsize=(10.8, 5.5), sharex=True)
    for column, graph in enumerate(GRAPH_ORDER):
        graph_frame = frame[frame["structure"] == graph]
        for role in ROLE_ORDER:
            selected = graph_frame[graph_frame["role"] == role].sort_values("active_cores")
            maximum = maximum_effective_cores(role)
            selected = selected[selected["active_cores"] <= maximum]
            if len(selected) != (3 if role == "mata" else 5):
                continue
            base = float(selected.iloc[0]["command_seconds_median"])
            speedup = base / selected["command_seconds_median"].astype(float)
            effective = selected["effective_role_cores"].astype(float)
            efficiency = speedup / effective
            axes[0, column].plot(effective, speedup, color=COLORS[role],
                                 marker=MARKERS[role], label=ROLE_LABEL[role])
            axes[1, column].plot(effective, efficiency, color=COLORS[role],
                                 marker=MARKERS[role])
        axes[0, column].plot(CORE_ORDER, CORE_ORDER, color="#777777",
                             linestyle="--", linewidth=1, label="Ideal")
        axes[0, column].set_title(GRAPH_LABEL[graph])
        axes[0, column].set_ylabel("Speedup vs. 1 core")
        axes[1, column].set_ylabel("Parallel efficiency")
        axes[1, column].set_xlabel("Effective role cores")
        axes[1, column].set_ylim(0, 1.08)
        for row in range(2):
            axes[row, column].set_xticks(CORE_ORDER)
    handles, labels = axes[0, 0].get_legend_handles_labels()
    fig.legend(handles, labels, loc="upper center", bbox_to_anchor=(0.5, 0.94),
               ncol=4, frameon=False)
    fig.suptitle("Parallel scaling at 1,966,080 rows (Mata capped at 4 cores)", y=0.995,
                 fontsize=12, fontweight="bold")
    fig.tight_layout(rect=(0, 0, 1, 0.89))
    fig.savefig(output / "speedup_efficiency.pdf")
    plt.close(fig)


def heatmap_panels(frame, field, output_path, title, color_label, plt,
                   *, vmin=None, vmax=None, center_one=False) -> None:
    import numpy as np
    fig, axes = plt.subplots(2, 2, figsize=(8.4, 6.2), sharex=True, sharey=True)
    values = frame[field].dropna().astype(float)
    if vmin is None:
        vmin = float(values.quantile(0.05)) if len(values) else 0.0
    if vmax is None:
        vmax = float(values.quantile(0.95)) if len(values) else 1.0
    if center_one:
        bound = max(abs(vmin - 1), abs(vmax - 1), 0.05)
        vmin, vmax = 1 - bound, 1 + bound
    image = None
    for ax, graph in zip(axes.flat, GRAPH_ORDER):
        selected = frame[frame["structure"] == graph]
        matrix = np.full((len(ROW_ORDER), len(CORE_ORDER)), np.nan)
        for row_index, rows in enumerate(ROW_ORDER):
            for core_index, cores in enumerate(CORE_ORDER):
                item = selected[(selected["rows"] == rows) &
                                (selected["active_cores"] == cores)]
                if len(item) == 1 and item.iloc[0][field] == item.iloc[0][field]:
                    matrix[row_index, core_index] = float(item.iloc[0][field])
        image = ax.imshow(matrix, origin="lower", aspect="auto", cmap="RdYlBu_r",
                          vmin=vmin, vmax=vmax)
        ax.set_title(GRAPH_LABEL[graph])
        ax.set_xticks(range(len(CORE_ORDER)), CORE_ORDER)
        ax.set_yticks(range(len(ROW_ORDER)),
                      ["7.7k", "30.7k", "123k", "492k", "1.97m"])
        ax.set_xlabel("Target core cell")
        ax.set_ylabel("Stored rows")
        for row_index in range(len(ROW_ORDER)):
            for core_index in range(len(CORE_ORDER)):
                value = matrix[row_index, core_index]
                if math.isfinite(value):
                    ax.text(core_index, row_index, f"{value:.2f}", ha="center",
                            va="center", fontsize=6.5,
                            color="white" if value > (vmin + vmax) / 2 else "black")
    assert image is not None
    bar = fig.colorbar(image, ax=axes.ravel().tolist(), shrink=0.82, pad=0.03)
    bar.set_label(color_label)
    fig.suptitle(title, fontsize=12, fontweight="bold")
    fig.savefig(output_path)
    plt.close(fig)


def plot_memory_rows(cells, output, plt, *, field: str, filename: str,
                     title: str, ylabel: str) -> None:
    frame = complete_cells(cells)
    frame = frame[frame["active_cores"] == 4].copy()
    frame["rss_gib"] = frame[field].astype(float) / 1024**3
    fig, axes = plt.subplots(2, 2, figsize=(8.2, 6.0), sharex=True, sharey=True)
    for ax, graph in zip(axes.flat, GRAPH_ORDER):
        role_lines(ax, frame[frame["structure"] == graph], "rows", "rss_gib")
        ax.set_title(GRAPH_LABEL[graph])
        ax.set_xscale("log", base=2)
        ax.set_yscale("log")
        ax.set_xticks(ROW_ORDER, ["7.7k", "30.7k", "123k", "492k", "1.97m"])
        ax.set_xlabel("Stored rows")
        ax.set_ylabel(ylabel)
    handles, labels = axes.flat[0].get_legend_handles_labels()
    fig.legend(handles, labels, loc="upper center", bbox_to_anchor=(0.5, 0.94),
               ncol=3, frameon=False)
    fig.suptitle(title, y=0.995, fontsize=12, fontweight="bold")
    fig.tight_layout(rect=(0, 0, 1, 0.89))
    fig.savefig(output / filename)
    plt.close(fig)


def plot_pareto(cells, output, plt) -> None:
    frame = complete_cells(cells)
    frame = frame[frame["rows"] == max(ROW_ORDER)].copy()
    frame["phase_rss_gib"] = frame["phase_rss_bytes_median"].astype(float) / 1024**3
    fig, axes = plt.subplots(2, 2, figsize=(8.3, 6.2), sharex=True, sharey=True)
    for ax, graph in zip(axes.flat, GRAPH_ORDER):
        selected = frame[frame["structure"] == graph]
        for role in ROLE_ORDER:
            values = selected[selected["role"] == role]
            ax.scatter(values["phase_rss_gib"], values["command_seconds_median"],
                       s=18 + 4 * values["active_cores"].astype(float),
                       color=COLORS[role], marker=MARKERS[role], alpha=0.85,
                       label=ROLE_LABEL[role])
            for _, row in values.iterrows():
                ax.annotate(
                            f"{int(row['active_cores'])}/{int(row['effective_role_cores'])}",
                            (row["phase_rss_gib"], row["command_seconds_median"]),
                            xytext=(3, 2), textcoords="offset points", fontsize=6)
        ax.set_title(GRAPH_LABEL[graph])
        ax.set_xscale("log")
        ax.set_yscale("log")
        ax.set_xlabel("Estimator-phase peak RSS (GiB)")
        ax.set_ylabel("Command time (seconds)")
    handles, labels = axes.flat[0].get_legend_handles_labels()
    fig.legend(handles, labels, loc="upper center", bbox_to_anchor=(0.5, 0.94),
               ncol=3, frameon=False)
    fig.suptitle("Time--memory frontier (labels: target/effective cores)", y=0.995,
                 fontsize=12, fontweight="bold")
    fig.tight_layout(rect=(0, 0, 1, 0.89))
    fig.savefig(output / "time_memory_pareto.pdf")
    plt.close(fig)


def plot_failures(cells, output, plt) -> None:
    import numpy as np
    fig, axes = plt.subplots(2, 2, figsize=(8.4, 6.2), sharex=True, sharey=True)
    image = None
    for ax, graph in zip(axes.flat, GRAPH_ORDER):
        selected = cells[cells["structure"] == graph]
        matrix = np.zeros((len(ROW_ORDER), len(CORE_ORDER)))
        for row_index, rows in enumerate(ROW_ORDER):
            for core_index, cores in enumerate(CORE_ORDER):
                item = selected[(selected["rows"] == rows) &
                                (selected["active_cores"] == cores)]
                matrix[row_index, core_index] = item["successful_repetitions"].min()
        image = ax.imshow(matrix, origin="lower", aspect="auto", cmap="viridis",
                          vmin=0, vmax=3)
        ax.set_title(GRAPH_LABEL[graph])
        ax.set_xticks(range(len(CORE_ORDER)), CORE_ORDER)
        ax.set_yticks(range(len(ROW_ORDER)),
                      ["7.7k", "30.7k", "123k", "492k", "1.97m"])
        ax.set_xlabel("Target core cell")
        ax.set_ylabel("Stored rows")
        for row_index in range(len(ROW_ORDER)):
            for core_index in range(len(CORE_ORDER)):
                value = matrix[row_index, core_index]
                ax.text(core_index, row_index, str(int(value)), ha="center",
                        va="center", fontsize=8,
                        color="black" if value >= 2 else "white")
    assert image is not None
    bar = fig.colorbar(image, ax=axes.ravel().tolist(), shrink=0.82, pad=0.03,
                       ticks=(0, 1, 2, 3))
    bar.set_label("Minimum successful repetitions across routes")
    fig.suptitle("Scientific completion and censoring map", fontsize=12,
                 fontweight="bold")
    fig.savefig(output / "failure_map.pdf")
    plt.close(fig)


def write_tables(cells, results, output, pd) -> dict[str, object]:
    complete = complete_cells(cells)
    representative = cells[(cells["rows"] == max(ROW_ORDER)) &
                           (cells["active_cores"].isin((1, 4, 16)))].copy()
    representative["Route"] = representative["role"].map(ROLE_LABEL)
    representative["Graph"] = representative["structure"].map(GRAPH_TABLE_LABEL)
    representative["Target/effective cores"] = (
        representative["active_cores"].astype(int).astype(str) + "/" +
        representative["effective_role_cores"].astype(int).astype(str)
    )
    representative["Time (s)"] = pd.to_numeric(
        representative["command_seconds_median"], errors="coerce").round(2)
    representative["Phase GiB"] = (pd.to_numeric(
        representative["phase_rss_bytes_median"], errors="coerce") / 1024**3).round(2)
    representative["Process GiB"] = (pd.to_numeric(
        representative["process_rss_bytes_median"], errors="coerce") / 1024**3).round(2)
    representative["Status"] = representative["cell_status"]
    representative_columns = ["Graph", "Route", "Target/effective cores", "Time (s)",
                              "Phase GiB", "Process GiB", "Status"]
    write_latex_table(
        output / "representative.tex", representative_columns,
        representative[representative_columns].values.tolist(),
        "Representative results at 1,966,080 stored rows.",
        "tab:representative", longtable=True)

    scaling_rows = []
    for graph in GRAPH_ORDER:
        for role in ROLE_ORDER:
            selected = complete[(complete["structure"] == graph) &
                                (complete["rows"] == max(ROW_ORDER)) &
                                (complete["role"] == role)]
            maximum = maximum_effective_cores(role)
            selected = selected[selected["active_cores"] <= maximum]
            by_core = {int(row.active_cores): float(row.command_seconds_median)
                       for row in selected.itertuples()}
            scaling_rows.append({
                "Graph": GRAPH_TABLE_LABEL[graph], "Route": ROLE_LABEL[role],
                "Maximum effective cores": maximum,
                "1 to max speedup": (
                    by_core[1] / by_core[maximum] if maximum in by_core else math.nan
                ),
                "8 to 16 gain (pct)": (
                    100 * (by_core[8] - by_core[16]) / by_core[8]
                    if role != "mata" and 8 in by_core and 16 in by_core else math.nan
                ),
            })
    scaling = pd.DataFrame(scaling_rows).round(2)
    write_latex_table(output / "scaling.tex", list(scaling.columns),
                      scaling.values.tolist(),
                      "Role-effective scaling at the largest size; Mata is capped at four cores.",
                      "tab:scaling")

    failures = results[results["scientific_status"] != "PASS"].groupby(
        ["structure", "rows", "active_cores", "role", "scientific_status"],
        dropna=False).size().reset_index(name="Count")
    if failures.empty:
        (output / "failures.tex").write_text(
            "\\begin{center}No scientific failures or timeouts were recorded.\\end{center}\n",
            encoding="utf-8")
    else:
        write_latex_table(output / "failures.tex", list(failures.columns),
                          failures.values.tolist(),
                          "Failed, rejected, or censored calls.",
                          "tab:failures", longtable=True)

    numerical = results.groupby(["role", "scientific_status"], dropna=False).size(
        ).reset_index(name="Calls")
    numerical["Route"] = numerical["role"].map(ROLE_LABEL)
    numerical_columns = ["Route", "scientific_status", "Calls"]
    write_latex_table(output / "numerical_status.tex", numerical_columns,
                      numerical[numerical_columns].values.tolist(),
                      "Application and scientific-status counts.",
                      "tab:numerical-status")

    timing_fields = [
        "command_seconds", "process_wall_seconds", "import_seconds",
        "pool_startup_seconds", "pool_teardown_seconds", "selection_seconds",
        "graph_seconds", "compression_seconds", "setup_seconds", "work_seconds",
        "fit_seconds", "leverage_seconds", "target_seconds", "correction_seconds",
        "rng_seconds", "schur_seconds", "pcg_seconds", "cmg_graph_seconds",
        "cmg_hierarchy_seconds", "cmg_rhs_seconds", "cmg_solve_seconds",
        "cmg_extraction_seconds",
    ]
    timing = results[(results["rows"] == max(ROW_ORDER)) &
                     (results["active_cores"] == 4) &
                     (results["scientific_status"] == "PASS")].copy()
    for field in timing_fields:
        timing[field] = pd.to_numeric(timing[field], errors="coerce")
    timing = timing.groupby(["structure", "role"], as_index=False)[timing_fields].median()
    timing.to_csv(output / "timing_components.tsv", sep="\t", index=False)
    timing_display = timing.copy()
    timing_display["Graph"] = timing_display["structure"].map(GRAPH_TABLE_LABEL)
    timing_display["Route"] = timing_display["role"].map(ROLE_LABEL)
    timing_display["Command"] = timing_display["command_seconds"].round(2)
    timing_display["Process"] = timing_display["process_wall_seconds"].round(2)
    timing_display["Import"] = timing_display["import_seconds"].round(2)
    timing_display["Pool start"] = timing_display["pool_startup_seconds"].round(2)
    timing_display["Pool stop"] = timing_display["pool_teardown_seconds"].round(2)
    timing_columns = ["Graph", "Route", "Command", "Process", "Import",
                      "Pool start", "Pool stop"]
    write_latex_table(output / "timing_components.tex", timing_columns,
                      timing_display[timing_columns].values.tolist(),
                      "Representative timing components at 1,966,080 rows and four cores (seconds).",
                      "tab:timing-components", longtable=True)

    rust_timing = timing[timing["role"] == "rust"].copy()
    rust_timing["Graph"] = rust_timing["structure"].map(GRAPH_TABLE_LABEL)
    rust_columns = [
        ("Graph", "Graph"), ("selection_seconds", "Selection"),
        ("graph_seconds", "Graph build"), ("fit_seconds", "Fit"),
        ("leverage_seconds", "Leverage"), ("target_seconds", "Target"),
        ("cmg_solve_seconds", "CMG solve"),
    ]
    for source, _ in rust_columns[1:]:
        rust_timing[source] = rust_timing[source].round(2)
    rust_timing.to_csv(output / "rust_phase_timing.tsv", sep="\t", index=False)
    write_latex_table(output / "rust_phase_timing.tex",
                      [label for _, label in rust_columns],
                      rust_timing[[source for source, _ in rust_columns]].values.tolist(),
                      "VCkss--Rust phase medians at 1,966,080 rows and four cores (seconds).",
                      "tab:rust-phase-timing")

    fastest = complete[complete["role"] == "rust"][
        ["structure", "rows", "active_cores", "fastest_role"]].copy()
    fastest_counts = fastest["fastest_role"].value_counts().to_dict()
    fastest["Fastest route"] = fastest["fastest_role"].map(ROLE_LABEL)
    fastest["Graph"] = fastest["structure"].map(GRAPH_LABEL)
    fastest["fastest_effective_cores"] = fastest.apply(
        lambda row: min(int(row["active_cores"]), 4)
        if row["fastest_role"] == "mata" else int(row["active_cores"]), axis=1)
    fastest["comparison_contract"] = fastest["active_cores"].apply(
        lambda target: "CAPPED_MATA" if int(target) > 4 else "EQUAL_CORE")
    fastest[["Graph", "rows", "active_cores", "fastest_effective_cores",
             "comparison_contract", "Fastest route"]].to_csv(
        output / "fastest_route_grid.tsv", sep="\t", index=False)

    core_guidance = []
    for (graph, rows, role), selected in complete.groupby(["structure", "rows", "role"]):
        maximum = maximum_effective_cores(role)
        selected = selected[selected["active_cores"] <= maximum]
        by_core = {int(row.active_cores): float(row.command_seconds_median)
                   for row in selected.itertuples()}
        if maximum not in by_core:
            continue
        eligible = [core for core in CORE_ORDER
                    if core <= maximum and core in by_core and
                    by_core[core] <= 1.10 * by_core[maximum]]
        core_guidance.append({
            "structure": graph, "rows": int(rows), "role": role,
            "maximum_effective_cores": maximum,
            "smallest_effective_core_within_10pct_of_max": (
                min(eligible) if eligible else ""
            ),
        })
    pd.DataFrame(core_guidance).to_csv(output / "core_guidance.tsv", sep="\t", index=False)

    budget_rows = []
    for graph in GRAPH_ORDER:
        for role in ROLE_ORDER:
            for cores in CORE_ORDER:
                selected = complete[(complete["structure"] == graph) &
                                    (complete["role"] == role) &
                                    (complete["active_cores"] == cores)]
                for budget in (8, 16, 32, 64):
                    fit = selected[1.25 * selected["process_rss_bytes_median"].astype(float)
                                   <= budget * 1024**3]
                    budget_rows.append({
                        "structure": graph, "role": role, "active_cores": cores,
                        "effective_role_cores": min(cores, 4) if role == "mata" else cores,
                        "budget_gib": budget,
                        "largest_measured_rows": int(fit["rows"].max()) if len(fit) else "",
                    })
    pd.DataFrame(budget_rows).to_csv(output / "memory_budget_guidance.tsv",
                                     sep="\t", index=False)
    return {"fastest_counts": fastest_counts,
            "complete_cells": int(len(complete[complete["role"] == "rust"]))}


def write_markdown(collection, cells, summary, output) -> None:
    complete = complete_cells(cells)
    rust = complete[complete["role"] == "rust"]
    time_matlab = rust["rust_to_matlab_time_ratio"].astype(float).median() if len(rust) else math.nan
    time_mata = rust["rust_to_mata_time_ratio"].astype(float).median() if len(rust) else math.nan
    counts = summary["fastest_counts"]
    runtime = collection["runtime_identity"]
    lines = [
        "# VCkss three-way scaling benchmark",
        "",
        f"Source commit: `{collection['source_commit']}`  ",
        f"Source bundle SHA-256: `{collection['bundle_sha256']}`  ",
        f"Compact result ledger: `{collection['artifact_sha256']['results_900.tsv']}`",
        f"Rust compiler: `{runtime['rustc']}`  ",
        f"Stata: `{runtime['stata_module']}`; MATLAB: `{runtime['matlab_module']}`  ",
        f"Plugin SHA-256: `{runtime['plugin_sha256']}`  ",
        f"Maintained MATLAB source: `{runtime['matlab_upstream_commit']}`",
        "",
        "## Which route should I use?",
        "",
        f"The registered grid contains {summary['complete_cells']} complete rankable cells. "
        f"VCkss--Mata is fastest in {counts.get('mata', 0)}, VCkss--Rust in "
        f"{counts.get('rust', 0)}, and MATLAB in {counts.get('matlab', 0)}.",
        "",
        f"Across complete cells, the median Rust/MATLAB command-time ratio is "
        f"{time_matlab:.3f}; the median Rust/Mata ratio is {time_mata:.3f}. "
        "Use the cell-specific tables and figures rather than these pooled diagnostics "
        "for an applied choice.",
        "For target cells 8 and 16, Rust and MATLAB use the full target while Stata/Mata "
        "remains capped at four processors. Those Rust/Mata ratios are explicitly "
        "capped-Mata comparisons, not equal-core scaling comparisons.",
        "",
        "The qualified Rust route is preferred only for the measured match-JLA tuple when "
        "its plugin and memory requirements are acceptable. Mata remains the source-only "
        "portable choice. MATLAB comparisons are descriptive because RNG and numerical "
        "policies differ.",
        "",
        "## Exact invocation contract",
        "",
        "```stata",
        "vckss y, worker(worker) firm(firm) deletion(match) probeorder(observation_key) ///",
        "    backend(rust) rng(counter_v1) algorithm(jla) engine(auto) ///",
        "    preconditioner(auto) batch(auto) memory_gib(MEMORY) wallseconds(10800) ///",
        "    probes(200) seed(SEED) maxiter(20000) nodisplay",
        "",
        "vckss y, worker(worker) firm(firm) deletion(match) probeorder(observation_key) ///",
        "    backend(mata) rng(stata) algorithm(jla) engine(auto) ///",
        "    preconditioner(auto) batch(auto) memory_gib(MEMORY) wallseconds(10800) ///",
        "    probes(200) seed(SEED) maxiter(20000) nodisplay",
        "```",
        "",
        "```matlab",
        "[firm_variance,covariance,worker_variance] = leave_out_KSS( ...",
        "    outcome,worker,firm,[], 'matches','JLA',200,0,[],[],detail_stub);",
        "```",
        "",
        "`SEED`, `MEMORY`, the CPU list, and execution order are literal values from "
        "`task_manifest_300.tsv`; each application is launched in a fresh `taskset`-restricted process.",
        "",
        "## Evidence and limitations",
        "",
        "- Four graph families, five target-core counts, and three "
        "position-balanced seed repetitions were registered.",
        "- Rust and MATLAB use 1/2/4/8/16 effective cores; Stata and Mata use "
        "1/2/4/4/4. The SCC allocation remains 16 bound slots for every task.",
        "- Cell summaries use medians and full three-run ranges, not confidence intervals.",
        "- Route time and memory ratios are paired within each same-host task before "
        "being summarized across repetitions.",
        "- The prototype accepts any eligible SCC host. Absolute timing and cross-core "
        "speedups spanning CPU models are descriptive pending a homogeneous confirmation.",
        "- Fixed row counts imply different worker and firm counts across graph degrees.",
        "- Findings apply only within the measured grid and do not establish a universal "
        "language ranking.",
        "",
        "See the PDF for the full protocol, figures, tables, failure evidence, and exact "
        "reproduction commands.",
    ]
    (output / "VCkss_Three_Way_Scaling_Benchmark.md").write_text(
        "\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--collection-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--template", type=Path, required=True)
    args = parser.parse_args()
    import matplotlib.pyplot as plt
    import pandas as pd
    require(args.collection_dir.is_dir(), "collection directory is missing")
    require(not args.output_dir.exists(), "report output directory already exists")
    args.output_dir.mkdir(parents=True)
    figures = args.output_dir / "figures"
    tables = args.output_dir / "tables"
    generated = args.output_dir / "generated"
    figures.mkdir(); tables.mkdir(); generated.mkdir()
    collection = json.loads((args.collection_dir / "collection.json").read_text())
    require(collection.get("status") == "PASS" and
            collection.get("validated_tasks") == 300 and
            collection.get("estimator_calls") == 900, "collection receipt changed")
    runtime = collection.get("runtime_identity")
    require(isinstance(runtime, dict) and all(runtime.get(name) for name in (
        "rustc", "cargo", "stata_module", "matlab_module", "plugin_sha256",
        "matlab_upstream_commit", "matlab_runtime_tree_sha256",
    )), "runtime identity changed")
    results = pd.read_csv(args.collection_dir / "results_900.tsv", sep="\t")
    cells = pd.read_csv(args.collection_dir / "cell_summary_300.tsv", sep="\t")
    require(len(results) == 900 and len(cells) == 300, "compact evidence cardinality changed")
    require(all(field in cells.columns for field in (
        "active_cores", "stata_processors", "effective_role_cores")),
        "role-specific core evidence is missing")
    expected_effective = cells.apply(
        lambda row: min(int(row["active_cores"]), 4)
        if row["role"] == "mata" else int(row["active_cores"]), axis=1)
    require((cells["effective_role_cores"].astype(int) == expected_effective).all() and
            (cells["stata_processors"].astype(int) ==
             cells["active_cores"].astype(int).clip(upper=4)).all(),
            "role-specific core contract changed")
    prepare_style(plt)
    plot_time_rows(cells, figures, plt)
    plot_scaling(cells, figures, plt)
    ratio_frame = cells[cells["role"] == "rust"]
    heatmap_panels(ratio_frame, "rust_to_matlab_time_ratio",
                   figures / "rust_matlab_time_ratio.pdf",
                   "VCkss--Rust / MATLAB command-time ratio",
                   "Ratio below one favors Rust", plt, center_one=True)
    heatmap_panels(ratio_frame, "rust_to_mata_time_ratio",
                   figures / "rust_mata_time_ratio.pdf",
                   "VCkss--Rust / VCkss--Mata command-time ratio",
                   "Below one favors Rust; targets 8/16 use capped 4-core Mata",
                   plt, center_one=True)
    plot_memory_rows(cells, figures, plt,
                     field="phase_rss_bytes_median",
                     filename="memory_by_rows_4cores.pdf",
                     title="Estimator-phase peak memory at four active cores",
                     ylabel="Estimator-phase peak RSS (GiB)")
    plot_memory_rows(cells, figures, plt,
                     field="process_rss_bytes_median",
                     filename="process_memory_by_rows_4cores.pdf",
                     title="Full-process peak memory at four active cores",
                     ylabel="Full-process peak RSS (GiB)")
    heatmap_panels(ratio_frame, "rust_to_matlab_memory_ratio",
                   figures / "rust_matlab_memory_ratio.pdf",
                   "VCkss--Rust / MATLAB estimator-phase RSS ratio",
                   "Ratio below one favors Rust", plt, center_one=True)
    heatmap_panels(ratio_frame, "rust_to_mata_memory_ratio",
                   figures / "rust_mata_memory_ratio.pdf",
                   "VCkss--Rust / VCkss--Mata estimator-phase RSS ratio",
                   "Targets 8/16 use capped 4-core Mata", plt, center_one=True)
    plot_pareto(cells, figures, plt)
    plot_failures(cells, figures, plt)
    summary = write_tables(cells, results, tables, pd)
    write_markdown(collection, cells, summary, args.output_dir)
    source_tex = (
        f"\\newcommand{{\\SourceCommit}}{{\\texttt{{{collection['source_commit']}}}}}\n"
        f"\\newcommand{{\\BundleSha}}{{\\texttt{{{collection['bundle_sha256']}}}}}\n"
        f"\\newcommand{{\\CompleteCells}}{{{summary['complete_cells']}}}\n"
        f"\\newcommand{{\\RustFastest}}{{{summary['fastest_counts'].get('rust', 0)}}}\n"
        f"\\newcommand{{\\MataFastest}}{{{summary['fastest_counts'].get('mata', 0)}}}\n"
        f"\\newcommand{{\\MatlabFastest}}{{{summary['fastest_counts'].get('matlab', 0)}}}\n"
        f"\\newcommand{{\\RustcVersion}}{{{latex_escape(runtime['rustc'])}}}\n"
        f"\\newcommand{{\\CargoVersion}}{{{latex_escape(runtime['cargo'])}}}\n"
        f"\\newcommand{{\\StataModule}}{{{latex_escape(runtime['stata_module'])}}}\n"
        f"\\newcommand{{\\MatlabModule}}{{{latex_escape(runtime['matlab_module'])}}}\n"
        f"\\newcommand{{\\PluginSha}}{{{runtime['plugin_sha256']}}}\n"
        f"\\newcommand{{\\MatlabUpstream}}{{{runtime['matlab_upstream_commit']}}}\n"
    )
    (generated / "source.tex").write_text(source_tex, encoding="utf-8")
    shutil.copy2(args.template, args.output_dir / "report.tex")
    print("VCKSS_COMPARATIVE_SCALING_REPORT_ANALYSIS_PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
