# KSS MATLAB Package: Publication and Current Versions

## Scope

There are three relevant snapshots, not two:

1. The official Econometrica **Data and Programs** archive distributed with the
   2020 article. The archive copy inspected here has SHA-256
   `02ada29f89b90f94026bc10ffca491e9ac4a21b4f9c031bbd8032bd6da38ee24`.
2. The GitHub `master` branch as it stood when the article was published on
   2020-09-25. Its last preceding commit was
   `521ed9a2b84494c4067381a9bd713924c4df87f0` from 2020-02-01.
3. The current GitHub head inspected on 2026-08-29,
   `8b957ffeb10b8465a3584fceb0265cccc48379e1`, dated 2024-10-17.

## Current VCkss Benchmark Pin

VCkss currently benchmarks against snapshot 3: the maintained version-3
GitHub implementation at
`8b957ffeb10b8465a3584fceb0265cccc48379e1`. It does **not** use the official
2020 journal archive as its active performance comparator.

The checked-in benchmark contract binds the comparator more narrowly than the
commit alone:

| Bound object | Registered identity |
|---|---|
| Upstream repository | `https://github.com/rsaggio87/LeaveOutTwoWay` |
| Upstream commit | `8b957ffeb10b8465a3584fceb0265cccc48379e1` |
| Runtime roots | `codes/` and `CMG/` |
| Runtime file count | 184 |
| Runtime-tree SHA-256 | `7d7581e77bcea131d0041cf7bab2d7a462fd5d535ca22110d080da51ded4f192` |
| Main routine | `codes/leave_out_KSS.m` |
| Main-routine SHA-256 | `7ab72bcf1f9e1a0091a6a423b1ef5cbd23688f7c64d753cf9adcc6243989a120` |

These identities are enforced by
`vckss/benchmarks/matlab_scale/source_contract.json` and reused by the
source-bound performance evidence. The maintained MATLAB results are treated
as descriptive cross-package comparisons because VCkss and MATLAB differ in
RNG, tolerances, and the finite-projection mixed-fourth-moment coefficient.

The first two are not identical. The journal archive contains a later
`leave_out_COMPLETE` version 2.2 and an additional improved routine, while the
publication-date GitHub tree contains an earlier version 2.15-era legacy
routine. Consequently, the journal archive is the appropriate source for
reproducing the publication, while the exact GitHub commits are the appropriate
sources for repository history and current-comparator work.

## Main Finding

The current package is a post-publication version-3 redesign, introduced in
January 2021. It is optimized around a simpler point-estimation workflow:
match deletion by default, automatic exact-versus-random-projection selection,
an improved leverage algorithm, built-in MATLAB graph functions, and a much
smaller distribution. It is not a strict superset of the publication package.
The new main entry point no longer exposes the publication routine's standard
errors for variance components, weak-identification diagnostics, Andrews
correction, flexible control-estimation modes, or mover-only switch. The
current repository retains much of the legacy code alongside the redesigned
main routine.

## Detailed Comparison

