# VCkss 0.4.0-alpha.1 full-CMG macOS gate

The ordinary-build `CMG_FULL_V2` route at source
`4b6874ededa1244bca389e4ee82148b42b9693a1` passes the registered
8,192-firm/200-probe macOS promotion matrix. This source is the private
`0.4.0-alpha.1` package candidate. The comparison used Stata/MP 19, MATLAB
R2024b Update 5 with Parallel Computing Toolbox, four application workers,
Rust 1.85.1, vendored CMG
`dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`, one cold repetition, and five
position-balanced warm repetitions.

| Warm-median measure | VCkss | MATLAB | VCkss / MATLAB |
| --- | ---: | ---: | ---: |
| Complete estimator command | 74.774 s | 104.489 s | 0.716 |
| Process-tree peak RSS | 5.121 GB | 12.161 GB | 0.421 |

The selected alpha gate passes. VCkss is 1.40 times as fast as matched MATLAB
and its command median is 0.921 times the source-bound private winner's
81.145-second checkpoint. The longer-run 2x MATLAB objective does not pass on
this matrix.

## VCkss timing and solver receipt

| Warm-median production phase | Seconds |
| --- | ---: |
| Native ingest | 0.032 |
| Canonicalization | 0.573 |
| Graph | 0.447 |
| Compression | 0.522 |
| Plan | 0.152 |
| Native solve total | 71.841 |
| Full-CMG graph | 0.428 |
| Full-CMG hierarchy | 0.133 |
| RHS construction | 1.850 |
| Repeated solve | 59.826 |
| Extraction, recovery, and certification | 4.523 |

All 601 RHSs completed with warm-median totals of 4,817 PCG iterations, 5,418
operator applications, and 4,817 CMG applications. The maximum reduced and
complete original-system residuals were `7.862e-6` and `6.971e-6`,
respectively, against the default probe gate of `1e-5`; no refinement was
needed. Data, caller RNG, sort RNG, `e(sample)`, target identity, and lifecycle
checks passed on every run.

The conservative pre-RNG forecast was 32.828 GB, the admitted full-CMG peak
was 19.500 GB, actual retained native storage was 1.184 GB, and observed
process RSS had a 5.121 GB warm median. MATLAB's complete five-process tree had
a 12.161 GB warm median.

## Statistical comparison

The maintained MATLAB program uses a different RNG and solver contract and
does not emit a compatible numerical MCSE. Its comparison is therefore a
descriptive statistical check, not a common-probe numerical-equality gate.
The corrected VCkss-minus-MATLAB absolute differences were `6.19e-6`
(worker), `1.44e-5` (firm), `1.11e-5` (covariance), and `1.64e-6` (total).
Against the frozen common-Counter-V1 private winner, all four corrected VCkss
targets are identical and pass the registered MCSE-aware gate.

## Qualification and reproduction

The same source passed the ordinary arm64, x86_64, and universal plugin
qualifier, including Rosetta, clean-install, ABI, lifecycle, cancellation,
explicit `backend(rust)`, and eligible `backend(auto)` routing. The benchmark
command was:

```bash
/opt/anaconda3/bin/python3 vckss/benchmarks/full_cmg_production/run_local.py \
  --output-dir /private/tmp/vckss-full-cmg-alpha-4b6874e \
  --input-csv /private/tmp/vckss-local-matrix-787327f/run/input/input.csv \
  --matlab-root /Users/johannes/Git/varcomp_hdfe/Monte_Carlo/LeaveOutTwoWay
```

The byte-preserved case, build, MATLAB-source, qualification, and complete
alternating-run receipts are under
[`evidence/macos/4b6874ededa1244bca389e4ee82148b42b9693a1`](evidence/macos/4b6874ededa1244bca389e4ee82148b42b9693a1).
