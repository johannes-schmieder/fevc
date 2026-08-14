# Adjudication: KSS-BC-API7-E

## Review metadata

- Packet: `reviews/gpt-pro/requests/KSS-BC-API7-E/PACKET.zip`
- Response: `reviews/gpt-pro/responses/KSS-BC-API7-E.md`
- Packet SHA-256: `54ff42b495ca5d49380eb5f1ad669fa670da4fa1211d1ce642d9b6d141003597`
- Adjudication date: 2026-08-14

## Disposition table

| Finding | Disposition | Repair or evidence |
|---|---|---|
| The coefficient-one finite-projection formula, match contraction, joint-control block algebra, target scaling, and accounting identity are internally correct. | accepted | These formulas remain unchanged and retain their registered symbolic, exhaustive, dense-overlap, and Monte Carlo tests. |
| Pooled observation-frequency JLA is coherent for its selected random variable but is not literal coordinatewise expansion at finite `R`; it also depends on how identical copies are stored. | accepted and repaired | API level 9 retains copy identities through two sufficient cross-probe correlations, forms each copy's constrained ratio, bias, variance, and inverse multiplier separately, and averages only final multipliers. `test_frequency_probes.py` checks the sufficient-statistic identities; `test_frequency.do` couples weighted and expanded public-command runs and requires finite-probe agreement. |
| Selecting an exactly tied component through its union-find root violates ID-relabeling invariance and can change the point estimate. | accepted and repaired | Component ties on firm count and physical mass now return `AMBIGUOUS_LARGEST_COMPONENT`. `test_graph_pruning.do` checks both an initial tie after ID relabeling and a tie created by articulation pruning. |
| Exact `control_schur_rcond` availability did not match the return documentation. | accepted and repaired by narrowing | The help and return contract now state that this diagnostic belongs to matrix-free JLA joint-control preparation. Dense exact mode reports full-information conditioning. |
| The stored-row count check unnecessarily withheld literal-copy designs with enough physical residual degrees of freedom. | accepted and repaired | Both backends now rely on weighted information and deletion-rank gates. A four-stored-row/four-parameter, eight-copy exact observation fixture must return successfully after every copy deletion. JLA may still withhold under its documented conservative control-rank certificate. |
| An API-level-only guard can execute a stale same-level Mata runtime. | accepted and repaired | The caller now checks API level, version, and an exact semantic build token. It fails closed as `STALE_MATA_RUNTIME` when another `kss_bc` runtime is already loaded and validates the token after first loading the bundled runtime. |
| Public documentation should disclose conservative finite-probe withholding and avoid a conditional-unbiasedness claim. | accepted and repaired | The help now states both points explicitly; the longer derivation and failure contract retain the same limitation. |
| The uploaded chat attachment was not byte-identical to the asserted wrapper digest, although its embedded source bodies matched their source-map hashes. | recorded limitation | The response is preserved verbatim and is treated as source-bound to the independently checked component bodies, not as independent authentication of the transport wrapper. |

## Status decision

The response remains a valid `false` review of the API-level-7 candidate and
is `ai_reviewed` evidence only. Its critical counterexamples are accepted and
repaired in API level 9. Because those repairs change the reviewed candidate,
this adjudication does not close KB5; two fresh independent reviews of the
current source-bound candidate are required.
