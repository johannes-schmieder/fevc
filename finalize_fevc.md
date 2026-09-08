# Approximate inference: implementation and paper completion

Owner-approved scope, 8 September 2026. Point estimation remains the main
contribution. Complete the practical inference candidate with its known
limitations; do not start another coverage campaign or tune the method.

## Implementation

Use the existing residual-moment fitter for observation and fixed-offset match
deletion. Estimate its small Gram matrix as half the centered sample covariance
of `Z'[(g-Pg)^2]`, with 2,048 Gaussian probes by default. Keep the sample
covariance denominator `R-1`. Remove the subtractive approximate-leverage term
from the current public route. Preserve V1–V3 native semantics through an
internal representation selector and expose new V4 augmentation entrypoints
with an explicit Gram count, reusing the V1 interrupt request and V5 results.

Add `inferencegramprobes(#)` for supported explicit structured Rust component
inference: integer 512 through 2,147,483,647, default 2,048. Reject explicit
use outside that tuple before estimator RNG. Reconcile requested, planned,
executed and returned counts, expose the Gram method, and retain resource and
overflow gates. Do not change the 200 point probes, 1,000 covariance probes,
spectral budgets, critical-value budgets, RNG addresses in other domains,
point corrections, q0/q1 formulas, positivity floor or rank safeguards.
Do not add automatic escalation, ridge fitting, PSD clipping or exact fallback.

## Fixed validation inventory

Add independent same-probe dense covariance checks, weighted-collapse checks,
default/explicit-count equivalence, count/tuple/receipt failures, batch/order
invariance and caller-state regressions. Preserve legacy tests. Run the Python,
CMG assembly, pinned Rust formatting/Clippy/tests, C ABI and local Mac native,
Stata and clean-install gates affected by this boundary change.

Freeze a source-bound manifest before a 70-call engineering replay using
`residual_probe_validation_v1.json`: eight primary cells at replications
0,49,99,149,199 and six diagnostic cells at 0,9,19,29,39. Reconstruct the saved
outcomes with unchanged generators/seeds, 200 point and 2,048 Gram probes.
Account for all 280 target attempts and shared failures. Compare primary
results to the archived 2,048 candidate with scaled numerical tolerance 1e-8
and exact discrete/status agreement. These five draws per cell are software
replays, not coverage evidence. Record compatibility with the archived
1,600-call assessment and independently reaggregate the old 400-draw controls
case by half, explicitly identifying its old fitter.

Run public Stata/native parity for observation q0/q1 and match q0/q1 using
`1e-7 * max(1, abs(native))`. Run the fixed 13-call Veneto inventory: one
matching point-only call and q0/q1 at inference seeds 8675309 through 8675314,
fixed point seed 8675309. Compare the archived 2,048 candidate and report
availability, widths, seed variation and floors.

Numerical disagreement, unexplained point/sample changes, count/state errors
and same-route replay failures block completion and require a software fix.
Existing calibration errors, modest 2,048-probe approximation discrepancies,
conservative coverage and correctly typed unavailability remain limitations.
Historical failed confirmations remain failures. Do not add exact-Gram
accuracy gates or experiments at larger probe budgets.

## Paper and reproducibility

Update the companion paper in its current format, keeping 20–24 main-text
pages and roughly half a page of main inference discussion. Put formulas and
detailed evidence in the supplement. Explain diffuse q0 versus one-mode q1,
the population and mover-only fixed-offset match scope, independence across
matches (including different matches of one worker), omitted control-estimation
uncertainty, variance-model misspecification and the absence of a universal KSS
coverage claim. Describe the practical default and numerical seed sensitivity;
do not suggest selecting seeds or q after seeing preferred intervals.

Describe the exact-Gram diagnosis honestly: it resolves two calibration cases,
not every limitation. Distinguish historical source-specific confirmation from
current bounded development evidence. Synchronize help, diagnostics, examples
and active contracts; do not rewrite source-bound historical reports.

Reuse saved scaling inputs at 15,360, 61,440 and 245,760 rows: point-only,
q0 and q1, three repetitions, 600 seconds per call (27 attempts). Retain
failures/timeouts and keep old million-row results historical. Refresh generated
tables and Stata color/grayscale figures, use humanize for focused prose,
run the complete paper checks, refresh source/claims hashes, build the PDFs
and render/inspect every page before delivering the candidate.

## Completion boundary

Deliver the source-local Mac software checkpoint, source bundle and binary
identities, complete replay/example/scaling inventories, compatibility review,
updated package/paper plans, completion report and final PDFs. This is an
approximate-inference paper candidate, not an all-platform or public release.
Windows/Linux qualification of the new plugin, full private payload validation,
human review, commits, tags, publication and submission remain separate.
