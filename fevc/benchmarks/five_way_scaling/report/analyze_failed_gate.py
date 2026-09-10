#!/usr/bin/env python3
"""Summarize a five-way run that stopped at the exact-consistency gate."""

from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path
from typing import Any

from math import log10

from PIL import Image, ImageDraw, ImageFont


HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from aggregate import elapsed_seconds  # noqa: E402
from common import ROLES, TARGETS, atomic_json, exact_tolerance  # noqa: E402
from validate_task import load_result  # noqa: E402


LABELS = {
    "fevc": "FEVC",
    "matlab": "KSS MATLAB",
    "julia": "Julia",
    "r": "R",
    "pytwoway": "PyTwoWay",
}
COLORS = {
    "fevc": "#0072B2",
    "matlab": "#D55E00",
    "julia": "#009E73",
    "r": "#CC79A7",
    "pytwoway": "#E69F00",
}
FONT_REGULAR = "/System/Library/Fonts/Supplemental/Arial.ttf"
FONT_BOLD = "/System/Library/Fonts/Supplemental/Arial Bold.ttf"


def font(size: int, *, bold: bool = False) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(FONT_BOLD if bold else FONT_REGULAR, size)


def centered(draw: ImageDraw.ImageDraw, xy: tuple[float, float], value: str,
             face: ImageFont.FreeTypeFont, fill: str = "#17202A") -> None:
    draw.text(xy, value, font=face, fill=fill, anchor="mm")


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    if not rows:
        raise ValueError(f"refusing to write empty table: {path}")
    temporary = path.with_suffix(path.suffix + ".tmp")
    with temporary.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    temporary.replace(path)


def smoke_rows(run: Path) -> list[dict[str, Any]]:
    task_dir = run / "output" / "smoke" / "task-001"
    validation = json.loads((task_dir / "validation.json").read_text(encoding="utf-8"))
    rows: list[dict[str, Any]] = []
    for role in ROLES:
        item = validation["roles"][role]
        result = item["result"]
        monitor = item["monitor"]
        rows.append(
            {
                "role": role,
                "label": LABELS[role],
                "rows": int(validation["rows"]),
                "cores": int(validation["cores"]),
                "primary_seconds": float(result["primary_seconds"]),
                "estimator_seconds": float(result["estimator_seconds"]),
                "cold_seconds": elapsed_seconds(task_dir / role / "resources.txt"),
                "phase_peak_rss_mib": float(monitor["phase_peak_rss_kib"]) / 1024,
                "whole_peak_rss_mib": float(monitor["whole_peak_rss_kib"]) / 1024,
            }
        )
    return rows


def exact_rows(run: Path) -> list[dict[str, Any]]:
    task_dir = run / "output" / "exact" / "task-001"
    oracle = json.loads((task_dir / "oracle.json").read_text(encoding="utf-8"))
    rows: list[dict[str, Any]] = []
    for role in ROLES:
        result = load_result(role, task_dir)
        for target in TARGETS:
            observed = float(result[f"normalized_{target}"])
            expected = float(oracle["targets"][target])
            gap = abs(observed - expected)
            tolerance = exact_tolerance(expected)
            rows.append(
                {
                    "role": role,
                    "label": LABELS[role],
                    "target": target,
                    "observed": observed,
                    "oracle": expected,
                    "absolute_gap": gap,
                    "tolerance": tolerance,
                    "gap_to_tolerance": gap / tolerance,
                    "pass": gap <= tolerance,
                }
            )
    return rows


