# Adjudication: KSS-BC-REPAIRED-D

## Review metadata

- Packet: `reviews/gpt-pro/requests/KSS-BC-REPAIRED-D/PACKET.zip`
- Response: `reviews/gpt-pro/responses/KSS-BC-REPAIRED-D.md`
- Packet SHA-256: `7fc350e0434c2584ad553358f303b0eb09c85e487070321b696dd0bfc4c829a5`
- Adjudication date: 2026-08-14

## Disposition table

| Finding | Disposition | Repair or evidence |
|---|---|---|
| Observation-frequency JLA used a fourth-moment object inconsistent with its pooled residual-share ratio. | accepted and superseded | API level 7 repaired internal pooling. API level 9 implements the governing literal coordinatewise algorithm and retains copy identity through sufficient statistics instead of relying on pooled nonlinear ratios. |
| An ill-conditioned control reparameterization could make the JLA trace gate accept an exactly singular deletion. | accepted and repaired | API level 8 adds whitening-error and rounding margins, stable centered scatter losses, and a direct eigendecomposition plus inverse-residual gate for every deleted whitened scatter. The exact review fixture now returns `UNVERIFIED_DELETION_RANK`. |
| The dense Woodbury eigenvalue gate could also accept the same truly singular deletion under floating error. | accepted and repaired | Dense exact mode computes an inverse forward-error proxy and directly factors deleted information whenever a Woodbury maker eigenvalue is near that uncertainty boundary. The registered two-control fixture now returns `NONESTIMABLE_DELETION`. |
| Exact algebra away from numerical boundaries, match-frequency contraction, coefficient-one bias algebra, and block-control signs were valid. | accepted | The formulas remain unchanged and are tested independently. |
| Conservative numerical gates may reject some valid designs. | accepted limitation | The public contract permits typed conservative withholding and disallows silently weakening a gate. |
| The chat transport wrapper was not independently authenticated although embedded component hashes matched. | recorded limitation | The verbatim response is used only as source-bound model-review evidence. |

## Status decision

The response remains a valid `false` review of its submitted candidate and is
`ai_reviewed` evidence only. The numerical false acceptances were locally
reproduced before repair and are now permanent regression fixtures. Fresh
API-level-9 reviews are required for KB5.
