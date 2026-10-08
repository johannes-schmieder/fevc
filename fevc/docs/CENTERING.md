# Outcome centering

`centering(none|mean|corrected)` selects the outcome factor in the leave-out
bias correction. The default is `mean` for both exact and JLA. All three
modes work in exact and JLA point estimation in Mata and in a matching Rust
build.

Centering leaves the fitted coefficients, full-sample residuals, plug-in
components, retained sample, target weights and deletion units unchanged.
The `corrected` row of `e(results)` always means the final leave-out point
estimate; it does not imply that `centering(corrected)` was requested.

## The three modes

| Mode | Bias-correction outcome factor | Additional point work |
| --- | --- | --- |
| `none` | Original retained working outcome `u`. | Ordinary estimator. |
| `mean` (default) | `z = u - ubar`. | One weighted mean and subtraction; no additional solves or random directions. |
| `corrected` | The Mean correction plus an adjustment for estimating `ubar`. | Exact: one shared correction system. JLA: correction systems using the existing full leverage pool and its two halves. |

The observed mean is
\[
\bar u=\frac{\sum_i f_i u_i}{\sum_i f_i},
\]
where `f_i` is the positive integer frequency weight on retained row `i`.
It is the mean over literal observation copies. `targetweight()` defines
the variance-component population and does not weight this mean.

Under `nuisance(joint)`, `u` is the retained outcome. Under
`nuisance(fixedoffset)`, first subtract the full fitted nuisance index,
`u = y - Z gammahat`, then calculate its mean. The fixedoffset convention
still holds the fitted nuisance index fixed.

Centering supports the existing point-estimation combinations of controls,
frequency and target weights, match or observation deletion, and
`stayers(movers|both)`. Match deletion with `stayers(both)` retains its hybrid
correction: mover match blocks and separate stayer observation copies.

## Component inference with a fixed observed mean

`inference(highrank|q1)` accepts Mean (the default) and None on every
previously supported inference tuple. Mean uses the observed retained
physical-frequency mean `c = ubar` as a fixed constant in the inference
calculation. This omits uncertainty from estimating the mean; it is a working
approximation, not a claim about the sampling distribution conditional on the
observed mean. Corrected component inference remains unsupported.

The exact Mata route uses `z = u - c` in its target-specific variance proxy,
influence vectors and q1 leading/remainder terms. It still fits primitive and
polarized targets separately. The native structured route uses the centered
outcome in influence and q1 terms; its residual-moment variance fit is
unchanged because a constant shift leaves regression residuals unchanged.
For fixed-offset match inference, collapse gives
`z_g = sqrt(F_g) (ubar_g - c)`, using the physical-frequency mean before
square-root-mass transformation. Frequency mass does not create independent
matches. The separate omission of estimated nuisance-offset uncertainty
remains.

For the exact unit-observation kernel `C`, a constant-shift-invariant target
has fixed-c point estimate `z'Cz` and influence `Cz`. The covariance trace
continues to use the original kernel `C`. Inference Gaussian error probes
are never shifted by `c` or by their own realized sample means. q1 uses the
raw product `sum_i v_i^2 z_i ehat_i,-i` to recenter the leading square and
checks its exact rank-one-subtracted remainder identity. A positive fitted
variance is not substituted for that raw product.

Replacing a population mean `c0` by the observed mean changes the estimator
by `(ubar-c0) sum_i B_ii ehat_i,-i` in the unit-observation exact case.
The omitted term must be negligible relative to the relevant component and
q1 remainder sampling scales for the approximation to preserve inference.
High leverage, concentrated frequency mass, weak signal and a small number
of independent matches can matter. A large row count or a small observed
point difference alone does not establish this condition. Mean inference
neither corrects estimated-mean bias nor guarantees conservative intervals.

