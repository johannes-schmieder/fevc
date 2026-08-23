# Forced generic CMG public-route candidate

Date: 2026-08-23
Branch: `codex/rust-backend-completion`

The already-qualified planned V4/V7 public lifecycle has been widened from
`preconditioner(auto)` to the explicit pair `preconditioner(auto|cmg)` for
`backend(rust) algorithm(jla) engine(generic)`. The legacy explicit
generic/diagonal V2 route remains unchanged.

The forced-CMG branch sets fallback permission to zero, requires requested and
selected route code 3, and posts `fallback_status=NOT_ELIGIBLE`. A new licensed
Stata fixture compares the CMG point estimates with the same Counter-V1 probe
atoms solved by the diagonal route, verifies complete residuals and target
identities, reconciles the V3/V7 route and memory receipts, and certifies RNG,
sort, data, release, and idle-state restoration.

This file records a candidate, not a qualification claim. The next required
evidence is an exact-SHA `quick` receipt followed by a successful exact-SHA
`plugin-build` receipt covering native arm64, Rosetta x86_64, and the universal
plugin.
