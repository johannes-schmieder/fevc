# All-point-probe numerical covariance — implementation contract

Current applicability: `mcse(all)` is now the public default, as recorded
in the September 30 interface/evidence documents. The historical derivation
and work packages below describe the ordinary all-probe calculation.
Under the default `centering(mean)` that calculation holds the observed mean
fixed.
Under `centering(corrected)` it returns exactly Mean's covariance and also
holds the added centering increment fixed; it does not estimate the
increment's uncertainty. See [CENTERING.md](CENTERING.md) for the current
scope and [FAILURES_AND_RETURNS.md](FAILURES_AND_RETURNS.md) for returns.
With Mean projection, this diagnostic still covers only the four main point
estimates, not projection coefficients or their covariance. Component inference
continues to require explicit None.

Issue [#7](https://github.com/johannes-schmieder/fevc/issues/7), revision
September 29, 2026, is the prospective contract below. Review anchor and
starting checkout both equal `ecd62544a132906aad1ffa7b8e4a3aa0a03ee9c3`;
there were no subsequent changes or existing uncommitted changes to reconcile.

The maintained independent dense reference lives in
`../tools/all_probe_reference.py`. This contract does not advertise a supported
public option before the backend and native milestones pass. Implementation
and validation status is recorded separately in `ALL_PROBE_MCSE_STATUS.md`.
Historical qualification is not carried to this feature.

## 0. Decision and deliverable

Implement an **opt-in, two-target-fold, whole-leverage-probe influence covariance with deterministic leverage replay**. First build an independent complete-randomization reference harness. Retain the current point estimator and conditional target-probe MCSE.

The user-facing question is:

> With these same data, fit, sample, weights, deletion units, and probe budgets, how much would the reported components vary if both point-estimation probe stages were redrawn?

The efficient answer will be a **local-linear approximation**, not an exact finite-probe covariance theorem. The target-fold construction below removes noisy-sensitivity interaction double counting exactly within that linearization. It does not remove nonlinear leverage-sketch bias or make the approximation uniform near a singular deletion.

Deliver:

1. The unchanged existing conditional MCSE, plus its joint covariance when this option is requested.
2. A signed leverage-stage covariance estimate and a joint all-probe numerical covariance for worker variance, firm variance, and worker–firm covariance.
3. Correct propagation to the variance of their sum, explicit method/status/conditioning metadata, and resource/RNG receipts.
4. A reproducible reference harness, independent algebraic oracles, backend tests, and a documented calibration envelope.

**Not in scope:** changing KSS point estimates, sampling inference, finite-projection formulas, retained populations, deletion definitions, solver tolerances, randomization laws, automatic stopping, public separate leverage/target budgets, or making the new diagnostic default-on. Do not put numerical covariance into econometric `e(V)`. Bootstrap/jackknife alternatives are optional research cross-checks, not competing production implementations to choose between during coding.

Follow root and package `AGENTS.md`, the current `fevc/PLAN.md`, and the registered development-acceptance policy. Preserve existing work. This issue is not authorization to release/tag, replace distributed binaries, change CI policy, or launch a large cluster campaign.

## 1. Source map and current behavior

Read these anchored contracts alongside the current checkout:

- [Numerical architecture](https://github.com/johannes-schmieder/fevc/blob/ecd62544a132906aad1ffa7b8e4a3aa0a03ee9c3/fevc/docs/NUMERICAL_ARCHITECTURE.md).
- [Actual finite-projection and literal-copy formulas](https://github.com/johannes-schmieder/fevc/blob/ecd62544a132906aad1ffa7b8e4a3aa0a03ee9c3/fevc/docs/JLA_FINITE_PROJECTION.md).
- [Joint-control/block derivation](https://github.com/johannes-schmieder/fevc/blob/ecd62544a132906aad1ffa7b8e4a3aa0a03ee9c3/fevc/docs/BLOCK_CONTROL_DERIVATION.md).
- [Counter RNG contract](https://github.com/johannes-schmieder/fevc/blob/ecd62544a132906aad1ffa7b8e4a3aa0a03ee9c3/rust/RNG_CONTRACT.md).
- `fevc/docs/ESTIMATOR_CONTRACT.md`, `FAILURES_AND_RETURNS.md`, `MEMORY.md`, and `rust_mata_parity.json`. The Markdown parity table is generated; edit its source, not the generated table.

The substantive Rust hooks are in [`generic_jla.rs`](https://github.com/johannes-schmieder/fevc/blob/ecd62544a132906aad1ffa7b8e4a3aa0a03ee9c3/rust/crates/vckss-core/src/generic_jla.rs):

- Lines 281–590: `GenericJlaOptions`, `GenericJlaResult`, `FiveMoments::finite`, and `FiniteMoment`.
- Lines 600–930: existing routed/attachment/hybrid entrypoints and RHS receipt counts.
- Lines 1650–2410: deterministic preparation, mixed mover/stayer leverage preparation, deleted adjustments, and target orchestration. Moments and geometry are currently released before the target pass; opted-in derivative state needs an explicit lifetime here.
- Lines 4700–5230: observation/hybrid leverage accumulation, `observation_deleted_adjustment`, and `match_deleted_adjustment`.
- Lines 7400–8010: `target_correction`, `component_mean`, `component_mcse`, the independent test contraction, and small-maker actions. Target statistics also dispatch through `generic_jla/statistical_batches.rs`.

Current generic point work is one FE leverage RHS and two target RHSs per logical direction. `component_mcse` uses centered target-draw sums divided by `T(T-1)`. Public `probes()` currently supplies both statistical counts. Current `stayers(both)` uses a combined fitted problem with mover-match and eligible-stayer-observation corrections; the diagnostic must cover that headline result, not silently substitute movers only.

Also integrate the compressed Rust path in `engine.rs`/`jla.rs`, native transport in `rust/crates/vckss-plugin` and `rust/stata_backend`, and Mata paths in `fevc.mata`/`fevc_scale_engine.mata` with the existing RNG/resource/runtime helpers. Do not assume an econometric-inference helper's three-target ordering equals the numerical ordering below.

## 2. Precisely define the covariance target

Condition throughout on all observed inputs and deterministic preparation, including the outcome, retained sample, fitted coefficients, controls, frequency counts, target masses, normalization, deletion partition, and solver request. Point-estimation probe randomness is the only randomness considered. Sampling uncertainty and separate inference/projection simulations are outside this diagnostic.

Use primitive ordering

$$\theta=(V_W,V_F,C_{WF})'. $$

Let `S` consist of all `R` leverage directions. Let `U_r` be the **whole vector across all deletion units/copies** of sufficient-statistic contributions from leverage direction `r`, and

$$\widehat u=R^{-1}\sum_{r=1}^R U_r.$$

The adjusted deleted-residual vector is a deterministic nonlinear function

$$z=h_R(\widehat u).$$

The explicit finite-projection coefficient `1/R` is part of this function. Independent target direction `t` produces a linear functional of `z`:

$$C_t(S)=L_tz,\qquad \widehat\theta=\theta_{\rm plugin}-\bar Lh_R(\widehat u),\qquad \bar L=T^{-1}\sum_tL_t.$$

Section 5 gives `L_t` for the actual observation, block, and hybrid formulas. The fit and target solves defining `L_t` do not depend on the leverage sketch.

When the numerical quantity is defined under the unfiltered probe law and has finite second moments,

$$\Sigma_{\rm all}=E_S[\Sigma_{T\mid S}]+\operatorname{Cov}_S(g_R(S)),\qquad g_R(S)=\theta_{\rm plugin}-L_0h_R(\widehat u),\quad L_0=E_TL_t.$$

The existing conditional estimator extends directly to a full matrix:

$$\widehat\Sigma_{T\mid S}=\frac1{T(T-1)}\sum_t(C_t-\bar C)(C_t-\bar C)'.$$

Its conditional expectation is the target-stage covariance at the realized sketch. It is not the unconditional target contribution at every individual sketch. “Conditional on the first probe” must be replaced by **conditional on the entire first-stage leverage sketch**.

**Acceptance qualification:** if some probe draws make the command return no number, there is also a failure event, not an ordinary numeric random variable on every draw. Do not claim an unconditional numeric covariance or success-conditional unbiasedness without addressing that event. The production fast diagnostic is labeled a local approximation to the unfiltered point-probe calculation; observed failures and selection are handled explicitly in Sections 9 and 12.

## 3. Efficient covariance: exact interaction correction for a local linearization

Let

$$D_R=\left.\frac{\partial h_R(u)}{\partial u'}\right|_{u=\widehat u}.$$

Partition target directions by **logical index parity**, independent of execution batches: zero-based even indices in fold A, odd indices in fold B. Counts are `T_A=ceil(T/2)` and `T_B=floor(T/2)`. Point estimation still uses all `T` directions with its existing arithmetic. Require `R>=2,T>=2`; one target direction in a fold is algebraically permissible, although small budgets can be very noisy.

Define independent fold means, conditional on `S`,

$$\bar L_A=T_A^{-1}\sum_{t\in A}L_t,\qquad \bar L_B=T_B^{-1}\sum_{t\in B}L_t,$$

and whole-direction scores

$$\psi_r^A=-\bar L_A D_R(U_r-\widehat u),\qquad \psi_r^B=-\bar L_B D_R(U_r-\widehat u).$$

These are three-vectors. Construct them by summing **all units first for a given direction**. In particular, include mover and stayer contributions before taking outer products. No independence of workers, matches, or rows is assumed for numerical errors.

With centered scores `a_r=psi_r^A-mean(psi^A)` and `b_r=psi_r^B-mean(psi^B)`, define

$$\boxed{\widehat\Sigma_{S,\times}=\frac1{2R(R-1)}\sum_r(a_rb_r'+b_ra_r').}$$

The proposed raw all-probe covariance is

$$\boxed{\widehat\Sigma_{{\rm all,raw}}=\widehat\Sigma_{T\mid S}+\widehat\Sigma_{S,\times}.}$$

There is **no additional division by the number of folds**, and no `1/T` in the leverage formula. Each fold has its own mean denominator, including for odd `T`. The scores contain no `1/R`; that scaling enters the covariance denominator.

### Why this removes the double counting

Write

$$\widehat\Omega_U=\frac1{R-1}\sum_r(U_r-\widehat u)(U_r-\widehat u)',\qquad J=-L_0D_R.$$

Because the target folds are independent conditional on `S`,

$$E_T[\widehat\Sigma_{S,\times}\mid S]=\frac1R J\widehat\Omega_UJ'.$$

This is an exact identity for the **empirical linearization at the realized sketch**, even though its Jacobian depends on that sketch. No independence between the two reported stage estimates is required for adding their expectations.

By contrast, using the all-target gradient `-bar L D_R` on both sides of the outer product has conditional expectation equal to the preceding expression **plus**

$$\frac1{RT}E_T\left[(L_t-L_0)D_R\widehat\Omega_UD_R'(L_t-L_0)'\mid S\right].$$

That extra matrix is positive semidefinite. The expectation of the conditional target covariance already contains the corresponding interaction. Naively adding it again overcounts the mixed term.

### Exact bilinear test and limits

If `h_R(u)=a+Du` is affine with deterministic `D`, the sum of the conditional covariance and the cross-fold estimate is **exactly unbiased for the covariance of one full run**, at finite `R,T`, under independent stages and finite moments. This follows from the conditional identity above and `E[Omega_hat_U]=Cov(U_r)`.

For the scalar example `theta=xbar*ybar`, with independent stage means,

$$\operatorname{Var}(\theta)=\mu_y^2\sigma_x^2/R+\mu_x^2\sigma_y^2/T+\sigma_x^2\sigma_y^2/(RT).$$

The naive sum counts the last term twice; the cross-fold formula counts it once. Include the zero-mean case, in which the entire variance is the interaction, and unequal fold sizes.

For the actual nonlinear `h_R`, only the leverage conditional-mean covariance is linearized. The observed conditional target covariance is retained without linearizing its `z`. Differentiability, sufficient probe moments, stable denominators/makers, and sufficiently small Taylor remainders are required. With `R,T` growing in a regular fixed problem, this gives the usual first-order covariance approximation; do not claim exact finite-budget unbiasedness, a universal `O(1/R^2)` error, or uniform sparse-network validity.

A useful statement for the derivation note: write `h_R(mu+delta)=h_R(mu)+D_0 delta+r`. The omitted covariance terms after contraction by `L_0` are the covariance of `L_0 r` and its two cross-covariances with `L_0D_0 delta`. Their norm is bounded by the remainder's second moment plus twice the square root of the linear and remainder second moments. Small estimated margins alone do not prove this remainder is small; qualification must measure the approximation.

## 4. Differentiate the actual finite-R deletion formulas

Implement one small, independently tested mathematical kernel. Do not replace the point-estimation implementation with a new formula merely to add derivatives. `R` is held fixed in every differential below.

### 4.1 Five raw moments and constrained shares

For one observation copy or rank-one match direction, let

$$u=(p,m,a,b,c)=(\overline{P_q^2},\overline{M_q^2},\overline{P_q^4},\overline{M_q^4},\overline{P_q^2M_q^2}).$$

These are raw means, not centered sample moments. Set

$$s=p+m,\quad H=p/s,\quad M=m/s,$$

$$B=\{Ma-Hb+(M-H)c\}/R,$$

$$V=\{M^2a+H^2b-2HMc\}/R.$$

This is the code's coefficient-**one** mixed-moment bias expression. Raw fourth moments are not divided by additional powers of `s`.

For any direction `du=(dp,dm,da,db,dc)`,

$$dH=(m\,dp-p\,dm)/s^2,\qquad dM=-dH,$$

$$dB=\{a\,dM+M\,da-b\,dH-H\,db+c(dM-dH)+(M-H)\,dc\}/R,$$

$$dV=\{2Ma\,dM+M^2\,da+2Hb\,dH+H^2\,db-2c(M\,dH+H\,dM)-2HM\,dc\}/R.$$

For a whole-probe score use `du=U_r-u_hat`, not a leave-one-out change and not its value divided by `R`.

Current code accepts a tiny negative raw `V` by clipping it to zero. Retain the original point behavior. Record whether this branch was used. A clipped/nonsmooth adjustment does not silently acquire a smooth diagnostic: return an explicit diagnostic status rather than differentiating an unrelated unclipped point estimator. An exact zero with zero empirical directional variation may contribute zero; independently test that case. The new method must never repair a failed point denominator or maker.

### 4.2 Observation deletion, including joint controls

Let `c_i` be the exact **per-physical-copy** residualized-control leverage, zero for a fixed-offset/no-control correction. Set

$$\ell=M-c_i,\qquad F_R(u)=\ell^{-1}+B\ell^{-2}-V\ell^{-3}.$$

Then

$$\boxed{dF_R=(-\ell^{-2}-2B\ell^{-3}+3V\ell^{-4})\,dM+\ell^{-2}\,dB-\ell^{-3}\,dV.}$$

For a single copy the adjusted residual is `e_i F_R`, so its differential is `e_i dF_R`. The residual and exact control contribution are fixed here.

With `f_i` literal copies, the production stored-row residual is

$$z_i=\frac{e_i}{f_i}\sum_{a=1}^{f_i}F_R(u_{ia}).$$

Differentiate each copy's nonlinear formula and average **afterward**. Averaging moments or denominators before inversion changes the estimator.

### 4.3 Match deletion with varying joint controls

In frequency-compressed coordinates for group `g`, let

$$v_i=\sqrt{f_i/F_g},\quad F_g=\sum_{i\in g}f_i,\quad Q=I-C_g-Hvv',\quad K=Q^{-1}.$$

`C_g` is the exact residualized-control projection restricted to the block; it is zero for no controls/fixed offset. Let

$$e^w_i=\sqrt{f_i}e_i,\quad a=Ke^w,\quad w=Kv,\quad t=v'a,\quad k=v'w.$$

The actual adjusted deleted residual is

$$z_g=a+(B-Vk)wt.$$

All inverses must have passed the existing low-rank maker, eigenvalue, and complete-action residual gates. Its complete differential is

$$\boxed{dz_g=wt\left[(1+2Bk-3Vk^2)\,dH+dB-k\,dV\right].}$$

Derivation: `da=wt dH`, `dw=wk dH`, `dt=kt dH`, and `dk=k^2 dH`. Substitution differentiates both finite-projection terms, including the effective third-inverse-derivative term `-3Vk^2`. No additional large solve or new block factorization is needed: `w,t,k` are already available from the two existing maker actions.

Form the five-vector

$$\beta_g=(1+2Bk-3Vk^2)\nabla H+\nabla B-k\nabla V.$$

Then `dz_g=wt beta_g'(U_gr-u_hat_g)`. Compute the five gradients analytically or by seeding the displayed differential with the five coordinate vectors. Production finite differences are unnecessary.

Do not differentiate the full fit, grouping, control canonicalization, solver iteration count, or a gate decision. Those are not part of the point-probe randomization under the fixed-preparation target. Do not replace the block with its mean when controls vary within a match.

## 5. Target operators and efficient reverse contractions

This section specifies the weights needed for the scores; no target sensitivities are left to guess.

### 5.1 Target directions and solves

Let stored-row target mass be `t_i`, total mass `T_mass`, and literal-copy sign sum `S_it`. The compressed centered target direction is

$$a_{it}=\sqrt{t_i/(f_iT_{mass})}\,S_{it},\qquad d_{it}=a_{it}-(t_i/T_{mass})\sum_j a_{jt}.$$

Generate it with the existing target-domain routine and its existing centering checks. This is the sum of the centered physical-copy directions, not a stored-row Rademacher draw.

Use the existing worker and firm target RHSs and working-system solves. Denote their predictions on stored row `i` by `s^W_it,s^F_it`. Under joint nuisance these are predictions of the **whole jointly solved model**, including the target solve's control coefficients. Under fixed offset use the FE working system and existing `working_y`. Never replace `working_y` with a newly residualized outcome.

For the numerical primitive ordering define

$$\kappa_{it}=((s^W_{it})^2,(s^F_{it})^2,s^W_{it}s^F_{it})'.$$

The production point path may continue deriving covariance as `(total-worker-firm)/2`. Do not change that arithmetic for this feature. Verify that the new covariance-coordinate sensitivity is the derivative of that same identity.

### 5.2 Observation rows

The correction is

$$C_t^{obs}=\sum_i f_i y_i^*z_i\kappa_{it}.$$

With `I_i=sum_a F_R(u_ia)`, this equals `sum_i y_i^* e_i kappa_it I_i`. Store the two target-fold means `bar kappa_i^A,bar kappa_i^B`. For replay direction `r`, let

$$\alpha_{ir}=\sum_a dF_R(u_{ia})[U_{ia,r}-\widehat u_{ia}].$$

The observation contribution to the two scores is

$$\psi_{r,obs}^{A/B}=-\sum_i y_i^*e_i\bar\kappa_i^{A/B}\alpha_{ir}.$$

There is no additional `f_i` in this last equation: summing the per-copy differentials already includes it.

### 5.3 Match rows

For each target direction and match, compute

$$A_g^W=\sum_{i\in g}f_i y_i^*s_i^W,\qquad A_g^F=\sum_{i\in g}f_i y_i^*s_i^F.$$

The `3 x block_width` linear operator has column

$$L_{t,gi}=\sqrt{f_i}\left(A_g^Ws_i^W,\ A_g^Fs_i^F,\ \tfrac12[A_g^Ws_i^F+A_g^Fs_i^W]\right)'.$$

It acts on the **frequency-transformed** `z_g` in Section 4.3. Unlike observation `z_i`, that vector already contains a square-root-frequency transformation.

During target streaming accumulate only the three-vector

$$b_g^{A/B}=t_g\,\overline{(L_{t,g}w_g)}_{A/B}.$$

During leverage replay form

$$\xi_{gr}=\beta_g'(U_{gr}-\widehat u_g),\qquad \psi_{r,match}^{A/B}=-\sum_g b_g^{A/B}\xi_{gr}.$$

For `stayers(both)`, add this mover-match score and the eligible-stayer observation score for the **same original global leverage direction**, then take cross products. The shared fit/projection creates numerical dependence between these parts even if their sign atoms are separately addressed.

### 5.4 Literal observation-copy storage without five gradients per copy per target

Existing normalized sufficient statistics are

$$a_{2i}=R^{-1}\sum_r p_{ir}^2,\quad a_{4i}=R^{-1}\sum_rp_{ir}^4,\quad c_{1,ia}=R^{-1}\sum_rq_{ia,r}p_{ir},\quad c_{3,ia}=R^{-1}\sum_rq_{ia,r}p_{ir}^3.$$

The five raw means for copy `a` are

$$(a_2,\ 1+a_2-2c_1,\ a_4,\ 1+6a_2+a_4-4c_1-4c_3,\ a_2+a_4-2c_3).$$

For `lambda=gradient_u F_R`, the pullback is

$$g_2=\lambda_p+\lambda_m+6\lambda_b+\lambda_c,\quad g_4=\lambda_a+\lambda_b+\lambda_c,$$

$$g_1=-2\lambda_m-4\lambda_b,\qquad g_3=-4\lambda_b-2\lambda_c.$$

Sum `g_2,g_4` across copies within a stored row, retaining `g_1,g_3` per copy. Store one row offset

$$o_i=(\sum_a g_{2,ia})a_{2i}+(\sum_a g_{4,ia})a_{4i}+\sum_a(g_{1,ia}c_{1,ia}+g_{3,ia}c_{3,ia}).$$

Then the replay response is exactly the same linearization:

$$\alpha_{ir}=(\sum_a g_{2,ia})p_{ir}^2+(\sum_a g_{4,ia})p_{ir}^4+\sum_a[g_{1,ia}q_{ia,r}p_{ir}+g_{3,ia}q_{ia,r}p_{ir}^3]-o_i.$$

This needs two gradients per physical observation copy and three row scalars, rather than five gradients times six fold/target combinations per copy. The six fold target means live at stored-row level. Use compensated reductions for copy sums, offsets, target contractions, and scores.

For match replay, retain `beta_g` and `o_g=beta_g' u_hat_g`, then compute `xi_gr=beta_g' U_gr-o_g`. The per-direction match projection/residual contractions must be the existing literal-copy contractions, with each declared deletion ID kept distinct from its coefficient cell.

## 6. Concrete execution algorithm

Add an optional `NumericalMcPlan`/attachment, not a separate estimator. Keep existing entrypoints delegating with the feature disabled. New mathematical helpers belong in a small dedicated module such as `numerical_mc.rs`; do not build a general autodiff framework.

```text
PREPARE
  Read effective request; preserve the existing point route and probe plan.
  Freeze R,T, parity folds, required capability, replay route/width,
    allocations, receipts, and all permitted fallback before estimator RNG.
  Build the same graph, sample, fit, controls, and solver setup as before.

LEVERAGE PASS (unchanged statistical directions)
  Run the existing R leverage directions and all existing solve/moment gates.
  Form exactly the existing adjusted residuals.
  If all-probe requested, build local derivative state from those moments:
    observation copy pullbacks/offsets;
    match beta/offset and w,t for target contractions.
  Preserve any diagnostic nonsmooth/unavailable flag independently of point validity.

TARGET PASS (unchanged T directions and point accumulation)
  For each logical t, run the existing two target solves and point contraction.
  Retain the existing target-draw vector.
  Accumulate the diagnostic target-fold row kappas / block b vectors,
    keyed by logical parity, using compensated per-fold reductions.
  Finish the same corrected point estimates and legacy conditional MCSE.
  Compute the primitive 3x3 conditional covariance from these same draws.

LEVERAGE REPLAY (only for an admissible opted-in diagnostic)
  Reuse the same frozen operator, preparation, solver request, and tolerances.
  Replay the ORIGINAL leverage-domain atoms at each original logical index.
  Apply the complete original-system residual gate to every replay RHS.
  For each r, compute local alpha/xi, sum to psi_A[r],psi_B[r].
  Store only these 6R scalar scores, not all unit-by-probe responses.
  Replay is work, NOT additional independent statistical probes.

FINALIZE
  Center the scores stably; form the symmetric cross-fold covariance.
  Add the conditional covariance to obtain the raw total covariance.
  Apply Section 9's diagnostic-only validity/PSD policy.
  Propagate the primitive covariance, post metadata/receipts, and restore state.
```

The match/observation mathematical implementation above covers generic controls, no controls, fixed offsets, literal frequencies, parallel deletion IDs at a cell, and the mixed stayer route. Use shared pure kernels but route-specific atom/prediction adapters. Do not route an opted-in compressed request through a different point algorithm just to obtain the diagnostic.

**RNG:** Rust replay uses the original `ProbeDomain::Leverage`, original seed/entity/subdraw/index, not a new independent diagnostic domain. Unique point atoms and statistical counts remain unchanged; replay generator evaluations and RHSs get separate work counts. Mata must save the leverage-start state through the registered RNG helper, replay the identical logical stream, then restore the post-point internal state; outer caller algorithm/stream/complete state restoration remains unconditional. Do not claim Rust Counter-V1 and Mata Stata-RNG seed equality.

Do not use an altered warm start, looser tolerance, new random target probes, reordered physical-copy identity, or an execution batch as a statistical fold. Cache-versus-replay tests must verify all needed per-direction statistics on small problems. Record score means before numerical recentering as a replay-consistency diagnostic.

## 7. Resource and lifecycle plan

No production observation-by-observation covariance/projection, parameter inverse, unit-by-probe cache, or `R x T` interaction table is permitted.

Incremental derivative-state inventory, before overlap with baseline allocations:

- Observation/stayer-observation portion: `2*N_physical_obs + 3*n_stored_obs` scalar pullbacks/offsets; two three-coordinate target-fold means at stored-row level. Compensated target sums need separate corrections.
- Match portion: five `beta` entries and one offset per deletion group; one `w` entry per stored match row and one `t` per group until target streaming finishes; two three-coordinate `b` means per group, with compensated sums.
- Six scalar scores per leverage direction and small covariance/output matrices. Existing target draws already supply the conditional covariance.

Use actual retained capacities and allocation lifetimes, including the transition when original moments and derivative state coexist, parallel scratch, caller/native buffers, per-RHS receipts, and error cleanup. A formula for final retained vectors is not a whole-command peak forecast. Prefer releasing/reusing moment storage after its original point use; do not let diagnostics keep the entire control geometry alive unnecessarily. Add checked-size/overflow handling and interruption checkpoints.

For the reviewed generic kernel, ordinary probe work is `R + 2T` RHSs; replay makes it `2R + 2T`. Thus at `R=T` the added **probe RHS count** is one third of the old probe count. This is not a promised one-third wall-time overhead: FE and controlled RHSs have different cost, setup/fit are reused, and contractions/copy processing may matter. Measure complete-command time and peak allocations. Other routes must report measured work rather than inherit this estimate blindly.

Select the point solver route from the unchanged point request. Diagnostic replay work must not change an automatic point route's planned-RHS decision. Forecast combined allocations before RNG; preserve explicit batches and existing omitted/warn/error/off memory meanings. An explicit strict budget can reject the requested combined operation before RNG; no hidden memory-triggered reduction of statistical probes or switch to conditional-only estimation.

## 8. Reporting interface and derived quantities

Add `numericalmcse(conditional|all)` with default `conditional`. Default/conditional execution retains the existing fields and behavior. The all-probe option adds the following proposed fixed interface (all names fit Stata's name limit):

- `e(numerical_mcse)`: **unchanged**, four-target conditional target-probe MCSE.
- `e(numerical_mccov_cond)`: `3 x 3`, primitive conditional covariance.
- `e(numerical_mccov_leverage)`: `3 x 3`, signed cross-fold leverage contribution; do not report its square roots as stage SEs.
- `e(numerical_mccov_all_raw)`: `3 x 3`, exact sum of the preceding two estimated matrices.
- `e(numerical_mccov_all)`: `3 x 3`, usable total covariance, or missing with a typed diagnostic status.
- `e(numerical_mcse_all)`: `1 x 4`, worker, firm, covariance, total MCSE, or missing when unavailable.
- Metadata: `e(numerical_mc_method)` = `crossfit_if_v1`; `e(numerical_mc_status)`; `e(numerical_mc_scope)` = `local_unfiltered_point_probes`; `e(mc_leverage_probes)`, `e(mc_target_probes)`, `e(mc_target_fold_a)`, `e(mc_target_fold_b)`, `e(mc_replay_rhs)`, `e(mc_psd_adjustment)`, and derivative/replay residual and boundary diagnostics.

Assign explicit row/column names `var_worker var_firm cov_worker_firm`. Map from named result fields; do not inherit an unrelated inference primitive order. Default omission must not execute replay or allocate derivative state.

For all four targets use

$$A=\begin{pmatrix}1&0&0\\0&1&0\\0&0&1\\1&1&2\end{pmatrix},\qquad \Sigma_4=A\Sigma_3A'.$$

In particular,

$$\operatorname{Var}_{MC}(V_W+V_F+2C)=\Sigma_{11}+\Sigma_{22}+4\Sigma_{33}+2\Sigma_{12}+4\Sigma_{13}+4\Sigma_{23}.$$

Compute MCSEs from the diagonal after this propagation. Marginal MCSEs themselves do not obey the point-estimate accounting identity.

For any future displayed fixed-denominator share use its exact linear map. Nonlinear worker–firm correlation is not required for v1; its documented extension is `rho=c/sqrt(ab)` with gradient `(-rho/(2a),-rho/(2b),1/sqrt(ab))`, valid only for positive, sufficiently stable `a,b`. Do not return a correlation MCSE for negative corrected variances.

For a successfully selected deterministic exact algorithm, all point-probe covariance matrices requested by this option are zero and status is `exact_zero`; no estimator random draws or replay are performed. This says nothing about solver or sampling error. Do not change legacy exact-mode fields unnecessarily.

Display alongside, rather than overwrite, existing MCSEs: **Approximate all-point-probe numerical MCSE**. State that finite-probe bias, solver/floating-point error, and econometric sampling uncertainty are excluded. Numerical confidence intervals about the exact KSS result are not a v1 deliverable.

## 9. Diagnostic validity, PSD, and failure policy

The cross-fold stage estimate can be indefinite at finite probe counts. This is not automatically a coding error. It is an unbiased cross product in the affine benchmark, not a sample covariance formed from one identical set of scores.

- Never force the leverage estimate to be PSD or nonnegative. Never force all-probe MCSE to exceed this particular run's conditional MCSE.
- Check dimensions, finite entries, counts, symmetry, and propagation identities. Compute covariance reductions in scaled/compensated form to avoid overflow and catastrophic subtraction.
- Use the raw sum as the audit quantity. For usable total covariance, symmetrize and eigendecompose the `3 x 3` matrix. Register tolerance `tau=1e-12*(||Sigma_cond||_F+||Sigma_leverage||_F)`, evaluated with scaling; do not use an absolute `max(1,scale)` floor that breaks outcome-unit scaling.
- If any eigenvalue is below `-tau`, status `unstable_nonpsd`: retain valid point estimates, conditional MCSE, and finite raw matrices, but make usable all-probe covariance/MCSE missing. Do not increase probes or retry seeds automatically.
- Eigenvalues in `[-tau,0)` may be set to zero only in the separately stored usable matrix; preserve the raw matrix, report the adjustment norm, and use status `ok_local_psd_adjusted`. Call this a tolerance-level PSD adjustment, not proof that the negative value was caused by roundoff.
- If the scale is exactly zero, return exact zero matrices after reconciling the inputs. A substantive zero is different from unavailable/missing.

Suggested other statuses: `ok_local`, `nonsmooth_adjustment`, `nonfinite_derivative`, and `replay_failed`. An unsupported effective tuple or invalid count is a typed **pre-RNG request failure**, not a hidden backend change. Staged capability gaps must be advertised before execution.

Preserve all existing point-estimator failures as failures. A diagnostic-only nonfinite derivative or non-PSD estimate must not overwrite otherwise valid point output. Cancellation, allocation/transport corruption, and existing fit/point-solve/maker failures retain the repository's command-level failure behavior. A replay solve that fails certification is not accepted or replaced: the all-probe attachment becomes unavailable with its failure receipt, and no successful all-probe claim is posted. This is a diagnostic failure, not a repair of the point estimator.

Report minimum constrained denominator, minimum full observation residual leverage or block-maker margin, and the heuristic sensitivity ratio

$$\max\{\sqrt V/\ell\text{ over copies},\ \sqrt V\,k\text{ over blocks}\}.$$

Label this ratio a local sensitivity diagnostic, not a confidence bound or a rank certificate. It helps interpret unstable/high-curvature cases but must not loosen existing gates or be used to assert uniform validity. Retain solver residuals separately; MCSE does not bound their effect.

## 10. Reference harness: implement before production integration

### 10.1 Independent complete runs

For fixed data and fixed `R,T`, run `K` independent complete point randomizations and retain every attempt, including failures. On a numerically defined unfiltered benchmark,

$$\widehat\Sigma_{repeat}=\frac1{K-1}\sum_k(\widehat\theta_k-\bar\theta)(\widehat\theta_k-\bar\theta)'.$$

This estimates **one-run covariance**. Do not divide by `K` unless the reported point estimator is actually an average of the `K` runs. Do not replace a full-budget run by several smaller-budget estimators: nonlinear finite-R adjustments change.

Start with independent dense Python calculations, then native core runs using existing point code. Deterministic preparation may be reused only after equivalence with completely fresh preparation is tested. The reference must not call the production influence-covariance helper.

Add a private test/harness probe-plan interface allowing independent leverage and target seeds/keys and counts. Existing public constructors must still map `R=T=probes()` and retain their existing streams. Do not expose new public budget options in this issue.

### 10.2 Nested decomposition cross-check

For each of `K` independent leverage sketches, take `L>=2` independent complete target batches of size `T`. Let `theta_kl` be their results, `theta_bar_k` their within-sketch mean, and

$$W_k=\frac1{L-1}\sum_l(\theta_{kl}-\bar\theta_k)(\theta_{kl}-\bar\theta_k)',\qquad \bar W=K^{-1}\sum_kW_k.$$

Let `B_K` be the sample covariance across `theta_bar_k`, with divisor `K-1`. Then

$$\widehat\Sigma_{S,ref}=B_K-\bar W/L,\qquad \widehat\Sigma_{all,ref}=B_K+(1-1/L)\bar W.$$

These are unbiased stage/one-run covariance estimates under the stated well-defined law and independent target batches. Stage estimates can again be signed. Do not use `B_K` alone as the leverage contribution; it contains finite-target noise.

### 10.3 Failure accounting

Store fixed input identity, source SHA, route, seeds/keys, `R,T`, every attempted result/status, point/correction vectors, raw and usable diagnostic matrices, margins, RHS counts/residuals, and timings. Report point failures separately from diagnostic unavailability. Never average diagnostic covariances only over PSD-successful diagnostics; this induces selection. Use finite **raw** matrices for calibration, and treat nonfinite/missing cases explicitly.

If point runs fail, the covariance among successful numeric returns is success-conditional. Report that and the failure frequency; it is not the unfiltered covariance reference. A regular confirmation fixture with failures does not pass by dropping those attempts. Boundary fixtures are intended to expose this limitation, not to be repaired until they pass.

Measure dispersion about the repeated-run mean separately from bias against an exact-algebra reference. A calibrated variance estimate does not establish small bias or correct coverage around the exact target.

## 11. File-level implementation work packages

### M0 — Contract and independent oracle

- [ ] Add `fevc/docs/ALL_PROBE_MCSE.md` containing this derivation, assumptions, interface, and acceptance scope; link it from the documentation index and current plan.
- [ ] Add independent Python tests under `fevc/tests/python/` for five-moment derivatives, observation and block derivatives, physical-copy pullbacks, cross-fold matrix algebra, and the finite bilinear identity. Keep dense/direct oracles independent of production code.
- [ ] Add the fixed-data complete-run/nested harness with machine-readable inputs/results and deliberate-failure tests. Start with tiny local profiles.

Planning checks already performed, **not repository qualification**: 100 independently generated small positive-definite block cases matched complex-step derivatives of the five-moment, observation, and block expressions to maximum absolute error below `7e-16`. Exact enumeration of a scalar `R=3,T=4` two-point example gave true and cross-fold expected variance `0.2047416666666667`, versus naive `0.21495`. In the zero-mean example the naive estimate was exactly twice the correct variance. Odd `T=5` also matched. Recreate these as maintained tests; no Stata/native estimator suite was run as part of writing this plan.

### M1 — Rust mathematical state and generic integration

- [ ] Add small types for numerical primitive vectors, conditional joint moments, derivative states, cross-fold finalization, statuses, and resource forecasts. Use named coordinates.
- [ ] Implement the kernels in Sections 4–5 and a test-only full-response/cache implementation. Compare cache/replay and forward/reverse contractions.
- [ ] Attach optional preparation to the existing generic execution path. Retain the minimum necessary state before current `drop` boundaries. Accumulate fold target weights without changing point draw arithmetic.
- [ ] Implement certified original-atom leverage replay, score storage/centering, and total covariance.
- [ ] Add replay-specific RHS phases/counts and reconcile both statistical draws and executed work. Update expected receipt capacities, progress, memory forecasts, allocation tracking, cleanup, and interrupts. Do not append undocumented entries to a frozen receipt layout.
- [ ] Cover generic observation/match, both nuisance modes, variable controls, integer frequencies, custom target masses, `probeorder()`, and default mixed stayers. The combined result is mandatory, not an optional afterthought.

### M2 — Compressed engine and Mata parity

- [ ] Wire the compressed no-control Rust path through the same mathematical kernels, respecting separate coefficient cells, deletion units, and target strata. Verify equivalence to the generic/dense overlap without rerouting the user's point request.
- [ ] Implement the matching Mata derivative/replay calculation in the existing generic and scale-engine paths, preferably with one package-owned helper module if that avoids duplicated formulas.
- [ ] Register any new Mata module in both package manifests, loader/API/build-token checks, isolated-install tests, and source tests. Keep CMG generated sources untouched unless a separately justified generator change is required.
- [ ] Test the mathematical outputs across backends under matched injected small probes; test each public RNG contract separately for reproducibility. Equal numeric seeds across different RNG contracts are not a parity oracle.
- [ ] During staging, use an explicit unsupported-capability response. Completion of this issue requires coverage of the listed point-estimation routes; do not close it after only the easiest mover/no-control case.

### M3 — Versioned native boundary and frontend

- [ ] Add a capability-gated optional numerical attachment request/result in the native bridge. Inspect the then-current ABI and use a new versioned extension/export; never reinterpret or enlarge old request/receipt structs in place.
- [ ] Preflight missing/stale plugin capability before native preparation/RNG. Preserve strict backend selection and existing permitted pre-RNG fallback rules.
- [ ] Add the Ado option, typed result posting/reconciliation, default/explicit display, and documented returns. All numerical results remain separate from `e(V)` and component-inference attachments.
- [ ] Validate exact dimensions, counts, generation ownership, reserved fields, short buffers, corrupt statuses/matrices, failure cleanup, cancellation/reuse, and memory/export overlap.
- [ ] Update `fevc.sthlp`, `fevc__display.ado`, `fevc_estat.ado` where applicable, returns/architecture/memory docs, changelog, manifests, and capability-source JSON. Advertise exact supported intersections with existing projection/sampling-inference requests; no untested attachment combination may be silently accepted.

### M4 — Calibration, performance, and completion evidence

- [ ] Execute the tests and prospective protocol in Section 12, following the repository's affected-surface rules.
- [ ] Record actual complete-command overhead and allocation high-water, separated by route/deletion/frequency/control configuration.
- [ ] Have an independent review check the cross-stage identity, frequency factors, block derivative, hybrid covariance, return ordering, and failure conditioning against the tests.
- [ ] Publish source-bound implementation/validation status in the issue and current plan. Distinguish algebraic validation, empirical calibration, backend implementation, native qualification, and platform/release qualification. None implies all the others.

## 12. Test matrix and prospective acceptance protocol

### A. Deterministic mathematical tests

Require these before any end-to-end claim:

- Five-moment directional and basis derivatives versus complex-step/centered finite differences on interior fixtures; test the coefficient-one mixed term and fixed `R` explicitly.
- Observation inverses with/without control leverage, frequency-copy nonlinear averaging, and the four-statistic pullback versus explicit physical copies direction by direction.
- Match derivative versus direct dense inverse differentiation with varying controls, unequal frequencies, signs of `B`, nonzero `V`, and both maker representations. Verify `1+2Bk-3Vk^2`, not just the unadjusted reciprocal derivative.
- Primitive target weights versus independent observation-space contractions, including full joint target predictions and the mixed mover/stayer normalization.
- Matrix-valued affine examples with correlated components and units; exact enumeration of the scalar product, pure-interaction, odd-fold, shared-shift, and deterministic-leverage examples. Demonstrate that the intentionally naive implementation fails.
- Tiny cached `U_r`/`L_t` oracle versus the streamed/replay method; no whole-direction cross terms may disappear on collapsing copies or combining populations.
- Symmetry, `A Sigma A'`, total-variance formula, zero directions, all-zero covariance, outcome rescaling (`theta` scales by `c^2`, numerical covariance by `c^4`), negative cross-stage estimates, material/tolerance-level negative total eigenvalues, and nonfinite cases.

For well-scaled interior derivative fixtures, require normalized error at most `1e-9` for numerical-differentiation comparisons and `1e-11` for exact finite algebra/enumeration, with explicit scales. Hard production gates retain their registered tolerances. Stress fixtures get separately documented, conditioning-aware checks rather than retroactive tolerance changes.

### B. Fixed-data statistical calibration

Development: use fixed datasets and independent seed families to explore `R,T` in `{32,64,128,200,256,400}`, including unequal budgets. Inspect stage sizes, signed off-diagonals, and diagnostic instability. Development results cannot be relabeled as confirmation.

Before confirmation, freeze a machine-readable manifest of exact source/input identities, budgets, streams, fixtures, margins, repetitions, outputs, and thresholds. A concrete initial **small-problem confirmation** protocol is:

- Regular fixtures for observation and match deletion; no controls, joint controls, fixed offset; nonuniform target mass; integer-frequency/explicit-copy pairs; parallel deletion IDs; and a qualified default-stayer fixture. Use existing independent fixture/oracle infrastructure, and record the actual support and true dense maker margins. Names alone do not establish a regime.
- Fixed count `K=4096` complete randomizations per registered small fixture/budget, arranged into 64 prespecified independent blocks of 64; public-budget cases `(200,200)`, `(400,200)`, `(200,400)`. This is a prospective local/small-system protocol, not authorization for a large SCC array.
- For each block compute both the average raw estimated covariance and the sample covariance of complete point outputs. For each of the ten fixed contrasts `e1,e2,e3,e1+/-e2,e1+/-e3,e2+/-e3,(1,1,2)`, compare their blockwise difference with reference variance. Estimate uncertainty of the mean difference and variance ratio from the 64 independent blocks, including their covariance (delta method for the ratio).
- For each nondegenerate registered regular contrast, require `abs(relative discrepancy) + 2.576*SE(relative discrepancy) <= 0.15`. This is an equivalence screen for the **raw variance**, not a demand that every run's estimate match a fixed number. Report all contrasts/cells and near-zero references separately. Exact zero directions use the deterministic zero test, not division by an estimated tiny variance.
- Require zero observed point failures and at least 99% usable all-probe diagnostics in each registered regular cell. Do not discard unavailable diagnostics when assessing this rate. Nonfinite raw diagnostics or an incomplete inventory make a cell fail/inconclusive rather than silently shrinking its sample. Report finite raw covariance calibration across all valid point runs, including `unstable_nonpsd` runs.
- Report the observed failure rates and their finite-replication uncertainty; zero observed failures is not proof of zero failure probability under every possible probe realization.
- Include nested reference cells with fixed `K>=128,L>=8` to check stage decomposition independently. Report their reference uncertainty rather than imposing the regular total-variance tolerance on a noisy difference of stage estimates.

Register separate high-leverage/near-boundary fixtures and low-budget/pure-interaction fixtures as **stress/limitation tests**. Their goals are honest statuses, complete failure accounting, and measured approximation breakdown; they must not be quietly promoted into the regular envelope or removed after inspecting results. Failure of a registered regular cell requires diagnosis/revision and a fresh confirmation manifest, not relaxed thresholds.

Increasing target count while holding leverage count fixed must not spuriously eliminate leverage variability. Increasing leverage count must reduce its contribution in regular cases without relabeling replay as additional independent samples. Evaluate bias relative to exact separately from these dispersion tests.

### C. Integration and resource tests

Test enabled/disabled and exact/JLA, every advertised deletion/nuisance/population route, partial and odd batches, explicit and automatic planning, supported thread counts, row sorting, regrouped literal copies, semantic ordering/`probeorder`, caller state, and attachment intersections. Use the repository's statistical-equivalence policy; bitwise checks are useful regression diagnostics but do not replace the scientific gates.

Verify pre-RNG rejection for unsupported/stale capability and strict insufficient memory; omitted/warn/error/off policies; overflow and short buffers; interrupted replay; native failure/reuse; complete original-system residuals for every extra solve; unchanged unique estimator atoms and point counts; separate replay work; and no hidden allocation proportional to rows times probe count.

Run the standard source gates and the smallest relevant integration/native gates, recording exact commands and skipped dependencies:

```bash
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
# In rust/, use the pinned toolchain and locked dependencies:
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
# When licensed Stata is available:
./.venv/bin/python fevc/tools/run_checks.py
```

Consult `rust/TEST_PLAN.md` and `fevc/TESTING.md` for current exact-source plugin/install profiles. A green Python/Rust test run is not native qualification; a Mac result is not Linux/Windows qualification. Large scale tests and additional platform campaigns require the existing authorization/affected-surface process.