The existing deletion, nuisance, weighting, backend and q-specific
identification restrictions still apply. In particular, observation component
inference remains unit-frequency; the supported native match tuple remains
mover-only with fixed offsets. Existing combined exact-Mata requests support
Mean; native combined component/projection requests remain unsupported.
See [the inference guide](INFERENCE.md) for supported requests and
[the implementation contract](MATRIX_FREE_COMPONENT_INFERENCE.md) for algebra.
Historical None coverage and native receipts do not qualify this extension;
the [accepted implementation plan](MEAN_COMPONENT_INFERENCE_PLAN.md) requires
fresh bounded sampling assessment and exact-source platform qualification.

## Projection and the mean convention

`project()` supports Mean (the default) and None. Mean replaces `u_g` by
`z_g = u_g - ubar 1_g` only in the symmetrized block covariance proxy,
\[
\widehat\Sigma_g^{\mathrm{mean}}
=\tfrac12\{z_g\widehat e_{g,-g}' + \widehat e_{g,-g}z_g'\}.
\]
The fitted projection coefficients, leave-out residuals, score loadings and
residual-squared naive covariance are unchanged. Corrected projection is not
implemented. Existing combined exact-Mata `project()` and component
`inference()` requests accept Mean or None, using the same observed mean.
Native combined component/projection requests remain unsupported for either
centering mode.

The mean uses Stata's frequency-weight convention on the retained working
outcome: `sum(f_i*u_i)/sum(f_i)`, as in `summarize u if e(sample) [fw=f]`.
The unweighted case is the ordinary observation mean. Neither `targetweight()`
nor `projectweight()` changes this centering mean; those options retain their
separate population/loading roles. With `nuisance(fixedoffset)`, summarize the
working outcome after subtracting the fitted nuisance index, not raw `y`.
See Stata's [summarize methods and formulas](https://www.stata.com/manuals/rsummarize.pdf)
for the frequency-weighted mean.

