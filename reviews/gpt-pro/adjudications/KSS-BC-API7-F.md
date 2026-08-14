# Adjudication: KSS-BC-API7-F

## Review metadata

- Packet: `reviews/gpt-pro/requests/KSS-BC-API7-F/PACKET.zip`
- Response: `reviews/gpt-pro/responses/KSS-BC-API7-F.md`
- Packet SHA-256: `0923b52200cb0224002763438da5e825174d06b495ea0d446b6c807acfbbd5a8`
- Adjudication date: 2026-08-14

## Disposition table

| Finding | Disposition | Repair or evidence |
|---|---|---|
| Observation JLA with frequency above one pooled copy residual squares before the nonlinear finite-`R` ratio and therefore differed from literal coordinatewise expansion. | accepted and repaired | API level 9 retains two sufficient correlations for every physical copy, forms every nonlinear multiplier copywise, and averages only the final multipliers. Exhaustive algebra tests and coupled public-command weighted/expanded runs now require finite-probe equality. |
| An API-level-only caller guard could silently execute a stale or foreign Mata runtime and label it as the current build. | accepted and repaired | The caller checks level, version, and an exact semantic build token, then fails closed on a preloaded mismatch. `test_stale_runtime.do` dynamically replaces the same-level token and requires `STALE_MATA_RUNTIME`. |
| The joint-control matrix solver used a global Frobenius residual, allowing a bad right-hand side to be hidden by another column. | accepted and repaired before API 9 | API level 8 computes the maximum separately scaled column residual in joint solves, inverse checks, and Schur preparation. `test_load.do` registers the masking counterexample and the source audit verifies every call site. |
| Exact component ties were broken by encoded union-find roots, so ID relabeling could change the retained sample and point estimate. | accepted and repaired | API level 9 returns `AMBIGUOUS_LARGEST_COMPONENT` at every selection stage. Public tests cover an initial tied graph under relabeling and a tie created by articulation pruning. |
| The finite-precision control-rank gap could be smaller than the permitted whitening error and therefore was not a valid numerical lower bound. | accepted and repaired before API 9 | API level 8 subtracts measured whitening error and a rounding margin, uses stable two-pass within-cell centering and nonnegative scatter-loss formulas, and directly eigendecomposes and inverse-checks every deleted low-dimensional scatter. The two supplied ill-conditioned singular designs now withhold. |
| Requiring stored `n>p` can reject a literal-copy design with physical residual degrees of freedom. | accepted and repaired | The shortcut was removed from both backends. Dense exact mode now accepts a registered four-row/four-parameter design with eight copies and retains rank after each copy deletion; JLA remains allowed to withhold under its separately documented conservative certificate. |
| The coefficient-one delta formula, `+B,-V` inverse expansion, noncommuting block-control algebra, target contractions, and exact-arithmetic trace theorem are valid. | accepted | These derivations remain in the candidate with independent symbolic, exhaustive, dense-overlap, and simulation evidence. |
| The raw chat transport could not independently authenticate the asserted ZIP digest although embedded bodies matched the source map. | recorded limitation | The verbatim response is treated as source-bound to the verified embedded bodies. Fresh final packets preserve deterministic manifests and per-file hashes. |

## Status decision

The response remains a valid `false` review of API level 7 and is
`ai_reviewed` evidence only. All five critical accepted-path defects and the
stored-row false-withholding issue are repaired in API levels 8--9. Because
the candidate changed materially, this adjudication does not close KB5; the
fresh independent API-level-9 reviews G and H are the applicable closure
gate.
