# CMG API5 / KSS API18 implementation status

## Outcome

The clean-room API5 core is installed in the KSS API18 package and is a
supported public backend. `kss_bc` routes deterministically among exact, B1,
bounded dense-terminal CMG, and multilevel CMG before consuming estimator
probes. The package, installed CMG path, automatic routing, fixed-point sample
selector, complete residual gates, and hierarchy scale tests pass locally.
KSS production qualification remains open until the source-bound SCC CZ24,
CZ25, full 200-probe CZ18, and larger-than-CZ18 gates complete. The PPML
adapter remains outside this milestone.

## KSS-PROD-1 hierarchy hardening

The canonical Mata template is now API 5 with design label
`clean-room-cmg-inspired-degree3-hybrid-v5-robust-hierarchy`. The dense
terminal cap remains 6,144. API 5 retains the existing screened-forest
aggregation and invokes a deterministic normalized-heavy-edge fallback only
when the screened proposal misses the same 20% reduction bound. The fallback
uses component-contained binary aggregates and exact Galerkin contraction. It
adds no regularization and leaves the fixed one-child symmetric V-cycle,
package operator, and original complete-residual acceptance semantics
unchanged.

The reduction denominator is now component surplus `V-C`. Every committed
nonterminal level still reduces that surplus by at least 20%. The adaptive
depth cap is 96 levels, while cumulative edge complexity remains 3 and
cumulative vertex complexity remains 4. Attempted-level diagnostics survive a
typed construction failure and record the exact hierarchy status/message,
aggregation method, proposed coarse size, reduction, complexities, and
terminal flag.

Source-instantiated Stata 18 tests pass on 257-vertex star/hub, path, and
irregular graphs, a 40-vertex barbell, and a 256-vertex degree-six circulant
expander-like graph. The first reductions measured by the benchmark driver are
0.875 for the star (fallback), 0.75390625 for the path, 0.7734375 for the
irregular graph, and 0.8235294118 for the expander-like graph. The adversarial
test also passes deterministic repeat construction, randomized bilinear
symmetry, positive curvature, the unchanged 6,144/6,145 resource boundary,
and failure-preserving `HIERARCHY_STALLED`, `HIERARCHY_LEVEL_LIMIT`, and
`INVALID_OPTIONS` checks.

Exact low-degree elimination and lifting are deferred. They would require new
solver-validity mathematics and are not needed for this repair. Generated
namespace artifacts are deterministically regenerated and hash-checked; the
installed package ships the KSS namespace and binds both API level and exact
design label.

## Historical CMG-MATA-V1 milestone record

The milestone bullets and API15/API17 narrative below record earlier gates at
the time they ran. Statements there that call CMG forced-only, uninstalled, or
ineligible for automatic routing are superseded by the current outcome and the
KSS-PROD qualification boundary at the end of this document.

- CMG0: plan, subtree governance, source provenance, and baseline evidence are
  recorded. Closure still requires ownership resolution and external review.
- CMG1: the exact Schur/hybrid, Galerkin, quotient-SPD V-cycle, terminal solve,
  and package pullback contracts are documented and independently represented
  in the dense Python oracle.
- CMG2: the bounded directed-arc graph action is selected for Stata 18. API 4
  retains API 2's one row-chunked traversal over every requested RHS and its
  explicit scratch cap. For at least 512 repeated RHSs, at least 16 GiB, and
  no more than 6,144 hybrid vertices, its resource profile may select one
  preflighted exact terminal factor rather than attempt recursive coarsening.
- CMG3: duplicate cell collapse, power-of-two weight normalization, degree-one
  zero contribution, exact degree-two/three clique edges, degree-four-plus
  auxiliary stars, duplicate-edge collapse, and allocation bounds are
  implemented.
- CMG5: strict-key forest selection, high-degree pruning, deterministic capped
  aggregation, conductance screening/splitting, component-aware normalized
  fallback, exact Galerkin contraction, component preservation, hierarchy
  depth, reduction, attempted-level diagnostics, and edge/vertex complexity
  guards are implemented.
- CMG6: constrained Jacobi, symmetric one-child pre/post V-cycle, component
  projection, fixed equilibrated grounded Cholesky, scalar/matrix RHS
  application, KSS quotient pullback, and PPML adjoint grounded pullback are
  implemented. The ordinary recursive apply is the selected Stata 18 path.
