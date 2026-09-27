# Current checkpoint — 2026-09-27

The owner authorized investigation and repair of a real-data control-basis
certification refusal. A direct final-basis certificate is implemented in Mata
and Rust. It preserves all prior numerical gates and statistical decisions.
Focused oracle and public panel checks are passing; integrated source and
rebuilt native qualification are in progress. No candidate binary has been
adopted downstream or publicly released. See the active
[certificate contract](docs/CONTROL_BASIS_CERTIFICATION.md).

## Previous accepted checkpoint


The subsample repair is complete on `main`. Both backends keep eligible-stayer
and combined-sample masks inside the frozen requested `if`/`in` sample.
JLA stayer ordering is stable when excluded rows are removed, including Rust
projection attachment. Deletion-unit mover eligibility, graph selection,
identification and numerical acceptance gates remain unchanged.

Implementation `9681684a` passed the native Mac profile on arm64 and Rosetta,
including thin/universal candidates and isolated installs. Source `16d5564a`
passed the full integrated check: 829 Python tests, CMG, Stata quick/full,
portable installation, helper migration and harness smokes. The 116 paired
subsample scenarios pass on Stata/MP 18/19 and each shipped Mac plugin form.
Fresh and replacement native installs validate all 53 installed-file hashes.
See the [repair record](docs/SUBSAMPLE_REPAIR_2026-09-26.md) and
[compatibility evidence](../native/subsample-repair-20260926/compatibility.json).

All five shipped plugin files retain their previous bytes; the local native
profile's rebuilt candidates are retained only as ignored test evidence.
The prior [deletion-unit mover integration](docs/DELETION_UNIT_MOVERS_2026-09-26.md)
and its exact-source receipts remain intact. The old Codex development
worktree is removed; its backup remains under ignored
`.local/deletion-unit-movers/original-worktree/`.

Candidate promotion and evidence reuse follow the registered
[acceptance policy](docs/development_acceptance_v1.json).