def performance_figure(rows: list[dict[str, Any]], output: Path) -> None:
    image = Image.new("RGB", (2500, 1050), "white")
    draw = ImageDraw.Draw(image)
    centered(draw, (1250, 70), "Operational smoke: 7,680 rows, 2 CPU cores, one run",
             font(44, bold=True))
    panels = (
        ("primary_seconds", "Primary runtime (seconds)", (0.5, 100), (1, 10, 100)),
        ("phase_peak_rss_mib", "Primary phase peak RSS (MiB)", (100, 10000), (100, 1000, 10000)),
    )
    for panel_index, (field, ylabel, limits, ticks) in enumerate(panels):
        left = 135 + panel_index * 1235
        right = left + 1090
        top, bottom = 175, 855
        values = [float(row[field]) for row in rows]
        lo, hi = limits

        def y_position(value: float) -> float:
            fraction = (log10(value) - log10(lo)) / (log10(hi) - log10(lo))
            return bottom - fraction * (bottom - top)

        draw.line((left, top, left, bottom), fill="#273746", width=3)
        draw.line((left, bottom, right, bottom), fill="#273746", width=3)
        for tick in ticks:
            y = y_position(tick)
            draw.line((left, y, right, y), fill="#D5D8DC", width=2)
            draw.text((left - 18, y), f"{tick:,}", font=font(24), fill="#566573", anchor="rm")
        centered(draw, ((left + right) / 2, 125), ylabel, font(31, bold=True))
        slot = (right - left) / len(rows)
        for index, (row, value) in enumerate(zip(rows, values)):
            center = left + slot * (index + 0.5)
            bar_width = slot * 0.58
            y = y_position(max(value, lo))
            draw.rounded_rectangle(
                (center - bar_width / 2, y, center + bar_width / 2, bottom),
                radius=8,
                fill=COLORS[str(row["role"])],
            )
            label = f"{value:.2f}" if value < 100 else f"{value:,.0f}"
            draw.text((center, y - 12), label, font=font(23, bold=True),
                      fill="#17202A", anchor="mb")
            centered(draw, (center, bottom + 42), str(row["label"]), font(22))
    centered(draw, (1250, 1005),
             "Primary phase includes implementation-specific cleaning, pool setup, estimation, and target extraction.",
             font(25), fill="#566573")
    image.save(output.with_suffix(".png"), dpi=(240, 240))


def exact_figure(rows: list[dict[str, Any]], output: Path) -> None:
    image = Image.new("RGB", (2500, 1150), "white")
    draw = ImageDraw.Draw(image)
    centered(draw, (1250, 55), "Exact audit: 960 rows, 1 CPU core", font(44, bold=True))
    centered(draw, (1250, 108), "Absolute gap / registered tolerance", font(27, bold=True))
    left, right, top, bottom = 220, 2375, 220, 945
    lo, hi = 1e-12, 100

    def y_position(value: float) -> float:
        fraction = (log10(value) - log10(lo)) / (log10(hi) - log10(lo))
        return bottom - fraction * (bottom - top)

    boundary = y_position(1)
    draw.rectangle((left, boundary, right, bottom), fill="#EDF8F3")
    draw.line((left, top, left, bottom), fill="#273746", width=3)
    draw.line((left, bottom, right, bottom), fill="#273746", width=3)
    for exponent in (-12, -9, -6, -3, 0, 2):
        value = 10.0 ** exponent
        y = y_position(value)
        color = "#B22222" if exponent == 0 else "#D5D8DC"
        width = 5 if exponent == 0 else 2
        draw.line((left, y, right, y), fill=color, width=width)
        label = "1 (pass boundary)" if exponent == 0 else f"1e{exponent:+d}"
        draw.text((left - 18, y), label, font=font(23), fill="#566573", anchor="rm")
    slot = (right - left) / len(ROLES)
    target_colors = dict(zip(TARGETS, ("#0072B2", "#D55E00", "#009E73", "#CC79A7")))
    offsets = dict(zip(TARGETS, (-54, -18, 18, 54)))
    for role_index, role in enumerate(ROLES):
        center = left + slot * (role_index + 0.5)
        centered(draw, (center, bottom + 40), LABELS[role], font(26))
        for target in TARGETS:
            row = next(item for item in rows if item["role"] == role and item["target"] == target)
            x = center + offsets[target]
            y = y_position(max(float(row["gap_to_tolerance"]), lo))
            draw.ellipse((x - 15, y - 15, x + 15, y + 15), fill=target_colors[target],
                         outline="white", width=3)
    legend_x = 530
    for target in TARGETS:
        draw.ellipse((legend_x, 151, legend_x + 24, 175), fill=target_colors[target])
        draw.text((legend_x + 34, 163), target.title(), font=font(23), fill="#17202A", anchor="lm")
        legend_x += 300
    centered(draw, (1250, 1085),
             "Points below the red line pass. The shaded area is the registered acceptance region.",
             font(25), fill="#566573")
    image.save(output.with_suffix(".png"), dpi=(240, 240))


def qacct_fields(path: Path) -> dict[str, str]:
    fields: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        parts = line.split(None, 1)
        if len(parts) == 2:
            fields[parts[0]] = parts[1].strip()
    return fields


