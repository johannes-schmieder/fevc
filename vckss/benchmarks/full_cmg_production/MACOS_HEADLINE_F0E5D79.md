# Production full-CMG macOS headline

Normal-build VCkss source `f0e5d7967cd7e38c4cbf329460fd94a0403266a0`
passes the registered 8,192-firm/200-probe macOS comparison. The benchmark
used Stata/MP 19, MATLAB R2024b Update 5 with Parallel Computing Toolbox, four
application workers, Rust 1.85.1, vendored CMG
`dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`, one cold repetition, and five
position-balanced warm repetitions.

| Warm-median measure | VCkss | MATLAB | VCkss / MATLAB |
| --- | ---: | ---: | ---: |
| Complete estimator command | 74.930 s | 98.405 s | 0.761 |
| Process-tree peak RSS | 5.162 GB | 12.093 GB | 0.427 |

The selected production gate passes: VCkss is 1.31 times as fast as matched
MATLAB and its 74.930-second median is 0.923 times the source-bound private
winner's 81.145-second single-run checkpoint. The longer-run 2x MATLAB
objective does not pass on this local matrix.

## VCkss timing and solver receipt

| Warm-median production phase | Seconds |
| --- | ---: |
| Native ingest | 0.031 |
| Canonicalization | 1.630 |
| Graph | 0.473 |
| Compression | 0.828 |
| Plan | 0.690 |
| Native solve total | 69.915 |
| Full-CMG graph | 0.393 |
| Full-CMG hierarchy/plan | 0.127 |
| RHS construction | 1.765 |
| Repeated solve | 57.664 |
| Extraction, recovery, and certification | 4.307 |

All 601 RHSs completed with a warm-median 4,817 PCG iterations, 5,418
operator applications, and 4,817 CMG applications. The maximum complete
original-system residual was `6.97e-6` against the default probe gate of
`1e-5`; no refinement was needed. Data, RNG, sort RNG, `e(sample)`, target
identity, and lifecycle checks passed on every run.

The conservative pre-RNG forecast was 32.415 GB, the admitted full-CMG peak
was 19.087 GB, actual retained native storage was 0.840 GB, and observed
process RSS had a 5.162 GB warm median. MATLAB's complete five-process tree
had a 12.093 GB warm median.

## Statistical comparison

The maintained MATLAB program uses a different RNG and solver contract and
does not emit a compatible numerical MCSE. Its comparison is therefore a
descriptive statistical check, not a common-probe numerical-equality gate.
The corrected VCkss-minus-MATLAB absolute differences were `6.19e-6`
(worker), `1.44e-5` (firm), `1.11e-5` (covariance), and `1.64e-6` (total).
VCkss's release-blocking exact-reference and common-probe differential gates
remain separate and unchanged.

The byte-preserved case, build, MATLAB-source, and full run summary receipts
are under
[`evidence/macos/f0e5d7967cd7e38c4cbf329460fd94a0403266a0`](evidence/macos/f0e5d7967cd7e38c4cbf329460fd94a0403266a0).
