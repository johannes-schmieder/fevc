#!/usr/bin/env python3
"""Build a standalone PDF for a five-way benchmark stopped at preflight."""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from typing import Any

from PIL import Image as PILImage
from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER, TA_LEFT
from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import inch
from reportlab.platypus import (
    HRFlowable,
    Image,
    PageBreak,
    Paragraph,
    SimpleDocTemplate,
    Spacer,
    Table,
    TableStyle,
)


NAVY = colors.HexColor("#17324D")
BLUE = colors.HexColor("#0072B2")
RED = colors.HexColor("#B22222")
GREEN = colors.HexColor("#007A5E")
PALE_RED = colors.HexColor("#FCEBEC")
PALE_BLUE = colors.HexColor("#EAF3F8")
PALE_GRAY = colors.HexColor("#F4F6F7")
MID_GRAY = colors.HexColor("#5D6D7E")
TARGETS = ("worker", "firm", "covariance", "total")
ROLES = ("fevc", "matlab", "julia", "r", "pytwoway")
LABELS = {
    "fevc": "FEVC",
    "matlab": "KSS MATLAB",
    "julia": "Julia",
    "r": "R",
    "pytwoway": "PyTwoWay",
}


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def stylesheet() -> dict[str, ParagraphStyle]:
    base = getSampleStyleSheet()
    return {
        "title": ParagraphStyle(
            "ReportTitle",
            parent=base["Title"],
            fontName="Helvetica-Bold",
            fontSize=24,
            leading=28,
            textColor=NAVY,
            alignment=TA_LEFT,
            spaceAfter=10,
        ),
        "subtitle": ParagraphStyle(
            "ReportSubtitle",
            parent=base["Normal"],
            fontName="Helvetica",
            fontSize=11,
            leading=15,
            textColor=MID_GRAY,
            spaceAfter=14,
        ),
        "status": ParagraphStyle(
            "Status",
            parent=base["Normal"],
            fontName="Helvetica-Bold",
            fontSize=15,
            leading=19,
            textColor=RED,
            alignment=TA_CENTER,
        ),
        "h1": ParagraphStyle(
            "H1",
            parent=base["Heading1"],
            fontName="Helvetica-Bold",
            fontSize=16,
            leading=20,
            textColor=NAVY,
            spaceBefore=4,
            spaceAfter=8,
        ),
        "h2": ParagraphStyle(
            "H2",
            parent=base["Heading2"],
            fontName="Helvetica-Bold",
            fontSize=12,
            leading=15,
            textColor=NAVY,
            spaceBefore=7,
            spaceAfter=5,
        ),
        "body": ParagraphStyle(
            "Body",
            parent=base["BodyText"],
            fontName="Helvetica",
            fontSize=9.4,
            leading=13.2,
            textColor=colors.HexColor("#1F2D3A"),
            spaceAfter=7,
        ),
        "small": ParagraphStyle(
            "Small",
            parent=base["BodyText"],
            fontName="Helvetica",
            fontSize=7.6,
            leading=10,
            textColor=MID_GRAY,
        ),
        "cell": ParagraphStyle(
            "Cell",
            parent=base["BodyText"],
            fontName="Helvetica",
            fontSize=7.7,
            leading=9.4,
            textColor=colors.HexColor("#1F2D3A"),
        ),
        "cell_bold": ParagraphStyle(
            "CellBold",
            parent=base["BodyText"],
            fontName="Helvetica-Bold",
            fontSize=7.7,
            leading=9.4,
            textColor=colors.HexColor("#1F2D3A"),
        ),
        "bullet": ParagraphStyle(
            "Bullet",
            parent=base["BodyText"],
            fontName="Helvetica",
            fontSize=9.2,
            leading=12.8,
            leftIndent=12,
            firstLineIndent=-7,
            bulletIndent=0,
            textColor=colors.HexColor("#1F2D3A"),
            spaceAfter=5,
        ),
    }


