# CMG-MATA-PLAN adjudication

**Date:** 2026-08-14
**Milestone:** CMG0 (owner-authorized planning review; not a proof milestone)
**Obligation:** CMG-MATA-ARCHITECTURE-PLAN
**Status:** blocked before external submission

## Scope

This round asks two independent ChatGPT Pro instances for a decision-complete,
clean-room plan for a shared pure-Mata CMG core used only by the two-way
fixed-effect paths in `ppml_talo` and `kss_bc`. It does not authorize estimator
code, proof, manuscript, or release changes. `reghdfe`, `ppmlhdfe`, three-or-more
fixed effects, and heterogeneous slopes remain out of scope.

## Packet comparison

| Item | Review A | Review B |
|---|---|---|
| Request ID | `CMG-MATA-PLAN-A` | `CMG-MATA-PLAN-B` |
| Packet SHA-256 | `1b55d5c9b910753df74627a7ca3b7ff80d3fdf13e6be41bcf4793bdff9101713` | `a22c86bfa7d36d194969ea5f8c98a5a2fae3be7b325ac62a684be7a9bfb6ee60` |
| Prompt SHA-256 | `3cb3d5faf84e6c0ab6fc8f05c3f246cb9002e3396218a6ccb92be1889423d81f` | `3cb3d5faf84e6c0ab6fc8f05c3f246cb9002e3396218a6ccb92be1889423d81f` |
| Sources | 21 hash-bound files | Same 21 hash-bound files |
| External verdict | Not submitted | Not submitted |

The substantive prompts are byte-identical. The packet hashes differ because
each packet contains its own request identifier and manifest.

## Execution finding

Chrome reached ChatGPT in a fresh chat, the account was signed in, and the
visible model control read `Pro`. The documented packet-upload flow repeatedly
failed before a file chooser opened. At the owner's request, Chrome was quit
gracefully, confirmed stopped, relaunched, and reconnected. A new fresh Pro chat
again exposed the visible upload item, but the file chooser still failed to
open. A further owner-requested retry failed at the same point. Read-only
diagnostics confirmed that Chrome is running, the extension is installed and
enabled in the selected profile, and the native-host manifest is correct. This
narrows the blocker to file-chooser/file-access handling rather than general
extension communication. No repository file or prompt was transmitted. Review
B was not started because the same environment-wide blocker would prevent a
packet-bound independent review.

## Adjudication

There are no external findings to compare, score, select, reject, or combine.
Both records are `browser_blocked` and contribute no evidence about the
mathematics, Mata design, expected speed, memory behavior, testing plan, or
milestone structure. The implementation decision remains open.

The next action is mechanical: enable file access for the ChatGPT Chrome
extension, then rerun A and B in separate fresh Pro chats. After both verbatim
responses are saved and validated, replace this section with a scored comparison
covering mathematical validity, performance architecture, Mata feasibility,
testing and benchmarks, milestones and red teams, and scope/licensing compliance.

## Files left untouched

No file under `kss_bc/`, `ppml_talo/`, `paper/`, `theory/`, `proof-audit/`,
`state/`, or `archive/` was changed by this review attempt. The unrelated
untracked `varcomp_naming.md` was also left untouched.
