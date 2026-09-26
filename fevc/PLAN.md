# Current checkpoint — 2026-09-26

Deletion-unit mover semantics are integrated into `main` for Mata and Rust.
For match deletion, a worker with more than one original deletion ID is a mover,
including multiple blocks at one model firm. Population selection, graph
pruning/final certification, and match-component inference agree. Existing
identification, rank, residual and supported-route restrictions remain intact.
Readiness bit 15 protects users with older plugins.

Source tests, independent deletion/refit oracles, Stata/MP 18 and 19 checks, and
all five native plugin qualifications pass. Mac/Windows build source is
`d6f5571d`; Linux is `31cf2ea7`. Source `c3ab9e52` changes only the private
Windows harness. Exact artifacts and evidence are in
[`native/deletion-unit-movers-20260926`](../native/deletion-unit-movers-20260926/manifest.json).

The remaining publication steps are to push the assembled package, verify
fresh/replacement public installs through `net install` and `github install`
on Mac and Linux, then remove the preserved development worktree at
`/Users/johannes/.codex/worktrees/86db/fevc`. The owner authorized these steps.
No release tag or archive is requested.

See the [integration record](docs/DELETION_UNIT_MOVERS_2026-09-26.md) for the
implementation, commands, failed attempts and limitations; the
[parity matrix](docs/RUST_MATA_PARITY.md) for current capabilities; and
[acceptance policy](docs/development_acceptance_v1.json) for evidence reuse.
Historical receipts remain immutable. Original worktree source and useful logs
are backed up in ignored `.local/deletion-unit-movers/original-worktree/`.