- CMG4: KSS true lockstep diagonal PCG is implemented as B1. Five-repeat local
  solver tests show 2.69--3.06x median speedups over scalar B0 on the registered
  10,000-worker easy/moderate/weak cases without changing iteration counts or
  complete residual acceptance.
- CMG8: the generated KSS namespace and a forced test-only adapter preserve the
  KSS operator, quotient, grounding, worker reconstruction, per-RHS status, and
  complete residual gate. The adapter is not shipped in `kss_bc.pkg`.
- CMG9: deterministic Park--Miller pilots, preflight eligibility and memory
  forecasts, fixed work/iteration route gates, typed failures, diagnostics,
  RNG-state preservation, and a local adversarial campaign are implemented.
- CMG10: source-bound Stata 19 forced-KSS jobs pass estimator equality,
  complete-residual, timing, RSS, and scheduler gates on registered moderate
  and weak synthetic graphs and on small, moderate, and natural all-mover
  MATLAB-retained real-data benchmark samples. Easy CMG still fails closed.
- CMG11: one namespace-tokenized source, deterministic generation, manifest
  hashes, reverse-substitution checks, and separate `ppmltalo_cmg__*` and
  `kssbc_cmg__*` artifacts are implemented locally.

CMG7 remains untouched. CMG11 package loader binding and promotion remain
open.

## Mathematical and numerical scope

The fine graph is an exact hybrid realization of the worker-eliminated
two-way-FE Schur complement. The hierarchy uses exact binary Galerkin
contraction. The implemented preconditioner is a fixed symmetric positive map
on each component quotient under the registered one-child V-cycle contract.
It is CMG-inspired; it is not a port or behavioral clone of imported GPL CMG
code and does not implement the full multiple-recursive-call KMT scheme.

The core returns inverse-action candidates only. It does not certify PCG or an
estimator. Each package must preserve its existing Schur action, recurrence
checks, worker reconstruction, and fresh complete normal-equation residual.

## Validation evidence

The final standalone command

```bash
./.venv/bin/python shared/cmg/tools/run_checks.py
```

passes:

- deterministic artifact and reverse-substitution checks;
- 26 Python tests;
- exhaustive connected incidence masks through three workers/four firms with
  unit and single-cell rational perturbations;
- 10,000 fixed-seed random rational-grid cases through five workers/six firms;
- six generic Python--Mata matrix comparisons;
- Mata Galerkin, materialized/randomized symmetry, linearity, curvature,
  extreme scale, disconnected component, relabeling, batch partition,
  workspace reuse, typed failure, and exact RNG-state gates;
- a 20,000-edge degree-20,000 auxiliary hub forced through multiple bounded
  action chunks; and
- independent compilation of both generated package namespaces.

The repository handover gate and complete repository gate also pass. The full
gate includes M2--M13 executable audits, paper and supplement builds, source
audit, and finite verification. Those passes do not prove an asymptotic
theorem and do not qualify CMG as package software.

Local hierarchy path measurements reached one million vertices with 8 RHSs:
18.701 seconds setup, 0.573 seconds per apply, 8 levels, edge complexity
1.333305, vertex complexity 1.333312, and 255,993,032 accounted structural
bytes. These are one-run forecasts/timings, not RSS qualification.

The standalone PCG harness found a concrete weak rescue at 10,000 path
vertices and 8 RHSs: diagonal PCG reached its 500-iteration cap, while CMG
converged in at most 166 iterations with maximum freshly recomputed relative
residual `8.34e-9`. The reusable workspace was correct but about 63--64%
slower than the ordinary apply at 10,000 and 100,000 vertices, so it remains
test-only.

Resource calibration reduced the 10,000-path, eight-RHS solve from 166
iterations / 1.192 seconds with `coarse_max=128` to 79 iterations / 0.608
seconds with 256. Larger scratch caps had no material gain there. A
1,000-vertex barbell regressed by 6.25% at 256, so the resource profile retains
128 below 2,048 vertices. Construction and dense-factor caps may rise to 8 GiB
and 512 MiB under a large declared envelope without weakening pre-allocation
checks.

