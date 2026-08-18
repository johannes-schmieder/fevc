# Testing and development evidence

## Active local gate

Use the repository interpreter for Python:

```bash
./.venv/bin/python -m pytest kss_bc/tests/python -q
```

Run the Stata/MP suites from the repository root when Stata is available:

```bash
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp \
  -b do kss_bc/tests/stata/run_all.do quick

/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp \
  -b do kss_bc/tests/stata/run_all.do full
```

Run the package gate, including an isolated `net install` smoke test, with:

```bash
./.venv/bin/python kss_bc/tools/run_checks.py
```

Generated `.log` files are transient and ignored. A successful Stata suite
prints `KSS_BC TEST SUITE PASS`; the runner requires that marker because some
Stata launchers return process status zero after a do-file error.

`KSS-NUMOPT-2` adds a matched four-processor P200 benchmark and source-bound
SCC scale matrix after these local gates. It makes no beta, production,
public-release, license, inference, or full-target execution claim.

## Hard acceptance checks

The active tests cover:

- supported inputs, graph selection, deletion semantics, literal frequency
  copies, target accounting, and caller-data/`e(sample)` restoration;
- exact and JLA equality against independent dense or general-engine oracles
  on small designs;
- identification, control-basis conditioning, deletion-rank certificates,
  and typed failures without hidden regularization;
- every accepted JLA right-hand side's complete original worker-plus-firm
  residual at `max(1e-11,10*tolerance())`;
- `total = worker + firm + 2*covariance` and
  `corrected = plugin - correction` for all target rows;
- structural automatic routing before estimator RNG, explicit B1/CMG
  behavior, and typed pre-RNG CMG fallback;
- direct peak allocation against the caller's positive `memory_gib()` value;
  and
- runtime-scoped Stata 18/19 RNG contracts, separate leverage/target domains,
  row-order and batch invariance, and full caller RNG-state restoration.

Exact estimation does not require a production RNG registration. Production
JLA has no 16,383-probe registry cap. The slow K1 candidate comparison is
reusable historical evidence and is not rerun during ordinary development.

`probeorder()` is an optional tie-breaker. Tests require randomized draws to
be stable under row permutation, regrouping, batching, and routing within the
same observed IDs. Arbitrary ID relabeling may change a valid draw, so it is
tested for statistical/numerical validity rather than pathwise equality.

## Structural routing and resource diagnostics

`preconditioner(auto)` makes a structural decision. It uses diagonal B1 for
small or CMG-unavailable structures and CMG when an eligible hierarchy builds.
There are no routing pilot solves, projected-work cutoff, or
`NO_REALISTIC_SOLVER_ROUTE` gate in the installed path. Actual estimator
solves still must converge and pass all complete-residual checks.

The resource model exposes selection, transition, numerical, and restoration
peaks. The direct predicted peak must fit `memory_gib()`. The 30-percent memory
headroom value, 50-percent wall allowance, historical wall model, and batch
percentages remain diagnostics or selection heuristics. They do not withhold
an otherwise scientifically and directly memory-safe command.

## Optional benchmark evidence

Bounded solver and end-to-end benchmarks remain available for engineering
questions. Their timing, iteration, RSS, and route results are advisory. A
performance change should be judged from complete-command measurements and
its implementation/regression cost; there is no universal five- or
ten-percent acceptance threshold.

KSS-PROD-1 and KSS-SCALE-1 reports preserve the source-bound runs, failures,
and receipts that motivated the current design. KSS-SCALE-1 is owner-stopped.
Its fixed ladder, predecessor chain, pilot ritual, 56-GiB/12-hour envelope,
and mandatory connected 4x/P200 job are not active requirements. Never
retroactively revalidate an old run with newer source.

## Optimization III SCC matrix

Optimization III uses `numopt2_generate.do`, `run_numopt2_scale.sge`,
`submit_numopt2_scale.sh`, and `validate_numopt2_scale.py` for deterministic
`W/F=40` cell-density, raw-row, and weak-connectivity rungs. Each estimate is
one Stata process in one scalar job. The generator is a separate sequential
preparation process. Each task freezes dimensions, probes, seed, batch,
resources, source/bundle hashes, and a task hash. Validation requires passing
qacct, application markers, exact dimensions/identities, and every complete
RHS residual.

The local comparison is `numopt2_local.do` plus
`validate_numopt2_local.py`: one cold and three warm P200 runs of each source,
with unchanged dimensions, solver work, estimates, route, and residual gates.

## Historical optional SCC diagnostic

When cluster evidence is useful, submit one scalar SGE job containing one
Stata process. Specify the fixture, copy count, probes, slots, memory per core,
Stata processor count, and wall time for that run. See
`benchmarks/scc/submit_kss_scale.sh` for the exact positional interface.

The submitter, wrapper, driver, and validator are respectively
`submit_kss_scale.sh`, `run_kss_scale.sge`, `kss_scale_driver.do`, and
`validate_kss_scale.py` under `benchmarks/scc/`. Each run is independent:
there is no predecessor receipt and its validation receipt never unlocks a
future run. Scheduler slots and Stata processors remain separate quantities.
The wrapper enforces actual scheduler memory/wall and node-local disk limits.
The deployed `KSS-STREAMLINE-SOURCE-BUNDLE-V1` contains this diagnostic path,
not the retired K1 timing job or MATLAB scale harness.

The validator distinguishes:

- `KSS_STREAMLINE_SCIENTIFIC_PASS`, which requires the scientific identities,
  complete RHS certificates, structural route, direct memory safety, and
  restoration evidence; and
- `KSS_STREAMLINE_DIAGNOSTIC_COLLECTED`, a typed estimator failure whose
  scheduler and wrapper completed normally.

A diagnostic collection is useful evidence, not a successful estimate.
Source and input hashes remain ordinary provenance.

The canonical replicated fixture is `replicated_blocks`; `well_connected` is
accepted only as an input alias. Its reported spectral values describe the
connector copy meta-graph and must be accompanied by connector volume relative
to the full replicated design. `ring` is a separate adverse-connectivity
diagnostic. Neither fixture establishes behavior on a user dataset.

## Historical comparators

Maintained MATLAB LeaveOutTwoWay runs are descriptive timing and
sample-selection comparators. Its legacy finite-projection formula and
language-specific probe schedule preclude corrected-estimate equality. Keep
startup, import, selection, pool/MEX setup, core estimator, serialization,
teardown, wall, CPU, and RSS measurements separate. Preserve an unweakened
failure rather than changing its input or maintained estimator.
