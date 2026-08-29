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
and `e(numerical_mcse)`. Point-only and projection-only calls post no `e(V)`.
An accepted explicit `inference(highrank|q1)` request posts the four-target
component covariance as `e(V)`.

## Opt-in inference matrices

Exact observation-deletion component inference posts `e(V_primitive)` for
worker variance, firm variance, and worker--firm covariance. `e(V)` adds the
total target through the exact linear identity
`total=worker+firm+2*covariance`. `e(component_inference)` stores estimate,
standard error, and ordinary Wald endpoints. `inference(q1)` also posts
`e(q1_inference)` with the high-rank and Anderson--Rubin-style endpoints,
dominant eigenvalue and spectral share, observation-mode concentration,
rank-one covariance terms, F statistic, curvature, and critical value.

Fixed-effect projections post `e(projection_b)`, `e(projection_V)`,
`e(projection_V_naive)`, and `e(projection_results)`. They do not populate the
component `e(V)` unless component inference is requested in the same call.
`e(inference_diagnostics)` records simulations, seed, bins, confidence level,
component and projection PSD cleanups, component and projection variance-proxy
ranges, mover/stayer row counts, and the count of tiny fitted variances set to zero. See
[`INFERENCE.md`](INFERENCE.md) for definitions and limitations.

`e(decomposition)` is the applied additive view. Its rows are
`worker_variance`, `firm_variance`, `sorting_2covariance`, and
`total_worker_firm`; its columns are `plugin`, `bias_correction`, `corrected`,
`plugin_share_outcome`, `corrected_share_outcome`,
`plugin_share_worker_firm`, and `corrected_share_worker_firm`. Unlike the raw
covariance in the established matrices, the sorting row is twice the
worker--firm covariance, so the first three rows add to the fourth. Shares are
stored as proportions and are missing when their denominator is nonpositive.
Negative component contributions remain valid when the denominator is
positive.

## Main metadata

The command records the requested, complete, initial-component, mover-input,
retained stored-row, and physical-copy counts. It also records worker and firm
levels, parameters, deletion units, target mass, maximum leverage, weighted
RSS, dense-information reciprocal conditioning when available, the
matrix-free Schur-diagonal ratio, the exact low-dimensional control-Schur
reciprocal conditioning, inverse/solver residuals, solver iterations, probes,
batch, seed, and tolerances. Timing scalars separately report graph selection,
fit, matrix-free setup, Schur actions, preconditioner
applications, PCG, leverage probes, target probes, the combined correction,
and the solver backend. `e(preconditioner_seconds)` remains a compatibility
alias for `e(setup_seconds)`. Under API 18, setup and fit are disjoint; exact
mode reports zero for iterative fields.
JLA additionally records RHS-equivalent action counts, physical matrix-batch
counts, and `e(solver_rhs_diagnostics)` with stage, batch start, RHS index,
iterations, freshly recomputed complete relative residual, and convergence.
It also posts `e(route_diagnostics)`, `e(preconditioner_requested)`,
`e(preconditioner_selected)`, `e(routing_reason)`, `e(fallback_status)`, and
`e(fallback_message)`. `e(route_api)` identifies structural routing.
`e(memory_gib)` is the declared direct allocation envelope,
`e(batch_requested)` preserves `auto` or the caller's integer, and `e(batch)`
is the selected numeric batch. `e(batch_routing_reason)`,
`e(batch_column_forecast_bytes)`, `e(batch_scratch_forecast_bytes)`, and
`e(batch_memory_budget_bytes)` identify the deterministic choice. Exact mode
labels routing not applicable.
CMG routes expose the fine hybrid vertex/edge counts, hierarchy level count,
and bounded terminal vertex count in `e(route_hybrid_vertices)`,
`e(route_hybrid_edges)`, `e(route_hierarchy_levels)`, and
`e(route_terminal_vertices)`.
Forced CMG never reports a diagonal fallback. Automatic fallback retains the
typed structural CMG boundary that caused diagonal selection. Historical
pilot matrices, pilot iteration scalars, and projected-work ratios are not
part of the active return contract.

`e(full_parameters)` is the dimension of the preliminary full design.
`e(correction_parameters)` is the dimension of the design used for the
leave-out correction. Under `nuisance(fixedoffset)`, the latter therefore
counts the pure two-way working design after the preliminary full joint fit;
under joint mode the two values agree. `e(parameters)` remains a compatibility
alias for `e(correction_parameters)`.

