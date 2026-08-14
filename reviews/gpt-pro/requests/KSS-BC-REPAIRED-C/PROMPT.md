# External proof-review request: KSS repaired finite-projection implementation and caller audit, independent review C

You are an adversarial mathematical reviewer. Reconstruct and challenge the supplied proof step. Do not assume the author's conclusion. Distinguish an exposition omission from a genuine missing argument.

## Milestone and obligation

- Milestone: KB5
- Obligation: KB5-JLA-FINITE-PROJECTION / KB5-GENERAL-BLOCK-CONTROLS / KB5-PRODUCTION-CALLERS
- Critical: True

## Question

Does the repaired documentation, Mata implementation, Stata caller, and public-entry integration evidence consistently implement the coefficient-one finite-projection KSS correction for literal frequency copies and the exact low-dimensional-control block correction? In particular, determine whether the deterministic within-cell trace certificate is a valid sufficient certificate for full rank after every accepted deletion, and identify any remaining caller mismatch or finite counterexample that can falsely return a point estimate.

## Assumptions you may use

- The review concerns the finite fixed-design estimator and conditional probe law documented in the packet; it makes no sampling-inference or uniform-asymptotic claim.
- Frequency weights are positive integers representing literal physical copies, and the runtime uses the documented binomial/Rademacher aggregate rather than a fresh stored-row Rademacher.
- The retained mover graph has passed the documented deterministic leave-one-worker connectedness checks before JLA computation.
- Low-dimensional controls are residualized against the two-way-FE span and their Schur complement is inverted exactly, subject to the documented numerical gates.
- Any failed graph, rank, spectral, leverage, or PCG gate aborts the entire command; no failed block or target is silently dropped.
- Numerical and Monte Carlo tests are falsification evidence only; they do not replace algebraic verification.

## Review constraints

- Use only the supplied packet unless a standard theorem is explicitly identified and stated.
- Check quantifiers, conditioning, uniformity, exceptional events, normalization, and rate arithmetic.
- Search for counterexamples and hidden dependence.
- Do not certify the entire paper from a local argument.
- Cite packet file paths and line ranges for every criticism.

## Required output

- A verdict: valid, valid with repairs, unresolved, or false.
- Trace the public caller into every production formula and report any sign, coefficient, normalization, centering, probe-independence, or matrix-order mismatch with exact file and line references.
- Re-derive the literal physical-copy aggregation law, including its fourth moment, and verify that observation and match deletion use the correct stored-row scaling.
- Re-derive the coefficient-one ratio expansion and check how plug-in first moments and failed finite-probe draws affect its stated order.
- Re-derive the general block-control inverse correction, including the rank-one FE term, exact control term, spectral gates, and `+B,-V` order/signs.
- Prove or refute that the within-cell trace test combined with the FE graph certificate is sufficient for full joint-design rank after every accepted deletion; search for a smallest counterexample and distinguish conservatism from false acceptance.
- Check that solver residual gates and exact algebra are described with the correct logical status and that no forward-error or conditional-unbiasedness claim is smuggled in.
- List every critical objection that must be repaired before KB5 can close, separately from optional hardening or exposition suggestions.