| Dimension | Official 2020 journal package | Current GitHub package | Practical consequence |
|---|---|---|---|
| Intended role | Full replication bundle for the article's empirical application, including Stata build scripts, MATLAB analysis, and general routines | Maintained, reusable MATLAB estimator package with examples and documentation | Use the archive for article replication; use the current commit for present-day computational comparison |
| Main general routine | `leave_out_COMPLETE.m` version 2.2; the archive also contains an improved `leave_out_KSS.m` variant | Redesigned `codes/leave_out_KSS.m` version-3 line | The same broad estimator family is exposed through materially different interfaces and defaults |
| Interface | Fifteen positional inputs and six outputs; the API exposes year, control handling, Andrews correction, diagnostics, smoothing, mover restriction, standard errors, algorithm, tolerance, and output filename | Eleven positional inputs, only `y`, `id`, and `firmid` mandatory; three variance-component outputs | The current common case is much easier to call, but specialized publication-era options are not reachable through the new main function |
| Default deletion unit | Observation deletion is the documented baseline; match deletion is explicitly selectable | Worker--firm match deletion is the default | Running both packages without harmonizing options changes the estimand and sample transformation |
| Match handling | Supports match deletion and cluster-robust variance-component inference at the match level | Collapses each worker--firm match to its mean and uses match length as the FGLS weight before estimation | The current implementation makes match deletion first-class and targets micro-observation-weighted moments after collapsing |
| Exact versus approximate calculation | Exact or Johnson--Lindenstrauss calculation selected explicitly; approximation size is tolerance-driven, with an improved archive variant accepting an explicit simulation count | Exact for samples up to 10,000 observations and JLA above 10,000 by default; default is 200 simulations | Current behavior can change automatically with sample size; reproducible comparisons must set the algorithm and probe count explicitly |
| Random-projection leverage method | Earlier effective-resistance/JLL construction; the archive's improved routine uses Rademacher projections with an explicit scale | Dedicated `leverages.m` estimates `Pii` and `Mii` jointly, normalizes them to sum to one, estimates required fourth moments, and applies nonlinear bias corrections | The current approximation is more numerically constrained and computationally focused, so identical random seeds or probe counts do not imply identical finite-simulation paths |
| Controls | Supports joint estimation, one-time residualization, or omission of controls from estimation while retaining them in the data | Estimates controls once with worker and firm effects on the retained sample, then partials them out before the two-way correction | The current new main corresponds to a fixed-offset/partialled-out nuisance workflow, not the archive's full set of nuisance specifications |
| Variance-component point estimates | Firm-effect variance, worker--firm covariance, and worker-effect variance | The same three headline point-estimate categories | Point-estimate benchmarking is the strongest common denominator, conditional on matched sample, weights, controls, and deletion unit |
| Variance-component inference | Returns standard errors; supports match-cluster dependence, local-linear-regression smoothing, Hutchinson calculations, conservative alternatives, eigen/Lindeberg diagnostics, and weak-identification confidence procedures | The new main returns no standard errors for the three variance components and exposes none of those diagnostics | The maintained MATLAB main is not a drop-in replacement for the publication package's full inferential workflow; VCkss 0.5 independently restores a bounded exact-observation subset |
| Andrews correction | Optional homoskedastic Andrews correction | Not exposed by the new main | Publication tables using that correction require the legacy/archive route |
| Mover and stayer treatment | Explicit `restrict_movers` option; publication code can focus on the mover-identified sample | No mover-only switch in the new main; match-out person-effect calculations use a separate stayer routine and the documentation discusses the extra assumptions and bounds relevant to stayers | Person-effect variance comparisons need special care; match-out identification is cleanest among movers |
| Second-stage firm-effect regression | General `lincom_KSS.m` machinery is included in the archive | Optional `lincom_do` runs a firm-effect regression on one or more covariates with KSS and White standard errors | The current package adds an easy second-stage path even though it removes first-stage variance-component standard errors; a 2021 patch fixed multiple-covariate match-out use |
| Connectivity | Depends on the external/vendored MATLAB BGL graph library | Uses MATLAB's built-in `graph`, `conncomp`, and `biconncomp` functions | Installation is substantially simpler and the repository is much smaller |
| Linear solvers | CMG-supported sparse exact/JLL workflow | CMG plus PCG/preconditioner retry logic and the redesigned random-projection module | Both require careful solver control, but the present package has additional post-publication robustness fixes |
| Files written | Saves several MATLAB workspaces/matrices plus a CSV diagnostic output | The current code writes a tab-delimited `.csv` containing original outcome/IDs and leverage; its header also mentions a `.mat` output that the function does not actually save | Downstream scripts written around archive `.mat` files require adaptation; current comments and behavior are not perfectly aligned |
| Documentation | Replication README, article analysis scripts, and general-function comments; requires separate dependencies | Expanded README and vignettes for the main estimator and firm-effect regressions | The current estimator is more approachable as a standalone package |
| MATLAB/dependencies | Archive instructions target MATLAB 2018a or newer and require MATLAB BGL and CMG | README states MATLAB R2015b or newer; CMG remains required, and the implementation uses parallel constructs | BGL is gone, but CMG and a functioning MATLAB parallel/solver environment remain operational dependencies |
| Versioning and license metadata | Version labels appear in files/README rather than formal releases; no clear root package license grant | GitHub has no releases and reports no repository license; an open license issue remains | Cite and archive exact commits rather than an informal package version, and complete a separate human provenance/license review before redistribution |

