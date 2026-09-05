# Fixed-offset uncertainty diagnosis — 2026-09-04

## Finding and scope

Estimating only two controls on the same observations can materially change
component uncertainty after match collapse. In the original 400-match
fixture, the exact standard deviation of the total component is 0.007804
with estimated controls, versus 0.004656 with the true control offset: a
factor of **1.676**. The development simulation's empirical SD was 0.007890,
while its mean reported SE was 0.004580. These are close to the estimated-
and known-offset population quantities, respectively.

This strongly supports omitted nuisance-estimation covariance as the main
explanation of the observed SE shortfall. It does not by itself attribute
every coverage error: the diagnostic computes Gaussian population moments,
not random fitted-variance estimates or q1 confidence sets. The historical
76.25% coverage remains a failed calibration diagnostic, not a new method's
coverage result.

The subsequently audited independent confirmation reproduces the same gap:
total empirical SD 0.007704, mean SE 0.004606, ratio 1.6728, and coverage
77.551% among 2,499 computed intervals. This separate evidence is reported in
[`INFERENCE_REPAIR_MATCH_CONFIRMATION_2026-09-04.md`](INFERENCE_REPAIR_MATCH_CONFIRMATION_2026-09-04.md).
It was not used to select or tune the five diagnostic designs.

The calculation follows the prospectively committed
[`fixed_offset_diagnostic_v1.json`](fixed_offset_diagnostic_v1.json), source
`506c170e7621ccc0b20f510d634c706f13cfcc7d`. All five designs and all four
targets were retained. The full manifest, hashes, runtime receipt, 20 rows,
and independent production replay check are in
[`fixed_offset_diagnostic_v1_result.json`](fixed_offset_diagnostic_v1_result.json).
No estimator, production variance model, interval calculation, public route,
or second-stage nuisance correction was changed.

## What the oracle isolates

Let `Z` contain the physical-row FE indicators, `C` the two controls, `W`
frequency weights, and `Sigma` the known block-diagonal physical-row error
covariance. Every match has its registered serial AR(1) covariance, scaled
so that the frequency-weighted scalar aggregate has variance `tau_g`.
The collapse matrix has entries `S[g,i] = frequency_i / sqrt(F_g)` inside
match `g`. Hence the known-offset error covariance is exactly
`D = S Sigma S' = diag(tau)`.

The full-sample weighted control estimate satisfies
`gamma_hat - gamma = H_gamma u`, where
`H_gamma = (C' W M_Z C)^(-1) C' W M_Z` and `M_Z` removes the weighted FE fit.
After subtracting the fitted control offset, the collapsed error is
`S (I - C H_gamma) u`. Its actual covariance is therefore

```
Omega = S (I - C H_gamma) Sigma (I - C H_gamma)' S'
      = D - U A' - A U' + U V_gamma U',
U = S C,  A = S Sigma H_gamma',  V_gamma = H_gamma Sigma H_gamma'.
```

The correction has rank at most four, but is shared across many matches.
Original match-error independence does not imply independence of these
estimated-offset errors. Merely fitting the correct original aggregate
variance `tau_g` does not account for the induced cross-match covariance.

The independent exact collapsed leave-match estimator is `y_star' K y_star`,
with zero-diagonal symmetric kernel `K`. Under Gaussian errors, its bias
relative to the target is `tr(K Omega)` and its variance is
`4 mu' K Omega K mu + 2 tr(K Omega K Omega)`. Under known offsets this bias
is zero because `Omega=D` is diagonal. Estimated offsets introduce a small
bias in these fixtures, but it is not the dominant problem: the total
bias at 400 matches is only about 0.0093 of its actual SD.

We also evaluate the expectation of the exact-trace variance estimator that
still uses `D` while the data have covariance `Omega`:

```
E[Vhat_D] = 4 mu' K D K mu + 4 tr(K D K Omega) - 2 tr(K D K D).
```

For the original total component, actual SD divided by `sqrt(E[Vhat_D])`
is 1.675. Thus even **known original aggregate variances** leave almost the
entire uncertainty gap in this exact-kernel diagnostic. This comparison is
not the simulation ratio `SD/E[SE]`, and neither ratio alone proves coverage.

The q1 remainder is especially affected: its actual estimated-offset
variance is `5.318e-5`, versus expected diagonal-known-tau studentization
`1.282e-5`. The leading variance increases much less, from approximately
0.2702 to 0.2950. A universal scalar adjustment to every reported SE would
not recover the full leading/remainder covariance structure.

## Growing samples with two controls

The historical controls contain linear worker/firm-index loadings. Their
FE components grow with `k` even though the within-FE variation identifying
the two control coefficients stays comparable. A second, prospectively
specified design scales only those FE components by `20/k`. This bounded
version agrees with the historical design at `k=20`. It does not change
the within-FE control residuals or their coefficient-estimation covariance.

| Matches | Control FE loadings | Known-offset total SD | Estimated-offset total SD | SD multiplier |
| ---: | --- | ---: | ---: | ---: |
| 400 | Historical = bounded | 0.004656 | 0.007804 | 1.676 |
| 1,600 | Historical | 0.004514 | 0.013583 | 3.009 |
| 1,600 | Bounded | 0.004514 | 0.007838 | 1.737 |
| 6,400 | Historical | 0.002702 | 0.017143 | 6.344 |
| 6,400 | Bounded | 0.002702 | 0.005037 | 1.864 |

