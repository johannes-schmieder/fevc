# Adjudication: KSS-BC-API11-FINAL-K

## Review metadata

- Packet: `reviews/gpt-pro/requests/KSS-BC-API11-FINAL-K/PACKET.zip`
- Response: `reviews/gpt-pro/responses/KSS-BC-API11-FINAL-K.md`
- Packet SHA-256: `c1d2728068e5703710091fa467289d6c92e6f5535e89875ba59c865296bed675`
- Verbatim response-body SHA-256: `85b459e749987ebd855313253629ece9bd60671be4fb5ebb9b64e024ba649417`
- Adjudication date: 2026-08-14

## Disposition table

| Finding | Disposition | Repair or evidence |
|---|---|---|
| API11's fuzzy row-anchor eligibility band could accept two invertible bases, select different anchors, and return basis-dependent fixed-seed JLA results. | accepted and repaired | API12 propagates a forward-error envelope from whitening and inverse residuals divided by reciprocal-conditioning margins. It withholds as `AMBIGUOUS_CONTROL_BASIS` if the envelope is too large or a pivot score lies within the envelope of the eligibility cutoff. |
| The reviewer's eight-control-row witness embeds in a 14-row weighted `K(2,5)` public design. | reproduced natively and registered | Stata 18 API11 accepted both bases at `probes(2)`, `batch(1)`, `seed(2)`, and `tolerance(1e-4)`, with `mreldif(e(results)) = 3.308e-5`. API12 accepts the well-conditioned basis and withholds the transformed basis under exact/JLA/auto, joint/fixed-offset, and batch sizes one and two. A direct four-row cutoff-boundary fixture withholds both bases. |
| The full-firm zero-sum quotient solver removes encoded-firm dependence for scalar and batched right-hand sides. | accepted as scoped evidence | The quotient code is unchanged in API12. The six-row `K(2,3)` maximum-tolerance regression remains in the suite across displayed base firms and batch sizes. |
| Explicit numeric zero/collinear controls reach the rank gates; literal copies, coefficient-one moments, block controls, target accounting, and posting metadata satisfy the reviewed finite contract. | accepted as scoped evidence | These API11 repairs and their independent dense/public-command regressions remain unchanged and pass under API12. This positive finding did not rescue the false API11 candidate. |
| Reconstructing the final omitted firm equation after a combined joint solve would align the implementation more literally with the documentation. | recorded optional hardening | The omitted equation is algebraically dependent; the reviewer found no counterexample and classified this as optional. The existing grounded joint-equation and pure-FE all-firm residual gates remain unchanged. |
| `TESTING.md` referred to API10 although the runtime was API11. | accepted and repaired | The qualification guide now names the API10--API12 regression family explicitly. |
| Outer ZIP, native Stata, package metadata, and SCC scripts were unavailable in the chat transport. | recorded limitation | The reviewer authenticated all 32 embedded bodies. Native Stata replay is recorded locally; SCC evidence remains a separate KB6 gate. |

## Status decision

The response is a valid `false` review of API11 and is
`ai_reviewed_once` evidence only. Its critical anchor objection is repaired in
API12. It does not close KB5 because the reviewed candidate changed; API12
requires two fresh independent completed reviews with no access to this
response.