The retained-sample descriptive moments are
`e(target_outcome_variance)`, `e(regression_outcome_variance)`,
`e(residual_variance)`, `e(full_model_explained_variance)`, and
`e(full_model_explained_share)`. The first uses the KSS target mass. The second
uses literal frequency mass; the residual variance is
`e(weighted_rss)/e(N_physical)`, and their difference is the descriptive
full-model explained variance. This full-model quantity includes controls and
is not a KSS correction of control components. When `targetweight()` differs
from frequency weights, the target and regression moments describe different
populations and are not presented as one additive decomposition.

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

Graph fields report the initial deletion-unit edge count, retained edge count,
articulation workers removed, largest number of components encountered,
insufficient workers removed, bridge units and rows removed, bridge passes,
total fixed-point passes, and the final bridge count. A successful match
calculation requires `e(graph_final_bridge_units)==0`. String metadata names the model, correction,
algorithm, deletion unit, nuisance convention, sample-selection convention,
target population, weight conventions, numerical error label, and the
requested inference method or its absence.

## Successful status

`e(status)` is `KSS_POINT_ESTIMATES_ONLY` for point-only general and exact engines
and `KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES` for the compressed engine. Both
mean that the requested finite point calculation passed its registered
scientific, numerical, direct-memory, and restoration gates. The latter name
is an engine-development label, not a scale qualification. Neither status
means that the application's independence assumptions were verified.

Accepted opt-in requests instead use `KSS_HIGHRANK_INFERENCE`,
`KSS_Q1_INFERENCE`, or `KSS_PROJECTION_INFERENCE`, with combined component and
projection variants when both are requested. These statuses mean that the
finite inference calculation passed the registered numerical and capability
gates; they do not verify the sampling assumptions in an application.

## Withholding statuses

Before exiting a recognized failure path, the command sets `e(status)` to
`WITHHELD` and records a typed `e(withholding_status)`. It also records the
technical detail in `e(withholding_detail)`, an applied explanation in
`e(withholding_reason)`, and concrete next steps in
`e(withholding_suggestion)`. The displayed failure block links to the help
file's troubleshooting section. These suggestions explain available remedies
but never silently alter the deletion assumption, tolerance, sample, or
estimand. The catalog includes:

- `INVALID_FREQUENCY`, `PHYSICAL_TOTAL_LIMIT` when the exact literal count
  exceeds `2^53`, and `INVALID_TARGET_WEIGHT`;
- `INVALID_DEPVAR`, `INVALID_CONTROLS`, `INVALID_INPUT`, `NONFINITE_INPUT`,
  `INVALID_TUNING`, `INVALID_TOLERANCE`, `INVALID_NUISANCE`, and
  `INVALID_STAYER_CONVENTION`, plus `INVALID_PRECONDITIONER` and
  `INVALID_MEMORY_ENVELOPE`, `INVALID_WALL_ENVELOPE`, and `INVALID_ENGINE`;
- `UNSUPPORTED_ALGORITHM`, `UNSUPPORTED_DELETION`,
  `UNSUPPORTED_DELETION_ID`, and `UNSUPPORTED_STAYER_CONVENTION`;
- `INVALID_INFERENCE`, `INVALID_INFERENCE_TUNING`,
  `INVALID_PROJECTION_EFFECT`, `INVALID_PROJECTION_WEIGHT`, and
  `PROJECTION_OPTIONS_INCOMPLETE` for malformed inference requests;
- `INFERENCE_DELETION_UNSUPPORTED`, `INFERENCE_STAYER_UNSUPPORTED`,
  `INFERENCE_FREQUENCY_UNSUPPORTED`, `JLA_INFERENCE_UNSUPPORTED`,
  `RUST_INFERENCE_UNSUPPORTED`, and `COUNTER_INFERENCE_UNSUPPORTED` for
  requests outside the initial exact-observation Mata capability;
- `NEGATIVE_INFERENCE_VARIANCE`, `INFERENCE_VARIANCE_INVALID`,
  `INFERENCE_COVARIANCE_NOT_PSD`, `INFERENCE_EIGEN_FAILURE`,
  `INFERENCE_INTERVAL_FAILED`, and
  `Q1_COVARIANCE_INVALID` for nonfinite, nonpositive, singular, or materially
  indefinite component inference;