This differs from the main mover-match proxy in the KSS Matlab package. At
pinned commit `8b957ffe`, its main interface first forms
`v_g = sqrt(n_g) * ubar_g`, then subtracts the unweighted mean of those
transformed match values, `a = sum_g(v_g)/G`. FEVC instead uses the transformed
factor `sqrt(n_g) * (ubar_g - ubar)`, with `ubar` calculated over the retained
physical observations. On the same mover-only sample these agree when match
sizes are equal, but generally differ when sizes vary. The KSS main interface
uses a separate person-year proxy for eligible stayers; the mover formula is
not a description of every retained unit. See the pinned
[match transformation and centering](https://github.com/rsaggio87/LeaveOutTwoWay/blob/8b957ffeb10b8465a3584fceb0265cccc48379e1/codes/leave_out_KSS.m#L489-L542).

FEVC's convention is invariant to adding a common constant to the working
outcome: both `u_i` and `ubar` shift by that constant. Centering transformed
matches by their unweighted mean lacks that property when match sizes differ.
This makes the physical-observation convention easier to interpret alongside
Stata frequency weights. It does not establish uniform bias, variance or MSE
superiority. Both sample means depend on the estimation errors; under
heteroskedasticity, subtracting them can destroy the exact unbiasedness of the
uncentered leave-out variance proxy. Mean projection does not correct that
estimated-mean bias and does not carry a universal finite-sample unbiasedness
or coverage guarantee.

## MCSE and choosing a mode

| Request | Numerical diagnostic |
| --- | --- |
| Exact, any centering, `mcse(all)` or developer `mcse(conditional)` | Zero MCSE: there is no JLA approximation. |
| Exact or JLA, any centering, `mcse(off)` | MCSE and covariance unavailable/missing; additional MCSE work is skipped. |
| JLA, None, `mcse(all)` | Ordinary local all-point-probe MCSE. |
| JLA, Mean, `mcse(all)` | Ordinary calculation with the observed mean held fixed. |
| JLA, Corrected, `mcse(all)` | Exactly Mean's MCSE and covariance for the same call and probes; the added centering increment is also held fixed. |

The developer conditional diagnostic follows the same centering assumptions
but remains conditional on the realized leverage sketch. No diagnostic here
includes sampling uncertainty, finite-probe bias, or solver/roundoff error.

**Corrected JLA's reported MCSE excludes numerical uncertainty in its added
centering increment.** It is not the complete numerical standard error of the
Corrected point estimate. Its calculation does not differentiate or replay
the increment. `mcse(off)` skips diagnostic work, but still calculates the
requested Corrected point estimate.

Omitting `centering()` selects Mean for both exact and JLA. Explicit
`centering(none)` reproduces the former default.
For small exact problems, `centering(corrected)` is a practical choice when
an adjustment for estimated-mean bias is wanted. For JLA, `centering(mean)`
with the default `mcse(all)` gives the inexpensive centered estimate and a
diagnostic under the fixed observed mean convention. Corrected JLA is also
available when its additional point adjustment is wanted. The default is
fixed rather than selected from the data; small fixtures do not establish
that the Corrected increment is negligible in every large sample.

With worker–firm data already loaded:

```stata
* Small exact problem: estimated-mean adjustment, no numerical MCSE needed
fevc log_wage i.year, worker(worker_id) firm(firm_id) ///
    algorithm(exact) centering(corrected) mcse(off) backend(mata)

* JLA: ordinary MCSE with observed mean held fixed
fevc log_wage i.year, worker(worker_id) firm(firm_id) ///
    algorithm(jla) centering(mean) probes(200) seed(12345) backend(mata)

* JLA estimated-mean adjustment; MCSE still holds its increment fixed
fevc log_wage, worker(worker_id) firm(firm_id) ///
    algorithm(jla) centering(corrected) probes(200) seed(12345) backend(mata)
```

## Restrictions, availability and failure

- Only Corrected JLA requires an even `probes()` budget of at least four.
  Mean and None keep the ordinary probe rules. Exact Corrected does not
  require an even budget; probes are not used by exact estimation.
- Component `inference(highrank|q1)` and `project()` accept Mean (the default)
  or None on their existing supported tuples. Corrected with either request
  returns `CENTERING_INFERENCE_UNSUPPORTED` (return code 498) before estimator
  RNG. The command does not silently switch to None.
- An unknown mode returns `INVALID_CENTERING` (return code 198).
  Additional correction-system failures return a typed error; the command
  does not substitute None or Mean or fall back after RNG.
- Mata needs the current source. Rust active centering additionally needs
  centering API 1. Native numerical API 2 is a separate capability.
  `fevc_rust probe` exposes `r(centering_api)`; a missing/zero capability
  does not support active centering. Mean projection additionally requires
  projection-centering API 1, exposed as `r(projection_centering_api)`.
  Mean component inference additionally requires component-centering API 1,
  exposed as `r(component_centering_api)`. Existing combined exact-Mata
  requests support Mean; native combined component/projection requests remain
  unsupported. Missing metadata is zero; explicit None needs neither
  attachment-centering capability on its separately supported native requests.
- The local Mac arm64, Rosetta x86-64 and universal candidates at clean
  source `24754269` expose point and projection centering APIs and pass full qualification
  plus 24 isolated-install capability, point-centering and Mean-projection
  checks. Linux x86-64 also passes full qualification and installed point-centering
  and Mean-projection checks at the same source (SCC job `7962808`).
  Windows runtime qualification remains pending; the retained manual-test
  payload has centering API 1 but lacks projection-centering API 1.
  Native Intel hardware is not claimed. See the
  [current candidate record](../../native/prerelease-20261008/manifest.json).
  These local candidates have not been published or tagged. Their point and
  projection evidence does not qualify Mean component inference, which needs
  the new component capability and fresh exact-source qualification.
  Use current Mata source or a matching qualified native build.
  An older plugin may lead `backend(auto)` to Mata before preparation/RNG,
  subject to the ordinary strict-consent rules. `backend(rust)` and explicit
  Counter-V1 requests require the matching native capability and fail
  preflight with `RUST_BACKEND_UNAVAILABLE` if it is absent.

### Preserved October 7 development evidence

The local Mean-projection checkpoint passes the 48-cell exact covariance
oracle, eight-cell native projection tests, complete Stata quick suite,
Rust workspace checks, and Mac arm64/Rosetta separate/universal runtime and
isolated-install checks. The native qualifier records
`LOCAL_CHECKPOINT_DIRTY_TREE`: its tests pass, but the outer CI clean-checkout
gate remains failed. The rebuilt Mac package binaries are tracked working-tree
changes; prior distributed manifests and receipts keep their original bytes
and scope. Linux, Windows and native Intel hardware were not requalified for
Mean projection. This is local development evidence, not release qualification
or a new coverage/MSE campaign. See the [current checkpoint](../PLAN.md).

See [installation](../../INSTALLATION.md), [memory](MEMORY.md) and
[native provenance](../../native/README.md). The package version remains
`0.5.0-rc.1`; repository publication does not imply a release. The [native adoption record](../../native/centering-20261006/manifest.json)
binds the four updated payloads. The owner authorized source publication on
October 7. The Windows hosted build passes, but full qualification and a
focused centering diagnostic fail. The owner subsequently requested adoption
of that exact candidate for manual testing; see the [manual-test record](../../native/centering-windows-manual-20261007.json).
The [October 7 attempt record](../../native/centering-20261007/manifest.json)
preserves the aggregate failures and cleanup. Runtime qualification remains
pending; no failing assertion is available from the private collector.

## Stored results

| Return | Value |
| --- | --- |
| `e(centering)` | `none`, `mean` or `corrected`; `mean` when omitted. |
| `e(mcse_centering)` for None | `uncentered` |
| `e(mcse_centering)` for Mean | `fixed observed mean` |
| `e(mcse_centering)` for Corrected | `fixed observed mean and fixed centering increment` |
| `e(inference_centering)` with active component inference | `uncentered` for None; `fixed observed mean` for Mean. |
| `e(inference_mean_omitted)` with active component inference | `0` for None; `1` for Mean. |

The centering and MCSE macros are recorded even in exact mode and with MCSE off.
The two inference returns are present only for active component inference;
Mean component output and diagnostic replay state the omitted mean uncertainty.
They describe sampling-inference assumptions separately from numerical MCSE.
`e(mcse_centering)` describes the convention if a diagnostic is calculated;
availability is determined separately by `e(mcse_available)` and status.
`e(plugin)` is unchanged. `e(correction)` contains the selected correction,
and `e(kss) = e(plugin) - e(correction)`. No separate covariance for the
centering increment is returned. Record both macros with exported MCSE
mode, method, status, availability, SEs and raw covariance; see
[the return contract](FAILURES_AND_RETURNS.md).

## Exact adjustment and its implementation

The following notation expands frequency weights into `n` literal copies
for the derivation only. Production keeps the collapsed representation.
Let `X` be the active working design, `S = X'X`, `A` its inverse on the
identified quotient, `H = X A X'`, `M = I - H` and `e = M u`.
For deletion unit `g`, let `m_g` be its literal-copy count and let `1_g`
be a vector of ones. Define
\[
r_g=M_{gg}^{-1}e_g,\qquad
t_g=z_g(1_g'r_g),\qquad
\xi_g=\frac{z_g(1_g'r_g)-r_g(1_g'z_g)}{m_g},\qquad
a_g=\frac{1_g'M_{gg}1_g}{m_g}.
\]
Let `Z_d` be the deletion-unit indicator matrix and concatenate `t_g`
into `t`. Solve
\[
K\kappa=b,\qquad K=n\,\mathrm{diag}(a)-Z_d'MZ_d,\qquad
b_g=1_g'(Mt)_g-n\,1_g'M_{gg}\xi_g.
\]
The second term in `b_g` is block-local: it uses `M_gg xi_g`, not a
global residual projection of all units' `xi`. Set
\[
\widehat c_g=\xi_g+\frac{\kappa_g}{m_g}1_g,\qquad
B_{Q,g}=X_g A Q A X_g'.
\]
For the unchanged plug-in quadratic target `beta_hat' Q beta_hat`,
\[
\widehat\theta_Q^{\mathrm{mean}}
=\widehat\theta_Q^{\mathrm{plugin}}-\sum_g z_g'B_{Q,g}r_g,\qquad
\Delta_Q=-\sum_g1_g'B_{Q,g}\widehat c_g,\qquad
\widehat\theta_Q^{\mathrm{corrected}}
=\widehat\theta_Q^{\mathrm{mean}}+\Delta_Q.
\]
`Q` uses the requested target population; it does not change `ubar`.

The implementation reuses the ordinary fit, inverse and deletion work.
With `F_g=X_g'1_g` and `delta_g=n a_g-m_g`,
`K = diag(delta) + F' A F`. It solves the corresponding coefficient
system using the existing model geometry, with a bounded small Schur system
for exceptional units and guards for literal-copy contrasts. It checks the
original correction-system residual. There are no per-target refits and no
production observation-square matrix. See
[the numerical architecture](NUMERICAL_ARCHITECTURE.md).

## Corrected JLA

The correction map uses the ordinary deletion-maker inputs from the existing
full leverage pool and from each of its two equal halves, retaining the exact
low-dimensional control geometry. If `Delta_full`, `Delta_half1` and
`Delta_half2` are the signed added target increments from these maps, it adds
\[
\Delta_{\mathrm{JLA}}
=2\Delta_{\mathrm{full}}
-\frac{\Delta_{\mathrm{half1}}+\Delta_{\mathrm{half2}}}{2}
\]
to the Mean point estimate. Each map uses the constrained projection/residual shares from its own
pool and the exact low-dimensional control geometry. The increment does not
reuse the ordinary finite-projection bias/variance formulas. No new leverage
directions or target solves are drawn. This estimated-mean adjustment is distinct from the ordinary
[finite-projection JLA correction](JLA_FINITE_PROJECTION.md).

## Measured cost and validation scope

The October 6 local timing check ran full `fevc` commands on two small
weighted fixtures, both backends, exact/JLA(64)/JLA(256), MCSE off/all and
all three centering modes. Each cell had one warmup and three rotated
measured repetitions with the same data and seeds (288 commands).
Data generation and Stata process startup were excluded.

| Added median command time over None | Rust | Mata |
| --- | --- | --- |
| Mean, all 12 backend-specific cells | At most 1 ms | At most 1 ms |
| Corrected exact, MCSE off/all, both fixtures | At most 1.9% | At most 6 ms |
| Corrected JLA, MCSE off/all, both budgets/fixtures | At most 2 ms | 4–51 ms |

The fixtures had 120 and 600 stored rows, frequency weights 1–3 and unequal
target mass; the larger fixture had one joint control and observation
deletion with stayers. All 24 Mean gates and eight Corrected exact gates
passed the unchanged timing thresholds in the quiet repeat. The first run
with background scientific work passed 23/24 Mean gates and is preserved.
These are small local checks, not universal speed guarantees or evidence
about representative-scale performance.

Independent dense exact and three-pool JLA oracles, fixed-mean point/MCSE
checks, both-backend option guards, explicit-CMG checks, the full Rust
workspace, Python (879 tests), and integrated Stata/install checks passed.
Astra/high reviewed each milestone and final source/timing evidence.
The [testing guide](../TESTING.md) lists focused tests. The immutable local
implementation checkpoint, logs, native build, source manifest, original
timings and quiet repeat are under
`~/research/Variance_Components/fevc-centering-simple-2026-10-06/final-local/`.
These checks do not extend platform qualification or statistical coverage.