The two coefficient SDs do fall, roughly halving when `k` doubles:
`(0.01082, 0.01134)`, `(0.00529, 0.00573)`, `(0.00260, 0.00285)`.
But coefficient consistency is not enough for its effect to be negligible
relative to a component estimator whose own uncertainty is also shrinking.
With historical loadings the offset's effect worsens sharply. Bounded
loadings improve absolute uncertainty, but do not eliminate the relative
gap in these finite designs. This is evidence against a blanket
"many matches and few controls is sufficient" rule, not an asymptotic theorem
about every design or a claim that fixed-offset inference never works.

The target's original weight multiplier of 1,000 is retained at every size;
it is not rescaled to enforce a spectral regime. Total leading spectral
share falls from 0.9547 at 400 matches to 0.9168 and 0.6855. The larger
designs are descriptive, not newly certified q1 coverage cells.

## Correction to the earlier geometry interpretation

The exact worker, firm, and total remainder concentration at 400 matches is
0.06144, 0.06208, and 0.04283, respectively. The independent 4,096-probe
preflight gives approximately 0.05187, 0.05281, and 0.04092. These support
the intended diffuse-remainder geometry; the covariance target remains
multi-mode, with exact remainder concentration 0.99992.

The earlier development summary reported much larger *mean 128-probe*
remainder diagnostics: 0.3159, 0.3270, and 0.4014. Their respective medians
are 0.05709, 0.06636, and 0.04610. In 102, 103, and 150 of 400 draws the
reported ratio is essentially one. The production diagnostic subtracts a
large leading-mode square from a noisy total trace and reconciles the total
to at least the two certified modes; the small remainder is particularly
sensitive to this operation. The adverse averages are not evidence that
the fixed design's true spectrum changed across outcome draws.

The previous checkpoint's suggestion of unfavorable controls remainder
geometry should therefore be read as a warning about those noisy diagnostics,
not an established independent cause of controls undercoverage. The exact
calculation makes omitted nuisance covariance the clearer explanation.
No diagnostic algorithm or frozen campaign threshold was changed after
viewing results. A direct deflated trace diagnostic is a separate possible
engineering improvement requiring its own independent comparison.

## Verification and reproducibility

- Independent dense physical-row joint WLS, transformed covariance, and
  quadratic moments agree with the collapsed low-rank oracle on small
  fixtures. Tests also cover signed low-rank trace identities, zero known-
  offset bias, unbiased known-tau studentization, invariant within-control
  identification, invalid designs, malformed inventories, source mismatches,
  new-only output, and a complete tiny manifest-to-receipt run.
- Focused diagnostic tests: 13 passed. Final full source suite: 598 passed
  in 54.03 seconds. CMG assembly check passed; no generated or CMG source was
  changed. Pytest emitted only the previously observed cleanup warnings for
  old sandbox-protected temporary symlink fixtures, after all tests passed.
- One original production controls replay (`k=20`, replication 0, 256/512/128
  probes, 256 spectral iterations, 4,000 critical draws) agrees with the
  independently generated inputs. Frequencies, IDs, masses, and known
  aggregate variances agree exactly; maximum control discrepancy is
  `8.88e-16`. Using its exported randomized correction diagonal, the
  independent exact FE fit matches all production point estimates within
  `3.04e-13`; truth discrepancies are below `2.84e-15`.
- The production replay is a deterministic implementation cross-check, not
  part of the independently seeded confirmation or a coverage observation
  for this outcome-free study. Its raw synthetic export hash is retained.
- No large local `G`-by-`G` covariance or kernel is allocated. The largest
  design has 6,400 matches; low-rank traces are validated against a separately
  constructed physical-row dense oracle at small dimensions.

Commands (from repository root):

```bash
./.venv/bin/python -m pytest -q fevc/tests/python/test_fixed_offset_diagnostic.py
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
./.venv/bin/python fevc/tools/diagnose_fixed_offset.py create-manifest /private/tmp/fevc-fixed-offset-506c170-manifest.json
./.venv/bin/python fevc/tools/diagnose_fixed_offset.py run /private/tmp/fevc-fixed-offset-506c170-manifest.json /private/tmp/fevc-fixed-offset-506c170
rust/target/release/examples/inference_repair_match repair-replay-v1 controls_varying_fixedoffset 20 0 1 256 512 128 256 4000
./.venv/bin/python fevc/tools/diagnose_fixed_offset.py verify-export /private/tmp/fevc-fixed-offset-506c170-production.jsonl
```

Manifest and run require their exact clean committed source and new output
paths. The saved result embeds the original manifest; a later source is not
silently substituted. Historical confirmation and native harnesses were not
edited by this diagnostic.

## Next decision

Retain the owner's no-second-stage-correction constraint. The next controls
experiment should pair the same errors across known/estimated offsets and
known/fitted aggregate variance, report every interval failure, and preserve
the exact geometry diagnostics. That can determine how well the existing
q1 reference works when offset uncertainty is genuinely absent, and how much
additional error comes from variance fitting. Do not promise that increasing
the sample or changing the structured variance model fixes the fitted-offset
problem. Public integration must distinguish supported calibrated cases
from a clearly labeled approximation that can substantially understate
uncertainty with estimated controls. Full match q0 and fresh observation q1
confirmation remain separate steps.