def markdown_report(
    run: Path, smoke: list[dict[str, Any]], exact: list[dict[str, Any]], output: Path
) -> None:
    passed = sum(bool(row["pass"]) for row in exact)
    role_passes = {
        role: sum(bool(row["pass"]) for row in exact if row["role"] == role)
        for role in ROLES
    }
    smoke_lines = "\n".join(
        f"| {row['label']} | {row['primary_seconds']:.3f} | {row['cold_seconds']:.2f} | "
        f"{row['phase_peak_rss_mib']:.1f} |"
        for row in smoke
    )
    exact_lines = "\n".join(
        f"| {LABELS[role]} | {role_passes[role]}/4 | "
        + ", ".join(
            f"{row['target']} {row['gap_to_tolerance']:.3g}x"
            for row in exact
            if row["role"] == role
        )
        + " |"
        for role in ROLES
    )
    text = f"""# Five-way FE variance-component benchmark: preflight report

**Status: exact-consistency gate failed.** The preparation job and the five-role smoke task passed, but only {passed} of {len(exact)} exact target checks passed. Under the frozen protocol, the pilot and confirmation size/core sweeps were therefore not submitted. This report contains diagnostic one-cell measurements only and makes no scaling claim.

## Operational smoke

All five implementations completed the same deterministic 7,680-row dataset with 2 requested CPU cores and 280 projections. The primary interval includes implementation-specific cleaning, pool setup, estimation, and target extraction. Memory is the 100 ms process-tree RSS peak during that interval.

![One-cell runtime and memory](smoke_performance.png)

| Implementation | Primary seconds | Cold seconds | Peak RSS MiB |
|---|---:|---:|---:|
{smoke_lines}

These are single-run smoke measurements, not benchmark estimates. They do not establish runtime or memory scaling.

## Exact consistency

The 960-row/1-core audit compared normalized worker, firm, covariance, and total components with an independent dense KSS oracle. The registered rule was `absolute gap <= max(1e-8, 1e-5 * max(1, abs(oracle)))`.

![Exact gap relative to tolerance](exact_gap_ratio.png)

| Implementation | Checks passed | Gap/tolerance by target |
|---|---:|---|
{exact_lines}

FEVC matched the oracle on all four targets. MATLAB and Julia matched worker and total but missed firm and covariance. R and PyTwoWay missed all four registered checks. The estimates form closely related numerical clusters, but the benchmark does not identify whether their differences arise from estimator definitions, finite-sample corrections, or implementation details.

## Stage disposition

| Stage | SGE job | Result |
|---|---:|---|
| Preparation | 7502659 | PASS |
| Smoke | 7502666 | PASS |
| Exact audit | 7502671 | FAIL - numerical consistency (all five programs completed) |
| Pilot | - | NOT SUBMITTED |
| Confirmation | - | NOT SUBMITTED |

The exact SGE task had `failed=0` and `exit_status=1`: the scheduler and applications ran normally, while the wrapper deliberately returned failure after the scientific validator rejected the results.

## Reproducibility

- Run: `{run.name}`
- Remote path: `/projectnb/welfgr/vckss/five_way_scaling/runs/{run.name}`
- FEVC source: `f3098bc1369992fccbc1d276aac5fc65ceb3f404`
- Collected code: all 29 entries in `input/code.sha256` verified locally
- Raw outputs, process-tree samples, application logs, environment receipts, SGE accounting, and frozen manifests are preserved with this report
"""
    output.write_text(text, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)

    smoke = smoke_rows(args.run)
    exact = exact_rows(args.run)
    write_csv(args.output / "smoke_performance.csv", smoke)
    write_csv(args.output / "exact_consistency.csv", exact)
    performance_figure(smoke, args.output / "smoke_performance")
    exact_figure(exact, args.output / "exact_gap_ratio")

    exact_qacct = qacct_fields(args.run / "qacct" / "exact.txt")
    passed = sum(bool(row["pass"]) for row in exact)
    summary = {
        "schema": "FEVC-FIVE-WAY-PREFLIGHT-REPORT-V1",
        "status": "FAIL",
        "failure_stage": "exact_consistency",
        "reason": "registered exact numerical tolerance failed",
        "run_id": args.run.name,
        "smoke_roles_passed": len(smoke),
        "exact_checks_passed": passed,
        "exact_checks_total": len(exact),
        "pilot_submitted": False,
        "confirmation_submitted": False,
        "exact_qacct_failed": int(exact_qacct["failed"]),
        "exact_qacct_exit_status": int(exact_qacct["exit_status"]),
        "exact_qacct_wallclock_seconds": int(exact_qacct["ru_wallclock"]),
        "exact_qacct_maxvmem": exact_qacct["maxvmem"],
    }
    atomic_json(args.output / "summary.json", summary)
    markdown_report(args.run, smoke, exact, args.output / "report.md")
    print(
        f"FEVC_FIVE_WAY_PREFLIGHT_REPORT_FAIL exact_checks={passed}/{len(exact)} "
        "pilot_submitted=false confirmation_submitted=false"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
