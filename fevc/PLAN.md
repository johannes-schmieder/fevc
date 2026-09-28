# Current checkpoint — 2026-09-27

The owner-approved control-basis repair has passed final Mac and Linux
qualification and its four exact qualified native artifacts are adopted locally.
Source `4064febe` contains the direct final-basis certificate, signed-zero Jacobi
tie repair, and independent high-precision anchor oracle. The estimator, sample,
rank/residual gates and existing deterministic tolerance remain unchanged.

SCC job `7761676` passed the full Stata/MP 19 suite and isolated install with
`failed=0` and `exit_status=0`. Mac thin/universal arm64 and Rosetta qualification
is carried through an explicit test-only compatibility review that preserves
the original receipt classification. Source gates pass 831 Python tests, CMG,
Rust workspace/backend tests, formatting and strict Clippy. The local native
full suite and downstream oracle/integration/application checks also pass.

The Windows binary is retained unchanged and this repair is unqualified there.
Nothing has been pushed, tagged or publicly released. Exact artifact identities
and retained limitations are in the
[adoption manifest](../native/control-certificate-20260927/manifest.json) and
[repair record](docs/CONTROL_BASIS_REPAIR_2026-09-27.md).

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
