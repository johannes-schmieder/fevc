# Veneto conditioning diagnosis, 2026-09-08

Status: **DIAGNOSIS_COMPLETE_NOT_REPAIRED**. The owner approved a local,
diagnosis-only replay of the existing Veneto input. No estimator, threshold,
production binary, paper claim, or acceptance decision was changed.

## Finding

Both q0 and q1 reproduce rc=498. The reduced predictor basis passes its rank
check; the later estimated residual-moment Gram matrix has one materially
negative eigenvalue. The captured basis, direct term, and estimated Gram
matrices are exactly identical between q0 and q1. This is a shared variance-fit
failure before interval-specific calculations, not evidence against one of
the two interval constructions separately.

The original 21 candidate terms reduce to 15 with zero measured span error.
The fitting system has 12,828 rows. For each matrix below, eigenvalues refer
to diagonal scaling to unit diagonal, as in the conditioning check.

| Diagnostic | Predictor matrix | Estimated moment matrix |
| --- | ---: | ---: |
| Smallest eigenvalue | 0.0006133402 | -0.0142960534 |
| Largest eigenvalue | 8.7731116 | 9.9643645 |
| Smallest/largest | 0.0000699114 | -0.0014347180 |
| Negative eigenvalues | 0 | 1 |
| Eigenpair residual norm | 2.42e-15 | 5.17e-15 |

The unchanged rank tolerance is 1e-10. The first check passes; the second
fails. The negative eigenvalue is not a marginal positive value rejected by
a strict tolerance, nor an eigensolver rounding artifact. All 512 projection
actions pass their existing checks; the maximum complete-system residual is
5.573e-11.

In the scaled negative-eigenvector direction, the direct term
`Z' diag(1 - 2 hhat) Z` contributes -0.3401030515, while half the probe sample
covariance contributes +0.3258069981. Their sum is -0.0142960534. The probe
covariance contribution is itself positive semidefinite (its unscaled minimum
eigenvalue is 0.5627362). Thus the subtractive approximation loses positive
semidefiniteness in this direction.

This does **not** separately identify the effects of approximate leverage,
finite Gram probes, and potentially weak information in the exact moment
matrix. Full column rank of the predictor basis does not establish positive
definiteness of the exact moment matrix. No exact-matrix comparison, alternative
probe estimator, or coverage experiment was performed.

## Evidence and reproduction

Local evidence root:
`.local/diagnostics/veneto-conditioning-20260908/`.
Its `diagnosis.json` SHA-256 is
`12fb4a46682d6a6fa35235cad8386bdfd5071e1a16bc54ea27d1073717b3532e`.
The receipt records source, binary, input, harness, capture and analyzer hashes,
the full spectra, and the directional decomposition. Both native captures have
SHA-256 `f74d61b9c93fda5e7086eca212f408e1f2db583e30448c922b1355e7d52763c8`.

Input: the existing paper `veneto_match_rc_20260905_v2/veneto.csv`, 71,614
imported rows, SHA-256
`f9cbbc16626bfc2848fcca945cf5d2c869c0354252f00e9000f66042d9aa2c36`.
Original failure manifest: paper
`unified_veneto_20260907_v1/manifest.json`, SHA-256
`b068484bb38a51f10df21346f5b1e90969dab5b0f72609253dd8eb47b9f17509`.

The isolated source copy differs only by diagnostic `eprintln!` lines in
`residual_moments.rs` and `residual_moments/basis.rs`. Removing those lines
reconstructs the original files byte for byte. The analyzer also verifies
production source/binary and frozen input/harness identities against the
original manifest. The production package was not rebuilt or replaced.

The diagnostic plugin was built with pinned Rust 1.85.1 using
`cargo build --release --locked --offline`, the isolated
`source/rust/stata_backend/Cargo.toml`, and an isolated target directory. It
was copied and ad-hoc signed only inside the diagnostic package. Each run used
the unchanged paper `run_unified_match_scaling.do`, with positional arguments
`<diagnostic-source> <existing-veneto.csv> <q0-or-q1-output> <q0-or-q1> 1000`.
Execution used the local Stata-MP executable, version 19, MP flag 1, eight
processors and `RAYON_NUM_THREADS=8`. The captured flavor string is `IC`;
the receipt retains it without alteration. Settings remain 200 JLA probes,
512 Gram probes, seed 8675309, fixed-offset match deletion, movers, and the
existing solver and admission thresholds. Diagnostic runtimes were 7.518
and 7.199 seconds; these are not performance benchmarks.

Analysis command (passed):

```bash
./.venv/bin/python .local/diagnostics/veneto-conditioning-20260908/analyze.py
```

The analyzer creates a new receipt exclusively and deliberately refuses to
overwrite it. This is a diagnostic replay, not fresh software qualification;
no broad platform or coverage gates were rerun.

## Proposed next decision

Test the already reviewed direct residual-probe representation on this same
input in an isolated candidate, retaining the model, probe count and admission
checks. See [the mathematical review](UNIFIED_GRAM_DIAGNOSIS_2026-09-07.md).
It targets the same exact moment matrix while expressing the estimate as a
sample covariance, avoiding the subtraction above. Positive semidefiniteness
alone does not ensure adequate rank, stable fitted variances, or coverage.
Require algebraic/oracle checks and inspect fit/interval diagnostics before
considering integration. Do not clip eigenvalues, add ridge, weaken thresholds,
or promote this diagnostic as a repair. Implementation requires a new owner
decision.
