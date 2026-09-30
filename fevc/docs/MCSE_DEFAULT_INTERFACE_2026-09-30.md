# Default numerical MCSE and native V2 interface

The owner-authorized September 30 interface decision supersedes the opt-in
frontend clauses of the [September 29 derivation](ALL_PROBE_MCSE.md). Its
mathematics, estimands, finite-R formula, scientific gates, RNG semantics and
source-bound historical evidence remain unchanged.

## Public behavior

- `mcse(all|off)` is the public option, default `all`. `numericalmcse()` remains
  a compatibility alias. Both names together are invalid before estimator RNG.
- `mcse(conditional)` remains accepted for development. It reports the earlier
  target-probe MCSE conditional on the realized leverage sketch.
- All-MCSE covers the four main point estimates. Supported projection and
  sampling-inference requests may accompany it; their coefficients, covariance,
  intervals and extra simulations receive no MCSE.
- `mcse(off)` skips additional derivatives, target folds, leverage scores and
  replay. The point estimator's existing reductions and finite-output checks
  remain. Public numerical MCSE is unavailable/off, never reported as zero.
- Exact point estimates have numerical zero covariance and no replay.
- Point estimates, conditional calculations, sample, route, literal batch
  intent, RNG contracts, caller state and scientific gates remain fixed.
  Numerical covariance is never sampling `e(V)`.

The main display and `estat diagnostics` report approximate all-probe MCSE and
availability. `e(numerical_mcse_all)` holds four values; primitive conditional,
leverage, signed raw and usable covariance matrices remain separate.
`e(mcse_mode)` records intent. The developer conditional return retains its
legacy meaning; raw signed covariance is preserved when usable all-MCSE is
withheld. A withholding status does not silently substitute conditional MCSE.

## Native boundary and compatibility

Numerical V2 advertises `numerical_api=2`. The 344-byte request extends the
frozen 328-byte numerical V1 prefix with original executor, component-batch and
full-CMG intent. It carries prepared projection/component flags and validates
them before solve. Numerical V1 keeps its old point-only platform restrictions.
The 512-byte numerical result and 24-byte replay RHS layouts are unchanged.

The 176-byte combined work receipt includes the frozen 160-byte V1 point-work
prefix and separate replay RHS count. The prefix excludes replay; the native
boundary reconciles actual queue/direct/CMG work against point plus replay.
The old V1 work getter continues to reject numerical-attached contexts, so old
consumers cannot misinterpret additional work. Legacy compressed V2 and generic
V3 point requests have additive numerical V2 solve selectors consuming their
unchanged interrupt requests and returning unchanged point receipts.

Windows enables numerical V2 on its existing supported point routes. This is
an implementation fact, separate from exact-artifact platform qualification;
existing solver, projection and inference platform restrictions remain.

With an older plugin, an implicit default preserves point routing and reports
`unavailable_capability`. Explicit all fails before preparation/RNG with
`STALE_NUMERICAL_RUNTIME`. MCSE capability does not cause a different backend,
route, estimator, conditional substitution or post-RNG fallback.

## Cost, memory and qualification

All-MCSE requires bounded derivative/fold state and replays original leverage
directions using the prepared solver. Rust replay tiles are at most four and
Mata tiles at most eight, limited by the admitted point width. Storage is linear;
no production observation-by-probe cache is retained. A strict budget can admit
point/off and reject all; admission and residual gates are not relaxed.

Focused checks and fresh prescribed calibration, complete-command timing and
exact-artifact platform qualification are recorded in the companion development
report when complete. Prior receipts qualify only their recorded sources.
No release, tag, CI-policy change or large cluster campaign is authorized by
this interface decision. The owner selected hosted Windows builds plus private
qualification; source publication is a separate approval step.
