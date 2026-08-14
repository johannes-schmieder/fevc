# Adjudication: KSS-BC-API10-FINAL-I

## Review metadata

- Packet: `reviews/gpt-pro/requests/KSS-BC-API10-FINAL-I/PACKET.zip`
- Response: `reviews/gpt-pro/responses/KSS-BC-API10-FINAL-I.md`
- Packet SHA-256: `6eaf7d1fa3f2efe7b2f77321f2e6d63affe8a8fc2e5689709be8c9b6e7e1ecb9`
- Verbatim response-body SHA-256: `01741abb15f75e6debd02e5dd1b29441f962540ed6f0a192a7e87a21fec5f94d`
- Adjudication date: 2026-08-14

## Disposition table

| Finding | Disposition | Repair or evidence |
|---|---|---|
| The ado caller removed every materialized all-zero control before either backend, allowing an explicitly requested singular design to be accepted. | accepted and repaired | API level 11 uses `fvexpand` and one-to-one `fvrevar` materialization, parses every expanded term with `_ms_parse_parts`, and removes only terms carrying Stata's `r(omit)` metadata. Ordinary zero and collinear numeric controls remain in the design and reach the registered rank gates. |
| The bypass affected exact/JLA, joint/fixed-offset, observation/match, and automatic dispatch. | accepted and registered | `test_failures.do` exercises the explicit zero column across both deletion conventions, both nuisance conventions, direct exact and JLA selection, and both exact- and JLA-selecting `algorithm(auto)` calls. It also registers an invertible transformation of a redundant basis that exposes a zero coordinate. Exact must return `SINGULAR_INFORMATION`; JLA must return `SINGULAR_NUISANCE_BLOCK`. |
| The API9 literal-copy ordering and fixed-offset six-cycle repairs, copywise moment algebra, coefficient-one term, block-control projection, inverse signs, target normalization, and accounting identity pass review. | accepted as scoped evidence | Those registered algebraic paths remain unchanged in API11. Their dense, exhaustive, public-command, and convergence tests remain part of the local gate. This positive finding cannot validate the API10 candidate because the accepted-path rank bypass is critical. |
| Finite-projection variance is evaluated through a cancellation-prone moment combination rather than accumulated as an empirical square. | recorded noncritical limitation | The existing negative-moment gate is fail-closed for a material negative value and truncates only a small numerical negative. Direct square accumulation would require retaining or replaying probe-level state and is deferred as numerical hardening; it is not used to claim finite-probe exactness or inference. |
| Exact fixed-offset metadata documents `information_rcond` and `parameters` too broadly. | accepted and repaired | The help and return contract now state that `parameters` is the working correction-design dimension and that exact fixed offset reports the minimum reciprocal conditioning across the preliminary full joint and pure-FE working factorizations. |
| The pasted transport did not expose the outer ZIP bytes. | recorded limitation | The reviewer authenticated all 31 embedded bodies against the manifest. The packet ZIP and manifest were separately generated and verified locally. The record is source-bound model-review evidence, not independent transport certification. |

## Status decision

The response is a valid `false` review of the API-level-10 candidate and is
`ai_reviewed_once` evidence only. Its critical control-preprocessing objection
is repaired in API level 11 and covered by public-command regressions. Because
the candidate changed, this adjudication does not close KB5; two fresh
independent API11 reviews with no access to API10 responses are required.
