# Return and failure contract

## Point-estimate matrices

The column order of every result matrix is:

1. `worker_variance`
2. `firm_variance`
3. `worker_firm_covariance`
4. `total_variance`

`e(results)` has rows `plugin`, `bias_correction`, `corrected`, and
`numerical_mcse`. The corrected row is also stored in `e(b)` and `e(kss)`.
The component rows are available separately as `e(plugin)`, `e(correction)`,
and `e(numerical_mcse)`. No `e(V)` is posted.

## Main metadata

The command records the requested, complete, initial-component, mover-input,
retained stored-row, and physical-copy counts. It also records worker and firm
levels, parameters, deletion units, target mass, maximum leverage, weighted
RSS, dense-information reciprocal conditioning when available, the
matrix-free Schur-diagonal ratio, the exact low-dimensional control-Schur
reciprocal conditioning, inverse/solver residuals, solver iterations, probes,
batch, seed, and tolerances. Timing scalars separately report graph selection,
fit, matrix-free preconditioner setup (a subset of fit time), leverage-sketch,
target-sketch, and total correction time. Exact mode reports zero
preconditioner time because it uses a dense inverse rather than PCG.

`e(full_parameters)` is the dimension of the preliminary full design.
`e(correction_parameters)` is the dimension of the design used for the
leave-out correction. Under `nuisance(fixedoffset)`, the latter therefore
counts the pure two-way working design after the preliminary full joint fit;
under joint mode the two values agree. `e(parameters)` remains a compatibility
alias for `e(correction_parameters)`.

`e(information_rcond)` is populated only by the dense exact backend.
`e(preconditioner_ratio)` is the minimum-to-maximum Schur-diagonal ratio used
by JLA and is not mislabeled as an information-matrix condition number.
`e(control_schur_rcond)` reports the exact residualized-control Schur
reciprocal condition number prepared by the matrix-free JLA backend. Dense
exact joint mode reports the reciprocal conditioning of its complete
identified information matrix in `e(information_rcond)`. Dense exact fixed
offset reports the minimum reciprocal conditioning of the preliminary full
joint fit and the pure-FE working fit, because both factorizations authorize
the returned conditional calculation.
`e(deletion_rank_gap)` is the probe-independent lower bound from the
within-cell full-fit and deletion-rank certificate. It is populated for every
accepted JLA calculation with controls, including fixed offset. A positive
value certifies that the control block retains rank in the full fit and after
every declared deletion at the registered numerical margin, conditional on
the graph certificate for the FE block. The reported bound already subtracts
the measured whitening residual and a rounding allowance and is no larger
than either the trace lower bound or the smallest directly factored
deleted-scatter eigenvalue.

Graph fields report the initial edge count, articulation workers removed,
largest number of components encountered after pruning, insufficient workers
removed, and pruning iterations. String metadata names the model, correction,
algorithm, deletion unit, nuisance convention, sample-selection convention,
target population, weight conventions, numerical error label, and the absence
of inference.

## Successful status

`e(status)` is `KSS_POINT_ESTIMATES_ONLY`. This means the requested finite
point calculation passed its registered numerical gates. It does not mean the
application's independence assumptions were verified.

## Withholding statuses

Before exiting a recognized failure path, the command sets `e(status)` to
`WITHHELD` and records a typed `e(withholding_status)`. The catalog includes:

- `INVALID_FREQUENCY` and `INVALID_TARGET_WEIGHT`;
- `INVALID_DEPVAR`, `INVALID_CONTROLS`, `INVALID_INPUT`, `NONFINITE_INPUT`,
  `INVALID_TUNING`, `INVALID_TOLERANCE`, `INVALID_NUISANCE`, and
  `INVALID_STAYER_CONVENTION`;
- `UNSUPPORTED_ALGORITHM`, `UNSUPPORTED_DELETION`,
  `UNSUPPORTED_DELETION_ID`, and `UNSUPPORTED_STAYER_CONVENTION`;
- `INVALID_IDENTIFIER`, `CROSS_COORDINATE_MATCH`, and
  `MATCH_INPUT_MISSING`;
- `NO_USABLE_OBSERVATIONS`, `NO_MOVER_SAMPLE`, and
  `NO_LEAVEOUT_COMPONENT`;
- `AMBIGUOUS_LARGEST_COMPONENT` when the registered component ranking ties
  and therefore has no ID-relabeling-invariant winner;
- `AMBIGUOUS_PROBE_ORDER` when outcome and per-copy target mass tie while
  controls differ or rows span nonexchangeable model coordinates or match
  blocks, so fixed-seed JLA has no authorized pathwise ordering; deterministic
  exact mode remains available;
- `SINGULAR_INFORMATION`, `SINGULAR_NUISANCE_BLOCK`, and
  `NONESTIMABLE_DELETION`;
- `INVERSE_FORWARD_ERROR_FAILED` when a dense information inverse is too
  ill-conditioned relative to its recomputed residual for a fail-closed
  Woodbury rank decision;
- `INVERSE_RESIDUAL_FAILED`, `BLOCK_INVERSE_FAILED`, and
  `CONTROL_SCHUR_RESIDUAL_FAILED` when a recomputed dense, block, or
  low-dimensional inverse residual fails its registered numerical gate;
- `UNVERIFIED_DELETION_RANK` when a JLA control design, including a fixed-offset
  full fit, does not satisfy the deterministic sufficient certificate and
  therefore requires exact verification or revised controls;
- `EXACT_SIZE_LIMIT` and `BLOCK_SIZE_LIMIT`;
- `PHYSICAL_COPY_LIMIT` when observation JLA would allocate more literal-copy
  state than `physical_limit()` authorizes;
- `PCG_BREAKDOWN`, `PCG_NONCONVERGENCE`, and
  `SOLVER_RESIDUAL_FAILED`;
- `JLA_CONSTRAINT_FAILED`, `JLA_MOMENT_FAILED`, and
  `JLA_INVERSE_FAILED`;
- `STALE_MATA_RUNTIME` and `INVALID_MATA_RUNTIME` when the loaded semantic
  build token does not match the ado caller;
- `INVALID_GRAPH_INPUT`, `GRAPH_ITERATION_FAILED`,
  `GRAPH_RUNTIME_FAILED`, and `MATA_RUNTIME_FAILED` for typed graph or backend
  execution failures;
- `NONFINITE_FIT`, `NONFINITE_LEVERAGE`, and
  `NONFINITE_CORRECTION`; and
- `STAYER_HYBRID_NOT_IMPLEMENTED` for the deliberately withheld hybrid.

The command never repairs these states through an undisclosed ridge,
different component, changed deletion unit, reduced probe count, or loosened
tolerance.

`STAYER_HYBRID_NOT_IMPLEMENTED` is deliberate. It prevents
`stayers(both)` from being mistaken for a match-robust all-worker variance.
