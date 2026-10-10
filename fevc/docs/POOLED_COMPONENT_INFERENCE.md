# Pooled mover–stayer component inference

The October 9 development implementation supports pooled match component
inference in scalable Rust/JLA and exact Mata. Select a structured variance
model explicitly. Omitting `inferencemodel()` retains the existing exact
observation-deletion method. Native distribution and statistical assessment
status are recorded in [the current checkpoint](../PLAN.md). The
[bounded assessment](POOLED_COMPONENT_ASSESSMENT.md) reports calibration and
numerical sensitivity; an older shipped
plugin does not acquire this capability from an ado update.

```stata
fevc y controls, worker(worker) firm(firm) deletion(match) ///
    nuisance(fixedoffset) stayers(both) backend(mata) algorithm(exact) ///
    inference(highrank) inferencemodel(structured_common)

fevc y controls, worker(worker) firm(firm) deletion(match) ///
    nuisance(fixedoffset) stayers(both) backend(rust) rng(counter_v1) ///
    algorithm(jla) engine(generic) preconditioner(diagonal) ///
    inference(q1) inferencemodel(structured_common)
```

`stayers(both)` is the default; `stayers(movers)` restricts the population.
Both routes accept `structured_leverage` as a separate sensitivity model,
`deletionid()`, target weights and literal positive integer frequency weights.
Rust also accepts explicit CMG. The exact Mata route is intended for designs
that fit its existing exact-size and memory limits; there is no Mata JLA
attachment. Structured component inference cannot be combined with `project()`.

## Independent units and approximations

A mover has multiple **original deletion units**, including distinct units at
the same worker–firm coordinate. A stayer has one original unit. Existing
sample selection is unchanged: eligible original stayers have at least two
physical observations at a retained mover firm; dropped movers are never
reclassified. Independence is assumed across mover units and across every
physical stayer observation, including repeated observations of one worker.
Dependence within a mover unit is unrestricted. Unequal variances are allowed,
subject to the selected structured variance model.

The full regression first estimates controls. Its fitted control index is
then held fixed. On the resulting FE-only design, mover unit `g` becomes one
row with outcome `sqrt(F_g) * mean_g(y - controls_index)`. Each physical stayer
observation remains one row. Frequency copies of stayer rows therefore count
as separate independent units. Exact Mata is checked against literal expansion.
Rust retains its existing finite-probe address convention: frequency-coded and
literally expanded representations are each checked against their own dense
probe oracle, rather than promised bitwise equality for the same seed. The fit and all four target matrices describe
the combined population; two separate decompositions are not averaged.

Mean centering is computed over the retained physical weighted working outcome
**before** this transformation. Inference freezes that observed mean. Gaussian
error draws are uncentered. Both mean and control estimation uncertainty are
omitted, which can matter even with few controls. None is also supported;
Corrected component inference is rejected before RNG. These approximations
do not imply validity conditional on the estimated controls or observed mean.

## Common variance fit

Write the transformed FE design as `X`, its inverse information as `A`,
`P = X A X'`, and residuals as `e`. The two unit types have separate variance
coefficients in one system:

```
G gamma = Z' e^2,       G = Z' [(I-P) ◦ (I-P)] Z.
```

The common model has 21 mover candidate terms (intercept, within-type leverage,
three primitive target diagonals, regression mass, squares and interactions)
and 15 stayer terms (the same without mass). The leverage model has three
terms per type. A global intercept plus a stayer intercept contrast is merely
a representation of two separate intercepts. Cross-type terms in `G` remain:
the combined fit couples the residuals even when the original errors are
independent. Fitting two unrelated residual regressions would omit those terms.

Features use normalized within-type midranks. For the new pooled models,
leverage and target-diagonal features are first rounded to the binary grid
`2^(floor(log2(max(abs(feature)))) - 40)`, with half ties away from zero.
The zero feature stays zero. This outcome-free rule preserves design ties
across roundoff from equivalent solver reductions and literal expansion;
it changes neither the target, leverage, deletion denominator nor rank gate.
Firm and worker–firm covariance diagonal features are structural zeros for
fixed-offset stayers: their outcome shocks are absorbed entirely by worker
effects. These two features are explicitly zeroed before ranking, preventing
solver roundoff from creating regressors. Regression mass ranks are exact. The existing native single-type models keep
their existing feature convention. Exact Mata uses the new convention on its
explicit structured match route.

Outcome-free redundant columns are removed within the specified span. Each
type must supply at least five units per active term; rank and residual checks
must pass. There is no ridge, automatic pooling, selection using outcomes or
fallback to the other model. The existing positive floor, proportional to the
median HC2 scale, is applied only to fitted variances; the raw leave-out
products still define the point and q1 leading-square correction. All-floor
fits fail. Positivity, conditioning and support diagnostics remain necessary.

Rust estimates `G` with its direct residual covariance probes (default 2,048)
and uses matrix-free influence, covariance and spectral operations. Exact
Mata forms the same Gram through coefficient contractions:

```
G_ab = sum_i (1-2h_i) Z_ia Z_ib
       + trace(A X' diag(Z_a) X A X' diag(Z_b) X).
```

Neither implementation forms an observation-square matrix. Exact Mata still
simulates covariance traces using common Gaussian draws; “exact” describes
the design solves and Gram, not simulation-free standard errors.

## Intervals, diagnostics and native compatibility

All targets use the same fitted variance vector. Individual target availability
is separate from joint covariance admissibility. Highrank posts `e(V)` only
when the joint covariance and all individual checks pass. Structured q1 never
posts a substitute Gaussian `e(V)` or falls back to highrank intervals.
One-mode q1 also needs a diffuse remainder; numerical availability alone
does not verify that condition. Covariance targets can have multiple modes.

`e(inference_mover_units)`, `e(inference_stayer_units)` and
`e(inference_independent_units)` identify the partition. Match mass diagnostics
refer only to original mover units. `e(inference_nuisance_omitted)` is one;
`e(inference_mean_omitted)` identifies the additional Mean approximation.
Exact structured Mata reports `e(component_raw_covariance)`,
`e(inference_joint_available)`, `e(inference_joint_posted)`,
`e(residual_moment_diagnostics)` and `e(component_spectrum)`.
Its Gram method is `exact_coefficient_contraction`, with zero Gram probes.

Native pooled requests require `r(component_mixed_api)==1`, backed by readiness
bit 17 in both the core and transport. Unit receipt V2 is 80 bytes: the original
64-byte field prefix with schema 2, followed by mover and physical-stayer
counts. V1 retains its layout and rejects mixed results. The frontend reconciles
all three counts and mixed design ordering before posting; older plugins fail
at capability preflight before preparation or RNG. Preparation must attach
stayers before component inference.

The [assessment registration](pooled_component_inference_validation_v1.json)
separates primary cells, stress cases, engineering failures and descriptive
scientific screens. Previous mover-only or observation coverage evidence is
not pooled coverage evidence. Severe variance-model misspecification, estimated
offsets, short histories and concentrated modes can invalidate the reported
uncertainty even when computation succeeds.
