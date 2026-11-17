# Repeat-measurement standard errors: method note and prototype evidence

This directory holds the note that derived the repeat-measurement standard errors of the four variance
components, its addendum on estimated control coefficients, and the prototype code, aggregate results
and development log behind the note's evidence. The note describes a prototype. `fevc` 0.6.0 implements
the method as its only standard-error method and its default (`inference(auto)`); what the command does is
specified in [INFERENCE.md](../INFERENCE.md), and where the note and INFERENCE.md differ, INFERENCE.md
describes the command. The list of differences is [below](#the-note-and-the-package).

`unified/` combines the note, the addendum, a section on the implementation in 0.6.0 and a comparison
with the standard errors of KSS (2020) and of the KSS Matlab package into one document; the note and the
addendum remain here as they were written.

## Contents

| Path | What it is | Date |
|---|---|---|
| `note/repeat_measurement_se_note_20261113.pdf` | *A Standard Error for Leave-Out Variance Components: Repeat Measurements for Stayers, a Fitted Variance Model for Movers*, the method note (43 pages): estimator, assumptions, propositions with proofs, computation, controls and centring, evidence on a six-year Veneto panel, the `q = 1` interval for one dominant direction, scope | November 13, 2026 |
| `note/main.tex`, `note/s*.tex`, `note/a*.tex` | its LaTeX sources (`main.tex` inputs the sections and appendices) | November 13, 2026 |
| `note/gen/` | the numbers (`numbers.tex`, `q1_numbers.tex`), tables and figure coordinates that the sources input, and plain-text logs of every generated number (`summary.txt`, `detail.txt`, `q1_summary.txt`) | November 13, 2026 |
| `note/gen2.py`, `note/gen_q1.py` | the generators that wrote `note/gen/` from the results: `gen2.py` Sections 2 to 6 and Appendix B, `gen_q1.py` Section 7 | November 13, 2026 |
| `note/gen.py` | the first version of the generator, superseded by `gen2.py` | November 12, 2026 |
| `note/build_appendix.py` | assembled `a1_proofs.tex` from an earlier proofs note (not included) and text for this note | November 13, 2026 |
| `control_term/control_term_v3.pdf`, `.tex` | *Estimated control coefficients in the repeat-measurement standard error*, the addendum (6 pages, version 3): the variance of the fixed-offset estimate with estimated control coefficients, its estimator within the method, the `q = 1` pieces, what is exact, conditional or only checked numerically, and a dense numerical check | November 14, 2026 |
| `control_term/ctl_theory.py`, `ctl_v2.py`, `ctl_estimator.py` | the addendum's numerical check: the dense design and exact variance, the check of the variance formula and of the estimator against it (NumPy, no code shared with the package) | November 14, 2026 |
| `unified/se_unified_20261116.pdf` | *Standard errors for leave-out variance components: the repeat-measurement method, its implementation in fevc 0.6.0, and a comparison with the KSS standard errors* (72 pages): the note's Sections 1 to 7 with the addendum folded into the section on controls, the implementation in 0.6.0 (the differences listed below, the interval rule, the package Monte Carlo runs of [default_interval](../default_interval/README.md)), and the comparison with KSS on the six samples of KSS Table IV and in a simulation of KSS's Monte Carlo design, with KSS's code run unchanged; appendices with the proofs, the simulation details, the addendum's numerical check and the reproduction details of the comparison. Cells marked "stopped" are KSS reruns on the two pooled samples, stopped after 23 hours before their first call had finished; KSS's published values are shown for them | November 16, 2026 |
| `unified/se_unified_20261116.tex`, `unified/s*.tex`, `unified/k_*.tex`, `unified/a*.tex` | its LaTeX sources; they input the note's `../note/gen/` | November 16, 2026 |
| `prototype_simulations/PLAN.md` | the running development log of the prototype and of the implementation that followed, with the owner's decisions | November 11 to 14, 2026 |
| `prototype_simulations/code/` | the prototype (Python with NumPy and SciPy): estimator versions (`lp*.py`; `lp7.py` is the final form of Sections 2 to 6, `lp_q1.py` of Section 7), simulation drivers, aggregators and verdict scripts (`lp_mc*.py`, `lp_agg.py`, `lp_val7.py`, `lp_q1sim.py`, `lp_q1agg.py`), the synthetic designs (`akm_design.py`), the Lanczos search (`lp_lead.py`), local tests (`test_*.py`), run manifests (`manifest_*.json`) and the SCC job and submission scripts (`job_*.sh`, `submit_*.sh`) | November 11 to 13, 2026 |
| `prototype_simulations/results/` | aggregate results: simulation rounds on the Veneto design (`mc_v*_aggregate.*`), one-shot analyses of the Veneto panel (`real_v*.json`), size of the control term (`controls_*.json`), checks with known truth (`check_v*.json`), the model's error at fixed characteristics (`bdelta*`), the validation campaign with its manifests and verdicts (`val_v1/`), the `q = 1` confirmation (`q1_conf1/`) and the `q = 1` calculation on the Veneto designs (`q1_real/`), the cost of the Lanczos search (`lead/`), and the package's end-to-end runs on three Veneto panels with their scripts (`e2e-250a760c/`, `e2e-fb49e16d/`) | November 11 to 13, 2026 |
| `prototype_simulations/dev_auto/` | the after-the-fact analysis of automatic rules on the stored `q = 1` prototype draws (`switch.py`, `switch2.py`, `eigshare.py`, with `switch.json` and `eigshare.json`), and the dense checks for observation deletion (`obs_theory.py`, `oracle_obs.py`, `oracle_obs_q1.py`, `make_panel.do`) | November 13, 2026 |
| `prototype_simulations/runs_q1/` | the manifest and aggregate of the `q = 1` development run `q1-dev1` (200 draws per design), which `gen_q1.py` reads | November 13, 2026 |

The results are kept as they were produced. Some intermediate files were later corrected or superseded,
and the log records it (for example the sign error found in `diag_v1.json`); the note uses the final runs.
The log also names review rounds, fact-checks and scratch files that are not in this directory (see
[What is not here](#what-is-not-here)).

## The note and the package

The note says in its title footnote that the estimator "exists as a prototype and is not part of the
package". Its numbers come from that prototype: exact leverages for stayers, 1,600 random projections for
movers, one direct factorisation of the firm system and 16 pseudo-noise draws. The package implemented the
method afterwards, between November 13 and 14, 2026, and released it in 0.6.0. Against
[INFERENCE.md](../INFERENCE.md), and the Rust source where noted, the implementation differs from the note
as follows.

1. **Deletion modes and repeat-measurement sets.** The note covers FEVC's match-deletion design (a mover's
   match collapsed to one row, a stayer's person-years as rows) and says that stayers are the only sets of
   rows with identical regressors there (Section 8). The package also covers observation deletion: every
   worker-firm cell with two or more observations, a mover's match included, is a set, with the pair
   weight `B/(1-P)` for every target. Mover-only populations are covered too. The only sets are deletion
   units that share a worker-firm cell. Co-mover groups (two or more movers between the same two firms,
   whose wage changes an earlier draft treated as repeat measurements) and cycles of matches appear in
   earlier drafts of the note, which are not included; the final note does not use them, the log lists
   them as deferred (`PLAN.md`, "Steps"), and the package does not implement them.
2. **Fitting the model for the other units.** The two steps are the note's (a scale on the ranks of
   leverage and match size and their squares, floored at 5% of the mean leave-out product; a quadratic in
   the standardised effects capped at three standard deviations; a final floor of `1e-8` times the median
   of `e^2/(1-P)`). The package adds rules the note does not have (`residual_noise/repeat.rs`): each step
   keeps at most one coefficient per hundred fitted units, shortening the basis in a fixed order; a term
   that the others span is dropped, also in a fixed order, where the note withholds the standard error
   (collinear regressors, effects that do not vary); with fewer than a hundred units outside sets the model
   is fitted on all units of movers; the final floor also applies to the sample variance of a set; and the
   pass stops with `JLA_CONSTRAINT_FAILED` when the mean leave-out product is not positive (as the note)
   or when every fitted variance is floored.
3. **Leverages, projections and draws.** The prototype replaces the stayers' projected leverages and
   kernel diagonals by their exact values `1/T_g` and `A_gg/T_g^2` and uses 1,600 projections for movers
   without a finite-projection correction (Section 4). The package takes them from the route: on the
   native route the pass uses those of the point estimate and checks them against its bias correction
   (JLA with 200 projections by default, see [JLA_FINITE_PROJECTION.md](../JLA_FINITE_PROJECTION.md), or
   exact); the Mata `algorithm(jla)` stage draws its own projections. INFERENCE.md describes no
   replacement by the stayers' exact values. The default is eight pseudo-noise draws (`inferencedraws()`),
   not 16, and the linear systems are solved iteratively, not by a factorisation.
4. **Kurtosis.** The pair term is the note's for errors with the kurtosis of the normal distribution. The
   variant for another stated kurtosis (Appendix A) is not offered.
5. **No projection of the covariance.** The prototype's simulation of Section 6.5 set negative
   eigenvalues of the estimated covariance of the three base components to zero, as the note states. The
   package reports the covariance as computed; a variance estimate that is not positive withholds the
   normal interval of that component, and the default then reports its `q = 1` interval where it exists.
6. **Controls.** The note treats the control coefficients as known and leaves jointly estimated controls
   out (Section 5). The package includes their estimation to first order, as derived in the addendum
   (`control_term/`): the variance of the fixed-offset estimate under the noise of the control-adjusted
   outcome, on every route. What it does not cover is listed in INFERENCE.md (the joint estimate's
   additional term in the control index, error variances that move with a control inside a cell, the
   within-unit covariance that the pseudo-noise omits, poorly identified coefficients). The repair
   `q_full` of the addendum's item 1 is, as the addendum says, not implemented.
7. **Centring.** The note treats the sample mean used for centring as a constant. Under `centering(mean)`
   the package centres the pseudo-noise as the outcome, so no fixed-mean approximation is made
   (`e(inference_mean_omitted)` is zero); under `centering(corrected)` the standard error is that of the
   mean-centred estimate, the increment held fixed.
8. **Leading direction.** The note finds it by power iteration (the prototype stopped at 200 steps, and
   one Veneto case did not converge). The package uses a thick-restart Lanczos iteration in the `S` inner
   product (basis six, three Ritz vectors kept, stopped at a relative residual of `1e-3` or after 40
   solves, each search started from the previous target's direction), and a dense eigendecomposition on
   the exact Mata route. The draws of the pass are reused for the remainder: about 90 solves in all with
   the default eight draws, where the pass alone takes 35.
9. **One interval per component.** The note gives the `q = 1` interval for designs with one weak
   direction and a screening value for which "no threshold ... has been validated"; it leaves the choice
   between the two intervals open. The
   package chooses per component: the `q = 1` interval when the leading direction carries more than ten
   per cent of the estimated sampling variance or the high-rank variance estimate is not positive, the
   normal interval otherwise; the variance of the sum always normal; `e(V)` only when every component is
   normal. The statistic and the threshold were chosen afterwards on simulations through the command
   (`dev1`), confirmed on new designs (`conf2`), and the threshold fixed by the owner on 2026-11-14
   ([DECISIONS.md](../DECISIONS.md)). Neither the note nor the package has a theoretical result for this
   data-dependent choice.
10. **The other variance models.** Appendix C compares the cost with the KSS Matlab package, with FEVC's
    structured route under `inference(highrank)` and with the leave-out residual standard error that was
    FEVC's development default at the time (never released). 0.6.0 removed those models (`target_lowess`,
    `structured_common`, `structured_leverage`, `leaveout_residual`); `inference(highrank)` now selects the
    normal intervals of the repeat-measurement method ([Removed in 0.6.0](../INFERENCE.md#removed-in-060)).

The note's own failed tolerances stand: in the validation of Section 6.6 the standard error of the firm
variance in the three-year window was up to 16% too large, beyond the registered 15% (the log records the
owner's acceptance of that outcome), and the `q = 1` confirmation of Section 7 missed its registered length
tolerance in one design without a bottleneck (the interval for the covariance 12.7% longer than the normal
one).

## Prototype evidence and the package Monte Carlo

The evidence in the note was produced by the Python prototype, not by the Stata command: simulations on
the design of the six-year Veneto panel, with the effects fitted on the panel taken as the truth and new
errors in every draw (rounds `mc_v1` to `mc_v6`; the note reports ten scenarios of error variances with
200 draws each); three validation runs with thresholds fixed beforehand, on fresh draws and on two further
designs cut from the same data (`val_v1/`); and, for Section 7, synthetic two-block panels (`q1-dev1` for
development, the confirmation `q1_conf1/` with 27 designs and 1,000 draws each). It speaks to the method;
it does not qualify the package.

The package's own Monte Carlo evidence passes the plain command, with a plugin built from a named commit,
through synthetic panels: the harness is [`fevc/tools/default_interval/`](../../tools/default_interval/)
and the records are in [`fevc/docs/default_interval/`](../default_interval/README.md) (development `dev1`,
the failed first confirmation `conf1`, the passing `conf2`, the development checks with controls and the
passing `conf3_controls`). The two sets are linked in four places:

- The harness's design generator `designs.py` was imported from the prototype's `lp_q1sim.py`, `lp6.py`
  and `akm_design.py`. The manifests of `dev1`, `conf1`, `conf2` and `dev2_controls` hash those three
  prototype files; the copies in `prototype_simulations/code/` have exactly those hashes.
- INFERENCE.md's comparison of thresholds "on earlier stored draws of the prototype (27 designs)" is
  `dev_auto/switch2.py` on the draws of the `q = 1` prototype confirmation (it prints its result, which
  `PLAN.md` records); `switch.py` compares further rules on the same draws and wrote `switch.json`.
- INFERENCE.md's numbers for the three Veneto panels (every component normal, shares 0.03% to 8.1%, 24% to
  30% more time than the point estimates) come from the package runs in `results/e2e-250a760c/` (explicit
  options, development) and `results/e2e-fb49e16d/` (the plain default of the final rule).
- The cost of the Lanczos search on the Veneto designs, which decided that the leading direction is
  computed by default, is `results/lead/` (prototype `lp_lead.py`).

The dense checks in `dev_auto/` (`obs_theory.py`, `oracle_obs.py`, `oracle_obs_q1.py`) are the independent
calculations behind the extension to observation deletion.

## What is not here

- **Raw runs.** The per-replication task files, logs and per-design records of the `q = 1` runs
  (`longpanel/runs_q1/` in the scratch area, 101 MB: 85 MB for `q1-conf1`, 16 MB for `q1-dev1`), the
  package Monte Carlo staging and run directories (`longpanel/pkgmc/`, 1.1 GB; compact records are in
  `docs/default_interval/`), and everything the prototype wrote on SCC beyond the aggregates copied back.
  Only the aggregates of `q1-dev1` and its manifest are copied here; the aggregate, manifest and verdict
  of `q1-conf1` are in `results/q1_conf1/` (identical to the files in the scratch `runs_q1/q1-conf1/`).
  Without the task files and per-design records, `gen_q1.py` and the scripts of `dev_auto/` that read
  them (`switch.py`, `switch2.py`, `eigshare.py`) cannot be rerun.
- **Private data.** The Veneto panels are private and stay on SCC, as do the arrays and sub-panels the
  prototype built from them. The files here hold aggregates only: counts, estimates, standard errors,
  coverage, timings.
- **Review material.** The prompts, responses, resolutions and submission records of the external review
  rounds, and the fact-check reports, are not included: GPT Pro reviews were removed from the repository at
  the owner's request in September ([ARCHIVE.md](../../../docs/ARCHIVE.md)) and that stands. The log and the
  addendum mention the review rounds; the reviews themselves are not here.
- **Drafts and build artifacts.** Earlier versions of the note and their PDFs, the replacement texts and
  helper scripts of the editing rounds, the proofs note that `build_appendix.py` read, LaTeX auxiliary
  files and Python caches.

## Paths and hashes

The files are copied from the git-ignored scratch area of the design work (`.local/se-design-20261110/` in
the main checkout): `memo/final/` is `note/` here (the PDF is `memo/se_note_q1_20261113.pdf`, identical to
`memo/final/main.pdf`), the addendum and its scripts come from `memo/review12/`, and `longpanel/` is
`prototype_simulations/`. Relative paths inside the scripts refer to that layout; for example
`../../longpanel/` in `gen_q1.py` is `../prototype_simulations/` here.

Every file is copied unchanged except for one substitution: machine-local absolute path prefixes are
replaced by placeholders, longest prefix first.

| Placeholder | Stands for |
|---|---|
| `<scratch>` | the prototype's local working directory, `.local/se-design-20261110/longpanel` in the main checkout (the same placeholder as in `docs/default_interval/`) |
| `<session-tmp>` | a temporary directory of the development session on the local machine |
| `<repo>` | the local checkout of this repository |
| `<workspace>` | the directory that contains that checkout |
| `<scc-private>` | the private directory on Boston University's Shared Computing Cluster that holds the Veneto data and the prototype's SCC workspace |
| `<scc-project>` | the project directory on that cluster (Rust toolchains) |

The note, the addendum and their scripts contain no such paths and are byte-identical copies. In
`prototype_simulations/` 36 files carry 62 substitutions. The sha256 of each file before substitution is
below; restoring the prefixes reproduces it. These are the hashes recorded in the prototype's receipts
where it hashed its own scripts (`job_bdelta.sh` in `results/bdelta_v1/source.sha256`, `job_val7.sh` in
`results/val_v1/*/source.sha256`); the scripts with placeholders are records and do not run as they stand.

| File in `prototype_simulations/` | Substitutions | sha256 before substitution |
|---|---|---|
| `PLAN.md` | `<workspace>` 1, `<scc-private>` 3 | `e8c1b6cb89e8e091a05d796b54b6ffb9961c248eeaa7d00f3f72f081cc5bacee` |
| `code/job_bdelta.sh` | `<scc-private>` 1 | `6e7fd51243a1d28aa6b784bce7407182dbff047edefc7c5e63b32d0ecabb515c` |
| `code/job_bdelta_cap.sh` | `<scc-private>` 1 | `2b1db26706a3240f9e9c92d978a94460bca79fcc90ac55926c6d201542ab4614` |
| `code/job_build.sh` | `<scc-private>` 2 | `e2aca0393b450c91abba69ba361ee3a72a2ed8c5301816102b6836b31e43376c` |
| `code/job_diag9.sh` | `<scc-private>` 1 | `4b3e9486fc4fcbed1abe3ad05216a386d42e67126d338f780887e4365dd7f65a` |
| `code/job_lead.sh` | `<scc-private>` 1 | `8330e87a9d3e9ca5ef5bee0222a0bbbcb0a1e6ba339d17dc10088bfb92c096ce` |
| `code/job_mc.sh` | `<scc-private>` 1 | `1098011f78261846ea3215b29e44fe573e8790bc85ce16d07e63cf461b745af6` |
| `code/job_mc5.sh` | `<scc-private>` 1 | `07300618336cc8dbe0730717188274108fcb2c51e45649b58a90babcdde1a929` |
| `code/job_mc6.sh` | `<scc-private>` 1 | `b7c0af3409970ec4800746f90d431304d8b243dd1dec0f100f41cf78d4b1f947` |
| `code/job_q1real.sh` | `<scc-private>` 1 | `f5659a7909343a2508681226906ba8dc05f810763c035195c6d4f1038278cae9` |
| `code/job_real.sh` | `<scc-private>` 3 | `e1cdf4535d73e1312508f026e937be5606653e72cae2f2b530ca3dfeed8dd388` |
| `code/job_real8.sh` | `<scc-private>` 1 | `c71bedcf368c2892f31978185b1bcd658c02b5b2a9aaa92e9a73b92f4f414a29` |
| `code/job_small.sh` | `<scc-private>` 1 | `fb54bc85c2a86818d1b7f5268fb31c7a840e87d946306b4a7a8b109760b73c58` |
| `code/job_val7.sh` | `<scc-private>` 1 | `fc5bb4bbcaa546c5ce327a243a24e33e3befebb9cd325432a55938f42ba9628e` |
| `code/job_valfinish.sh` | `<scc-private>` 1 | `f485bce9423a99392ba3ae762c776c45f537c00643328688354a58fca74d8051` |
| `code/submit_val_stage1.sh` | `<scc-private>` 1 | `f306ecd6ccb67e0d04bc06eb98de0776e51131881265be138c6c0e90b0df881e` |
| `code/submit_val_stage2.sh` | `<scc-private>` 1 | `c7822705e71a87b050c27cba314682ee97903f5c2541e91ee738c34ef11fc5cf` |
| `results/e2e-250a760c/deploy.sh` | `<session-tmp>` 1, `<repo>` 1, `<scc-private>` 1 | `af3151c81213b4c71996e3c2160c4cf37284614e47e327da22463595d3dddb52` |
| `results/e2e-250a760c/job_build.sh` | `<scc-private>` 1, `<scc-project>` 2 | `b78f8d36e05a0de2be73296f530d37192e70980038dafda27ed10f95289a0058` |
| `results/e2e-250a760c/job_run.sh` | `<scc-private>` 1 | `0485814002b5059629d3c25532f5448f8d497ac1d077d75e4d1c3066247a8bbc` |
| `results/e2e-250a760c/local_test.sh` | `<scratch>` 2, `<session-tmp>` 2, `<repo>` 1 | `31ed945c87076fb66505327f91c8410f790c262ea171a0b61488cb103647dab2` |
| `results/e2e-250a760c/patch_e2e.py` | `<session-tmp>` 1 | `11debd53fe3f47542f06e7ca32c6a140119185ef076d5c5335a2120ce47fdd68` |
| `results/e2e-250a760c/patch_e2e2.py` | `<session-tmp>` 1 | `b5cd53441ba1577e943cf1a8f4c4e14171a0aef3ddf6c8f1975b41631321c4d0` |
| `results/e2e-250a760c/wait_jobs.sh` | `<scc-private>` 1 | `0854169670d59e1ff18dbd6b62942dec1586b5536e6da428f6108cd9f4126ff3` |
| `results/e2e-fb49e16d/deploy.sh` | `<session-tmp>` 1, `<repo>` 1, `<scc-private>` 1 | `af3151c81213b4c71996e3c2160c4cf37284614e47e327da22463595d3dddb52` |
| `results/e2e-fb49e16d/deploy2.sh` | `<session-tmp>` 1, `<repo>` 1, `<scc-private>` 1 | `fdb777022d51c3335732cd63a67fb1becf0c0f9b4c12aece7fd7ce8bca10e1a3` |
| `results/e2e-fb49e16d/job_build.sh` | `<scc-private>` 1, `<scc-project>` 2 | `46c41382b5392ba569b33eb91ca00b8b01c1c794994f184cd8b647d503d02d0e` |
| `results/e2e-fb49e16d/job_run.sh` | `<scc-private>` 1 | `8b7ca851d9df6545362fb4e794c9a3bde5d061059e7ce302b8b411e0651a7a44` |
| `results/e2e-fb49e16d/local_test.sh` | `<scratch>` 1, `<session-tmp>` 3, `<repo>` 1 | `ad3b62eacf5323d9db7da049cf67df236622861f54106bb50c2b71b9fe733c22` |
| `results/e2e-fb49e16d/patch_e2e.py` | `<session-tmp>` 1 | `11debd53fe3f47542f06e7ca32c6a140119185ef076d5c5335a2120ce47fdd68` |
| `results/e2e-fb49e16d/patch_e2e2.py` | `<session-tmp>` 1 | `b5cd53441ba1577e943cf1a8f4c4e14171a0aef3ddf6c8f1975b41631321c4d0` |
| `results/e2e-fb49e16d/patch_e2e3.py` | `<scratch>` 1, `<session-tmp>` 2 | `12deda10c052bf05b139a4ece65c9dd8a18aa2ec50179bcdae46a5177e14443a` |
| `results/e2e-fb49e16d/wait_jobs.sh` | `<scc-private>` 1 | `0854169670d59e1ff18dbd6b62942dec1586b5536e6da428f6108cd9f4126ff3` |
| `dev_auto/eigshare.py` | `<scratch>` 1 | `39d24fa94b7d7a69b3037f5572d88f53929b9409f2344ac019b2652df99dbc81` |
| `dev_auto/switch.py` | `<scratch>` 1 | `33b5804d088490f25af114c5ccd7a19019dbb3121829d4e4bd76fd29619c46f7` |
| `dev_auto/switch2.py` | `<scratch>` 1 | `ba248c54da65de0840e7e2c8546b787b63cff2080955b7e7dd523eca9d0f6e10` |

The result files name the SCC compute nodes they ran on; those are left as they are.

## Rebuilding

The PDFs build with pdfLaTeX (TeX Live 2023 was used; the note needs `pgfplots`):

```bash
cd fevc/docs/repeat_measurement/note
latexmk -pdf main.tex            # or pdflatex main.tex, twice; a third pass settles the PDF bookmarks
cd ../control_term
latexmk -pdf control_term_v3.tex
cd ../unified
latexmk -pdf se_unified_20261116.tex   # or pdflatex three times
```

The unified document builds over the committed PDF as well; build it in a copy of `note/` and `unified/`
side by side to keep the committed bytes. Built that way its text is identical to the committed PDF. The
comparison's aggregate results (per-sample estimates, per-draw simulation records, summaries and manifests)
come from runs on private data and are kept outside the repository; Appendix F of the document gives their
sources, versions and hashes.

Built from this layout, both give PDFs whose text (`pdftotext -layout`) is identical to the committed
ones; the bytes differ by build metadata (dates and identifiers). The note builds to `note/main.pdf`, the
committed copy being the dated file; the addendum builds over the committed `control_term_v3.pdf`, so
build it in a copy to keep the committed bytes. The auxiliary files are ignored by `.gitignore`.

The generated files of Sections 2 to 6 can be regenerated from the results here. From `note/`:

```bash
R=../prototype_simulations/results
python gen2.py dq $R/real_v7_dq.json $R/controls_lcs_v3.json $R/mc_v6_aggregate.json \
    $R/mc_v4_aggregate.json $R/mc_v5_aggregate.json $R/check_v6.json $R/mc_v1_aggregate.json \
    $R/bdelta_v1 $R/bdelta_cap_dq_v1 $R/real_v9_dq.json $R/val_v1
```

It rewrites the twelve files `gen/numbers.tex`, `summary.txt`, `detail.txt`, `fig_profile.tex` and
`tab_{bdelta,deciles,real,regime,sim,simcov,val,val_cells}.tex` byte for byte (checked; NumPy is needed).
`gen_q1.py`, which wrote `q1_numbers.tex`, `q1_summary.txt` and `tab_q1*.tex`, reads the per-replication
task files of the `q = 1` confirmation, which are not here; its outputs are committed as generated.

## Licensing

The code in this directory (the Python, shell and Stata scripts, and `prototype_simulations/code/` with
its run manifests) is repository-authored code under MIT, recorded in `REUSE.toml`. The note, the addendum,
the generated parts of the note, the development log and the results are not covered by that grant; they
are a source note with proofs and research records, which [CODE_LICENSE.md](../../../CODE_LICENSE.md)
("Other materials") leaves to their own terms.