- `PROJECTION_DESIGN_SINGULAR` and `PROJECTION_COVARIANCE_INVALID` for an
  unidentified projection or an invalid KSS covariance;
- `STALE_INFERENCE_RUNTIME`, `INFERENCE_RUNTIME_NOT_FOUND`,
  `INVALID_INFERENCE_RUNTIME`, and `INFERENCE_RUNTIME_FAILED` for an absent,
  incompatible, or unexpectedly stopped inference runtime;
- `INVALID_IDENTIFIER`, `INVALID_PROBE_ORDER`, `CROSS_COORDINATE_MATCH`, and
  `MATCH_INPUT_MISSING`;
- `NO_USABLE_OBSERVATIONS`, `NO_MOVER_SAMPLE`, and
  `NO_LEAVEOUT_COMPONENT`;
- `AMBIGUOUS_LARGEST_COMPONENT` when the registered component ranking ties
  and therefore has no ID-relabeling-invariant winner;
- `AMBIGUOUS_CONTROL_BASIS` when controlled exact has the same unresolved
  numerical basis problem, more than 32 controls are requested, the dimensioned
  whitening/anchor/span envelope cannot certify a pivot, a pivot score lies
  within its cutoff envelope, or full/deletion conditioning cannot propagate
  the canonical basis within the registered `1e-8` ceiling;
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
- `PHYSICAL_COPY_LIMIT` when any selected JLA route would allocate more
  literal-copy state than `physical_limit()` authorizes;
- `RESOURCE_ADMISSION_FAILED` or `GENERIC_RESOURCE_ADMISSION_FAILED` when an
  unavoidable compressed or general direct allocation exceeds
  `memory_gib()`. A provisional CMG planning forecast, memory-headroom value,
  or wall forecast cannot produce these statuses by itself;
- `SOLVER_MEMORY_LIMIT` when the direct FE design and concurrent solver
  allocation exceed `memory_gib()` before estimator RNG;
- `PCG_BREAKDOWN`, `PCG_NONCONVERGENCE`, and
  `SOLVER_RESIDUAL_FAILED`;
- `FORCED_CMG_FAILED` when an explicitly requested CMG route fails setup or
  execution;
- typed CMG preflight, memory, hierarchy, construction, or application
  failures. Under `preconditioner(auto)`, only structural pre-RNG boundaries
  may produce `FALLBACK_TO_DIAGONAL`; `preconditioner(cmg)` fails closed and
  preserves the original status and message;
- `JLA_CONSTRAINT_FAILED`, `JLA_MOMENT_FAILED`, and
  `JLA_INVERSE_FAILED`;
- `STALE_MATA_RUNTIME`, `STALE_CMG_RUNTIME`, `STALE_SOLVER_RUNTIME`,
  `STALE_RNG_RUNTIME`, `STALE_RESOURCE_RUNTIME`,
  `INVALID_MATA_RUNTIME`, `INVALID_GRAPH_RUNTIME`, `INVALID_CMG_RUNTIME`,
  `INVALID_RNG_RUNTIME`, `INVALID_RESOURCE_RUNTIME`, and
  `INVALID_SOLVER_RUNTIME` when an installed runtime does not match the ado
  caller, plus `RNG_RUNTIME_UNREGISTERED` when JLA is requested on an
  unregistered Stata runtime. Exact estimation does not require that RNG
  registration;
- `INVALID_GRAPH_INPUT`, `GRAPH_ITERATION_FAILED`,
  `GRAPH_BRIDGE_CERTIFICATE_FAILED`, `GRAPH_RUNTIME_FAILED`, and
  `MATA_RUNTIME_FAILED` for typed graph or backend execution failures;
- `NONFINITE_FIT`, `NONFINITE_LEVERAGE`, `NONFINITE_CORRECTION`, and
  `NONFINITE_CORRECTED_TARGET` when the final finite component subtraction
  overflows; and
- `STAYER_HYBRID_NOT_IMPLEMENTED` for the deliberately withheld hybrid.

The command never repairs these states through an undisclosed ridge,
different component, changed deletion unit, reduced probe count, or loosened
tolerance.

`STAYER_HYBRID_NOT_IMPLEMENTED` is deliberate. It prevents
`stayers(both)` from being mistaken for a match-robust all-worker variance.