def p(value: str, style: ParagraphStyle) -> Paragraph:
    return Paragraph(value, style)


def table(
    data: list[list[Any]], widths: list[float], *, header: bool = True,
    font_size: float = 7.5, row_backgrounds: bool = True,
) -> Table:
    result = Table(data, colWidths=widths, repeatRows=1 if header else 0, hAlign="LEFT")
    commands: list[tuple[Any, ...]] = [
        ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
        ("FONTNAME", (0, 0), (-1, -1), "Helvetica"),
        ("FONTSIZE", (0, 0), (-1, -1), font_size),
        ("LEADING", (0, 0), (-1, -1), font_size + 2),
        ("TEXTCOLOR", (0, 0), (-1, -1), colors.HexColor("#1F2D3A")),
        ("GRID", (0, 0), (-1, -1), 0.35, colors.HexColor("#D5D8DC")),
        ("LEFTPADDING", (0, 0), (-1, -1), 5),
        ("RIGHTPADDING", (0, 0), (-1, -1), 5),
        ("TOPPADDING", (0, 0), (-1, -1), 4),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 4),
    ]
    if header:
        commands.extend(
            [
                ("BACKGROUND", (0, 0), (-1, 0), NAVY),
                ("TEXTCOLOR", (0, 0), (-1, 0), colors.white),
                ("FONTNAME", (0, 0), (-1, 0), "Helvetica-Bold"),
            ]
        )
    if row_backgrounds:
        first = 1 if header else 0
        for index in range(first, len(data)):
            if (index - first) % 2:
                commands.append(("BACKGROUND", (0, index), (-1, index), PALE_GRAY))
    result.setStyle(TableStyle(commands))
    return result


def scaled_image(path: Path, width: float) -> Image:
    with PILImage.open(path) as source:
        pixel_width, pixel_height = source.size
    height = width * pixel_height / pixel_width
    return Image(str(path), width=width, height=height)


def header_footer(canvas: Any, document: Any) -> None:
    canvas.saveState()
    width, height = letter
    canvas.setStrokeColor(colors.HexColor("#D5D8DC"))
    canvas.setLineWidth(0.5)
    canvas.line(document.leftMargin, height - 0.46 * inch,
                width - document.rightMargin, height - 0.46 * inch)
    canvas.setFont("Helvetica", 7.5)
    canvas.setFillColor(MID_GRAY)
    canvas.drawString(document.leftMargin, height - 0.35 * inch,
                      "FEVC five-way benchmark - preflight evidence")
    canvas.drawRightString(width - document.rightMargin, 0.34 * inch,
                           f"Page {document.page}")
    canvas.restoreState()


def exact_summary(exact: list[dict[str, str]]) -> list[list[Any]]:
    rows: list[list[Any]] = [["Implementation", "Passed", "Worker", "Firm", "Covariance", "Total"]]
    for role in ROLES:
        chosen = [row for row in exact if row["role"] == role]
        passed = sum(row["pass"] == "True" for row in chosen)
        cells: list[Any] = [LABELS[role], f"{passed}/4"]
        for target in TARGETS:
            item = next(row for row in chosen if row["target"] == target)
            ratio = float(item["gap_to_tolerance"])
            mark = "PASS" if item["pass"] == "True" else "FAIL"
            cells.append(f"{ratio:.3g}x {mark}")
        rows.append(cells)
    return rows


