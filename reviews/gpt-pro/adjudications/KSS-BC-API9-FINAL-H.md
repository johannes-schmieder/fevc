# Adjudication: KSS-BC-API9-FINAL-H

## Review metadata

- Packet: `reviews/gpt-pro/requests/KSS-BC-API9-FINAL-H/PACKET.zip`
- Response: `reviews/gpt-pro/responses/KSS-BC-API9-FINAL-H.md`
- Packet SHA-256: `b034abdeb8ce9545c5043875dd17015cf6d8872bf068da65277726bbbb45a619`
- Verbatim response-body SHA-256: `aac485d456ca39dad6d467358f03554485ad7480321527f72bce86f6ab960cba`
- Adjudication date: 2026-08-14

## Disposition table

| Finding | Disposition | Repair or evidence |
|---|---|---|
| API9 ordered the sequential physical-copy Rademacher stream by encoded IDs, raw controls, stored-row frequency, and total stored-row target mass, making accepted fixed-seed output representation dependent. | accepted and repaired | API level 10 orders the conceptual physical-copy stream only by per-copy outcome and per-copy target mass. Raw IDs, controls, stored-row frequency, total stored-row target mass, and the stored-row partition cannot order signs. Encoded IDs are used only as a tie break after the caller proves tied rows are exchangeable for the requested deletion semantics. |
| A frequency-two row and its literal frequency-one split could produce different finite-probe estimates. | accepted and repaired | Public tests couple collapsed and partially split representations under the same seed and require equality of the full `e(results)` matrix. The review's minimal six-copy observation-deletion construction is registered with `probes(2)` and seed 2; dense exact and finite-probe JLA agree across representations. |
| Pure worker/firm ID relabeling or an invertible control reparameterization could change the fixed-seed result. | accepted and repaired | Public tests require full-result equality after ID relabeling and after the invertible control transformation `(c1,c2) -> (-c1,c2)`. Neither representation enters the primary probe-order key. |
| Rows with identical ID-free per-copy keys can still represent distinct coordinates or deletion blocks, so no unique pathwise order exists. | accepted and repaired by withholding | Before JLA dispatch, tied primary-key rows must have identical controls, the same worker--firm coordinate, and, in match mode, the same deletion block. Otherwise the command returns `AMBIGUOUS_PROBE_ORDER`; dense exact mode remains available. |
| API9's per-copy observation formulas, general match-control inverse signs, target contractions, four-target accounting, and graph logic were otherwise sound. | accepted | Those formulas remain unchanged. Their exhaustive algebra, dense-oracle, public-command, and convergence tests remain registered. |
| The pasted chat attachment did not contain the ZIP bytes or manifest body, so it could not independently authenticate the asserted outer ZIP digest. | recorded limitation | The reviewer verified every supplied component body against the embedded source map. This is retained as source-bound model-review evidence only. The fresh API10 packets retain deterministic packet, manifest, and per-component hashes and include the independent dense Python oracle. |

## Status decision

The response is a valid `false` review of the API-level-9 candidate and is
`ai_reviewed_once` evidence only. The critical production-order defect is
repaired in API level 10 and covered by fixed-seed representation-invariance
tests plus a typed ambiguous-order rejection. Because the candidate changed,
this adjudication does not close KB5; two fresh independent API10 reviews with
no access to G or H are required.
