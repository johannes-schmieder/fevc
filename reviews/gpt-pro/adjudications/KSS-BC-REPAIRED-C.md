# Adjudication: KSS-BC-REPAIRED-C

## Review metadata

- Packet: `reviews/gpt-pro/requests/KSS-BC-REPAIRED-C/PACKET.zip`
- Response: `reviews/gpt-pro/responses/KSS-BC-REPAIRED-C.md`
- Packet SHA-256: `adef11c2e8ba0f5abbcc5bda78a3dbe2ff36b4190f60243c615a5edbacfba896`
- Adjudication date: 2026-08-14

## Disposition table

| Finding | Disposition | Repair or evidence |
|---|---|---|
| Frequency-weighted observation JLA mixed a pooled first moment with an average copywise fourth moment, so its coefficient-one delta correction did not correspond to one random ratio. | accepted and superseded | API level 7 first made the pooled moments internally consistent. API level 9 then replaced that narrowed estimator with literal coordinatewise physical-copy ratios, as required by the governing specification. Exhaustive and coupled public-command tests cover the final semantics. |
| Floating whitening error could exceed an accepted trace-certificate gap and permit a singular deleted joint design. | accepted and repaired | API level 8 subtracts the measured whitening error and rounding allowance, uses stable two-pass centering and nonnegative scatter-loss formulas, and directly factors every deleted whitened scatter. The reviewer's large cell-constant control fixture is registered and both backends withhold it. |
| Match residual-maker inverse errors were absent from the posted solver-residual maximum. | accepted and repaired | Every small JLA match-block inverse residual now enters `solver_max_residual`. |
| Calling the dense backend “exact” could be read as exact arithmetic. | accepted as exposition | The contract and help now state that exact means deterministic dense numerical algebra and document its conditioning, residual, forward-error, and direct-factor gates. |
| The deterministic JLA control certificate can withhold estimable designs. | accepted as deliberate conservatism | Public help and the failure contract state that the certificate is sufficient rather than necessary and that finite-probe gates may withhold exactly estimable designs. No automatic fallback or silent relaxation occurs. |
| Several finite-projection, block-control, target, and caller formulas were correct. | accepted | They remain covered by symbolic, exhaustive, dense-overlap, and public-command tests. |
| The raw chat attachment did not authenticate the outer packet digest although embedded bodies matched their source-map hashes. | recorded limitation | The response is preserved verbatim and treated as bound to the verified component bodies. |

## Status decision

The response remains a valid `false` review of its submitted candidate and is
`ai_reviewed` evidence only. All critical findings were reproduced and
repaired in later API levels. Fresh API-level-9 reviews are required for KB5.
