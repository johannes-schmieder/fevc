#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: docs/teaching/build.sh /absolute/path/to/kss_example_1999_2001.csv" >&2
  exit 2
fi

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
veneto_data="$1"
expected_hash="93e57a413a8cfccdcb043c5d793105a67b2dc9ebd27d5d3a4f1800abf89a2241"
rust_bin_dir="${CMG_RUST_BIN_DIR:-}"
python_bin="${CMG_TEACHING_PYTHON:-python3}"

if [[ -z "$rust_bin_dir" ]]; then
  if command -v cargo >/dev/null 2>&1; then
    rust_bin_dir="$(dirname "$(command -v cargo)")"
  elif [[ -x "$HOME/.rustup/toolchains/1.85.0-aarch64-apple-darwin/bin/cargo" ]]; then
    rust_bin_dir="$HOME/.rustup/toolchains/1.85.0-aarch64-apple-darwin/bin"
  else
    echo "cargo not found; set CMG_RUST_BIN_DIR to a Rust toolchain bin directory" >&2
    exit 1
  fi
fi

if [[ ! -f "$veneto_data" ]]; then
  echo "Veneto teaching extract not found: $veneto_data" >&2
  exit 1
fi
if command -v shasum >/dev/null 2>&1; then
  actual_hash="$(shasum -a 256 "$veneto_data" | awk '{print $1}')"
else
  actual_hash="$(sha256sum "$veneto_data" | awk '{print $1}')"
fi
if [[ "$actual_hash" != "$expected_hash" ]]; then
  echo "unexpected Veneto teaching-extract SHA-256: $actual_hash" >&2
  exit 1
fi
if [[ ! -x "$rust_bin_dir/cargo" ]]; then
  echo "cargo not found in CMG_RUST_BIN_DIR: $rust_bin_dir" >&2
  exit 1
fi
"$python_bin" -c 'import numpy, PIL' >/dev/null

mkdir -p "$script_dir/generated" "$script_dir/figures" "$script_dir/build" "$repo_root/output/pdf" "$repo_root/output/html"

PATH="$rust_bin_dir:/usr/bin:/bin" "$rust_bin_dir/cargo" run \
  --release \
  --manifest-path "$repo_root/benchmarks/Cargo.toml" \
  --bin teaching-supplement -- \
  --veneto "$veneto_data" \
  --output "$script_dir/generated"

"$python_bin" "$script_dir/generate_figures.py" \
  --data "$script_dir/generated" \
  --figures "$script_dir/figures" \
  --generated "$script_dir/generated"

(
  cd "$script_dir"
  latexmk -pdf -interaction=nonstopmode -halt-on-error -file-line-error \
    -outdir=build cmg-teaching-supplement.tex
)
"$python_bin" "$script_dir/check_latex_log.py" "$script_dir/build/cmg-teaching-supplement.log"
cp "$script_dir/build/cmg-teaching-supplement.pdf" "$repo_root/output/pdf/cmg-teaching-supplement.pdf"

(
  cd "$script_dir"
  latexpand --empty-comments cmg-teaching-supplement.tex > build/cmg-teaching-supplement-expanded.tex
  pandoc build/cmg-teaching-supplement-expanded.tex \
    --from=latex \
    --to=html5 \
    --standalone \
    --toc \
    --number-sections \
    --citeproc \
    --bibliography=references.bib \
    --mathml \
    --embed-resources \
    --css=teaching.css \
    --output="$repo_root/output/html/cmg-teaching-supplement.html"
)

echo "PDF:  $repo_root/output/pdf/cmg-teaching-supplement.pdf"
echo "HTML: $repo_root/output/html/cmg-teaching-supplement.html"
