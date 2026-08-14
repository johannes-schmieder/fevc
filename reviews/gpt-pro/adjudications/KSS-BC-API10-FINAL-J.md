# Adjudication: KSS-BC-API10-FINAL-J

## Review metadata

- Packet: `reviews/gpt-pro/requests/KSS-BC-API10-FINAL-J/PACKET.zip`
- Response: `reviews/gpt-pro/responses/KSS-BC-API10-FINAL-J.md`
- Packet SHA-256: `1d9eed35fdfcd0ece88f18c24ed354a9cad544fe62b5fc69a0233ecefbfa1bbd`
- Verbatim response-body SHA-256: `87ac7c8aff5fdd10f53dfc1738675c7a91200e85dda3ec369b95f08ed0f3dc73`
- Adjudication date: 2026-08-14

## Disposition table

| Finding | Disposition | Repair or evidence |
|---|---|---|
| An explicit positive- or negative-zero control was deleted before every full-design rank gate. | accepted and repaired | API level 11 removes only expanded factor terms carrying Stata's omitted metadata. Zero and collinear numeric controls remain requested columns. Public tests cover positive zero, negative zero, a zero beside a valid control, frequency two, exact/JLA, joint/fixed-offset, observation/match, and both automatic dispatch choices. |
| Grounding the final encoded firm before PCG made adaptive stopping depend on firm relabeling. | accepted and repaired | The FE service reconstructs the missing firm equation and solves the full singular firm Laplacian on its zero-sum quotient. It grounds the public coefficient vector only after convergence and checks all worker and firm normal equations. The reviewer's six-row `K(2,3)` design is registered at `probes(2)` and the maximum allowed tolerance under relabelings that make each original firm the displayed base. |
| Separately stopped control-column solves made accepted JLA output depend on the invertible transformation `T=((1,-3),(1,1))`. | accepted and repaired | Before iterative control solves, API11 whitens the requested weighted control span, selects row anchors using invariant row inner products in the ID-free conceptual-copy order, and maps those anchors to the identity. Therefore `Z` and `ZT` produce the same canonical numerical right-hand sides up to explicit whitening/anchor residual gates. The exact transformation is registered at `probes(2)`, the maximum solver tolerance, and both joint and fixed-offset conventions. |
| The public tolerance admitted values arbitrarily close to one and the residual gate could then admit errors near ten. | accepted and repaired | `tolerance()` is now bounded to `[1e-15,1e-4]`. The former `.09` attack returns typed `INVALID_TUNING`; relabeling and control-basis regressions run at the new maximum. This cap supplements rather than substitutes for quotient and canonical-basis repairs. |
| `e(parameters)` conflated full-fit and working correction dimensions. | accepted and repaired | The command now posts `e(full_parameters)` and `e(correction_parameters)` separately; `e(parameters)` is documented as a compatibility alias for the latter. Joint and fixed-offset exact/JLA tests register the values. |
| Several inverse failure labels were absent from the failure catalog. | accepted and repaired | The catalog now explicitly includes dense, block, control-Schur, graph, runtime, invalid, unsupported, and nonfinite statuses emitted by the public paths. |
| Observation JLA had no typed upper bound before allocating physical-copy state. | accepted as hardening and repaired | New `physical_limit()` defaults to 50,000,000 and is checked after graph selection but before observation-JLA copy allocation. Exceedance returns `PHYSICAL_COPY_LIMIT`; a public regression registers the boundary. |
| The semantic runtime token is not cryptographic authentication. | recorded limitation | The source-bound packet and clean committed SCC deployment supply cryptographic source hashes. The runtime token remains an accidental stale-namespace guard and is not represented as tamper-proof authentication. |
| No additional coefficient, sign, copy mapping, target contraction, block-control, or deleted-scatter error was found after a complete nonzero control matrix reached Mata. | accepted as scoped evidence | Those paths remain covered by the independent dense oracle, symbolic/exhaustive tests, public-command overlaps, convergence tests, and the fresh API11 review packet. |

## Status decision

The response is a valid `false` review of the API-level-10 candidate and is
`ai_reviewed_once` evidence only. Both independent critical objections are
accepted, repaired in API level 11, and registered against the supplied
counterexamples. Because the numerical operator and public API changed, this
adjudication does not close KB5; two fresh independent API11 reviews with no
access to API10 responses are required.
