# Pre-implementation steps for the repeat-measurement SE (started 2026-11-11)

User instruction: "Do all of your recommended steps (defer what you recommended for deferment). Yes you can
use the six year Veneto data. It should be on the scc."  Reading: steps 1-4 below plus full proofs; the
confirmation run needs the implementation and waits for an explicit go-ahead to implement. Earlier standing
instruction still holds: no change to the fevc source tree, no commit/push.

Steps
1. Fixed-offset gap on real specifications (year effects + age terms, six-year Veneto).
2. How does the package compute stayer leverages (exact or approximate)?  -> Explore agent.
3. Longer, unbalanced panel: real-data one-shots and simulation on the six-year Veneto design.
4. Rule for weakly identified designs (when to report this SE, when to fall back).
5. Full proofs of Propositions 3 and 4 (for the paper).
Deferred: co-movers with controls; cycles; convergence under approximate leverages for movers.

Data (SCC, private, never copied off SCC or into the repo; only aggregates come back):
- <scc-private>/20260917-panel-1996-2001/inputs/pooled_adjusted_1996_2001.csv
  columns year,worker,firm,observation_key,y_adjusted; 5,163,428 rows; 1,106,419 workers (666,090 stayers
  with 3,029,356 rows); 86,647 firms; 1,700,466 matches; leave-worker-out connected; wage pre-adjusted for
  year effects and ((age-40)/40)^2,^3 (fixed offset). sha256 e4a1928d...
- <scc-private>/20261030-lcs-1996-2001/inputs/long_panel_lcs_nosingletons.csv
  columns worker,firm,year,y,age; 5,509,415 rows (largest connected set, not leave-out pruned).
SCC: ssh alias `scc` (control socket), project welfgr, `module load python3/3.12.4` (numpy 1.26, scipy 1.14,
pandas 2.2; no cholmod/pyamg). Workspace: <scc-private>/20261111-se-longpanel/ (umask 077).
Local scratch: .local/se-design-20261110/longpanel/ (code/, results/).

## Findings so far
- Step 2 (2026-11-11, Explore agent, confirmed by reading the code): in every route the package takes stayer
  P_ii and B_ii from the generic machinery (random projections under JLA; dense inverse in the exact routes).
  The closed forms 1/T_g and A_gg/T_g^2 are not used anywhere (generic_jla.rs:5409-5886, 8734-8838;
  fevc.mata:4846-5142, 5323-5338). Inference reuses those JLA leverages (pooled_component.rs:167-196); the
  Mata matrix-free SE draws its own projections (fevc_residual_se.mata:592ff). Only the structured inference
  basis uses the fact that stayers' firm and covariance diagonals are zero (pooled_component.rs:58-67).
  => recommend exact closed forms for stayer rows (also removes projection noise from the point estimate).
