# Current checkpoint — 2026-09-26

Deletion-unit mover semantics are complete on `main` for Mata and Rust.
For match deletion, a worker with more than one original deletion ID is a mover,
including multiple blocks at one model firm. Population selection, graph
pruning/final certification, and match-component inference agree. Existing
identification, rank, residual and supported-route restrictions remain intact.
Readiness bit 15 protects users with older plugins.

Source tests (829 Python tests, generated CMG/parity, locked Rust and hosted
stable/MSRV matrices), independent graph/deletion/refit oracles, and Stata/MP
18/19 checks pass. All five plugins are rebuilt and qualified. Actual build
sources are `d6f5571d` for Mac/Windows and `31cf2ea7` for Linux; package
assembly source is `c3ab9e52`. Exact identities and compatibility reviews live
in the [native manifest](../native/deletion-unit-movers-20260926/manifest.json).

Package `72da5542` is pushed to `main`. Fresh and replacement public installs
through both advertised commands pass on Mac and Linux, checking all 52
installed-file hashes and native regressions. Windows passed separate private
exact-artifact Stata installation/runtime checks. See [publication evidence](../native/deletion-unit-movers-20260926/publication.json).
Later documentation and receipt commits preserve the tested payload bytes.
No implementation or qualification work remains. No tag or release archive
was requested or created.

Original worktree changes and useful logs are backed up in ignored
`.local/deletion-unit-movers/original-worktree/`, with all 31 source hashes
verified. This backup supports the owner-authorized removal of
`/Users/johannes/.codex/worktrees/86db/fevc` after final evidence publication.

See the [integration record](docs/DELETION_UNIT_MOVERS_2026-09-26.md) for exact
checks, failed attempts and limitations, the [parity matrix](docs/RUST_MATA_PARITY.md)
for current capabilities, and the [acceptance policy](docs/development_acceptance_v1.json)
for evidence reuse. Historical receipts remain immutable.
