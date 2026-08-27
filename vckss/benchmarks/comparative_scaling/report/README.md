# Comparative-scaling report build

The report consumes only a validated compact collection. It refuses a
collection that does not contain exactly 300 tasks and 900 estimator calls.
The analysis produces colorblind-safe vector PDF figures, LaTeX tables,
machine-readable applied-guidance TSVs, a source-bound Markdown report, and the
standalone benchmark PDF.

Use a Python environment with pandas, NumPy, and Matplotlib plus a TeX
installation with `pdflatex`:

```bash
VCKSS_REPORT_PYTHON=/path/to/python \
  vckss/benchmarks/comparative_scaling/report/compile_report.sh \
  /path/to/compact-collection /path/to/new-report-output
```

The final PDF must be rendered page by page to PNG and every page visually
inspected. Generated output is committed only with its exact SCC collection
receipt and hashes; this directory contains the reproducible source template,
not placeholder benchmark claims.
