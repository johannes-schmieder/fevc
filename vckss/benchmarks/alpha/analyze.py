#!/usr/bin/env python3
"""Validate, compare, and summarize VCKSS alpha benchmark receipts."""

from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
from datetime import datetime, timezone
from pathlib import Path

BACKENDS = ("rust", "mata")
STATE_FIELDS = ("sample_ok", "data_ok", "rng_ok", "sort_ok")
STRUCTURAL_FIELDS = (
    "n_stored",
    "n_physical",
    "n_retained",
    "workers",
    "firms",
    "cells",
    "units",
    "strata",
    "probes",
    "sample_n",
    "deletion",
    "controls",
    "processors",
)
RESULT_FIELDS = tuple(f"r{row}{column}" for column in range(1, 5) for row in range(1, 5))
RUST_PHASE_FIELDS = (
    "ingest_s",
    "canon_s",
    "graph_s",
    "compress_s",
    "plan_s",
    "stayer_s",
    "solve_s",
    "native_s",
)
MATA_PHASE_FIELDS = (
    "fit_s",
    "leverage_s",
    "target_s",
    "correction_s",
    "rng_s",
    "setup_s",
    "schur_s",
    "pcg_s",
    "precond_s",
)


def read_cases(path: Path) -> dict[str, dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    return {row["case_id"]: row for row in rows}


def read_rows(run_root: Path) -> list[dict[str, str]]:
    path = run_root / "rows.csv"
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    if not rows:
        raise RuntimeError(f"{path}: no benchmark rows")
    return rows


def as_float(row: dict[str, str], field: str) -> float:
    value = float(row[field])
    if not math.isfinite(value):
        raise RuntimeError(f"nonfinite {field} in {row['case_id']}/{row['backend']}")
    return value


def validate_backend_rows(
    case_id: str, backend: str, rows: list[dict[str, str]]
) -> None:
    rows.sort(key=lambda row: int(float(row["run"])))
    if len(rows) != 4:
        raise RuntimeError(f"{case_id}/{backend}: expected four rows")
    for index, row in enumerate(rows, 1):
        if int(float(row["run"])) != index:
            raise RuntimeError(f"{case_id}/{backend}: noncanonical run order")
        if row["temperature"] != ("cold" if index == 1 else "warm"):
            raise RuntimeError(f"{case_id}/{backend}: temperature label mismatch")
        for field in STATE_FIELDS:
            if int(as_float(row, field)) != 1:
                raise RuntimeError(f"{case_id}/{backend}: failed {field}")
        if as_float(row, "result_diff") != 0:
            raise RuntimeError(f"{case_id}/{backend}: within-process result drift")
        if as_float(row, "max_resid") > as_float(row, "accept_tol"):
            raise RuntimeError(f"{case_id}/{backend}: residual gate failed")
        if abs(as_float(row, "identity_resid")) > 1e-12:
            raise RuntimeError(f"{case_id}/{backend}: accounting gate failed")
        if row["estimator_status"] != "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES":
            raise RuntimeError(f"{case_id}/{backend}: unexpected status")


def cross_backend_parity(
    case_id: str,
    rust: list[dict[str, str]],
    mata: list[dict[str, str]],
) -> dict[str, object]:
    left = rust[0]
    right = mata[0]
    for field in STRUCTURAL_FIELDS:
        if left[field] != right[field]:
            raise RuntimeError(
                f"{case_id}: structural mismatch {field}: "
                f"{left[field]!r} != {right[field]!r}"
            )
    differences: list[float] = []
    standardized: list[float] = []
    for index, field in enumerate(RESULT_FIELDS):
        column = index // 4 + 1
        difference = abs(as_float(left, field) - as_float(right, field))
        joint_mcse = math.hypot(
            as_float(left, f"mcse{column}"),
            as_float(right, f"mcse{column}"),
        )
        differences.append(difference)
        standardized.append(difference / max(joint_mcse, 1e-14))
    max_z = max(standardized)
    return {
        "case_id": case_id,
        "max_absolute_result_difference": max(differences),
        "max_combined_mcse_units": max_z,
        "status": "PASS" if max_z <= 6 else "FAIL",
    }


def tex_escape(value: str) -> str:
    replacements = {
        "\\": r"\textbackslash{}",
        "_": r"\_",
        "%": r"\%",
        "&": r"\&",
        "#": r"\#",
    }
    return "".join(replacements.get(character, character) for character in value)


def write_tex(
    output: Path,
    *,
    summary: dict[str, object],
    timings: list[dict[str, object]],
    parity: list[dict[str, object]],
    phases: list[dict[str, object]],
) -> None:
    timing_by_case = {row["case_id"]: row for row in timings}
    synthetic = timing_by_case.get("headline_synthetic")
    cz18 = timing_by_case.get("cz18_headline")
    synthetic_speedup = f"{float(synthetic['speedup']):.2f}" if synthetic else "pending"
    cz18_speedup = f"{float(cz18['speedup']):.2f}" if cz18 else "pending"
    macros = [
        rf"\newcommand{{\AlphaStatus}}{{{tex_escape(str(summary['status']))}}}",
        rf"\newcommand{{\AlphaSource}}{{\texttt{{{summary['source_commit']}}}}}",
        rf"\newcommand{{\AlphaSourceShort}}{{\texttt{{{str(summary['source_commit'])[:12]}}}}}",
        rf"\newcommand{{\AlphaGenerated}}{{{tex_escape(str(summary['generated_at_utc']))}}}",
        rf"\newcommand{{\AlphaMaxParityZ}}{{{summary['max_combined_mcse_units']:.2f}}}",
        rf"\newcommand{{\AlphaMinimumSpeedup}}{{{summary['minimum_speedup']:.2f}}}",
        rf"\newcommand{{\AlphaSyntheticSpeedup}}{{{synthetic_speedup}}}",
        rf"\newcommand{{\AlphaCzSpeedup}}{{{cz18_speedup}}}",
    ]
    (output / "summary_macros.tex").write_text(
        "\n".join(macros) + "\n", encoding="utf-8"
    )
    labels = ",".join(tex_escape(str(row["case_id"])) for row in timings)
    coordinates = " ".join(
        f"({index},{float(row['speedup']):.6f})"
        for index, row in enumerate(timings, 1)
    )
    plot = rf"""\begin{{tikzpicture}}
\begin{{axis}}[
  ybar,
  width=\textwidth,
  height=7cm,
  ymin=0,
  ylabel={{Mata time / Rust time}},
  xtick={{{','.join(str(index) for index in range(1, len(timings)+1))}}},
  xticklabels={{{labels}}},
  x tick label style={{rotate=30,anchor=east,font=\small}},
  bar width=18pt,
  nodes near coords,
  nodes near coords style={{font=\scriptsize}},
  grid=major,
]
\addplot[fill=blue!55] coordinates {{{coordinates}}};
\addplot[red,dashed,domain=0:{len(timings)+1}] {{2}};
\end{{axis}}
\end{{tikzpicture}}
"""
    (output / "speedup_plot.tex").write_text(plot, encoding="utf-8")
    timing_lines = [
        r"\begin{tabular}{lrrrrr}",
        r"\toprule",
        r"Case & Rust cold & Rust warm & Mata warm & Speedup & Rust RSS (MiB) \\",
        r"\midrule",
    ]
    for row in timings:
        timing_lines.append(
            f"{tex_escape(str(row['case_id']))} & "
            f"{float(row['rust_cold_seconds']):.2f} & "
            f"{float(row['rust_warm_median_seconds']):.2f} & "
            f"{float(row['mata_warm_median_seconds']):.2f} & "
            f"{float(row['speedup']):.2f} & "
            f"{int(row['rust_peak_rss_bytes']) / 2**20:.0f} \\\\"
        )
    timing_lines.extend((r"\bottomrule", r"\end{tabular}"))
    (output / "timing_table.tex").write_text(
        "\n".join(timing_lines) + "\n", encoding="utf-8"
    )
    parity_lines = [
        r"\begin{tabular}{lrrl}",
        r"\toprule",
        r"Case & Maximum absolute gap & Combined-MCSE units & Gate \\",
        r"\midrule",
    ]
    for row in parity:
        parity_lines.append(
            f"{tex_escape(str(row['case_id']))} & "
            f"{float(row['max_absolute_result_difference']):.3g} & "
            f"{float(row['max_combined_mcse_units']):.2f} & "
            f"{row['status']} \\\\"
        )
    parity_lines.extend((r"\bottomrule", r"\end{tabular}"))
    (output / "parity_table.tex").write_text(
        "\n".join(parity_lines) + "\n", encoding="utf-8"
    )
    phase_lines = [
        r"\begin{tabular}{lllrr}",
        r"\toprule",
        r"Case & Backend & Phase & Median seconds & Share of command \\",
        r"\midrule",
    ]
    for row in phases:
        if row["case_id"] not in ("headline_synthetic", "cz18_headline"):
            continue
        phase_lines.append(
            f"{tex_escape(str(row['case_id']))} & {row['backend']} & "
            f"{tex_escape(str(row['phase']))} & "
            f"{float(row['median_seconds']):.3f} & "
            f"{100 * float(row['command_share']):.1f}\\% \\\\"
        )
    if len(phase_lines) == 4:
        phase_lines.append(r"\multicolumn{5}{l}{Headline phase evidence pending.} \\")
    phase_lines.extend((r"\bottomrule", r"\end{tabular}"))
    (output / "phase_table.tex").write_text(
        "\n".join(phase_lines) + "\n", encoding="utf-8"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-root", type=Path, action="append", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected-source")
    parser.add_argument("--require-alpha", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    cases = read_cases(Path(__file__).with_name("cases.tsv"))
    rows: list[dict[str, str]] = []
    for root in args.run_root:
        rows.extend(read_rows(root))
    sources = {row["source_commit"] for row in rows}
    fixtures = {row["fixture_spec_sha256"] for row in rows}
    if len(sources) != 1 or len(fixtures) != 1:
        raise RuntimeError("benchmark inputs do not share one source and fixture identity")
    source_commit = next(iter(sources))
    if args.expected_source and source_commit != args.expected_source:
        raise RuntimeError("benchmark source does not match --expected-source")

    grouped: dict[tuple[str, str], list[dict[str, str]]] = {}
    for row in rows:
        key = (row["case_id"], row["backend"])
        if key in grouped and any(
            int(float(existing["run"])) == int(float(row["run"]))
            for existing in grouped[key]
        ):
            raise RuntimeError(f"duplicate benchmark row {key} run={row['run']}")
        grouped.setdefault(key, []).append(row)
    for (case_id, backend), backend_rows in grouped.items():
        if case_id not in cases or backend not in BACKENDS:
            raise RuntimeError(f"unregistered benchmark cell: {case_id}/{backend}")
        validate_backend_rows(case_id, backend, backend_rows)

    complete_cases = sorted(
        case_id
        for case_id in cases
        if all((case_id, backend) in grouped for backend in BACKENDS)
    )
    timings: list[dict[str, object]] = []
    parity: list[dict[str, object]] = []
    phases: list[dict[str, object]] = []
    for case_id in complete_cases:
        rust = grouped[(case_id, "rust")]
        mata = grouped[(case_id, "mata")]
        rust_warm = statistics.median(as_float(row, "total_s") for row in rust[1:])
        mata_warm = statistics.median(as_float(row, "total_s") for row in mata[1:])
        timings.append(
            {
                "case_id": case_id,
                "rust_cold_seconds": as_float(rust[0], "total_s"),
                "mata_cold_seconds": as_float(mata[0], "total_s"),
                "rust_warm_median_seconds": rust_warm,
                "mata_warm_median_seconds": mata_warm,
                "speedup": mata_warm / rust_warm,
                "rust_peak_rss_bytes": int(float(rust[0]["peak_rss_bytes"])),
                "mata_peak_rss_bytes": int(float(mata[0]["peak_rss_bytes"])),
                "alpha_gate": int(cases[case_id]["alpha_gate"]),
            }
        )
        parity.append(cross_backend_parity(case_id, rust, mata))
        for backend, backend_rows, command_median, fields in (
            ("rust", rust, rust_warm, RUST_PHASE_FIELDS),
            ("mata", mata, mata_warm, MATA_PHASE_FIELDS),
        ):
            for field in fields:
                values = [
                    float(row[field])
                    for row in backend_rows[1:]
                    if row.get(field, "") not in ("", ".")
                ]
                if not values:
                    continue
                phase_median = statistics.median(values)
                phases.append(
                    {
                        "case_id": case_id,
                        "backend": backend,
                        "phase": field.removesuffix("_s"),
                        "median_seconds": phase_median,
                        "command_share": phase_median / command_median,
                    }
                )

    required_headlines = {
        case_id for case_id, case in cases.items() if case["alpha_gate"] == "1"
    }
    observed_headlines = required_headlines.intersection(complete_cases)
    all_cells_clear = bool(timings) and all(
        float(row["speedup"]) >= 1 / 1.10 for row in timings
    )
    headlines_clear = observed_headlines == required_headlines and all(
        float(row["speedup"]) >= 2
        for row in timings
        if row["case_id"] in required_headlines
    )
    parity_clear = bool(parity) and all(row["status"] == "PASS" for row in parity)
    complete = observed_headlines == required_headlines
    if complete and all_cells_clear and headlines_clear and parity_clear:
        status = "PASS"
    elif not complete and all_cells_clear and parity_clear:
        status = "INCOMPLETE"
    else:
        status = "FAIL"
    summary: dict[str, object] = {
        "schema": "vckss-alpha-benchmark-analysis-v1",
        "status": status,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "source_commit": source_commit,
        "fixture_spec_sha256": next(iter(fixtures)),
        "observed_cases": complete_cases,
        "required_headlines": sorted(required_headlines),
        "missing_headlines": sorted(required_headlines - observed_headlines),
        "all_supported_cells_at_most_ten_percent_slower": all_cells_clear,
        "headline_speedups_at_least_two": headlines_clear,
        "parity_within_six_combined_mcse": parity_clear,
        "minimum_speedup": min((float(row["speedup"]) for row in timings), default=0),
        "max_combined_mcse_units": max(
            (float(row["max_combined_mcse_units"]) for row in parity), default=0
        ),
        "timings": timings,
        "parity": parity,
        "phases": phases,
    }
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / "summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    for filename, records in (
        ("timings.csv", timings),
        ("parity.csv", parity),
        ("phases.csv", phases),
    ):
        with (args.output / filename).open("w", newline="", encoding="utf-8") as handle:
            writer = csv.DictWriter(handle, fieldnames=list(records[0]))
            writer.writeheader()
            writer.writerows(records)
    write_tex(
        args.output,
        summary=summary,
        timings=timings,
        parity=parity,
        phases=phases,
    )
    print(f"VCKSS_ALPHA_ANALYSIS_{status} {source_commit}")
    if args.require_alpha and status != "PASS":
        return 1
    return 0 if status != "FAIL" else 1


if __name__ == "__main__":
    raise SystemExit(main())
