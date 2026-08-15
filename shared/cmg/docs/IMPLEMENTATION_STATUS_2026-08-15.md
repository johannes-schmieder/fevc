# CMG-MATA-V1 implementation status

## Outcome

The clean-room core is implemented through local-candidate status. A forced
test-only KSS solver adapter now passes local gates, but the core is not
installed, automatically routed, or qualified for production in either
`ppml_talo` or `kss_bc`.

## Milestones and obligations addressed

- CMG0: plan, subtree governance, source provenance, and baseline evidence are
  recorded. Closure still requires ownership resolution and external review.
- CMG1: the exact Schur/hybrid, Galerkin, quotient-SPD V-cycle, terminal solve,
  and package pullback contracts are documented and independently represented
  in the dense Python oracle.
- CMG2: the bounded directed-arc graph action is selected for Stata 18. API 2
  processes every requested RHS column in one row-chunked traversal and derives
  the row chunk from an explicit scratch cap. The resource profile may raise
  that cap to 1 GiB under a declared memory envelope.
- CMG3: duplicate cell collapse, power-of-two weight normalization, degree-one
  zero contribution, exact degree-two/three clique edges, degree-four-plus
  auxiliary stars, duplicate-edge collapse, and allocation bounds are
  implemented.
- CMG5: strict-key forest selection, high-degree pruning, deterministic capped
  aggregation, conductance screening/splitting, exact Galerkin contraction,
  component preservation, hierarchy depth, reduction, and edge/vertex
  complexity guards are implemented.
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
- CMG11: one namespace-tokenized source, deterministic generation, manifest
  hashes, reverse-substitution checks, and separate `ppmltalo_cmg__*` and
  `kssbc_cmg__*` artifacts are implemented locally.

CMG7 remains untouched. CMG10 requires Stata 19/SCC access. CMG11 package
loader binding and promotion remain open.

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

Forced KSS C versus B1, using 10,000 workers, 1,000 firms, five fresh Stata 18
IC processes, and 200 RHSs, has median setup-inclusive speedups of 2.33x on the
moderate graph and 16.96x on the weak graph. The easy expander rejects setup at
the hierarchy edge-complexity cap. This rejection and missing end-to-end,
comparative RSS, and Stata 19 evidence prevent automatic routing.

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

## Unresolved objections and next step

- Two requested independent ChatGPT Pro reviews remain `browser_blocked`; no
  external mathematical or code finding was received.
- No named human independent review exists. No milestone is
  `independently_checked`.
- End-to-end forced-CMG estimator equality, automatic-route overhead,
  comparative package RSS, 5m/10m scale, and Stata 19 portability remain
  untested.
- Runtime package/hash loader binding is not implemented.

The next safe step is a source-bound short KSS portability/smoke run, followed
by accounting inspection. Keep diagonal B1 as the public default. A PPML
adapter remains a separate owner-authorized task.

The KSS/CMG candidate is frozen at commit
`b2ef752684a7f5267aa09e6979700571b5e1b9c0`. SCC run
`20260815T121809Z-b2ef752` has portability job `7185628` queued behind the
unchanged B0 large job; no CMG automatic route or long SCC job was submitted.
