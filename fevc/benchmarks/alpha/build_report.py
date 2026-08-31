#!/usr/bin/env python3
"""Compile the alpha benchmark report against validated analyzer output."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import tempfile
from pathlib import Path

REQUIRED = (
    "summary_macros.tex",
    "timing_table.tex",
    "parity_table.tex",
    "phase_table.tex",
    "speedup_plot.tex",
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--analysis", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = Path(__file__).with_name("report") / "report.tex"
    for name in REQUIRED:
        if not (args.analysis / name).is_file():
            raise RuntimeError(f"missing validated analyzer artifact: {name}")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="fevc-alpha-report-") as temporary:
        root = Path(temporary)
        shutil.copy2(source, root / "report.tex")
        for name in REQUIRED:
            shutil.copy2(args.analysis / name, root / name)
        completed = subprocess.run(
            [
                "latexmk",
                "-pdf",
                "-interaction=nonstopmode",
                "-halt-on-error",
                "report.tex",
            ],
            cwd=root,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        (args.output.parent / f"{args.output.stem}.build.log").write_text(
            completed.stdout, encoding="utf-8"
        )
        if completed.returncode != 0 or not (root / "report.pdf").is_file():
            raise RuntimeError("benchmark report compilation failed")
        shutil.copy2(root / "report.pdf", args.output)
    print(f"VCKSS_ALPHA_REPORT_PASS {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