## Repository-Level Delta

For an apples-to-apples comparison of the GitHub tree at publication commit
`521ed9a2` with current commit `8b957ffe`:

| File-level result | Count or finding |
|---|---|
| Paths in publication-date tree | 609 |
| Paths in current tree | 209 |
| Shared paths | 154 |
| Shared paths byte-identical | 149 |
| Publication-only paths | 455, dominated by roughly 450 vendored MATLAB-BGL files and older examples |
| Current-only paths | 55, including `leave_out_KSS.m`, `leverages.m`, stayer/bounds helpers, `installCMG.m`, and new documentation/data |
| Important legacy fact | The current tree's `leave_out_COMPLETE.m` is byte-identical to the February 2020 GitHub version, but it is not the later version 2.2 routine in the official journal archive |

This topology shows that the current repository is partly additive: it keeps
the old GitHub routine and many supporting files, removes the bundled graph
library, and adds a new version-3 front end and leverage engine. It does not
simply update the old entry point in place.

## Version Timeline

| Date | Commit/event | Material change |
|---|---|---|
| 2020-02-01 | `521ed9a2` | Last GitHub commit before publication; version-2-era package |
| 2020-09-25 | Econometrica publication | Article and official Data and Programs archive published; archive code differs from the February GitHub snapshot |
| 2021-01-09 | `5ac67092` | Version 3 introduced as a completely redesigned MATLAB package |
| 2021-01-14 to 2021-01-21 | versions 3.02, 3.1, and 3.1.1 | Improved routines/documentation, explained-variation output, and incomplete-Cholesky handling |
| 2021-03 | several commits | Additional incomplete-Cholesky retry, drop-tolerance, and diagonal-compensation fixes |
| 2021-09-29 | `dec8cd3` / merge `1e59459` | Fixed `lincom` for match deletion with more than one regressor |
| 2024-10-17 | `8b957ffe` | Latest code change inspected; keeps connected-set adjacency sparse |

## Implication for VCkss

The maintained current commit is the appropriate primary computational
comparator for VCkss point estimates, speed, and memory because it represents
the workflow contemporary users would run. The official 2020 archive remains
the appropriate comparator for exact article replication and the richer
publication-era inference surface. VCkss 0.5 adds an independently implemented,
opt-in subset of that surface for unit-weight exact observation deletion:
high-rank component covariance, q=1 weak-identification intervals, and
fixed-effect projections. It does not implement the archive's match-cluster,
JLA, conservative, or full diagnostic workflows. Results should not be
labeled equivalent without fixing the deletion unit, connected sample,
control treatment, target weighting, exact/JLA route, simulation count, and
random seed or probe stream.

The VCkss paper currently pins the maintained comparator at
`8b957ffeb10b8465a3584fceb0265cccc48379e1`; that is preferable to referring to
an untagged "latest version."

## Primary Sources

- [Econometrica article and Data and Programs supplement](https://onlinelibrary.wiley.com/doi/abs/10.3982/ECTA16410)
- [Publication-date GitHub snapshot](https://github.com/rsaggio87/LeaveOutTwoWay/tree/521ed9a2b84494c4067381a9bd713924c4df87f0)
- [Current GitHub snapshot](https://github.com/rsaggio87/LeaveOutTwoWay/tree/8b957ffeb10b8465a3584fceb0265cccc48379e1)
- [Version-3 redesign commit](https://github.com/rsaggio87/LeaveOutTwoWay/commit/5ac67092d1fd86af3c78e7b75b520cedfbe3ee1a)
- [Full publication-to-current GitHub comparison](https://github.com/rsaggio87/LeaveOutTwoWay/compare/521ed9a2b84494c4067381a9bd713924c4df87f0...8b957ffeb10b8465a3584fceb0265cccc48379e1)
