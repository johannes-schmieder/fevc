#!/usr/bin/env bash
set -euo pipefail
root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
cd "${root}"
output=VCkss_Full_CMG_Alpha_Benchmark.pdf
pdflatex -interaction=nonstopmode -halt-on-error report.tex
pdflatex -interaction=nonstopmode -halt-on-error report.tex
mv -f report.pdf "${output}"
pdfinfo "${output}" | grep -E '^(Pages|Page size|File size):'
