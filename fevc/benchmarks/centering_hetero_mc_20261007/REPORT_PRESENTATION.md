# Academic presentation of the centering Monte Carlo

The owner requested a PDF in the style of an existing academic Monte Carlo
report, with estimator-performance tables and component-density panels.
`render_report.py` and `report_figures.py` provide a separate presentation of
the already accepted October 7 evidence. They do not modify the original
report, accepted receipts, source manifest, raw outcomes, or summary tables.
No additional estimator runs are needed.

The presentation leads with the small size of Mean's bias relative to sampling
dispersion, followed by paired Mean-minus-Corrected contrasts. This makes the
adjustment visible without artificially separating nearly identical marginal
distributions. Exact-oracle RMSE differences also show that the extra correction
does not improve RMSE in this particular interaction design.

Each profile retains all four targets and all seven plug-in/exact/JLA estimator
variants in its table. Density panels show exact estimates and JLA(256) errors,
with common within-panel Gaussian bandwidths, all 2,000 outcome draws, truth
lines, and estimator means. The dedicated contrast figure labels its magnified
units. Numerical MCSE calibration is separate from sampling uncertainty.

From the repository root, with `reportlab` available to the repository Python:

```bash
./.venv/bin/python fevc/benchmarks/centering_hetero_mc_20261007/render_report.py \
  .local/centering-hetero-mc-20261007/scc \
  --output output/pdf/fevc_centering_mc.pdf
```

When using the desktop's bundled pure-Python PDF package, set
`FEVC_REPORT_PYTHONPATH` to its Python `site-packages` directory. The repository
environment continues to supply NumPy, SciPy, pandas, matplotlib, and Pillow.
The output has an adjacent presentation receipt, figures, and figure metadata.
The builder verifies source/input and accepted artifact hashes before and after
rendering, and records new builder and PDF hashes separately. Rendered-page
inspection provides the additional visual QA record.

The final PDF is a descriptive presentation, not a newly registered scientific
campaign or a release-qualification receipt. A future stress design should be
selected using oracle bias-to-SD and RMSE diagnostics, then frozen and tested
using fresh outcome draws; variance dispersion alone does not establish that
Mean and Corrected should differ substantially.
