# External proof-review request: KSS repaired implementation adversarial counterexample review, independent review D

You are an adversarial mathematical reviewer. Reconstruct and challenge the supplied proof step. Do not assume the author's conclusion. Distinguish an exposition omission from a genuine missing argument.

## Milestone and obligation

- Milestone: KB5
- Obligation: KB5-DELETION-ESTIMABILITY / KB5-FREQUENCY-JLA / KB5-BLOCK-CONTROL-CALLERS
- Critical: True

## Question

Starting from the supplied production caller rather than the prose conclusion, try to make repaired kss_bc falsely accept a singular deletion, use the wrong physical-copy probe law, or apply the wrong second-order block correction. Is there any unresolved critical mathematical or caller-level defect in the finite estimator as implemented?

## Assumptions you may use

- Treat the design, outcome, frequency counts, deletion partition, target weights, and seed as fixed inputs.
- Review only the point estimator returned by exact and JLA modes; the package deliberately posts no sampling variance matrix.
- Positive integer frequency weights mean literal repeated observations with a common stored-row target mass divided over copies.
- Controls may vary arbitrarily within a valid worker-firm match block.
- The JLA correction is a pointwise second-order finite-probe approximation on the accepted spectral event, not an exact finite-R identity.
- A conservative refusal of an otherwise valid design is allowed and should be distinguished from false acceptance of a rank-deficient deletion.

## Review constraints

- Use only the supplied packet unless a standard theorem is explicitly identified and stated.
- Check quantifiers, conditioning, uniformity, exceptional events, normalization, and rate arithmetic.
- Search for counterexamples and hidden dependence.
- Do not certify the entire paper from a local argument.
- Cite packet file paths and line ranges for every criticism.

## Required output

- A verdict: valid, valid with repairs, unresolved, or false.
- Audit executable control flow from `kss_bc.ado` through `kss_bc.mata`; do not infer caller correctness solely from the derivation documents.
- Construct adversarial tiny designs for graph-connected FE rank, within-cell control rank, nonconstant controls inside a match, high leverage, and frequency greater than one; state whether each is correctly accepted or withheld.
- Verify the trace-certificate inequality in the actual normalization and prove whether `max_g trace(G^{-1} Delta_g) < 1` is sufficient for positive definiteness of every deleted control scatter matrix.
- Check that the certificate is invariant to a nonsingular change of control basis and that its placement in the caller cannot be bypassed by seed, probe count, nuisance mode, or deletion mode.
- Verify the physical-copy Rademacher aggregation, leverage scaling, match contractions, target-probe centering, probe-stream independence, and coefficient-one correction in production source.
- Verify the rank-one-FE plus exact-control block inverse derivatives, signs, multiplication order, and spectral-event checks with controls varying inside the block.
- Separate critical false-acceptance or wrong-estimator defects from conservative withholding, numerical hardening, documentation issues, and out-of-scope inference claims.
