#!/usr/bin/env python3
"""Fail on LaTeX diagnostics that would make the teaching PDF unreliable."""

from __future__ import annotations

import re
import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: check_latex_log.py PATH", file=sys.stderr)
        return 2
    text = Path(sys.argv[1]).read_text(errors="replace")
    failures: list[str] = []
    patterns = {
        "undefined references": r"LaTeX Warning: There were undefined references",
        "undefined citations": r"Package natbib Warning: Citation .* undefined",
        "rerun request": r"Rerun to get (cross-references|citations) right",
        "multiply defined labels": r"multiply defined",
        "missing glyphs": r"Missing character:",
    }
    for label, pattern in patterns.items():
        if re.search(pattern, text, flags=re.IGNORECASE):
            failures.append(label)
    overfull = []
    for match in re.finditer(r"Overfull \\hbox \(([-0-9.]+)pt too wide\)", text):
        if float(match.group(1)) > 5.0:
            overfull.append(float(match.group(1)))
    if overfull:
        failures.append(f"overfull boxes above 5pt: {max(overfull):.2f}pt")
    if failures:
        print("LaTeX quality check failed: " + "; ".join(failures), file=sys.stderr)
        return 1
    print("LaTeX quality check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
