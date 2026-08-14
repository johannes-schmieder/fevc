# External proof-review request: KSS finite-projection and general block-control algebra, independent review B

You are an adversarial mathematical reviewer. Reconstruct and challenge the supplied proof step. Do not assume the author's conclusion. Distinguish an exposition omission from a genuine missing argument.

## Milestone and obligation

- Milestone: KB5
- Obligation: KB5-JLA-FINITE-PROJECTION / KB5-GENERAL-BLOCK-CONTROLS
- Critical: True

## Question

For the finite fixed-design linear model and conventions in the packet, are (i) the coefficient-one finite-projection expansion for the inverse residual leverage and (ii) the rank-one random-FE plus exact-control formula for a general match-deletion block algebraically correct through the stated order? Identify any missing conditioning, covariance term, sign, normalization, or estimability condition, and try to construct a finite counterexample.

## Assumptions you may use

- The retained weighted design has been mapped to literal physical copies and is full column rank after the stated firm-effect normalization.
- Each accepted deletion block has an invertible residual-maker block; failures are withheld rather than regularized.
- Rademacher probe vectors are independent across probes and independent of the fixed design and outcome; all expectations in the JLA derivation are conditional on the fixed inputs.
- The constrained first-moment estimates are treated as the expansion point; the review concerns the displayed second-order finite-probe ratio correction, not a sampling CLT.
- Low-dimensional controls are residualized against the two-way FE span and their Schur complement is inverted exactly.
- For match deletion, every stored row in a declared block shares one worker-firm FE coordinate, while controls may vary within the block.
- Numerical tests and simulations are falsification evidence only and may not replace the algebra.

## Review constraints

- Use only the supplied packet unless a standard theorem is explicitly identified and stated.
- Check quantifiers, conditioning, uniformity, exceptional events, normalization, and rate arithmetic.
- Search for counterexamples and hidden dependence.
- Do not certify the entire paper from a local argument.
- Cite packet file paths and line ranges for every criticism.

## Required output

- A verdict: valid, valid with repairs, unresolved, or false.
- Re-derive the second-order ratio expansion and determine whether the mixed fourth-moment coefficient is one or two under the packet's definitions.
- Check whether plug-in first moments inside the finite correction introduce omitted covariance or higher-order terms.
- Re-derive the general block-control inverse perturbation, including signs and matrix order, without assuming controls are constant within a match.
- Check the literal-frequency normalization and the transition from physical-copy probes to stored-row aggregates.
- State every rank, conditioning, and positivity condition needed for the displayed inverses.
- Search for the smallest finite design or moment configuration that contradicts either formula.
- Distinguish exact identities, second-order approximations, conditional numerical diagnostics, and econometric claims.
- Cite exact packet paths and line ranges for every criticism and propose minimal repairs plus falsification tests.
