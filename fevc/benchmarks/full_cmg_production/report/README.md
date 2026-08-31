# VCkss full-CMG alpha benchmark report

`VCkss_Full_CMG_Alpha_Benchmark.pdf` is the five-page source-bound report for
runtime source `4b6874ededa1244bca389e4ee82148b42b9693a1`, macOS/supply-chain
receipt tip `4dafec6734af4b8d3c25785f268f19f69f780684`, SCC Linux qualifier
source `992eba0947ca155c534f532750fc202e41ecf978`, and vendored CMG source
`dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`.

The report compares the ordinary `CMG_FULL_V2` runtime with maintained MATLAB
KSS on the registered macOS 8,192-firm and SCC fixed-CZ18 hard cases. It
documents exact commands, hardware/software identities, complete-command and
phase timing, process-tree and admitted memory, solver work, residuals,
corrected-target comparisons, cancellation/lifecycle behavior, acceptance
gates, and limitations.

Rebuild from this directory with:

```bash
./compile_report.sh
```

The build requires pdfLaTeX with TikZ/PGFPlots. `report.tex` contains the
layout and prose; `data/results.tex` contains the exact receipt-derived values.
The maintained PDF was compiled twice, rendered to five PNG pages with
Poppler, and visually inspected page by page. Generated LaTeX auxiliaries and
rendered QA PNGs are not tracked.
