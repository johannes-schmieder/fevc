# External proof-review request: KSS API 7 pooled-frequency and production-caller audit, independent review E

You are an adversarial mathematical reviewer. Reconstruct and challenge the supplied proof step. Do not assume the author's conclusion. Distinguish an exposition omission from a genuine missing argument.

## Milestone and obligation

- Milestone: KB5
- Obligation: KB5-JLA-FINITE-PROJECTION / KB5-FREQUENCY-SEMANTICS / KB5-GENERAL-BLOCK-CONTROLS / KB5-PRODUCTION-CALLERS
- Critical: True

## Question

Does the API 7 documentation, Mata implementation, Stata caller, and integration evidence consistently implement a valid coefficient-one finite-projection KSS point correction, including the selected per-probe pooled physical-copy statistic for collapsed frequency weights? Decide whether that pooled expanded-data reference satisfies the supplied specification, and identify any finite counterexample that can make the production caller return a point estimate outside its documented contract.

## Assumptions you may use

- The review concerns a finite fixed-design point estimator and the unconditional probe law; it makes no sampling-inference, exact finite-probe unbiasedness, or uniform-asymptotic claim.
- Positive integer frequency weights denote exchangeable literal physical copies. The candidate deliberately pools squared coordinates of those copies within each probe before the nonlinear ratio; whether this is permitted by the supplied specification is a question for review, not an allowed conclusion.
- The alternative algorithm that keeps a separate nonlinear ratio for every expanded copy may differ at finite probe counts. The reviewer must distinguish equality of pooled per-probe random quantities, equality of the entire coordinatewise finite-probe algorithm, and convergence to the common exact leverage.
- The retained mover graph has passed the documented deterministic graph checks before JLA computation.
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
- Trace the public Stata call through API level 7 into each production formula; report every sign, coefficient, normalization, centering, frequency, probe-stream, matrix-order, or result-posting mismatch with exact file and line references.
- Re-derive the pooled physical-copy variables U and V and decide whether the second raw moment must be E[V squared] or the average copywise fourth moment. Test the frequency-two fixture numerically or symbolically.
- Decide explicitly whether pooling exchangeable copies within each expanded probe before the nonlinear ratio satisfies the specification's literal-expansion language. State precisely what is and is not equal to a coordinatewise expanded-data JLA at finite R, and classify any gap as critical or documented narrowing.
- Re-derive the coefficient-one ratio expansion, its plug-in order, and the effect of failed finite-probe gates without asserting conditional unbiasedness.
- Re-derive the general block-control inverse correction and the deterministic deletion-rank certificate; search for a false-acceptance counterexample and distinguish conservatism from invalidity.
- Check target-probe scaling, physical-copy target mass, common draws, and the four-target accounting identity through the production caller.
- Separate every critical objection required before KB5 can close from optional hardening and exposition suggestions.