- Step 1 correction: the shipped leave-out-residual SE ALREADY carries the first-order term for estimated
  controls (fevc_residual_se.mata:1-3, 352-357, 821-825; generic_jla.rs:2716-2752; residual_noise.rs:477-595;
  docs/standard_errors/standard_errors.tex section "Controls", eq. (score): 2 g + Ztilde' Lambda J with
  J = -2 Z^c' g, plus a within-unit term). Only the structured / highrank / q1 component-inference routes omit
  it (documented: ESTIMATOR_CONTRACT.md:197, 291-293). The memo's "including the one shipped today" is WRONG
  and must be corrected; the proposal should reuse the package's term. nuisance(joint) is the default.
- Package method doc for the shipped SE: fevc/docs/standard_errors/standard_errors.tex (read before any
  implementation spec).

## Status log
- 2026-11-11 13:2x: code/ has lp.py (design builder, engine, estimators incl. the adjustment without stated
  skewness/kurtosis), lp_build.py, lp_feas.py, lp_real.py, lp_mc.py, lp_agg.py, manifest_tiny.json; local tests
  pass (test_local.py: with all stayers >= 4 years, adjusted estimator = 0.998 of the theoretical variance under
  adverse skew, 0.87 without; tiny end-to-end profile OK). SCC job 8576746 built design/panel.npz
  (4,063,732 rows; stayers by years 2:134,274 3:83,463 4:63,503 5:52,693 6:332,157) and is testing splu on the
  86,647-firm Laplacian (nnz 1.25M). Fallback if the direct factor is too large: block PCG.
- Background agents: weak-identification study (oracle_T2/REPORT_WEAKID.md), full proofs (memo/proofs/).
- Next: lp_real.py on SCC (needs solver decision); merge age from the lcs file for the fixed-offset size;
  choose simulation scenarios from the real-data diagnostics; manifest; array job; aggregate; update memo
  (correct the "shipped today" claim; add six-year results; weak-id rule; exact stayer leverages).
- Step 4 result (oracle_T2/REPORT_WEAKID.md, 144 cells at n~2,400, T=2, normal errors): with the TRUE variance
  the Wald interval covers 0.937-0.962 everywhere, so the damage is the variability of V-hat. No clean cut-off in
  rho = lambda_max^2/V (fine below 0.012, bad above 0.10, mixed between). Per-draw rule works: D2 =
  sqrt(mean off-diagonal squared Gram entry of S~^{1/2} K S~^{1/2} zeta_d over 16 draws) / V-hat; report Wald if
  D2 < 0.10, else switch; conditional coverage among unflagged draws >= 0.926 in every cell. Caveats: D2 has a
  noise floor that moves with n; flags 25-44% of draws for the covariance in small ordinary designs.
- Step 5: memo/proofs/proofs.pdf (13 pp; both propositions proved as stated); sent to GPT Pro A (review4/).

## Six-year Veneto: real-data results (runs/real-v1/real.json -> results/real_v1.json), 2026-11-11
- Design: 5,163,428 person-years -> 4,063,732 rows (1,034,376 mover matches; 3,029,356 stayer person-years).
  One factorisation 1,404 s; one full estimate (fit + 3 x 16 draws) 96 s.
- Estimates (worker, firm, cov, sum): 0.0969, 0.0260, 0.0135, 0.1499 (plug-in 0.1087, 0.0320, 0.0085, 0.1578).
- SE x1e-4: shipped 2.68, 2.40, 1.28, 0.946; model 2.12, 1.96, 0.93, 0.615; proposed (kappa=3) 2.135, 1.959,
  0.930, 0.6735; proposed without stated skewness/kurtosis for >=4 years 2.134, 1.958, 0.930, 0.670.
- Share of the leading term (proposed variances): sum: movers 56%, stayers 44% (2y 9.7, 3y 6.2, 4y 5.2, 5y 4.8,
  6y 17.7); worker 96.5% movers; firm 100%; cov 99.5%.  (Two-period extract: stayers 83% for the sum.)
- Adjustment sizes relative to the variance: kappa=3: -0.04% (worker), -0.37% (sum); free: +0.06%, +0.6%.
- Stayers with >= 4 years (448,353): within-worker kurtosis 4.5 (6y: 4.1; 5y: 4.2; 4y: 5.8); coefficient of
  variation of the variance across stayers 1.6; mean variance by years 2y .0227, 3y .0228, 4y .0213, 5y .0203,
  6y .0119; variance by quintile of m-bar (wage level): .031, .013, .011, .012, .018; third cumulant by the same
  quintiles: -.0030, -.0011, +.0004, +.0013, +.0025 (left-skewed at low wages, right-skewed at high wages).
- GAP CURVE (half mean squared difference k years apart, same firm): all stayers .0097, .0135, .0165, .0195,
  .0227; six-year stayers .0071, .0107, .0141, .0177, .0221 (ratio 3.1); movers within match .0103, .0149,
  .0186, .0225, .0329. Autocorrelation of first differences -0.23 (six-year stayers), -0.20 (all), -0.18 (movers
  within match); independent errors would give -0.5. Fits a random walk (innovation variance ~0.0077 a year)
  plus transitory noise (~0.0033).  => the independence the point estimator assumes across a stayer's years
  fails badly in this panel; no gap removes a random walk. Decision for the owner: estimand and treatment.
- Smoke run (runs/mc-smoke) passed validation (0 problems); ~250 s per draw on the full design; arrays 8578426
  (theory, 11) and 8578428 (tasks, 44) released 15:40.
- Diagnostics (runs/diag-v1): lambda_max^2/V-hat 0.0001-0.0006; D2 0.0006-0.0020 (threshold 0.10); largest
  stayer share of V-hat 0.75% (sum); largest mover-row share 0.7% (cov).
- Year effects (runs/controls-year-v1): omitted first-order term changes the leading term by +0.2% (worker),
  +0.15% (sum), <0.1% (firm, cov) with proposed variances; within-match part negligible.
- Persistence (runs/persist-v1): stayers' correction for sigma2_alpha: iid .00219, transitory-only .00061
  (estimate +1.6%), random walk as error .00815 (estimate -6.1%); SE .000213.
- Age cannot be merged (re-coded IDs); job 8578524 measures year+age terms on the age file after own pruning.
- Round 1 (runs/mc-v1, 11 cells x 100 draws, 0 failures): proposed understates the sum when mover variances
  follow the wage level (0.79/0.73/0.81 in scenarios 4/11/12; 1.00 with true mover variances); shipped 1.02-1.10;
  persistent stayer errors bias theta-hat by 13-49 SD; gap blocks cut it to about a third.
- Year+age controls (runs/controls-lcs-v1, file with age, own pruning): omitted first-order term raises the
  sum's leading term by 19% (package form) to 28%; SE of the sum +8 to +12%; components <1%; age drives it.
- Hybrid (stayers repeat measurement, movers shipped residual + calibration): exact check
  (oracle_T2/REPORT_HYBRID.md): unbiased with true calibration variances up to the squared-residual bias;
  sensitive to the working model through the calibration in noise-heavy designs; fourth moments enter.
  Round 2 (runs/mc-v2, same draws; shared estimators reproduce round 1 exactly): hybrid sum 0.99-1.08 but
  components 1.04-1.29, i.e. about the shipped estimator; real data: hybrid ~ shipped.
- Widened model with m-hat products: failed locally (firm/cov 1.15), dropped.
- Transfer model (profile in estimated effects learned from stayers' q_U, applied to movers' working basis):
  local test fixes scenario 4 (0.92 -> 1.00, components ~1.01); round 3 (runs/mc-v3) and real-v3 running.
- Aggregator now reports failed replications and empty cells as problems (an all-failed local test had
  aggregated silently to nothing).
- Fact-check of memo v7 (memo/factcheck6/REPORT.md): lp_diag.py added the double sum to V-hat instead of
  subtracting it (diag-v1). Corrected analytically from diag_v1.json (the corrected V-hat matches the real-run
  proposed SE^2): D1 0.00013-0.00077, D2 0.0007-0.0036; the control term raises the proposed SE of the sum by
  15% (not 8-12%); components <= 0.6%. Script fixed for any rerun.
- Final (2026-11-11): memo/repeat_measurement_se_v7_final.pdf (41 pp) after the referee (review5/response_D_5.md),
  fact-check (factcheck6) and verification (factcheck7); proofs memo/proofs_props_3_4.pdf (14 pp).
  Round 3 (runs/mc-v3): transfer model sum 1.00-1.06, components 0.95-1.04 except firm 0.88 in scenario 12;
  favourable test (DGP of the model's form). Real data with transfer: SE x1e-4 2.26/2.05/0.98/0.80.
  Source for "11 GB": logs/lp_build.o8576746 on SCC (splu rss 11.2 GB, factor 1,492 s; 1,404 s in real-v1).

## Phase 3 (2026-11-12): one streamlined document for the package

Owner decisions (2026-11-12): go with the rule (repeat measurements where they exist, a model elsewhere) and the
transfer model; do NOT treat stayers differently because of the random-walk finding (out of scope: it goes beyond
the point estimator's assumptions); write ONE self-contained document that includes the proofs, more streamlined
than v7; drop the shipped formula, the model-based SE and the hybrid; no references to earlier versions or
results; start from the six-year Veneto panel; briefly cover computation in FEVC (JLA) and controls/centring
(controls' coefficients treated as known; the mean used for centring treated as a constant).

Plan:
1. Round 4 simulation (runs/mc-v4): estimators transfer (tr), oracle movers (or), transfer with true effects as
   index (trx); 9 cells x 200 draws; adds profiles outside the transfer basis and two violations of the
   transfer assumption; weights from 1,600 projections (the 200-projection weights bias theta-hat by 0.3-0.9 SD:
   theory.json bias_draws/sd = MC bias, i.e. the projection non-linearity bias).
2. Real data (runs/real-v4): estimates, transfer SEs, profile by decile (stayers; movers within match),
   shares, diagnostics, with the 1,600-projection weights. Control-term size with transfer variances
   (runs/controls-lcs-v2).
3. Document memo/final/ (single .tex): estimator, assumptions, propositions, computation, controls/centring,
   evidence, proofs in the appendix. Exact check of the formulas as printed; GPT Pro referee; fact-check.
- 2026-11-12: document drafted in memo/final/ (main.tex + section files; build_appendix.py assembles the proofs
  appendix from memo/proofs/proofs.tex; gen.py turns result JSON into gen/*.tex). Round 4 submitted:
  prepare 8592258 (1,600 projections), smoke 8592259/8592260 (runs/mc-smoke4), real-v4 8592261, lcs2 8592262.
- Exact check of the note's formulas (oracle_T2/REPORT_FINAL.md): no formula fails; extra exact results used in
  the note: unequal variances within a stayer add 6 sum c_g^2 s_g^2 (T-1)^2 (T+4)/(T(T+1)); kurtosis kappa adds
  2(kappa-3) sum sigma_g^4 c_g^2 (T-1)^2(T+4)/(T(T+1)); fitted model with a fixed index has a downward bias of
  order 1/n_0 in small designs (about -5/n_0 relative to V in the Monte Carlo check).
- GPT Pro theory referee (memo/review6/response_T_1.md, new chat): core construction sound, proofs correct;
  12 required fixes applied (scope in abstract; A4 precise; no "similar blur is harmless" -- its example
  without firms gives V-hat/V -> 1.01, reproduced in oracle_T2/final_referee_example.py at 1.009; Prop. 3
  hypotheses; floors; sample centring adds -(2/N) h'Sigma C h; non-positive V-hat; eligibility; other sets;
  "can fail"; skewness wording; match-length wording).
- 2026-11-12 13:10: smoke4 passed (0 problems); mc-v4 arrays 8593604 (theory 1-9) and 8593605 (tasks 1-72) running.
  real-v4 (1,600 projections): theta .09697/.02608/.01345/.14995; transfer SE x1e4 2.25/2.04/0.98/0.80; movers'
  share of the first term 97/100/100/69%; max stayer share 0.7%, lambda^2/V .0003-.0014; no floor binds;
  profile max 24x mean. controls-lcs-v2: control term raises the transfer SE of the sum by 11%, others <=0.6%.
- IMPORTANT (real-v4 profiles + check-v1): movers' WITHIN-MATCH variance by decile of the estimated person effect
  tracks the stayers' profile closely (2.4x / 0.76x / 1.1x vs 2.7x / 0.67x / 1.2x). But movers' ROW variances are
  3-4 times larger (leave-out mean .050 vs .013 within match: a match-level component), and their leave-out
  averages by decile [.179 .056 .036 .029 .027 .026 .029 .036 .052 .093] are much steeper than the transfer
  model's fitted values [.113 .072 .061 .056 .052 .049 .047 .044 .041 .047]. check-v1 (3 draws per scenario on
  the real design, truth known) shows that decile averages of leave-out estimates track the true row variances
  within about 10% (N1: +10/+14% in the extreme deciles, -5% in the middle), so the steep real-data curve is
  mostly real: movers' row variances depend on the wage level more steeply than stayers' year-to-year
  variances. Under A4 (N6) the simulated leave-out curve is 2.7x / 0.63x / 1.45x of its mean; the data show
  3.6x / 0.53x / 1.85x. => A4 is doubtful for Veneto movers; the effect on the SE is not obvious (in N8, movers
  three times as steep, the smoke ratios were .98-1.09 because the scale regression leans on the tails).
- Round 5 (runs/mc-v5, code lp5.py / lp_mc5.py / job_mc5.sh, separate files so that the running round 4 is
  untouched): scenario N10 = stayers follow the data's stayer deciles, movers follow the data's own leave-out
  deciles (multipliers 3.57 1.13 .71 .58 .54 .53 .58 .73 1.04 1.85); estimator tl = transfer with loading
  (step-2 regressors z*profile and z); N1, N6, N8, N9 repeated with the same draws. Smoke 8595372/8595373,
  theory N10 8595371, real-v5 8595374 (tr vs tl on the data).
- Round 4 complete (runs/mc-v4, 9 x 200 draws, 0 failures, 0 problems; results/mc_v4_aggregate.json). Transfer
  model, ratio sqrt(mean V-hat / V) for alpha, psi, cov, sum: N1 1.00 1.00 1.00 1.02; N2 1.00 .98 1.01 1.02;
  N3 1.03 .98 1.06 1.06; N4 1.01 1.04 1.03 .99; N5 .95 .88 .95 1.06; N6 1.01 1.04 1.02 .99; N7 1.00 1.02 1.02
  1.04; N8 (movers steeper) 1.05 1.09 1.04 .99; N9 (movers flat, stayers steep) .75 .65 .76 1.07 (!). Movers'
  true variances: .97-1.03 everywhere. True effects in the profile: .95-1.04 in N1-N7, up to .09 from the
  estimated-effects version. Coverage .91-.98 in N1-N7; .81-.85 for the components in N9. Bias of theta-hat
  <= 0.2 SD with 1,600 projections. In single draws up to 23% of movers' rows get a negative fitted variance
  (floored): the unbounded quadratic profile (max 60-800 x mean) dominates the scale regression.
- real-v5 (tr vs tl on the data): tr 2.25/2.04/0.98/0.80; tl 2.14/1.90/0.96/0.87 (x1e-4); tl floors 3.6% of
  movers' rows and fits the top deciles worse (the stayers' quadratic profile is nearly monotone).
  Smoke5 (N10, 2 draws): tr 1.03 1.06 1.03 .93; tl 1.00 1.02 1.03 .97. mc-v5 tasks 8595836 running.
- 2026-11-12 ~16:20: SSH control connection to SCC dropped (Permission denied; the owner must log in again).
  mc-v5 tasks (8595836) were running; results are on SCC under runs/mc-v5 and still to be aggregated/fetched.
- Owner (2026-11-12, after seeing the draft and the A4 finding): "you can spend one more round, but it can't
  get too complex and has to stay intuitive and relatively easy to compute on large data."
- Round 6 design (code lp6.py / lp_mc6.py / job_mc6.sh / lp_real6.py / manifest_v6.json, prepared and tested
  locally, NOT yet submitted). All candidates: movers' variance = scale_i x profile_i; scale_i = fitted value of
  the leave-out estimates on (1, lr, mr, lr^2, mr^2) (ranks of leverage and match length); estimated effects
  standardised and winsorised at +-3 SD.
    tw   profile = stayers' quadratic profile, one level (pure transfer, stabilised)
    twl  profile = c0 + c1 x stayers' quadratic profile; (c0, c1) from movers' leave-out estimates
    dq   profile = movers' own quadratic in the winsorised effects (no transfer)
  (Cell means by decile x decile were tried locally and dropped: too coarse in the tails at K<10 and badly
  distorted in a small panel, firm/cov 1.13-1.36.)
  Local small panel (1,400 mover rows, 300 draws): twl within .98-1.06 in all five scenarios (S9 1.00-1.01,
  S10 1.01/1.00/1.06/.98); dq 1.01-1.05, S10 1.05-1.15; tw .91-1.07; old tr .91-1.11.
  DECISION RULE (fixed before the run): choose the candidate with the smallest worst shortfall (largest minimum
  ratio over 4 components x 10 scenarios) among those with no ratio above 1.10; if two are within 0.02, prefer
  twl (keeps the stayers' profile, two mover coefficients, least exposed to the correlation between leave-out
  estimates and estimated effects). Also require that on the data fewer than 0.5% of movers' rows are floored.
  Whatever is chosen is written up with all ten scenarios; this is the last round.
- 17:20 SSH restored. Round 5 complete (results/mc_v5_aggregate.json, 5 x 200 draws, 0 problems; tr and or
  reproduce round 4 exactly). N10 (movers follow the data's own decile profile): tr 1.02 1.05 1.02 .93;
  tl (regressors z and z x profile, unwinsorised) .99 1.01 1.02 .96; tl elsewhere: N1 1.00 1.00 1.00 1.01,
  N6 1.00 1.02 1.01 1.01, N8 .99 .99 1.00 1.02, N9 1.01 1.00 1.01 1.01.
- Round 6 resubmitted with a fourth candidate (first submission cancelled before any task ran):
    tlw  = one least-squares fit of movers' leave-out estimates on z and z x (winsorised stayers' profile);
           the winsorised sibling of tl; exactly nests "variance = z'c0 + profile x z'c1".
  Local small panel: twl .975-1.07, tlw .98-1.11 (more mover coefficients, more exposed in a noisy sample).
  Decision rule unchanged (largest minimum ratio, nothing above 1.10; within 0.02 prefer twl).
- 2026-11-12 ~18:00: owner asked whether the estimator could be a short econometrics paper. Sub-agent assessment
  in memo/pubcheck/REPORT.md: not as a stand-alone paper yet (incremental relative to the KSS 2018 arXiv draft
  and the KSS Matlab fitted-variance routine; movers' model carries most of the weight; theory stops short of
  the implementation; proofs not human-verified); home = companion paper's supplement + working paper; later a
  15-20 page note (The Econometrics Journal the best fit); stayer result alone at most a letter. Emailed to the
  owner with the working draft memo/final/draft/se_note_working_draft_20261112.pdf (transfer model as first
  specified, ten scenarios; sources in memo/final/draft_tr/).
- Verified directly in arXiv:1806.01494v1 (KSS, June 2018, p. 25 and App. B2.2): the variance estimator
  4 sum (sum C y)^2 s~_i - 2 sum sum C^2 s~ s~ with smoothed variances s~ = local average of the leave-out
  estimates over (B_ii, P_ii); Assumption 3 (Lipschitz), Lemma 6 (consistency), Remark 10 (local linear,
  tricube), leave-three-out called computationally intractable. Added a "Relation to KSS" paragraph and the
  reference to the note (memo/final/s2_estimator.tex).
- 2026-11-12 ~18:30: owner asked why the KSS and current FEVC standard errors were slow and the new one looks
  easy (Opus sub-agent; notes in memo/compute/REPORT.md). Answer relayed: premise half right. FEVC's current
  leave-out-residual SE already costs 3 + 4R solves (R = 8, 35 solves; standard_errors.tex Table tab:cost,
  verified by me); the prototype is the same recipe with 16 draws (68). Slow were FEVC highrank/q1 (~11,900
  solves), the KSS Matlab routine (~3,000 scalar-per-solve simulations + lowess; local copy, not checked
  against GitHub 8b957ffe) and the KSS 2020 split-sample construction. The "one minute" excludes the 9-34 min
  factorisation and the 1,600-probe point estimate. Section 4 of the note now says what the 68 solves would
  mean inside FEVC (about a tenth of 600-800 solves; an expectation, not a measurement).
- ~18:33 EST: follow-up referee round on the loading model sent to GPT Pro (chat T, Pro, maximum effort):
  memo/review7/ (note_review7.pdf/.tex = Sections 1-5, 6 up to the simulation, 7, Appendix A; prompt_T2.md).
  code/decide6.py applies the round-6 decision rule to the aggregate (tested on the smoke aggregate).
- ~19:15 EST: GPT Pro follow-up report received (memo/review7/response_T_2.md, 32 min, verbatim): the variance
  formula, Propositions 1-4, the KSS lemma and the revised three-step corollary are CORRECT (local fixes:
  "polynomials" sentence, psi outside the neighbourhood, n_0 -> infinity, flat profile outside the corollary,
  weighted orthogonality instead of "unrelated to z"); 17 required fixes, mostly claims around the results:
  A4 stated as b/V -> 0 with both terms; the 1/n_0 bias rate not established (deleted); the remark that the
  A2/A3 departure terms are "of the size of the adjustment" is wrong (two-year stayers: adjustment zero,
  relative bias (kappa-3)/2 in a stayers-only panel); KSS 2020 has a conservative fallback and needs group
  sizes >= 3 (>= 4 to avoid it) in the group-effects example (verified in the published text, p. 1875);
  "clean outcomes" and the sorting claim overstated; the 1.01 example must be specified as an analogue; the
  loading c1 = 2 is a statement about the fitted normalised response (identity c0 + c1 pibar = 1 -> centred
  form, adopted as eq. (8)); Table 2 does not establish "A4 approximately"; diagnostics need definitions;
  sample-centring order replaced by the O_p(lambda_max) argument. All applied to memo/final/ (backups in
  memo/final/draft/*_before_ref7.*).
- Referee's request "report b(Delta)/V per component, linear and quadratic parts": new code/lp_bdelta.py
  (three steps on TRUE variances at TRUE effects -> v*, Delta, b(Delta); no outcome draws), local test on the
  small panel reproduces the simulated ratios where the model is exact (S1, S9: 1.000) and for tw in S9
  (1.07-1.08 vs simulated 1.06-1.07). SCC run bdelta-v1 (hashes in runs/bdelta-v1/source.sha256): smoke task
  N10 through the real launcher (job 8598670.10, 2,159 s) OK; then tasks 1-9 (job 8599026).
  N10, twl: [1 + b/V]^(1/2) = 0.970 0.997 0.981 0.910; linear part/V -0.040 -0.003 -0.029 -0.060; quadratic
  part/V -0.020 -0.004 -0.009 -0.112 (the quadratic part is the same for all four candidates: it is the
  worker-specific variance factor, CV 1.5, which no candidate can see and which is shared by a mover's
  matches). Oracle loading -0.36 + 1.27 pi (centre 1.08).
- ~20:10 EST, owner: "Add how to implement q1 to the memo. Also add an explanation of why our computations are
  relatively computationally cheap compared to what KSS did in their original Matlab package and the fevc paper
  version. This part can be in an appendix."  (This lifts, for that appendix, the earlier "no prior estimators"
  rule.)  Added memo/final/s6b_q1.tex (new Section 7: KSS decomposition theta-hat = lambda_1 (b1^2 - V[b1]) +
  remainder; the remainder is a leave-out estimator for A - lambda_1 S^{1/2} q1 q1' S^{1/2}; stayer weight
  c_g^r = (A_gg/T^2 - lambda_1 w_g1^2) T/(T-1); three entries of the covariance matrix; interval of KSS 7.1;
  cost 17 solves + power iteration) and memo/final/a3_cost.tex (Appendix C, solve counts and four reasons).
  Checks: oracle_T2/q1_checks.py (+ .log): identity 7e-14, stayer weight 4e-15, sum of a stayer's leave-out
  estimates = T q_g 4e-13, E Sigma_rr-hat / truth - 1 = 4e-16 (exact), Monte Carlo (2M draws) all three entries
  within 0.2% (means) of the formulas, 8 design x component cases.  KSS Matlab facts read directly from the
  local copy <workspace>/Monte_Carlo/LeaveOutTwoWay/codes (not a git checkout; version
  not verified against commit 8b957ffe): leave_out_estimation_two_way.m NSIM=1000, scalar per draw,
  V = (4 sum W^2 sigma_predict - var(aux_SIM))/NT^2; q=1 block COV_R1 filled with sigma_predict; llr_fit.m
  lowess on (Pii, Bii), span NT^(-1/3), all observations by default; construc_W.m 2 solves per component.
  FEVC numbers from fevc/docs/standard_errors/standard_errors.tex Table tab:cost (11,901 vs 35).
- 20:17 EST preview of round 6 (73/80 tasks, aggregated in runs/preview-v6 so as not to touch runs/mc-v6):
  twl min .934 (N4 sum) max 1.064 (N3 cov); tw .850-1.269 (fails N8, N9); tlw .929-1.064 (fails the floor rule
  on the data); dq .959-1.105 (N2 psi 1.103 with all 200 draws: exceeds 1.10). Decision awaits the full run.
- 20:49 EST: ROUND 6 COMPLETE (runs/mc-v6: 80 tasks, 2,000 draws, 0 problems, 0 failed draws; aggregate
  results/mc_v6_aggregate.json sha256 598e4fde...; decision log results/mc_v6_decision.txt).
  sqrt(mean V-hat / V), min-max over 40 cells: tw .850-1.269; twl .934-1.064; tlw .929-1.064; dq .959-1.104.
  DECISION by the pre-registered rule: twl (tw and dq exceed 1.10: dq 1.103 N2 psi, 1.104 N10 psi; tlw floors
  2.1% of movers' rows on the data). twl by scenario (alpha psi cov sum): N1 1.00 1.00 1.00 1.01; N2 .99 .99
  1.02 .97; N3 1.00 .97 1.06 .99; N4 .97 .99 1.02 .93; N5 .98 .97 1.02 1.00; N6 .99 1.00 1.02 1.01; N7 .98
  .98 .99 .97; N8 .97 .98 1.02 .94; N9 1.01 1.00 1.01 1.02; N10 .98 .99 1.03 .96. Coverage .91-.98 (5 of 40
  cells below .93; with movers' true variances .92-.98). No non-positive variance estimate in 2,000 draws.
  Round 6 used the same data draws as rounds 4/5 (mc_sd and bias agree to 5e-13), so the oracle columns of
  rounds 4/5 are on the same draws.
- 20:48 EST: bdelta-v1 complete (10 cells; results/bdelta_v1/). twl at FIXED characteristics,
  [1 + b(Delta)/V]^(1/2): exact in N1, N9; N2 .97 .99 .98 .93; N3 .97 .97 1.00 .92; N4 = N5 .97 .99 .99 .95;
  N6 .99 1.01 .99 .96; N7 .98 .97 .99 .94; N8 .96 .98 .99 .93; N10 .97 1.00 .98 .91. Quadratic part for the
  sum: -5.8%, -6.1%, -9.6%, -11.2% of V in the scenarios with worker factors (N2, N3, N6, N10), <= 1.1%
  elsewhere. Implemented minus fixed-characteristics: -0.01 to +0.07 (sum). So the near-unit simulated ratios
  are partly two offsetting errors (model error negative, generated effects positive); written up as such.
- 20:49 EST: diagnostic bdelta-cap-v1 (code/lp_bdelta_cap.py; N4 and N8; winsorising limits 3, 4, 6, none)
  submitted to see how much of the fixed-characteristics shortfall in the no-worker-factor scenarios is the
  cap at 3 SD (local small panel cannot tell: only 0.3% of rows beyond 3 SD there, S8 exact at any cap).
- ~20:55-21:25 EST: third GPT Pro round on the complete note (memo/review8/: note_review8.pdf/.tex,
  prompt_T3.md, response_T_3.md verbatim, 26 min). Verdict: high-rank core sound; Table 5 "especially
  valuable"; q = 1 algebra (decomposition, stayer weight, sum of leave-out estimates, covariance entries,
  unbiasedness with true mover variances, curvature) CORRECT, but no covariance-consistency result, so no
  coverage claim; two statements in "Limits" were wrong (interval not "conservative by construction" with a
  replacement covariance estimator; stayers CAN carry much of the covariance remainder: A^r_gg =
  -lambda_1 T^2 w_g1^2). 19 required fixes, all applied: Section 7 rewritten as a conditional recipe with a
  "What is not shown" paragraph, the critical value defined, PD requirement, normalisation, 17 = incremental
  solves; Table 3 metric stated as root-mean-square SE over true SE; Table 5 left panel = movers'-model
  diagnostic only; quadratic part = all model errors; "net of opposing contributions", no causal claim;
  worker factors frozen across draws and error distributions specified (Appendix B); diagonal moment terms of
  the projected weights disclosed as not included; Appendix C: sample-variance formula 2/(R-1) + 12/(R r_eff),
  squared-inner-product estimator of sum lambda^4, "estimated bound for the fitted matrix", versions pinned
  (KSS Matlab routine checked against GitHub commit 8b957ffe via raw file: NSIM=1000, scalar, var, COV_R1 with
  sigma_predict; FEVC docs at commit 53947b43); (kappa-3)(G-1)/(2G-1); flat-profile rule stated.
- Fact-check sub-agent (memo/factcheck8/REPORT.md): 16 discrepancies, all resolved (see that file).
- bdelta-cap-v1 complete (results/bdelta_cap_v1/): [1 + b/V]^(1/2) for the sum at caps 3 / 4 / 6 / none:
  N4 .947 / .973 / .995 / 1.001; N8 .928 / .963 / .992 / 1.000. 1.04% of movers' rows lie beyond 3 SD and
  carry 59% (N4) and 62% (N8) of the first term for the sum in those scenarios. So the fixed-characteristics
  shortfall in the scenarios without worker factors is the cap at 3 SD. NOT simulated: the implemented
  procedure with a wider cap.
- real-v8-twl (results/real_v8_twl.json; code/lp_real8.py, job 8599411): the final model alone on the data.
  4 solves for the fit with influences (3 s), 64 for the 16 draws (31 s); SEs 2.10 1.87 0.97 0.86 (x1e-4),
  within 0.4% of real-v7; 3x3 variance matrix positive definite before the PSD projection (eigenvalues
  1.11e-9, 2.97e-8, 5.74e-8; min/max 1.9%), so lp6.psd() changes nothing on the data; bound
  sqrt(sum lambda^4)/V-hat from the SAME 16 draws 0.0006-0.0072; r_eff 1,100-6,300.
  Factorisation times in today's logs: 474 s to 2,246 s.
- FINAL: memo/se_note_final_20261112.pdf (32 pages; sources memo/final/; build: build_appendix.py, gen2.py
  twl real_v7_twl controls_lcs_v3 mc_v6 mc_v4 mc_v5 check_v6 mc_v1 bdelta_v1 bdelta_cap_v1 real_v8_twl,
  latexmk main.tex). Delivered to the owner in the session. Nothing committed; fevc source untouched.
  OPEN for the owner: (1) twl vs dq (dq .96-1.104, conservative, excluded narrowly by the 1.10 cap);
  (2) the cap at 3 SD (a wider cap removes the fixed-characteristics shortfall in N4/N8 but was not
  simulated as implemented); (3) a fresh validation run after freezing the model (other designs, new draws);
  (4) q = 1 needs a consistency argument for the cross entry and simulations; (5) a rule for flat profiles;
  (6) implementation in FEVC only on the owner's go-ahead.

## Owner's decision of 2026-11-12 ~21:40 EST and the validation campaign val-v1
- Owner: "Let's use the more conservative model. I'll go with your recommendation for the other points."
  => movers' model = dq (scale x quadratic in the movers' own winsorised standardised estimated effects, both
  fitted to the movers' leave-out estimates; no stayers' profile). My recommendations, stated to the owner:
  keep the cap at 3 SD; run a validation now with the model frozen (fresh draws on the ten six-year scenarios,
  plus two new designs from the same data); no flat-profile rule is needed (no loading).
- Frozen estimator: code/lp7.py (LPF.movers_model, LPF.estimate_final; no PSD projection; fitted movers'
  variances identical to round-6 dq on the local panel, rel. diff 0.0). Harness code/lp_mc7.py (lp_mc6's
  designs, scenarios, theory; draw stream from the manifest; non-positive variance estimates and eigenvalue
  ratio recorded), sub-panel builder code/lp_sub.py, acceptance check code/lp_val7.py.
- Campaign val-v1, registered before any task ran (manifests in runs/val-v1-*/manifest.json, hashes in
  runs/val-v1-*/source.sha256): six = six-year panel, ten scenarios, draw stream 11 (development used 1);
  y3 = years 1999-2001, six scenarios (N1, N2, N4, N6, N8, N10); half = random half of the workers (seed
  20261112), same six scenarios. 8 tasks x 25 draws per cell. ACCEPTANCE (every scenario x component cell):
  sqrt(mean V-hat / V) in [0.93, 1.15]; coverage >= 0.90; mean coverage over cells >= 0.94; no failed draw; no
  non-positive variance estimate; floors bind for <= 2% of movers' rows in any draw. A failure stays a
  failure; every failing cell is enumerated (lp_val7.py). Prototype validation, not a confirmation of an FEVC
  implementation.
- Local end-to-end test (tiny panel and two tiny sub-panels): prepare, theory, tasks, lp_agg, lp_val7; checked
  overwrite refusal, a missing task (lp_agg: 1 problem, nothing aggregated; lp_val7 exit 2), a failing
  threshold (exit 3 with the failing cells listed; the tiny panel fails the floor threshold, 13-17%), loose
  thresholds (PASSED).
- 22:19 EST stage 1 submitted with scheduler dependencies: six theory 8601188 -> smoke task 8601189;
  y3 sub 8601190 -> prepare 8601191 -> theory 8601192; half sub 8601193 -> prepare 8601194 -> theory 8601195.
  Stage 2 (remaining task arrays) only after the smoke task has been checked.
- Diagnostics for dq on the six-year panel: runs/real-v9-dq (code/lp_real9.py: clean run, timing, eigenvalues,
  bound from the same draws) and runs/bdelta-cap-dq-v1 (N4, N8; code/lp_bdelta_cap2.py with BD_MODEL=dq).
- 23:06 EST: dq diagnostics on the six-year panel done. real-v9-dq (results/real_v9_dq.json): fit with
  influences 4 solves 3.9 s, 16 draws 64 solves 43 s; SEs 2.18 1.96 0.96 0.95 (x1e-4; real-v7-dq within 0.3%);
  3x3 variance matrix positive definite, smallest/largest eigenvalue 2.1%; bound from the same draws
  0.0005-0.0074; r_eff 800-6,400; no floor binds. bdelta-cap-dq-v1 (results/bdelta_cap_dq_v1/): for the sum
  at caps 3 / 4 / 6 / none: N4 .949 / .974 / .995 / 1.000; N8 .931 / .965 / .993 / 1.000.
  dq at fixed characteristics (results/bdelta_v1, models.dq): exact in N1, N9; sum .92-.96 elsewhere; as
  implemented above the fixed-characteristics value in 36 of 40 cells, by up to +0.15 for the sum.
- Sub-panels built on SCC (aggregates only): design_y3 = years 1999-2001, 2,328,924 person-years, 832,064
  workers, 50,384 firms, 628,544 stayers (124,289 with two years, 504,255 with three), 203,520 movers with
  426,818 matches, 2,188,161 rows; design_half = half of the workers (seed 20261112), 2,389,081 person-years,
  512,724 workers, 58,214 firms, 307,039 stayers, 205,685 movers with 480,614 matches, 1,879,522 rows.
  y3 theory done (movers' share of the first term for the sum 28-44%: stayers dominate there).
  Six-year smoke 8601189 still held behind one slow theory task; y3 smoke task 8602131 submitted (small
  input, real launcher). Stage 2 (code/submit_val_stage2.sh, finishing job code/job_valfinish.sh) is
  submitted only after a smoke task has been checked.
- Note rewritten for dq in memo/final/ (loading-model version kept in memo/final/draft_twl/ and
  memo/final/draft_twl/se_note_twl_20261112.pdf): title "... a Fitted Variance Model for Movers", Section
  2.2 (two steps, eleven coefficients), A4 and Corollary 1 with a new proof (statistic N1, N2), evidence
  section with a new Figure 1 (movers' leave-out estimates against fitted variances by decile), findings,
  disclosure of the choice in Appendix B. Placeholders left for the validation.
- 23:23 EST: y3 smoke task (8602131, N1 task 0: 25 draws in 851 s, 0 failures, 0 non-positive, eigenvalue
  ratio 0.4%, no floors; ratio dq 0.996 1.007 0.995 1.010, or 0.993 1.004 0.994 1.000) checked; stage 2
  submitted: six tasks 8602785 (2-80; task 1 = 8601189), y3 tasks 8602786 (2-48), half tasks 8602787 (1-48,
  held on its theory array), finishing jobs 8602788-8602790 (lp_agg + lp_val7 per run, held on the arrays).
- 23:26 EST: fourth GPT Pro round sent (memo/review9/: note for dq with the validation marked pending;
  questions on the new corollary and proof, the generated-regressor statements and their direction, the
  characterisation "conservative for an unproven reason", and the validation design).
- 00:00 EST (Nov 13): fourth GPT Pro report (memo/review9/response_T_4.md): new corollary and proof correct;
  twelve fixes on interpretation, all applied (memo/review9/RESOLUTION.md). Note builds at 33 pages with
  only the validation results missing (VALRESULT, ABSTRACTVAL, SCOPEVAL, Tables 6 and 9).
- 02:03 EST (Nov 13): VALIDATION val-v1 COMPLETE (receipts results/val_v1/{six,y3,half}/validation.json,
  aggregates alongside; source hashes re-checked OK by the finishing jobs; 0 inventory problems in each run).
    six  (ten scenarios, fresh draws, no projection): PASSED. Ratio .959-1.102; coverage .910-.985 (mean
         .952); 0 failed draws, 0 non-positive; eigenvalue ratio >= 1.1%; floors <= 0.96%. Differs from the
         development run (projected) by -0.003..+0.006 in the ratios: the projection was immaterial.
    half (six scenarios): PASSED. Ratio .959-1.140; coverage .930-.990 (mean .962); floors <= 1.95%.
    y3   (six scenarios): FAILED, 2 of 24 cells: firm variance 1.152 (N6) and 1.162 (N10), limit 1.15.
         Ratio .948-1.162; coverage .925-.990 (mean .954); 0 failed, 0 non-positive; floors <= 0.98%.
  All 88 cells: ratio .948-1.162 (1 cell below .95, 11 above 1.10, 2 above 1.15); coverage .910-.990;
  4,400 draws, none failed, no non-positive variance estimate, 3x3 matrix positive definite in every draw.
  Largest ratio for the firm variance: 1.10 (six) -> 1.14 (half) -> 1.16 (y3). The failure stands as a
  failure; no threshold was changed. Written into Section 6.6, Table 6, Table 9, abstract and scope.
- Note for dq complete: memo/final/main.pdf, 34 pages (fact-check sub-agent running on it).
- ~02:40 EST (Nov 13): second fact-check sub-agent on the dq note (memo/factcheck9/REPORT.md): all table
  cells and generated numbers reproduce; twelve corrections applied (59% was a share of the movers' part,
  33% of the whole term; the 1,600-projection bias is systematic, up to 0.2 SD downwards, not "noise"; stale
  title footnote; the stylised 1.01 example removed; "at most" numbers rounded up; selection rule completed).
- FINAL for the owner's choice: memo/se_note_final_20261113.pdf (34 pages; sha256 9043dda6...; sources
  memo/final/; build: build_appendix.py; gen2.py dq real_v7_dq controls_lcs_v3 mc_v6 mc_v4 mc_v5 check_v6
  mc_v1 bdelta_v1 bdelta_cap_dq_v1 real_v9_dq val_v1; latexmk main.tex). The loading-model version of
  2026-11-12 is kept as memo/se_note_final_20261112.pdf and memo/final/draft_twl/.
  Nothing committed; fevc source untouched; no SCC jobs of this work left running.
  OPEN for the owner: (1) the three-year validation failed the upper tolerance (firm variance 15-16% too
  large in two scenarios) and the upward discrepancy grows as the design thins (1.10 -> 1.14 -> 1.16): accept
  as is, or compare the loading model (twl) on the two new designs with the same harness; (2) the q = 1
  interval needs a consistency argument for the cross entry and simulations; (3) implementation in FEVC only
  on the owner's go-ahead.
- 2026-11-13, owner on the validation outcome (three-year window failed the upper tolerance, firm variance
  15-16% too large in two scenarios): "Ok, this is acceptable". The model for movers (dq, cap 3 SD) and the
  note memo/se_note_final_20261113.pdf stand as they are; no comparison run of the loading model. Still open:
  the q = 1 interval (consistency argument for the cross entry, simulations); implementation in FEVC only
  on the owner's go-ahead.

## 2026-11-13: q = 1 phase (owner: "Do q1 first. Note and simulations. Then implement in fevc")

- Owner's request, verbatim: "Do q1 first. Note and simulations. Then implement in fevc". Order of work: (1) the q = 1
  interval: a consistency result for the three covariance entries, simulations, Section 7 of the note rewritten as a
  result; review and fact-check; deliver. (2) Implementation in FEVC (Stata/Mata + Rust). Open question put to the
  owner before (2): AGENTS.md says not to create a branch/worktree, the background session cannot edit the shared
  checkout without one; no answer yet.
- Prototype: code/lp_q1.py (critical value of Andrews and Mikusheva by quadrature; KSS interval; class LPQ: leading
  eigenpair by power iteration on S^{-1}A, remainder's influence K1, the three entries with 'dq' and with movers' true
  variances, exact truth). code/test_q1.py: dense checks on a tiny two-block design, worst deviations lam 9e-12,
  w1 2e-6, identity 2e-13, m_r 5e-13, stayer weight 2e-15; truth by draws vs exact 0.999-1.0001.
  Structural fact: for the variance of the sum A-tilde = (I - uu')/N, all non-zero eigenvalues equal 1/N, so the sum
  never has a dominant direction; q = 1 concerns person variance, firm variance and covariance only.
- Synthetic designs (code/lp_q1sim.py): two blocks of firms (nJ = 100, nW = 12,000, six years), b bridging workers
  who move once between the blocks; firm effects with a gap between blocks; variance kinds flat / profile (quadratic
  in the person effect x worker factors) / movers (movers steeper, three times the level) / bridge (bridging workers
  three times as noisy); errors normal or skewed (centred exponential). Scan (400 probes): curvature kappa falls
  from about 20-40 (b = 3) to 2.6-10 (b = 10), 1-4 (b = 30), 0.25-1 (b = 100), 0.02-0.08 (no bottleneck);
  max w^2 = .235, .085, .031, .010.
- Mini run (2 cells x 200 draws): normal interval covers 0.58-0.66 (b = 10, gap 0) and 0.80-0.91 (b = 30, gap 0.3);
  q = 1 interval 0.925-0.965. In 2% of draws at b = 10 the leave-out estimate of V[b1-hat] was negative (CV about
  0.7 with ten bridging workers): no interval. RULE ADDED (lp_q1.q1_interval): the variance of b1-hat is the
  leave-out estimate; if it is not positive or the 2x2 matrix is not positive definite, it is replaced by the value
  from the fitted variances (stayers' q_g, movers' model); the bias correction inside theta-hat_r is unchanged.
- Harness as production code: tasks keyed by (stream, crc32(cell name), task, replication); atomic JSON per task;
  refuses to overwrite; source hashes bound in the manifest for confirmation runs (tasks refuse other sources);
  code/lp_q1agg.py validates the inventory (missing, unexpected, truncated, wrong schema/identity/stream/sources,
  inconsistent counts), counts every failed replication, lists every failing cell, exit 0/3/2.
  code/test_q1agg.py: end-to-end on a tiny profile with deliberate failures, 23 of 23 checks pass.
- Acceptance thresholds, fixed before any grid run (code/make_q1_manifest.py): in every acceptance cell and for
  person, firm, covariance: coverage of the q = 1 interval (rule above, model 'dq') >= 0.92; mean over these >= 0.94;
  draws without an interval <= 1% (they count as not covered); no failed replication; coverage of the normal interval
  for the sum >= 0.92; without a bottleneck the q = 1 interval at most 1.10 times as long as the normal one.
  Cells: b in {10, 30, 100} x gap {0, 0.3} x variance {flat, profile, movers}; no bottleneck x 3 variances; skewed
  errors and bridge-variance at b in {10, 30} (gap 0.3): 25 acceptance cells. Stress cells b = 3 (two), reported, not
  judged (max w^2 = .235: outside KSS's Lindeberg condition).
- Development run q1-dev1 (design seed 11, stream 101, 200 draws per cell) started locally (20 processes).
  A confirmation run follows on new panels (design seed 12) and a new stream, 1,000 draws per cell, with the
  sources bound in the manifest; the rule and the thresholds may change only before it.
- Development run q1-dev1 done (runs_q1/q1-dev1/, 27 cells x 200 draws, no failed replication, no inventory problem).
  With the first rule (leave-out variance of b1-hat, fitted value only if unusable): coverage of the q = 1 interval
  0.910-0.995 in the 75 acceptance cell-components, mean 0.962; two below 0.92 (b = 10, skewed errors: 0.915, 0.910);
  normal interval 0.51-0.97 (0.51-0.91 with a bottleneck of 10 or 30 workers). Fitted variance of b1-hat alone:
  fails where the bridging workers are noisier than the model says (0.79-0.93; the fitted value is 0.35 of the
  truth). Leave-out estimate non-positive in up to 4% of draws at b = 10 and 8-17% at b = 3.
  Stress cells b = 3: 0.86-0.955; no interval in 3% of draws for the covariance (matrix not positive definite).
- RULE CHANGED BEFORE THE CONFIRMATION RUN (allowed: development): the variance of b1-hat is the LARGER of the
  leave-out estimate and the value from the fitted variances. Reasons: always positive, so no case distinction;
  protects against the downward noise of the leave-out estimate when few rows carry b1-hat (max w^2 = 0.085 means
  an effective sample of about 12); costs length. On the development draws: coverage 0.920-1.000, mean 0.977;
  intervals 7% longer on average (up to 22% at b = 10, 5-9% at b = 30, 0-2% elsewhere). The first rule and the
  fitted-only rule stay in the output as comparisons (cilo_dq, cifit_dq), as does the interval with the true
  covariance matrix (ci_true). Thresholds unchanged. Harness test re-run: 23 of 23.
- Confirmation run q1-conf1 registered: runs_q1/manifest_q1-conf1.json (sha256 in manifest_q1-conf1.sha256; sources
  bound), new panels (design seed 12), stream 202, 10 tasks x 100 draws per cell. Realised regimes checked before
  any outcome draw (runs_q1/q1-conf1-designs/): bridging workers carry 97-98% of w1^2; max w^2 = .090 / .031 / .010
  for b = 10 / 30 / 100, .256 for b = 3; kappa 2.7-11.4 (b = 10; 8-23 with noisy bridging workers), 0.95-5.1
  (b = 30), 0.30-1.4 (b = 100), 0.05-0.28 without a bottleneck (there max w^2 = .16: the leading direction is a
  sparse one and does not matter); kappa of the sum 0.002 everywhere. Started locally, 20 processes.
- 2026-11-13 04:33 EST (file time of the receipt): CONFIRMATION RUN q1-conf1 COMPLETE (runs_q1/q1-conf1/; aggregates and receipt copied to
  results/q1_conf1/; manifest and aggregator hashes re-checked, unchanged). 270 tasks, 27,000 draws, 0 failed
  replications, 0 inventory problems, an interval in every draw.
  VERDICT: FAILED, 1 check of the pre-set tolerances. All coverage tolerances met; the length tolerance for designs
  without a bottleneck failed once: covariance in none_movers, q = 1 interval 1.127 times the normal length
  (limit 1.10; the other eight ratios 1.02-1.07). Diagnosis (scientific, not harness): in that panel the leading
  direction is sparse (max w^2 = 0.16) with kappa = 0.28, so the critical value alone is 5.6% above 1.96 and even
  with the true matrix the interval is 8% longer than the normal one; the tolerance assumed the two intervals
  coincide without a bottleneck, which needs a small curvature. The failure stands; no re-run, no changed cutoff.
  Coverage of the q = 1 interval (rule): min 0.941, mean 0.978, max 1.000 over 75 cases; 55 above 0.97; with the
  true matrix 0.945-0.996 (the conservatism is KSS's construction). Normal interval: 0.487-0.964; by true kappa:
  <0.2: .939-.964; 0.2-0.6: .914-.956; 0.6-2: .681-.935; 2-6: .528-.937; >6: .487-.743. Variance of the sum
  (normal interval): 0.936-0.968, SE ratio 0.99-1.07.
  Leave-out variance of b1-hat alone: at b = 10 no interval in 2-14% of draws, coverage 0.795-0.955; at b >= 30
  or no bottleneck 0.935-0.991. Fitted alone: 0.803-0.925 with noisy bridging workers (fitted number 0.34-0.35 of
  the truth). Length relative to the true-matrix interval: 1.02 / 1.07 / 1.15 on average for b = 100 / 30 / 10,
  at most 1.32. Stress b = 3: 0.940-0.980 (leave-out unusable in up to 21% of draws).
  The standard error itself under a bottleneck: RMS ratio 0.59-0.67 with noisy bridging workers and 0.77-0.97
  with profile variances and a gap (the model is off on the bridging rows, which carry the variance).
  Entries: leave-out Sigma_bb mean ratio 0.95-1.05; Sigma_rr 0.95-1.17 (largest with skewed errors); cross entry
  bias <= 0.03 in units of sqrt(Sigma_bb Sigma_rr).
- Real designs on SCC (lp_real_q1.py through job_q1real.sh; aggregates only): three-year window done (job 8603996;
  results/q1_real/real_q1_y3.json): eigenvalue share 1.3-1.4%, kappa-hat 0.06-0.12, q = 1 interval 1.010-1.018
  times the normal one; leading direction sparse (max w^2 0.115; half of w^2 on 5 rows), leave-out number 0.52 of
  the fitted one; power iteration 152-200 steps (31-44 s). Six-year panel and half sample submitted (8604009,
  8604010).
- Fifth GPT Pro round sent at ~04:25 (memo/review10/: note with Section 7 rewritten, simulation numbers pending;
  Pro, thinking effort at maximum, recorded in SUBMISSIONS.md).
- Implementation phase prepared without touching the repository: scratch copy of the tracked source at HEAD
  53947b43 in .local/se-impl-20261113/src (own throwaway git history; not a branch or worktree of the project).
  Map of the existing standard-error code obtained (read-only agent). Finding: the package's residual pass already
  computes the trace term and a model-based covariance 4 sum v m m - 2 tr; the new method is a variant of that pass
  with other variances (stayers' q_g; movers' two-step model) plus the stayer pair term. Rust core module written
  (rust/crates/vckss-core/src/residual_noise/repeat.rs) with unit tests against independent oracles.
- 2026-11-13 ~05:50 EST: fifth GPT Pro report received (memo/review10/response_T_5.md, 28.9k characters; hash in
  hashes.txt): Proposition 5 and its proof correct with local fixes (cross terms vanish exactly; factor
  1/(1-|rho|) right); 14 required fixes on wording of coverage, the correlation in the rule's argument (must be
  the one before the entry is raised; counterexample given), the false sample "lower bound" for the curvature,
  the unconverged-iteration claim, and definitions for the simulation. All applied (memo/review10/RESOLUTION.md):
  new Corollary 2 (containment under rho-hat_lo^2 <= 2/3; coverage under KSS's Theorem 3 hypotheses). One optional
  suggestion (keep the leave-out critical value) not adopted: the rule was frozen and run.
- Real designs complete (results/q1_real/): six-year panel (4.06M rows): eigenvalue share 0.6-0.7%, kappa-hat
  0.02-0.05, q = 1 interval 1.005-1.011 times the normal one, max w^2 0.32 (half of w^2 on 3 rows), 90-126 power
  steps; half sample: share 2.3-2.6%, kappa-hat 0.12-0.26, ratio 1.025-1.046, max w^2 0.40 (2 rows), leave-out
  number 2.5 times the fitted one; three-year window as logged above (one component stopped at 200 steps).
- Benchmark precision (results/q1_conf1/benchmark_precision.json): Sigma_rr <= 0.2%, V <= 3% (b = 10), four seeds.
- Note rebuilt with Section 7 as a result: memo/final/main.pdf, 43 pages; fact-check sub-agent running on the new
  section.
- 2026-11-13 ~06:05 EST: fact-check sub-agent on the q = 1 section (memo/factcheck10/REPORT.md): no coverage
  figure, table cell or count wrong; 13 items applied. The substantive one: under a bottleneck the normal
  interval often does not exist (variance estimate not positive in 36 of 75 cases, up to 47.8% of draws); such
  draws count as not covered and drive the worst normal-coverage numbers (0.487; 0.933 among the other draws of
  that case; at least 0.658 conditionally over all cases). Now stated in the results, the table notes and the
  abstract; length ratios q = 1 / normal recomputed on common draws.
- DELIVERED to the owner: memo/se_note_q1_20261113.pdf (43 pages; sha256 087b0711...). Section 7 is now a result
  (Proposition 5: consistency of the matrix with KSS's leave-out entry; Corollary 2: containment and coverage for
  the rule under KSS's Theorem 3 hypotheses and |rho| < sqrt(2/3)), with the confirmation simulation (failed one
  pre-set tolerance on length; all coverage tolerances met), the Veneto check, and the limits.
  Build: memo/final: gen2.py (as before), gen_q1.py <run> <manifest> <dev run> <real json label>..., build_appendix.py,
  latexmk main.tex. The earlier final (se_note_final_20261113.pdf, 34 pages) is kept.

## 2026-11-13: implementation in FEVC (second half of the owner's request)

- Constraint: AGENTS.md forbids creating a branch/worktree; this background session may not edit the shared
  checkout without one; the owner has not answered which he prefers. Work therefore done in a scratch copy:
  .local/se-impl-20261113/src (tracked tree at HEAD 53947b43 via git archive, plus the seven export-ignored
  tracked files; own throwaway git history; .venv symlinked). The repository itself is untouched.
- Design: the package's leave-out residual pass already forms the pseudo-noise trace and a model-based covariance
  4 sum v m m - 2 tr. The new method is a variant of that pass: inferencemodel(repeat_measurement), explicit only,
  match deletion only; default (inference(auto) -> leaveout_residual) unchanged. Variance source 4, readiness bit 19.
- Done and tested in the scratch copy:
  Rust core: residual_noise/repeat.rs (fit_repeat_variance, finish_repeat_covariance), flag repeat_measurement on the
    prepared attachment, branch in the generic-JLA attachment; 7 unit tests with independent oracles; 3 end-to-end
    tests against the dense unit oracle (trace term within its MC error; covariance assembly; pair term; solver
    routes). Plugin: source 4, bit 19, fit code 4, export vckss_rust_engine_repeat_measurement_diagnostic_v1 (120
    bytes); boundary test. C shim and header: model string, capability scalar, reconciliation, scalars; shim tests
    pass under ASan/UBSan. clippy -D warnings clean; rustfmt applied.
  Stata/Mata: route (typed refusals REPEAT_MEASUREMENT_Q1_UNSUPPORTED, REPEAT_MEASUREMENT_TUPLE_REQUIRED), Mata
    runtime api3 (vckss_rse__repeat; both the exact and the matrix-free routine), posting (status
    FEVC_REPEAT_MEASUREMENT_SE, e(inference_repeat)), display, estat, native attach/fetch/post helpers, plugin
    wrapper. Stata/MP 19 locally: exact Mata route agrees with an independent dense Python calculation (estimates
    1e-13, pair term 11 digits, standard errors within 0.07% at 4,096 draws); new regression
    fevc/tests/stata/test_repeat_measurement_se.do PASSES and is registered in run_all.do.
  Docs: help file, docs/INFERENCE.md, docs/FAILURES_AND_RETURNS.md, CHANGELOG.md, rust/README.md, rust/TEST_PLAN.md.
  Gates: assemble.py --all --check OK; pytest 966 passed, 1 skipped, 3 not runnable in the scratch copy (they
    need the repository's history); static interface tests updated and extended.
- PRE-EXISTING DEFECT FOUND at HEAD 53947b43 (not from this work): the plain default command
  `fevc y, worker() firm()` fails with r(498) INFERENCE_DELETION_UNSUPPORTED unless FEVC_DEFAULT_INFERENCE=none is
  set (which the checker sets, so the suites never see it): under inference(auto) without a typed model the
  route resets inferencemodel to target_lowess. One-line fix in the scratch copy as its own commit; with it and a
  one-line fix of a transpose slip in the never-executed test_leaveout_residual_se.do, that test passes too.
- Not done: the native route run from Stata (needs a plugin built from this source); the q = 1 interval for the new
  method in the engine (rejected with a typed status); heap check of the memory forecast for the variant;
  parity ledger row; registered qualification of any kind.
- 2026-11-13 06:16 EST: implementation state in the scratch copy (.local/se-impl-20261113/src; patch against HEAD
  53947b43 in .local/se-impl-20261113/repeat-measurement-se.patch, 39 files, +2,763/-144, sha256 acf02daf...;
  `git apply --check` passes on HEAD and on the working checkout; NOT applied, repository untouched).
  Native route now exercised from Stata with a plugin built from this source (loaded from a temporary directory):
  the new regression fevc/tests/stata/test_repeat_measurement_se.do PASSES end to end, including backend(rust)
  without and with controls (offset pass), 78-column receipt, standard errors within 15% of the exact
  calculation at 800 probes and 64 draws. The native run found three checks in the C shim that did not admit fit
  code 4 (fixed).
  FURTHER PRE-EXISTING DEFECTS of the earlier feature's native Stata path, fixed in the patch as separate hunks
  (scratch commits 611b3af, 6968a71, cdcd0d0): (2) plan reconciliation demanded mem_command > batch_command, but
  the residual pass's own forecast can leave them equal, so native default standard errors stopped with a
  receipt mismatch and the command fell back to point estimates; (3) with controls under joint nuisance the
  offset pass's solves (two per point probe) were missing from the expected right-hand sides; plus two
  transpose slips inside mreldif() in the never-executed test. With these, test_leaveout_residual_se.do PASSES on
  Mata and on the native backend built from this source.
  Still not done: q = 1 interval for the new method in the engine (typed refusal); heap check of the variant's
  memory forecast; parity ledger; any registered qualification; distributed binaries are unchanged and predate
  both features.
- 2026-11-13 ~06:45 EST: final gates on the scratch state that the patch reproduces (verified: the patch applied to
  a pristine export of HEAD gives the identical tree): assemble --all --check OK; pytest 966 passed, 1 skipped, 3
  history-bound tests not runnable in the scratch copy; C shim tests under ASan/UBSan and the header layout test
  pass; cargo fmt --check and clippy -D warnings clean; Rust workspace 672 passed, 0 failed, 4 ignored; plugin
  crate 20 passed; Stata quick suite (run_all.do quick, FEVC_DEFAULT_INFERENCE=none, distributed plugins) PASS in
  8 min 23 s, including test_repeat_measurement_se.do (log in .local/se-impl-20261113/logs/). The native sections
  of the two standard-error tests were run separately with a plugin built from the source (both PASS).
  Waiting for the owner on: how to land the patch; whether the fixes to the default path go with it; whether to
  build the q = 1 interval into the engine, and whether the new method stays an explicit option.

## 2026-11-13 ~17:24 EST: development analysis for an automatic choice between the normal and the q = 1 interval

Owner's request of 2026-11-13 (discussion only, "Don't change anything yet"): the new method as the default; one
method; automatic diagnostics deciding between high rank and q = 1. Also: "Work on a work tree and separate branch."

After-the-fact analysis on the stored q1-conf1 draws (27 designs x 1,000 draws, 81 design-component pairs without the
sum); scripts and output in `dev_auto/` (`switch.py`, `switch2.py`, `eigshare.py`). Development evidence only: the
thresholds were looked at after the confirmation results were known. "No interval" counts as a miss.

- always normal: min coverage 0.487, 61 of 81 below 0.93; always q = 1: min 0.940.
- KSS's heuristic (eigenvalue share lambda_1^2 / sum lambda^2 >= 1/10, design only; trace by 96 probes): q = 1 chosen
  in 78 of 81 (all but the person variance without a bottleneck, share 0.04; firm and covariance there 0.11 and 0.12);
  min coverage 0.940. Veneto designs: share 0.006-0.026, so q = 0.
- switch on the estimated curvature (needs the whole q = 1 computation): threshold 0.3 min 0.939; 0.5 min 0.925.
- cheap screen 2|lambda_1| Sigma_bb / V^(1/2): threshold 0.1 min 0.930; 0.3 min 0.891 (6 below 0.93). On the Veneto
  designs it equals the estimated curvature to two digits; it fails where the linear part rests on the bridging rows.
- share of the estimated variance carried by the leading direction,
  (4 lam^2 max(b1^2 - Sbb, 0) Sbb + 2 lam^2 Sbb^2) / V, with q = 1 also when V <= 0: threshold 0.05 min 0.939, none
  below 0.93, q = 1 used in 15% of draws without a bottleneck; 0.1 min 0.937, 7%. Veneto designs: 0.0003-0.039.
- cost on the Veneto designs (prototype, power iteration, tolerance 1e-9): 90-126 solves per component (six years),
  152-200 (three years, one not converged), 15-19 (half sample); the q = 1 computation 17 per component by design.

Nothing in the package or the patch was changed.

## 2026-11-13 evening: owner's go-ahead; patch landed; observation deletion; cost of the leading direction

Owner (verbatim): "Okay, this is a good plan. Land the patch now. Q1 with interval and e(V) only for high rank makes
sense. The stata output should explain that. Try to make calculating the leading direction the default if it's not too
expensive. Not too expensive should probably mean that it's not more than, say, 100% of the point estimate alone. And
for observation deletion, yes, I would like to extend the new method now."

Code now lives in a git worktree of the main repository: `.claude/worktrees/repeat-measurement-se`, branch
`repeat-measurement-se` (based on 53947b43; local commits only, nothing pushed).
- a45cc26e: fix of the plain default command (model reset under inference(auto)); transposes in the unexecuted test.
- 0d3542b2: the tested patch (repeat-measurement standard errors, match deletion).
- next commit (in progress): observation deletion.

Observation deletion (theory): the appendix of the note already states S2/S3 for arbitrary sets of rows with identical
regressors. Under observation deletion every worker-firm cell with two or more observations is such a set, with
c_U = B_U / (1 - P_U) for every target (a stayer: P = 1/T, firm and covariance diagonals zero). Dense Monte Carlo
(`dev_auto/obs_theory.py`, 20,000 draws, cells of 1 to 6 observations, true variances for single rows): mean of V-hat
over true V 0.9997 to 1.0018 in all six entries (|z| <= 1.1); without the pair term 0.9936 for the person variance.
Implementation: units are grouped by cell; sets = cells with two or more units; units alone in their cell get the model,
fitted on them if there are at least 100, else on all units of movers (identical to before under match deletion).
Rust: `residual_noise::repeat` (inputs `set`, `mover`, three diagonals; full 3x3 pair term), attachment for observation
rows, stayers admitted for this method, offset pass for observation units under joint nuisance with controls. Mata:
`vckss_rse__repeat` and `vckss_rse__sets`; exact kernels also under observation deletion for this method. Checked:
Rust dense tests for observation units (with and without controls), Stata regression against an independent dense
Python calculation (`oracle_obs.py`): exact Mata SEs within 0.06% at 512 draws, pair term to 11 digits, native route
within 0.12% at probes(800), with and without a control.
Found on the way: at HEAD the native leave-out residual method refuses observation deletion whenever the sample has
stayers ("component inference requires a mover-only retained sample"); not touched.

Cost of the leading direction (owner's budget: at most 100% of the point estimate, which is about 3 x probes = 600
solves at the default): thick-restart Lanczos in the S inner product (`code/lp_lead.py`, basis 8, one solve per step),
on the Veneto designs (SCC runs lead-half-1, lead-y3-1; lead-six-1 pending): residual 1e-3 after 7 to 9 solves per
component on the half sample (power iteration to 1e-9: 15 to 19) and 13 to 14 on the three-year panel (power
iteration: 152 to 200, one not converged); a warm start from the previous component's direction needs 2. About 20
solves for the three components, some 3% of the point estimate. The q = 1 diagnostics from the direction at residual
1e-3 equal those at 1e-6 to five digits. Three-year panel: second eigenvalue 0.96 of the first, so no single
dominant direction there.

## 2026-11-13 night: leading directions, q = 1 and the automatic choice in the package

Cost (closing the question of the owner's budget). Six-year Veneto design (4.06M rows, SCC run lead-six-1): Lanczos
reaches a residual of 1e-3 after 15 to 17 solves for the first component and 2 for each later one with a warm start
(power iteration to 1e-9: 90 to 126 each); three-year panel 13 to 14 and 2; half sample 7 to 9 and 2. About 21 solves
for all four targets against about 600 for the point estimate at the default probes(200). With the default eight
draws the whole automatic request costs about 90 solves (3 + 32 for the high-rank covariance, the search, 4
remainder influences, 32 remainder kernels) where the high-rank pass alone takes 35: some 15% of the point estimate.
The leading direction is therefore on whenever the method runs under inference(auto). Results in
`results/lead/real_lead_{half,y3,six}.json`.

Design as built (branch `repeat-measurement-se`):
- Request: `inferencemodel(repeat_measurement)` with `inference(highrank)` = normal intervals, no direction;
  `inference(auto)` = directions and one interval per component by the rule; `inference(q1)` = the q = 1 interval
  wherever it exists. Internally the pass is the high-rank one in all three; the leading mode travels in
  `$VCKSS_SE_LEADING` and, natively, as the q1 reference of the attachment.
- Engine: `generic_jla/leading_direction.rs` (thick-restart Lanczos in the S inner product, basis 6, 3 kept, two
  orthogonalisation passes, start fixed by coefficient index or the previous target's direction);
  remainder kernel = target kernel with ratio (B - lambda w^2)/(1 - P) minus the rank-one part, so one solve per
  target for its influence and one per target and draw for its kernel; `finish_repeat_q1_target` (larger of the
  leave-out and the fitted variance of b; remainder variance = high-rank form with its own pair term;
  `remainder_pair_term` from the recorded sets); AM critical value and interval from the existing functions.
  Export `vckss_rust_engine_repeat_measurement_leading_v1` (872 bytes); no new capability bit (bit 19 has never
  been distributed); solves planned at the cap (4 x 40) and receipted as used.
- Mata: exact route by dense eigendecomposition; matrix-free route by the same Lanczos; one table layout (4 x 28).
- Rule (in `fevc__residual_se_post.ado`, one place for all routes): q = 1 for a component if the share
  (4 lam^2 max(b^2 - Vb, 0) Vb + 2 lam^2 Vb^2) / V-hat exceeds 0.05 or V-hat <= 0; the sum always normal under auto.
  e(V) only if no component is q = 1 (native joint status 3; Mata joint flag). The reported q = 1 interval is shifted
  to the posted estimate (it differs from the pass's own estimate by projection noise on the matrix-free Mata route
  and by the control estimator under joint nuisance).
- Stata output: a table Estimate / Std. err. / Lower / Upper / Interval (normal or q = 1) and a paragraph that says
  what the two intervals are, the rule, the shares, why there is no standard error, and that e(V) is posted only when
  all components are normal; warnings for a direction resting on few units (squared weight above 0.1) and for an
  unresolved search.

Checks: dense Rust test (eigenvalue by an independent power iteration, identity, influence, covariance, trace within
its MCSE, pair term, curvature; high-rank covariance bit-identical with and without the request); plugin boundary
test; Stata regression against an independent dense Python calculation (`dev_auto/oracle_obs_q1.py`) on the
observation-deletion panel: exact Mata eigenvalues to 1e-9, outcome pieces to 1e-7, simulated pieces within 3% at
512 draws; matrix-free Mata and native within the loose tolerances; match deletion on all routes. Slips found by the
tests: single-pass orthogonalisation degraded below the solver's accuracy (second pass added); `-x^2` on a negative
macro in the share (parenthesised); Mata's conditional operator binds more loosely than the comma.

Open: the 0.05 threshold is a development choice (after the fact, on the stored q1-conf1 draws); the development
campaign at package defaults and the registered confirmation come next; no theory for the data-dependent choice or
for two dominant directions; the default is still `leaveout_residual`.

## 2026-11-13, late evening: package Monte Carlo harness; development run dev1; SCC build

Harness for simulations through the package itself (Stata + the plugin built from the branch), in
`longpanel/pkgmc/` (git-ignored scratch of the main checkout; working copies in the job's tmp are synced by
`sync.sh`):

- `code/pkgmc.py` (`make` writes designs + manifest and refuses an existing run directory; `run` executes missing
  tasks in parallel; `check` is the inventory), `code/mc_task.do` (one cell x deletion mode x block of
  replications; atomic CSV; refuses to overwrite; records the model each fit used), `code/aggregate.py` (coverage
  with every attempted replication in the denominator, failures listed, candidate rules re-evaluated on the
  recorded draws, the package's own rule recomputed and compared with the recorded choice), `code/test_pkgmc.py`
  (19 tests on synthetic outputs), `smoke.sh` in the job tmp (split invariance through the real launcher,
  overwrite refusal, damage detection: all pass).
- `pkg-250a760c/fevc`: the package at commit 250a760c with the mac plugin built from it
  (sha256 4485a8c2...91e6).
- Random numbers: Stata's stream generator (mt64s); replication r of a cell draws from stream r + 1 under the
  cell's seed, so draws do not depend on the split into tasks (tested).

Things found on the way:

1. Stata's batch mode splits a quoted do-file argument at blanks: the options arrived truncated and the first
   smoke silently ran the old model. Options now travel as one word (`+` for blanks) and every row records
   `e(inference_model)`; the inventory rejects a fit with another model.
2. `matrix T = e(nothing)` does not fail in Stata (it gives a 1 x 1 missing matrix). Only `confirm matrix` tells
   whether an e() matrix exists. Fixed in the harness; the package's own tests already use `confirm`.
3. A first block of 100 replications (seeds 110000000 + r under the default generator) had a spread of the firm
   variance 25% above theory. Chased it down: the package's point estimate equals the prototype's on the same
   outcomes (correlation 1.0000; difference 5e-6 against a sampling sd of 8e-4); probe noise at 200 probes is
   about 1% of the sampling sd and the probes are fixed across replications; the package's rms standard error is
   within 2% of the theoretical sd under both deletion modes; the prototype's spread over 6,000 draws is 1.00-1.03
   of theory, and no block of 100 among them exceeds 1.19. The same 100 Stata draws give the same 1.25 in the
   prototype, and the next two blocks of 100 give 0.96 and 0.98. So: nothing wrong with estimator, theory or
   standard error; that block is an outlier of the draws (probability about 3e-4 under normal theory, unexplained).
   The harness now uses independent streams, which is the right design in any case. The aborted run is kept as
   `runs/dev0_aborted_consecutive_seeds`.
4. Twenty Stata/MP processes side by side put the load at 75 on 24 cores; tasks now `set processors 1`.

Development run `runs/dev1` (spec `specs/dev1.json`, manifest sha256 0e5aa61e...d5a9): 16 designs (4 without a
bottleneck, 4 weak, 2 middle, 4 strong, 2 with the direction dominated by signal), realised regimes recorded in
the manifest (true curvature 0.007 to 8.6, true share 0.001 to 1.0), match and observation deletion, 1,000
replications each, package defaults (200 probes, 8 draws), options
`inference(auto) inferencemodel(repeat_measurement)`. Development only: rules are compared on these draws, and
whatever is chosen needs a newly registered confirmation.

SCC: source bundle of 250a760c (sha256 be666d72...f07e) deployed to
`$ROOT/e2e-250a760c`; Linux plugin built there (job 8615008, sha256 7722ae43...4606) and smoke-tested through the
Stata command under both deletion modes (PASS). The plugin interface files were copied there for the build,
checked against the tracked hashes, and removed afterwards. End-to-end package runs on the private panels:
`code/job_run.sh` (panel exported to node-local scratch, aggregates only in `results/`); half sample submitted as
job 8615019.

## 2026-11-13, night: development result, rule frozen, default switched, confirmation registered

Development run `pkgmc/runs/dev1` (manifest sha256 0e5aa61e...d5a9; package 250a760c; 16 designs x match and
observation deletion x 1,000 replications, package defaults): 32,000 fits, no failure, inventory clean, the
package's recorded choice reproduced from the recorded pieces in every replication. Over the 96 cases (design,
deletion mode, component; nominal 0.95, Monte Carlo s.e. of one coverage 0.007):

| interval | lowest coverage | cases below 0.935 | q = 1 used, no bottleneck | e(V) posted, no bottleneck |
|---|---|---|---|---|
| always normal | 0.770 | 27 | 0% | 100% |
| always q = 1 | 0.940 | 0 | 100% | 0% |
| share > 0.05 | 0.940 | 0 | 9.8% | 75.9% |
| share > 0.10 | 0.937 | 0 | 4.3% | 88.5% |
| share > 0.20 | 0.935 | 0 | 1.7% | 95.4% |
| share > 0.50 | 0.935 | 0 | 0.3% | 99.2% |
| curvature > 0.3 | 0.919 | 4 | 6.6% | 83.3% |
| squared part > 0.05 | 0.919 | 10 | 5.6% | 85.7% |

Curvature rules and the squared part alone miss the designs where the direction is mostly signal (normal interval
0.919-0.926 under match deletion at 300 bridging workers with a gap of 3). A bounded version of the share
(direction's part over that part plus the remainder's variance) behaves like the share. The sum, always normal:
0.941-0.969. By group, lowest coverage of normal / q = 1 / rule at 10%: none 0.940 / 0.944 / 0.940; weak 0.934 /
0.943 / 0.937; middle 0.833 / 0.970 / 0.962; strong 0.461 / 0.972 / 0.972; signal 0.919 / 0.940 / 0.940. On the
stored prototype draws of q1-conf1 (dev_auto/switch2.py with more thresholds): lowest coverage 0.939 (5%), 0.937
(10%), 0.934 (15%), 0.933 (20%), 0.930 (30%), 0.924 (50%).

Choice: threshold 10%. Reason: the same protection as 5% on both sets of draws (0.937), half as many q = 1
intervals where there is no bottleneck, and every Veneto panel stays normal (largest share 8.1%, three-year panel
under observation deletion, where the q = 1 and the normal interval nearly coincide). 20% would post e(V) in 95%
of the no-bottleneck samples, at 0.933-0.935; that is the owner's call if he prefers it.

Package runs on the Veneto panels, commit 250a760c, explicit options, SCC (`results/e2e-250a760c/`, jobs
8615146-8615148; build 8615008): six-year panel 5,163,428 person-years, half sample 2,389,081, three-year
2,328,924. The automatic request adds 24-31% to the wall time of the point estimate (six-year: 377 s -> 472 s
match, 370 s -> 486 s observation, 16 threads); 13-21 Lanczos steps in all; no unresolved direction. At 5% all
components normal except the three-year panel under observation deletion (worker 8.1%, firm 5.7% -> q = 1, e(V)
not posted). Largest squared weight of one unit in the directions: 0.12 (three-year), 0.32 (six-year), 0.40
(half sample).

Default switched and threshold set in commit fb49e16d (branch repeat-measurement-se, local): `inference(auto)`
without a model selects `inferencemodel(repeat_measurement)`; share > 0.10; display prints "over 100%" / "none"
for shares that exceed one or do not exist; regression tests for the default (equals the explicit request; fweights
identical to expanded data; deletionid, targetweight; fallbacks with a reason); help and docs rewritten. Gates on
fb49e16d: pytest 970 passed / 1 skipped; assemble --all --check; the two SE regression files and the Stata quick
suite PASS with the plugin built from the branch (the Rust sources are unchanged since 250a760c: plugin sha256
4485a8c2...91e6).

Mata JLA route, small design, quiet machine: point 1.9 s / 1.0 s (match / observation), default 2.1 s / 1.7 s.
(Timings taken while the campaign ran were 20-70 times slower and meaningless.) The Mata exact route refuses the
6,000-row smoke design by its dense-size limit (EXACT_SIZE_LIMIT), by design.

### Confirmation conf1: registered before any of its outcomes was drawn

- Run directory `pkgmc/runs/conf1`, spec `pkgmc/specs/conf1.json`, manifest sha256
  6aee3af8e5e67a57653b24bcae38559c62094a0a69820a1fc9239ef00cf7a200.
- Source: commit fb49e16d; package copy `pkgmc/pkg-fb49e16d/fevc` with the plugin built from it (sha256
  4485a8c2...91e6); hashes of the plugin, four ado/Mata files, the task do-file, driver, aggregator, verdict script,
  the prototype's design code and all 20 exported designs are in the manifest; `run` refuses other sources.
- Command under test: the plain default, `fevc y, worker(worker) firm(firm) deletion(<mode>) backend(rust)
  nativethreads(1) nodisplay`, no inference option, no environment switch. Every fit must report the model
  `repeat_measurement` and the rule threshold 0.10 (inventory criteria).
- Designs: 20 two-block panels of about 48,000 person-years with design seeds 31-50 (none used before), groups
  none 4, weak 4, middle 5, strong 4, signal 3; variances profile / movers / flat / bridge; normal and skewed
  errors; realised regimes computed without outcomes (true curvature 0.005-6.3, true share 0.000-1.0;
  `conf_regimes.json`). Deletion modes match and observation. 1,000 replications per design and mode in 20 tasks of
  50: 800 tasks, 40,000 fits. Random numbers: mt64s, cell seed 2,900,000 + cell index, stream = replication + 1
  (development used 1,100,000 + cell index).
- Tolerances (in the manifest): every design x mode: at least 99% of attempted replications with a fit and an
  interval for all four components; every design x mode x component (sum included): coverage of the reported
  interval at least 0.92, a replication without fit or interval counting as not covered; mean of these coverages
  at least 0.94; in the four designs without a bottleneck: median length of the reported interval at most 1.10
  times that of the normal interval, and e(V) posted in at least 50% of the replications; inventory without
  problems; the package's choice reproduced from the recorded pieces in every replication.
- Verdict by `code/confirm.py runs/conf1` (lists every failing case; refuses to overwrite). A failure stays a
  failure.

## 2026-11-14, 00:15: confirmation conf1 FAILED its registered verdict; diagnosis; conf2 registered

Veneto, final commit fb49e16d, plain default command (`results/e2e-fb49e16d/`, SCC build 8615618, runs
8615619-8615621): all six runs (three panels x two deletion modes) report the normal interval for every component
and post e(V) (largest share 8.1%); the default takes 24-30% longer than the point estimates alone (six-year
panel, 16 threads: 261 s -> 336 s under match deletion).

**conf1: FAIL.** 800 of 800 tasks ran, 40,000 fits, no fit with a nonzero return code; harness files and plugin
matched the manifest at the verdict. `code/confirm.py runs/conf1`: verdict FAIL, one failing criterion: inventory,
"c_J400_b300_g3_profile__observation__15.csv: a fit did not use the model repeat_measurement". The verdict stands
as registered (`runs/conf1/confirmation.json`; the harness as registered is kept in
`runs/conf1/harness_as_registered/`).

Diagnosis (`runs/conf1/repro.do`, `conf1_info.py`, `information_after_failure.json`):

- One fit of 40,000 (design c_J400_b300_g3_profile, observation deletion, replication 761, cell seed 2900019,
  stream 762) returned point estimates only: its standard-error pass stopped with the typed status
  JLA_CONSTRAINT_FAILED, "the mean leave-out variance product of the fitted units is not positive", and the
  default did what it documents: it ran once more without standard errors and displayed the reason. Reproduced
  exactly; under match deletion the same outcome gets intervals.
- Why it can happen there: under observation deletion the model is fitted on the movers' matches of one
  observation (2,836 units in that design). The leave-out product of a unit has a standard deviation of about
  |mean of the outcome, centred| x (v / (1 - P))^(1/2); the design has a gap of 3 between the two blocks, so the
  variance of the outcome's mean part is 5.2 against an error variance of 0.04 (R-squared 0.99). The mean of
  2,836 such products is then only about three standard deviations above zero. In data like the Veneto panels
  (variance of log wages 0.17) the distance is far larger. Frequency so far: 1 in 3,000 fits of the three designs
  of this kind under observation deletion in conf1, 0 in 2,000 in dev1.
- Harness error: the inventory treated a fit without the model name as a corrupt or misconfigured output and
  rejected the whole task. It is an attempted replication without an interval and belongs in the success rate and
  in the coverage (as not covered). The registered criteria did not say so; hence the failure is the harness's,
  not a miss of a coverage tolerance.
- For information only (not a verdict): with that fit counted as a replication without an interval, on all 40,000
  fits: lowest share of fits with intervals in a design and mode 0.994 (tolerance 0.99); lowest coverage 0.937 of
  160 cases (0.92), none below; mean 0.960 (0.94); no-bottleneck length ratio at most 1.059 (1.10); e(V) posted
  in 60% to 97% of the no-bottleneck samples (0.50), 88.6% on average; the sum 0.939-0.971; normal interval alone
  0.76; lowest coverage by group: none 0.944, weak 0.937, middle 0.945, strong 0.961, signal 0.946; recorded
  choice reproduced in every replication.

Changes for the next attempt, with their justification: package, plugin, designs, command, replications and
tolerances unchanged (nothing was tuned). Harness: (i) `pkgmc.check` accepts a fit with status
KSS_POINT_ESTIMATES_ONLY, no model name and no interval as a fallback, and still rejects any other model or a
half-empty row; (ii) the task file records `e(inference_auto_reason)`; (iii) aggregate and verdict list the
fallbacks. Tests: 29 pass (two new); the fallback row reproduced through the real task file on conf1's
replication (`code/fallback_check.sh`); both smokes pass.

### Confirmation conf2: registered before any of its outcomes was drawn

- `pkgmc/runs/conf2`, spec `pkgmc/specs/conf2.json`, manifest sha256
  08427f94de4beb5be35eabb418a9a0fbf8644bb30eb539c42bd52d4a4de9a01f.
- Identical to conf1 (checked on the two manifests): the 20 designs (same files, same hashes), the plugin and the
  four hashed ado/Mata files of commit fb49e16d, the plain default command, 1,000 replications per design and
  mode, the tolerances (fits with intervals >= 99% per design and mode; coverage >= 0.92 in every case, mean >=
  0.94; without a bottleneck length ratio <= 1.10 and e(V) posted >= 50%; clean inventory; recorded choice
  reproduced).
- Different: random seeds (cell seed 4,300,000 + cell index instead of 2,900,000 + cell index: new outcome
  draws), and the harness as described above (hashes in the manifest: task file 7362b30d..., driver 93671bae...,
  aggregate 99040a40..., verdict 9664bf18...).
- A failure stays a failure.

## 2026-11-14, 02:30: conf2 PASSED; check with a control variable; documents committed

**conf2: PASS** (`pkgmc/runs/conf2/confirmation.json`; manifest sha256 08427f94...a01f; harness files and plugin
matched the manifest at the verdict): 800 of 800 tasks, 40,000 fits, no failed fit, no fallback to point estimates,
inventory clean, the package's choice reproduced in every replication. Coverage of the reported interval over the
160 cases: lowest 0.930 (variance of the sum, always normal, match deletion, design c_J400_b1000_g3_movers), mean
0.9593; lowest for the three components 0.935 (covariance, c_J400_b300_g0_profile, match). By group, lowest
coverage normal / q = 1 / reported: none 0.938 / 0.940 / 0.940; weak 0.931 / 0.939 / 0.935; middle 0.538 / 0.955
/ 0.946; strong 0.469 / 0.964 / 0.964; signal 0.931 / 0.940 / 0.940. Sum 0.930-0.967. At most 6 of 1,000
replications of a case without an interval. Without a bottleneck: reported interval at most 1.059 times the
normal one, q = 1 used for 6.2% of components, e(V) posted in 89.1% of samples. Normal interval alone: lowest
0.789. Other thresholds on the same draws (information): 5% 0.938, 20% 0.933, 50% 0.931; curvature > 0.3: 0.931.

Development check with a control variable (`pkgmc/runs/dev2_controls`, harness copy `pkgmc/code_controls/`;
not a confirmation): 4 designs (design seeds 61-64: none, weak, middle, strong) x 2 deletion modes x 500
replications, plain default command with one control under joint nuisance; the control is 0.4 alpha + 0.8 psi +
N(0, 0.25^2), coefficient 0.3 (correlation with alpha + psi 0.55). 4,000 fits, none failed, none without standard
errors; 4.4 s per fit against 3.3 s without a control. Standard error over spread / coverage of the reported
interval: worker 0.90-1.06 / 0.924-0.986; firm 0.86-1.06 / 0.944-0.986; covariance 0.95-1.29 / 0.934-0.992; sum
0.84-0.91 / 0.904-0.920 in all eight cases. Cause: the model treats the estimated control coefficient as known
(`e(inference_nuisance_omitted)` = 1; the owner accepted "treating the control variables as fixed offset"); a
rough calculation of the omitted term, 4 Cov(alpha + psi, x)^2 Var(beta-hat) with Cov = 0.058 and sd(beta-hat) of
about 0.004, gives a variance about a third above the reported one, which matches. The leave-out residual model
has a first-order term for the control coefficients; adding one to the new model (and to the q = 1 pieces) is the
natural next step and needs a derivation.

Post-estimation after the plain default (checked by hand): `estat decomposition|sample|computation|diagnostics`
and `estimates store` work with and without e(V); `lincom`, `test`, `estimates table` work when e(V) is posted and
return r(321) when it is not. Frequency weights give the numbers of the expanded data on the exact route;
`deletionid()` and `targetweight()` are carried and agree across backends; native observation deletion with
frequency weights returns point estimates with a reason.

Commit 9c8154c5 (documents only: help, INFERENCE.md, DECISIONS.md, CHANGELOG.md, PLAN.md; `git diff --stat
fb49e16d HEAD` lists only these five files, so the evidence bound to fb49e16d carries over unchanged): both
confirmation attempts, the two limits (no positive scale when the outcome has almost no noise; controls treated as
known), the owner's decisions and what is left to him. pytest 970 passed / 1 skipped on 9c8154c5.

Branch `repeat-measurement-se`, newest first: 9c8154c5, fb49e16d, 250a760c, 5ebac455, f1a20c3d, 0d3542b2, a45cc26e
(base 53947b43). Nothing pushed.

Open for the owner: (1) threshold 10% (my choice; 20% posts e(V) in 95% instead of 89% of no-bottleneck samples at
0.933-0.935 worst coverage); (2) first-order term for control coefficients in the new model; (3) whether the
simulation harness (now scratch, with machine-local paths and the prototype's design code) should be imported
into the repository; (4) plugins: the distributed binaries predate the method, so until they are rebuilt and
qualified the native default reports point estimates and says why; (5) merge, tag, release. Pre-existing and not
touched: native `inferencemodel(leaveout_residual)` refuses observation deletion when the sample has stayers.
Left on SCC (private directory): `$ROOT/e2e-250a760c`, `$ROOT/e2e-fb49e16d` (source bundle, Linux plugin, logs,
aggregate results; the plugin interface files were removed after each build).

## 2026-11-14, 07:30 — control term implemented (owner: "Ok add the control term. I think we stick with 10 percent for the interval threshold. Harness can go into the repo. Afterwards we want to merge this into main. Build and validate the plugins and release as 0.6.0")

Owner's decisions of this message: threshold stays 10%; harness goes into the repository (done, 29ace86c and
1d19d08a: `fevc/tools/default_interval/`, evidence `fevc/docs/default_interval/`); add the first-order control
term; then merge, plugins, release 0.6.0.

Derivation (memo/review11, review12). Fixed-offset estimator with the noise of the adjusted outcome,
v = Pi' eps - Z Lam omega, covariance S; exact Gaussian variance 4 m'Sm + 2 tr(CSCS). Review round 6 (review11,
ChatGPT Pro, effort 5 of 5, 26 min): the first version's noise corrections were incomplete, the claimed
first-order equivalence of the jointly estimated and the fixed-offset estimator is wrong, the remainder is not
negligible in general; rebuilt on S ("SPEC version 2"). Review round 7 (review12, same model and effort,
13 min 46 s): equations exact; trace identity and the coefficient -2 right; leaving the cross products K out of the
draw's covariance leaves -4 tr(CSCH) - 2 tr(CHCH), lower order for fixed k without a dominant direction (kept as
a listed approximation; repair recorded, not implemented); the one-variance-per-set condition made exact; the
q = 1 remainder needs a centring condition on tr(C^r S). Addendum version 3 (`memo/review12/control_term_v3.pdf`,
6 pages) and RESOLUTION.md record every point. Not implemented, named limits: the joint estimator's extra term
(m_joint - m* = -1/2 M^f [(D^f - D) mu + D^f Z gamma]); variances moving with the controls inside a set; weak
identification of the coefficient (the unbiased variance estimate can be non-positive).

Implementation (commit 02d7955f). Mata routes and native engine; no linear solve added; one further standard
normal per unit and draw under match deletion (own stream; counted in the engine's receipt); without controls
bitwise unchanged natively (74 values frozen as bit patterns). Independent dense oracle by a subagent
(`tmp/ctl/oracle/`): exact route 0.99996-1.00004 of the oracle at 32,768 draws; native within 0.1-0.7%
(observation) and 0.4-3.5% (match); the earlier treatment is 27-55% away on worker variance and sum with the test
control x2. Two repairs of mine on integration: the shim rejected a non-positive fitted variance of the leading
score, which with controls is a legitimate value (the engine reports it as a status); the Mata routes took the
leading-direction share without the control part while the engine used it (now alike, pinned across routes at
10%, measured 1.9% and 3.5%). All oracle comparisons of the regression file are now true relative differences
(Stata's reldif() divides by |y| + 1, so the earlier tolerances were looser than stated; nothing claimed earlier
was wrong, but two nominal tolerances had only passed because of it and are now stated as measured).
Gates on 02d7955f: pytest 1008 passed / 1 skipped; assembly check; cargo fmt, clippy -D warnings, cargo test
684 passed / 0 failed / 4 ignored; shim tests under ASan/UBSan; the two standard-error regression files with the
plugin built from the source (sha256 7614016e...).

## 2026-11-14, 08:30 — controls in simulation, release candidate source, PR

Simulations with controls (harness in the repository, package 02d7955f with the plugin built from it,
sha256 7614016e...). `dev3_controls`: 13 designs x 2 modes x 500 (13,000 fits, none failed, none without an
interval, inventory clean). The four designs and the run seed of dev2_controls give a paired comparison: SE of
the sum over spread 0.84-0.91 -> 0.97-1.04, its coverage 0.904-0.920 -> 0.938-0.956. Nine control variants
(harness extension, commit 23996103: loads, noise, coefficient, second control, variance moving with the control;
every fixture records what it realises): lowest coverage 0.918 = covariance with a control index as large as the
effects (coefficient 1; index sd 0.30, effects 0.35), no bottleneck, match deletion, default joint nuisance (SE
12% short); observation deletion 0.934-0.956. `dev4_fixedoffset`: the same design with nuisance(fixedoffset),
match deletion: SE 0.99-1.07 of spread, coverage 0.948-0.962 -> the shortfall is the joint estimate's extra term
(named limit 1), not the term. Under observation deletion nuisance(fixedoffset) is not covered: all 1,000 fits
returned point estimates with a reason (documented behaviour). `dev4_worker` (control loaded on the worker
effect): 0.932-0.984. Variance moving strongly with the control inside a cell (factor 0.17-2.16): 0.930-0.970.
Little within-match variation (share 0.06): 0.946-0.992. All development, no confirmation.

Release-candidate source on the branch (newest first): 0d565164 development records + guide; bfb0a14d method
documentation (five limits; supplement and help no longer claim first-order equivalence of joint and fixed
offset); 561c3609 version 0.6.0 (distribution date 20261114); 8d3b828f suites and qualifiers under the new
default (switch set where each suite starts; new test_default_inference.do in quick suite, qualifiers, installer
verification; macOS qualifier runs the two SE regression files per thin plugin); 23996103 harness control
variants; 02d7955f control term.

Gates: pytest 1018/1 skipped; quick suite with fresh plugin, with and without the switch in the environment;
`./ci/run_stata_ci.sh quick` on bfb0a14d with the distributed 0.5.2 plugins: success, 532 s of 900 s (receipt copy
in `.local/release-0.6.0/ci-quick-bfb0a14d/`).

Pushed `repeat-measurement-se` and opened draft PR #8 (owner asked for merge and release; hosted Windows build
needs a pushed ref). Dispatched on the branch: windows-test-package.yml (run 43599281449), rust-stata-backend.yml
(run 43599283554). macOS qualification (`./ci/run_ci_profile.sh plugin-build`) started on 0d565164.

Found for the release (details in job tmp `release/RUNBOOK.md`, to be copied to `.local/release-0.6.0/`):
(1) `deploy_linux_bundle.sh` requires HEAD == main; I do not merge (session rule), so the Linux qualification
waits for the owner's merge. (2) The private Windows runner's guard requires the build receipt's export list to
equal its `required_exports.json` (146 names); this source has 149 (three diagnostics exports of the new
methods). A collector change needs the owner's approval and runner reacceptance; proposed file
`.local/release-0.6.0/required_exports.proposed.json`. (3) `build_github_release.py` requires a complete Windows
CMG campaign for the adopted Windows binary; the 0.5.2 protocol compares with 0.5.1 and times plain calls, so a
0.6.0 campaign needs a new protocol (incumbent 0.5.2, timed calls with inference(none)): owner decision.

## 2026-11-14, 09:45 — owner: one way to get standard errors

Owner asked: "what do you mean by: 'inferencemodel(leaveout_residual) keeps the earlier default' What default? I
thought the plan was that we were offering only one way to get standard errors. explain." I explained (0.5.2:
point estimates by default, structured models opt-in; branch Nov 10: leave-out residual default; Nov 13:
repeat-measurement default, leave-out residual kept by name) and asked which models 0.6.0 should offer. His
answer: "Only the default (Recommended)" = remove inferencemodel(leaveout_residual) and the three 0.5.2
structured models (target_lowess, structured_common, structured_leverage) from the interface; a request for one
stops with a message that standard errors are now computed by default; inference(highrank) and inference(q1) stay
as interval overrides of the one method. I also corrected myself: on the branch as built, inference(highrank|q1)
without a model still selected target_lowess. Stopped the macOS qualification at ea46c49f (superseded); conf3
keeps running on the default path (a bitwise check of the default before/after the removal will carry it).

## 2026-11-14, 11:40 — conf3_controls PASS; removal committed; two fixes in progress

dev6_shapes (10 remaining conf2 shapes with controls, 5,000 fits): lowest 0.924; a control loaded on the firm
effect in gap-3 designs makes worker/firm SEs 8-13% short (dev7_adv single control 0.90-0.94; dev7_fixedoffset
0.92-0.99) -> assignment rule of conf3 (firm-loaded kinds only where blocks do not differ), fixed before any
confirmation draw. conf3_controls (manifest fcb4ffa0...93e4; package f3938986 + qualified arm64 plugin
95f67941...): PASS, 40,000 fits, 0 failed, 0 fallbacks, lowest 0.923 (kwrk_J400_b1000_g3_movers match worker,
SE 8% short), mean 0.9602, sum 0.940-0.954, no-bottleneck length ratio <= 1.013, e(V) posted 0.879-0.98.
macOS qualification at f3938986 (before the removal): CLEAN_LOCAL_MACOS_CANDIDATE_QUALIFICATION, artifacts
arm64 95f67941..., x86_64 9bca3c24..., universal 60685e51...; evidence `.local/release-0.6.0/macos-qualification-f3938986/`.
Linux diagnostic on SCC at f3938986 (job 8616992): default, both SE files, quick suite PASS; failed=0 exit 0.
Removal of the alternative models: commit 18ff8f67 (opus implementer; bitwise capture of 19 requests unchanged).
Found: automatic component batch with explicit engine(generic) preconditioner(...) fails in the shim
(declared Gram width must be 1; the residual pass declares 0), and _fevc_rust_abort is private to fevc.ado so
helper ados cannot call it; a second opus implementer is fixing both (plugin changes -> bitwise snapshot of the
default with the new plugin carries conf3; macOS requalification at the final build commit).

## 2026-11-14, 13:30 — release candidate at build commit d4b2d942

Commits since 11:40: 18ff8f67 removal of the alternative models (code, tests); dcdec97d conf3 records;
f8f1338c shim no-Gram automatic batch + fevc__rust_abort.ado / fevc__post_failure.ado (opus implementer);
d4b2d942 documentation pass (opus) + release notes against 0.5.2 + plan = BUILD COMMIT; 80e163c2 evidence records.
Bitwise snapshot (36 default requests, hex): conf3 package+qualified plugin == f8f1338c local build ==
d4b2d942 package + its qualified plugin; repeat run identical. At d4b2d942: macOS CLEAN qualification (arm64
6dc228a0..., x86_64 8df483a3..., universal 766aa624...; evidence .local/release-0.6.0/macos-qualification-d4b2d942/);
Linux diagnostic job 8618959 PASS (plugin 33e2f7ea...; failed=0); hosted Windows package (plugin addb9a4c...,
149 exports), Rust backend, SPI candidates, source checks pass. PR #8 updated. Branch is a fast-forward of main
(cefa00a4, 38 ahead). Owner-gated: fast-forward main to d4b2d942 (not a merge commit), registered Linux
qualification, Windows collector change (exports) + new campaign protocol (incumbent 0.5.2; inference(none) in
timed cells; cell 5 removed model) + 18 runs, adoption commit, tag, release.