def build(run: Path, diagnostic: Path, output: Path) -> None:
    summary = json.loads((diagnostic / "summary.json").read_text(encoding="utf-8"))
    smoke = read_csv(diagnostic / "smoke_performance.csv")
    exact = read_csv(diagnostic / "exact_consistency.csv")
    styles = stylesheet()

    document = SimpleDocTemplate(
        str(output),
        pagesize=letter,
        leftMargin=0.62 * inch,
        rightMargin=0.62 * inch,
        topMargin=0.63 * inch,
        bottomMargin=0.55 * inch,
        title="Five-way FE variance-component benchmark: preflight report",
        author="FEVC benchmark harness",
        subject="Failed exact-consistency gate and operational smoke evidence",
    )
    story: list[Any] = []

    story.extend(
        [
            Spacer(1, 0.20 * inch),
            p("Five-way FE variance-component benchmark", styles["title"]),
            p("Preflight evidence from Boston University's Shared Computing Cluster",
              styles["subtitle"]),
            Table(
                [[p("PREFLIGHT FAILED - SCALING SWEEPS NOT RUN", styles["status"])]],
                colWidths=[7.0 * inch],
                style=TableStyle(
                    [
                        ("BACKGROUND", (0, 0), (-1, -1), PALE_RED),
                        ("BOX", (0, 0), (-1, -1), 1.0, RED),
                        ("TOPPADDING", (0, 0), (-1, -1), 12),
                        ("BOTTOMPADDING", (0, 0), (-1, -1), 12),
                    ]
                ),
            ),
            Spacer(1, 0.20 * inch),
            p("Outcome", styles["h1"]),
            p(
                "The benchmark infrastructure is operational: preparation passed and all five "
                "implementations completed the common smoke task. The predeclared exact audit "
                f"passed only <b>{summary['exact_checks_passed']} of {summary['exact_checks_total']}</b> "
                "target checks. The frozen protocol therefore prohibited submission of the pilot "
                "and confirmation size/core sweeps.",
                styles["body"],
            ),
            p(
                "This document reports the valid one-cell diagnostics and the consistency failure. "
                "It does <b>not</b> present dataset-size or CPU-core scaling curves, and the smoke "
                "numbers must not be interpreted as comparative benchmark estimates.",
                styles["body"],
            ),
            p("Stage disposition", styles["h1"]),
            table(
                [
                    ["Stage", "SGE job", "Status", "Disposition"],
                    ["Preparation", "7502659", "PASS", "Environment and source receipts complete"],
                    ["Smoke", "7502666", "PASS", "Five of five implementations completed"],
                    ["Exact audit", "7502671", "FAIL", "8/20 numerical checks passed"],
                    ["Pilot", "-", "NOT RUN", "Blocked by exact gate"],
                    ["Confirmation", "-", "NOT RUN", "Blocked by exact gate"],
                ],
                [1.15 * inch, 0.78 * inch, 0.80 * inch, 4.27 * inch],
            ),
            Spacer(1, 0.14 * inch),
            p("What is established", styles["h1"]),
            p("&#8226; The five toolchains, adapters, scheduler wrapper, input generator, and process-tree monitor work end to end.", styles["bullet"]),
            p("&#8226; FEVC agrees with the independent dense oracle on all four exact targets.", styles["bullet"]),
            p("&#8226; The non-FEVC implementations do not all satisfy the common registered estimand/tolerance contract.", styles["bullet"]),
            p("&#8226; No runtime or memory scaling conclusion is supported because the sweep was not launched.", styles["bullet"]),
            Spacer(1, 0.10 * inch),
            HRFlowable(width="100%", thickness=0.6, color=colors.HexColor("#D5D8DC")),
            Spacer(1, 0.08 * inch),
            p(
                f"Run ID: <b>{run.name}</b><br/>FEVC source: "
                "<font name='Courier'>f3098bc1369992fccbc1d276aac5fc65ceb3f404</font><br/>"
                "Report date: 9 September 2026",
                styles["small"],
            ),
            PageBreak(),
            Spacer(1, 0.10 * inch),
        ]
    )

    story.extend(
        [
            p("Operational smoke: runtime and memory", styles["h1"]),
            p(
                "All implementations processed the same deterministic 7,680-row design with two "
                "requested CPU cores and 280 projections. Primary runtime covers implementation-specific "
                "cleaning, worker-pool setup, estimation, and target extraction after generic CSV import. "
                "Peak RSS is the maximum 100 ms process-tree sum during the same interval.",
                styles["body"],
            ),
            scaled_image(diagnostic / "smoke_performance.png", 7.0 * inch),
            Spacer(1, 0.08 * inch),
            table(
                [["Implementation", "Primary sec", "Cold sec", "Peak RSS MiB", "Whole-run RSS MiB"]]
                + [
                    [
                        row["label"],
                        f"{float(row['primary_seconds']):.3f}",
                        f"{float(row['cold_seconds']):.2f}",
                        f"{float(row['phase_peak_rss_mib']):,.1f}",
                        f"{float(row['whole_peak_rss_mib']):,.1f}",
                    ]
                    for row in smoke
                ],
                [1.65 * inch, 1.15 * inch, 1.05 * inch, 1.45 * inch, 1.55 * inch],
            ),
            Spacer(1, 0.10 * inch),
            Table(
                [[p(
                    "<b>Interpretation boundary.</b> These are one-run readiness measurements. They "
                    "show that each program completed and that instrumentation captured comparable "
                    "intervals. They do not estimate medians, variability, size scaling, parallel "
                    "speedup, or memory scaling.", styles["body"]) ]],
                colWidths=[7.0 * inch],
                style=TableStyle(
                    [
                        ("BACKGROUND", (0, 0), (-1, -1), PALE_BLUE),
                        ("BOX", (0, 0), (-1, -1), 0.6, BLUE),
                        ("LEFTPADDING", (0, 0), (-1, -1), 9),
                        ("RIGHTPADDING", (0, 0), (-1, -1), 9),
                        ("TOPPADDING", (0, 0), (-1, -1), 7),
                        ("BOTTOMPADDING", (0, 0), (-1, -1), 2),
                    ]
                ),
            ),
            PageBreak(),
            Spacer(1, 0.10 * inch),
        ]
    )

    story.extend(
        [
            p("Exact consistency audit", styles["h1"]),
            p(
                "The 960-row/one-core task compared normalized worker, firm, covariance, and total "
                "components against an independent dense KSS oracle. The registered rule was "
                "<font name='Courier'>abs(gap) &lt;= max(1e-8, 1e-5 * max(1, abs(oracle)))</font>. "
                "The chart reports each absolute gap divided by its tolerance; values at or below one pass.",
                styles["body"],
            ),
            scaled_image(diagnostic / "exact_gap_ratio.png", 7.0 * inch),
            Spacer(1, 0.08 * inch),
            table(
                exact_summary(exact),
                [1.12 * inch, 0.58 * inch, 1.28 * inch, 1.20 * inch, 1.42 * inch, 1.22 * inch],
                font_size=6.9,
            ),
            Spacer(1, 0.10 * inch),
            p(
                "FEVC passed all four targets. MATLAB and Julia passed worker and total but failed "
                "firm and covariance. R and PyTwoWay failed all four registered checks. MATLAB and "
                "Julia are nearly identical to each other in this audit, while the estimates otherwise "
                "fall into closely related numerical clusters. The evidence does not establish whether "
                "the discrepancies arise from estimator definitions, finite-sample corrections, or "
                "implementation details.",
                styles["body"],
            ),
            PageBreak(),
            Spacer(1, 0.10 * inch),
        ]
    )

    oracle = {row["target"]: float(row["oracle"]) for row in exact if row["role"] == "fevc"}
    details: list[list[Any]] = [["Implementation", "Target", "Normalized estimate", "Oracle", "Absolute gap", "Tolerance", "Result"]]
    for row in exact:
        details.append(
            [
                row["label"],
                row["target"].title(),
                f"{float(row['observed']):.12g}",
                f"{float(row['oracle']):.12g}",
                f"{float(row['absolute_gap']):.4g}",
                f"{float(row['tolerance']):.4g}",
                "PASS" if row["pass"] == "True" else "FAIL",
            ]
        )
    story.extend(
        [
            p("Exact audit details", styles["h1"]),
            table(
                details,
                [1.02 * inch, 0.72 * inch, 1.30 * inch, 1.16 * inch, 0.94 * inch, 0.84 * inch, 0.62 * inch],
                font_size=6.15,
            ),
            Spacer(1, 0.12 * inch),
            p("Independent dense-oracle targets", styles["h2"]),
            table(
                [["Worker", "Firm", "Covariance", "Total"]]
                + [[f"{oracle[target]:.15g}" for target in TARGETS]],
                [1.75 * inch] * 4,
                font_size=7.0,
            ),
            PageBreak(),
            Spacer(1, 0.10 * inch),
        ]
    )

    story.extend(
        [
            p("Frozen design and reproducibility boundary", styles["h1"]),
            p("Intended scaling design", styles["h2"]),
            table(
                [
                    ["Dimension", "Registered values"],
                    ["Implementations", "FEVC; KSS MATLAB; Julia; R; PyTwoWay"],
                    ["Rows", "7,680; 30,720; 122,880; 491,520"],
                    ["CPU cores", "1; 2; 4; 8; 14; 28"],
                    ["Repeated measurements", "Five fixed seeds per benchmark cell"],
                    ["Algorithm", "JLA with 280 projections; match deletion; no controls"],
                    ["Dataset", "Deterministic strong_d3 design; three firm matches per worker"],
                ],
                [1.65 * inch, 5.35 * inch],
            ),
            p("Execution and evidence", styles["h2"]),
            p(
                "The preparation and smoke jobs passed. The exact job completed every estimator, then "
                "the validator intentionally returned exit status 1. SGE recorded "
                "<font name='Courier'>failed=0</font>, 210 seconds of job wall time, and 10.348G maximum "
                "virtual memory. This distinguishes a scientific gate failure from a scheduler or "
                "application crash.",
                styles["body"],
            ),
            p(
                "The collected run contains frozen manifests for smoke, exact, pilot, and confirmation; "
                "raw role outputs; process-tree traces; GNU time records; application logs; environment "
                "receipts; and SGE accounting. All 29 files covered by the collected code manifest "
                "verified against their SHA-256 hashes. Only preparation, smoke, and exact job IDs exist; "
                "neither downstream stage was submitted.",
                styles["body"],
            ),
            p("Scientific next step", styles["h2"]),
            p(
                "Before any scaling campaign, the target definitions and finite-sample corrections in "
                "the four comparator adapters should be reconciled with the independent dense oracle. "
                "Any justified change to normalization, estimand mapping, or tolerance is a scientific "
                "contract change and should be registered before a fresh preflight; the failed result "
                "reported here must remain preserved.",
                styles["body"],
            ),
            Table(
                [[p(
                    "<b>Bottom line.</b> The harness is ready to measure runtime and process-tree memory, "
                    "but the five implementations are not yet certified as estimating the same targets "
                    "under the registered exact contract. Running the full size/core grid now would "
                    "produce performance curves for scientifically non-comparable quantities.",
                    styles["body"],
                )]],
                colWidths=[7.0 * inch],
                style=TableStyle(
                    [
                        ("BACKGROUND", (0, 0), (-1, -1), PALE_RED),
                        ("BOX", (0, 0), (-1, -1), 0.8, RED),
                        ("LEFTPADDING", (0, 0), (-1, -1), 10),
                        ("RIGHTPADDING", (0, 0), (-1, -1), 10),
                        ("TOPPADDING", (0, 0), (-1, -1), 8),
                        ("BOTTOMPADDING", (0, 0), (-1, -1), 2),
                    ]
                ),
            ),
            Spacer(1, 0.12 * inch),
            p(
                "Collected run directory:<br/><font name='Courier' size='7'>"
                f"{run.resolve()}</font>",
                styles["small"],
            ),
        ]
    )

    output.parent.mkdir(parents=True, exist_ok=True)
    document.build(story, onFirstPage=header_footer, onLaterPages=header_footer)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--diagnostic", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    build(args.run, args.diagnostic, args.output)
    print(f"FEVC_FIVE_WAY_PDF_PASS {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
