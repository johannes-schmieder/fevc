# Adjudication: KSS-BC-API12-FINAL-N

## Review metadata

- Packet: `reviews/gpt-pro/requests/KSS-BC-API12-FINAL-N/PACKET.zip`
- Response: `reviews/gpt-pro/responses/KSS-BC-API12-FINAL-N.md`
- Packet SHA-256: `83250f8a41183754f742a93cbc9798714c6c420a95c012e4b8d866e818e53ad1`
- Verbatim response-body SHA-256: `fd8f880bffcb7da96e17af7cf037ebbb8907c7ec872a38c74eb03e77ead2c177`
- Adjudication date: 2026-08-14

## Disposition table

| Finding | Disposition | Repair or evidence |
|---|---|---|
| Exact and auto-to-exact raw-coordinate/ID ordering changed safely eligible anchors for `Q` versus `-Q`. | accepted and repaired | API13's common semantic ordering is control-coordinate and ID free. The reviewer's nine-row `K(2,2)` fixture now produces matching canonical matrices and public exact/auto results under joint and fixed-offset nuisance modes. |
| Ordinary double summation could rank equal exact component masses differently above `2^53` after worker relabeling. | accepted and repaired conservatively | Before graph ranking, API13 incrementally tests remaining exact integer capacity and returns `PHYSICAL_TOTAL_LIMIT` if the literal total exceeds `2^53`. The two-`K(2,2)` `L=2^52` witness is registered under both worker labelings. Every accepted component mass, observation count, and `e(N_physical)` is therefore exactly represented. |
| Match leverage and target JLA had an unbounded literal-sign allocation path. | accepted and repaired | `physical_limit()` now gates the retained total for every JLA and auto-to-JLA route before backend allocation. |
| The universal anchor claim lacked dimensioned Gram, inverse, Cholesky, score/cutoff, pivot, and final-basis bounds. | accepted and repaired | API13 enforces the 32-control cap and the dimensioned a posteriori envelope recorded in `ESTIMATOR_CONTRACT.md`, including positive conditioning denominators and full/deletion propagation. |
| Quotient PCG, literal-copy formulas, coefficient-one moments, block controls, target accounting, stale-runtime protection, and the final subtraction gate survived review. | accepted as scoped positive evidence | These paths remain unchanged except for the upstream API13 fail-closed gates, and their local regressions pass. |
| Inference, asymptotic theorems, restricted data, licensing, and public release were outside the audit. | retained limitation | None is claimed by the internal sibling package. |

## Status decision

The response is a valid `false` review of API12 and is
`ai_reviewed_once` evidence only. Its critical objections are repaired in
API13. It does not close KB5 because the reviewed source changed; two fresh
independent completed API13 reviews are required.