API 3 added a separate repeated-RHS calibration without weakening the low-RHS
gate. On a 1,285-vertex ring, terminal 1,285 versus 128 changed
setup-plus-CMG-PCG time from 0.343 to 0.401 seconds at 64 RHSs, but from 4.237
to 1.801 seconds at 601 RHSs, a 2.35x gain. Maximum fresh residuals remained
below `5e-11`. The resource profile therefore uses a 512-RHS threshold and a
hard 1,536-vertex cap; the predicted factor at that cap is about 18 MiB and is
checked again against actual components before allocation. See the
[API 3 report](../benchmarks/reports/CMG_API3_REPEATED_RHS_TERMINAL_2026-08-15.md).

The first real all-mover attempt showed that a 1,536-vertex policy cap did not
cover the hybrid auxiliary envelope and therefore retained the same typed
`HIERARCHY_STALLED` failure. API 4 raises only this hard repeated-RHS cap to
6,144, still subject to the memory-derived factor budget. A 2,048-vertex,
601-RHS local ring calibration required 1.264 seconds setup and 4.247 seconds
CMG PCG, versus 84.828 seconds for diagonal PCG, with maximum CMG residual
`4.82e-11`. See the
[API 4 report](../benchmarks/reports/CMG_API4_MEMORY_RICH_TERMINAL_2026-08-15.md).

Forced KSS C versus B1, using 10,000 workers, 1,000 firms, five fresh Stata 18
IC processes, and 200 RHSs, has median setup-inclusive solver speedups of
2.33x on the moderate graph and 16.96x on the weak graph. Source-bound Stata
19 end-to-end estimator jobs with 200 probes record command-level C/B1 gains
of 1.725x and 4.620x, estimator-matrix relative differences of `3.23e-12` and
`2.01e-11`, complete residuals below `1e-10`, and peak RSS below 122 MiB. The
easy expander rejects setup as `HIERARCHY_STALLED`.

The MATLAB-retained fixed-sample SCC ladder then records C/B1 command gains of
2.283x on 11,549 rows, 2.585x on 60,160 rows, and 4.122x on the natural
256,472-row all-mover sample. On the last sample B1/C maximum complete
residuals are `9.998e-11` and `4.052e-13`, estimator `mreldif` is `1.288e-10`,
and peak RSS is 443,140/490,172 KiB. API 4 directly factors the 1,796-vertex,
5,948-edge hybrid in 25,776,200 bytes and solves every one of 601 RHSs in one
iteration. This evidence is test-only and does not enable production routing.

## Files created or updated

- `cmg_plan.md`: owner-authorized implementation and qualification plan.
- `shared/cmg/src/`: canonical Mata implementation.
- `shared/cmg/oracle/`: independent dense Python oracle.
- `shared/cmg/tests/`: Python, cross-language, Mata, namespace, hub, and solver
  tests.
- `shared/cmg/benchmarks/`: kernel, hierarchy, workspace, and solver drivers,
  raw rows, and source-bound reports.
- `shared/cmg/tools/`: deterministic assembler and local gate runner.
- `shared/cmg/generated/`: test and package-specific standalone generated
  artifacts plus manifest.
- `shared/cmg/docs/`, `shared/cmg/state/`, `shared/cmg/README.md`, and local
  governance files: contracts, evidence boundaries, and milestone status.

The owner-authorized KSS optimization milestone updates `kss_bc/` with B1,
the test adapter, tests, benchmarks, and evidence. No file under `ppml_talo/`,
`paper/`, `theory/`, `proof-audit/`, `state/`, `archive/`, `application/`, or
`software/` was edited. Existing unrelated untracked files were preserved.

## Historical unresolved objections and next step

- Two requested independent ChatGPT Pro reviews remain `browser_blocked`; no
  external mathematical or code finding was received.
- No named human independent review exists. No milestone is
  `independently_checked`.
- Automatic-route overhead and 5m/10m scale remain untested. Stata 19
  end-to-end forced-C estimator equality, complete residual, timing, and RSS
  gates pass on the registered moderate and weak synthetic graphs and the
  bounded MATLAB-retained real-data ladder. Easy CMG fails closed.
- Runtime package/hash loader binding is not implemented.

Keep diagonal B1 as the public default. Do not implement automatic routing:
the easy graph, installed-route, and production no-regression gates do not
pass. A PPML adapter remains a separate owner-authorized task.

SCC run `20260815T142657Z-3ac4abe` contains the accepted Stata 19 forced-C
easy/moderate/weak evidence. The earlier 5,000-worker B1 smoke took 12.54
seconds and 85,568 KB peak RSS; all 123 RHSs passed complete residual checks.
Medium job `7185654` passes in 1,580 seconds with 432,284 KB peak RSS and all
303 RHS residuals accepted. The observed scaling does not justify B1 large.
No CMG automatic route or B1 large job was submitted.

