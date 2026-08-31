#!/bin/bash
set -euo pipefail
if test "$#" != 2; then
  printf 'usage: compile_report.sh COLLECTION_DIR OUTPUT_DIR\n' >&2
  exit 198
fi
collection=${1%/}
output=${2%/}
script_dir=$(cd "$(dirname "$0")" && pwd)
python_bin=${VCKSS_REPORT_PYTHON:-python3}
mpl_config_dir=${VCKSS_MPLCONFIGDIR:-${TMPDIR:-/tmp}/fevc-matplotlib}
mkdir -p "$mpl_config_dir"
test -d "$collection" && test ! -e "$output"
MPLBACKEND=Agg MPLCONFIGDIR="$mpl_config_dir" \
  "$python_bin" "$script_dir/analyze_report.py" --collection-dir "$collection" \
  --output-dir "$output" --template "$script_dir/report.tex"
(cd "$output" && pdflatex -interaction=nonstopmode -halt-on-error report.tex \
  > pdflatex.1.log && pdflatex -interaction=nonstopmode -halt-on-error report.tex \
  > pdflatex.2.log)
mv "$output/report.pdf" "$output/VCkss_Three_Way_Scaling_Benchmark.pdf"
(cd "$output" && sha256sum VCkss_Three_Way_Scaling_Benchmark.pdf \
  VCkss_Three_Way_Scaling_Benchmark.md figures/*.pdf tables/*.tsv \
  > SHA256SUMS)
printf 'FEVC COMPARATIVE SCALING REPORT BUILD PASS: %s\n' \
  "$output/VCkss_Three_Way_Scaling_Benchmark.pdf"
