# CMG teaching supplement

This directory contains the source and reproducible numerical exhibits for
*Combinatorial Multigrid, Step by Step*. The supplement introduces graph
Laplacians, CG, PCG, and CMG; traces a complete 12-vertex hierarchy; and uses a
public Veneto worker–firm teaching extract for a real-data hierarchy and
matrix-block example.

The committed CSV files are derived numerical outputs. They contain no source
worker IDs, firm IDs, outcomes, or confidential data. The public input file is
not copied into this repository.

## Build

Prerequisites are Rust 1.85 or newer, TeX Live 2023, Pandoc 3, and Python with
NumPy and Pillow. Then run from the repository root:

```bash
CMG_TEACHING_PYTHON=/path/to/python \
  docs/teaching/build.sh /absolute/path/to/kss_example_1999_2001.csv
```

The input must have SHA-256
`93e57a413a8cfccdcb043c5d793105a67b2dc9ebd27d5d3a4f1800abf89a2241`.
It is distributed as `inst/extdata/test.csv` in CRAN package
[`LeaveOutKSS` 0.1.0](https://CRAN.R-project.org/package=LeaveOutKSS).

Outputs:

- `output/pdf/cmg-teaching-supplement.pdf`
- `output/html/cmg-teaching-supplement.html` (standalone, with embedded figures)

The benchmark-only Rust driver is
`benchmarks/src/bin/teaching-supplement.rs`. It uses the public library's
production graph, hierarchy, preconditioner, and PCG code paths. The 12-node
example lowers the direct threshold only to make coarsening visible; the Veneto
example uses production defaults.

## Privacy and interpretation

The public file is a teaching extract associated with a Veneto worker–firm
application, not the full confidential Veneto Workers History data. The driver
maps identifiers in memory, selects the largest connected component, and emits
only aggregate counts plus freshly relabeled nodes (`W1`, `F1`, and so on).
Dense preconditioner blocks are diagnostic materializations for teaching; the
production solver never stores a dense inverse.