## API-15 KSS end-to-end follow-up

The forced KSS adapter no longer duplicates the PCG algorithm. API 15 exposes
one internal solver-backend callback used by diagonal B1 and forced test-only
CMG. Both routes therefore use identical quotient projection, column-specific
recurrences and stopping, explicit-residual restarts, post-convergence
grounding, worker reconstruction, typed failure propagation, and fresh
complete worker-plus-firm residual checks. The installed package still loads
only diagonal B1 and exposes no preconditioner option.

A public-ado end-to-end test holds the retained sample, seed, probe stream,
tolerance, formulas, and RNG end state fixed and passes at 121 RHSs. Bounded
local Stata 18 benchmarks at 10,000 workers, 1,000 firms, 200 probes, seed
`8675309`, and tolerance `1e-10` give command-level C/B1 speedups of 1.35x on
the moderate graph and 3.60x on the weak graph. Estimator-matrix relative
differences are `3.24e-12` and `2.01e-11`; maximum complete residuals remain
below `1e-10`. Easy CMG fails closed as `HIERARCHY_STALLED`.

Stata 19 and comparative synthetic RSS gates pass on moderate and weak. In the
read-only Separations CZ24 wage ladder, the genuine 500-worker dense core lets
CMG build a hierarchy but B1 and CMG both reject the same nonestimable match
deletion before posting estimates. The larger all-eligible-mover graph makes
CMG fail hierarchy construction and B1 reaches the same estimator rejection.
The separately checksum-bound MATLAB reference passes on its own smaller
maintained leave-one-out set after both MEX families are built below the KSS
run directory. It is descriptive, not estimator-equality evidence. The API 15
failures block full-input scale-up and automatic routing. Every estimator job
remains subject to a measured projection no greater than 90 minutes and an
independent 5,400-second timeout.

## API 17/API 4 real-data closure

API 17 repaired the exact and JLA within-match projection calculation exposed
by the first MATLAB-retained attempts. The small exact oracle now passes
before either iterative route. API 4 then raises the memory-rich repeated-RHS
terminal cap to 6,144 hybrid vertices while retaining the 512-RHS, 16 GiB,
predicted-factor, and pre-allocation checks. The source-bound all-mover result
reported above completes CMG10's bounded KSS real-data evidence. It does not
change the public selector, estimator formulas, probe stream, seed, tolerance,
or failure policy, and it does not promote CMG into the installed package.

## Current KSS-PROD qualification boundary

API18 ships `kss_bc_cmg.mata` and `kss_bc_solver.mata` through the normal
package manifest. Normal `net install` followed by public forced-CMG
estimation passes on local Stata/MP 18. The public route reports the selected
backend, reason, fine hybrid dimensions, hierarchy levels, bounded terminal
vertices, pilot iterations, per-RHS complete residuals, disjoint timings, and
additive batch/solver memory forecasts. Setup failure can fall back only before
the registered probe stream and only after bounded B1 iteration, residual, and
deterministic-work gates.

API5 scale tests pass on adversarial star, path, barbell, irregular, and
expander-like graphs. At 32,768 vertices and 66,484 edges the hierarchy used
eight levels, reached a 92-vertex terminal, forecast 79.82 MiB, built in 7.025
seconds, and passed symmetry and workspace-equality gates. The reusable
workspace was 34.4% slower at batch 4 and 73.7% slower at batch 16, so the
production adapter uses ordinary batched applications.

The SCC harness deploys one approximately 141 KiB allowlisted source bundle
per clean commit. Its staged plan verifies installed CMG under Stata 18/19,
exact four/eight-processor binding, pure-Stata CZ24/CZ25/CZ18 preparation,
exact MATLAB retained-match comparison on CZ24/CZ25, automatic and forced CMG
calibrations, a full 200-probe CZ18 multilevel route, and a separately
calibrated full 200-probe graph larger than CZ18. Those remote gates are still
pending in this local candidate and must not be inferred from the local scale
tests.

No named human independent review exists. The runtime is pure Stata/Mata and
adds no MATLAB or native-library dependency. Public distribution remains
blocked by the repository's unresolved software-licensing decision; the SCC
qualification itself does not grant a public license.
