# Current checkpoint — 2026-09-27

The owner authorized investigation and repair of a real-data control-basis
certification refusal. Source commits `cc3b3efc` and `6fc08fee` add a direct
final-basis certificate in Mata/Rust and repair a signed-zero Jacobi rotation
tie. The estimator, sample and numerical tolerances are unchanged. The original
integrated source checks passed (830 Python tests, CMG, Stata quick/full,
installation and harness checks). The rebuilt Mac arm64/Rosetta native checks
and isolated installs pass, with a documented dirty-end test-only compatibility
review; qualified candidate binaries remain separate from tracked payloads.

SCC job 7760983 passed the new exact/JLA public panel but stopped in the full
suite at an obsolete blanket-refusal assertion for the eight-row anchor
witness. An independent high-precision deleted-regression oracle confirms the
accepted native results satisfy the existing deterministic tolerance. That
regression now tests the oracle and retains harder-case withholding; it passes
on both qualified Mac architectures. The full local native suite, 831 Python tests, CMG and 19 downstream
oracle/integration/application tests pass. Linux full qualification and
downstream adoption remain pending;
no additional SCC qualification has been submitted after this second failure.
No candidate binary has been adopted downstream or publicly released. See the
[certificate contract](docs/CONTROL_BASIS_CERTIFICATION.md) and
[qualification record](docs/CONTROL_BASIS_REPAIR_2026-09-27.md).

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
