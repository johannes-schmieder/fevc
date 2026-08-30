# Exact-source local forced-CMG receipts

This compact receipt binds the focused forced-CMG scaling probes to VCkss
source commit `33ede864111c319185949ede4ef6d2bcc44b1383`.  It is local macOS
arm64 evidence, not a maintained-MATLAB comparison, release-binary receipt, or
cross-platform qualification.

The run used Stata/MP 19 with four processors, `memory_gib(14)`, explicit Rust
generic JLA, Counter-V1, `preconditioner(cmg)`, batch 16, seed 20260830, and
the registered probe counts.  The harness is the committed
`benchmarks/projection_scaling/stata_run.do` with only the two changes in
`harness.patch`: request CMG and require the selected CMG receipt.  The
unchanged complete-residual, PSD, memory, data/RNG restoration, and output
schema assertions all remained active.

Identity:

- production source: `33ede864111c319185949ede4ef6d2bcc44b1383`;
- `vckss.ado` SHA-256:
  `30c794b559890a2542bc57dd4181660a10750ebb91678b4aeacea40d34e65fdd`;
- exercised macOS arm64 plugin SHA-256:
  `ad964bde1bcbe14edf505646487df2b78e5a139cdb8d8b14c971d63eaa666425`;
- 6,000-row input SHA-256:
  `cac9827f9a8c93d68d4eba30f96d0d3041a0067c8d8b756c2fe0d49391e0e17c`;
- 24,000-row input SHA-256:
  `884c92da1c9309a376b95dd6d144b170332bccdc719dde37077ae0b92b527f1d`;
- 96,000-row input SHA-256:
  `cc26688290805c1fb299f39231d318a28ec4b8be81e7223c5c8d7d79b8a2385f`.

The native artifact is carried forward through a bounded compatibility review.
No Rust crate, C shim, ABI header, generated CMG source, plugin loader, or
native result poster changed between accepted source `96e7a66` and
`33ede86`; the implementation change is the Stata capability predicate that
admits the existing forced generic-CMG composition.  Pinned Rust formatting,
strict Clippy, workspace/all-target tests, generated-CMG checks, C shim/ABI
tests, and focused Stata native tests were rerun.  The binary hash above is
recorded because this local receipt does not claim a new release build.

All three rows in `results.csv` passed.  Their coefficients, full KSS and naive
covariance matrices, residuals, memory forecasts, PSD cleanup, and iteration
counts are byte-for-byte identical to the pre-commit candidate probes.  The
96,000-row result was repeated once more after source freeze and was identical
again; only elapsed time changed.  `resources.csv` records complete-command
wall time and process peak RSS from `/usr/bin/time -l`.
