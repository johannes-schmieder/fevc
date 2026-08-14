# Adjudication: KSS-BC-API9-FINAL-G

## Review metadata

- Packet: `reviews/gpt-pro/requests/KSS-BC-API9-FINAL-G/PACKET.zip`
- Response: `reviews/gpt-pro/responses/KSS-BC-API9-FINAL-G.md`
- Packet SHA-256: `b3abcac6b302f3271849af9c8bd55f3907901c7f5333a1ea7daec8f4a6fcfe0e`
- Verbatim response-body SHA-256: `305e2a70f85f7c34e4217f54645f8ea7689dea50a221864890a184f9a48fb2a7`
- Adjudication date: 2026-08-14

## Disposition table

| Finding | Disposition | Repair or evidence |
|---|---|---|
| `algorithm(jla) nuisance(fixedoffset)` could accept a control exactly in the worker--firm FE span because an approximate FE solve manufactured a small positive Schur complement. | accepted and repaired | API level 10 applies the deterministic full-fit within-cell trace and direct-factor certificate to every JLA control fit, including `fixedoffset`, before posting results. The fixed-offset use is deliberately conservative: its deletion checks are stronger than the full-fit condition required by that convention. |
| Mixed absolute/relative PCG gates made the manufactured-rank path scale dependent. | accepted and repaired | Nonzero right-hand sides now use genuinely relative per-column residuals, and zero right-hand sides use absolute residuals. PCG stopping uses `tolerance * ||b_reduced||` for nonzero right-hand sides rather than `tolerance * (1+||b_reduced||)`. |
| The six-cycle provides both a small-scale default-tolerance counterexample and an ordinary-scale loose-tolerance counterexample. | accepted and registered | `test_failures.do` includes the exact six-cycle for match and observation deletion, at both scales. Dense exact and JLA fixed-offset modes must withhold; the JLA route may return only one of the documented typed numerical/rank failures. A normal identified fixed-offset fixture separately requires a positive reported certificate gap. |
| The coefficient-one finite-projection formula, general block-control algebra, graph tie handling, four-target accounting, and literal-copy mechanics were sound in the reviewed packet. | accepted | These parts remain unchanged except for the separate API10 invariant stream repair prompted by review H. Their symbolic, dense-oracle, exhaustive, public-command, and Monte Carlo tests remain registered. |
| The pasted chat attachment did not contain the ZIP bytes or manifest body, so it could not independently authenticate the asserted outer ZIP digest. | recorded limitation | The reviewer verified all 22 supplied source bodies against the embedded source map. The response is retained as source-bound model-review evidence, not independent transport authentication. The fresh API10 packets include the dense Python oracle and retain deterministic packet, manifest, and per-component hashes. |

## Status decision

The response is a valid `false` review of the API-level-9 candidate and is
`ai_reviewed_once` evidence only. Its critical accepted-path defect is repaired
in API level 10 and covered by adversarial public-command tests. Because the
candidate changed, this adjudication does not close KB5; two fresh independent
API10 reviews with no cross-contamination are required.
