# Current checkpoint — 2026-09-29

The bounded interrupt optimization moves branches out of the scalar and batched
generic model operators, RHS reduction and worker reconstruction. Arithmetic
and callback positions, including flattened batch offsets, are preserved.
Five local complete-command cells improve by 4.2–26.6%, with bitwise-identical
targets and full residuals. Boundary/reuse tests, source gates, integrated Stata
quick/full checks and exact-source Mac thin/universal arm64/Rosetta qualification
pass at `d9692f6d`. Distributed binaries remain unchanged. See the
[experiment and validation record](docs/INTERRUPT_CHUNKS_2026-09-29.md).

## Review repair checkpoint — 2026-09-29

The confirmed review fixes repair root-catalog drift, explicit Rust toolchain
selection, caller-safe optional timing, C ingestion bounds, and environment-
sensitive tests. Python source tests (835), CMG, Rust 1.85.1/stable gates,
Stata quick/full suites, installs and focused regressions pass. Source
`70516881` passes exact-source native qualification on thin/universal arm64 and
Rosetta plugins; `5387db8d` repairs standalone harness path setup, with ten
isolated tests and three small harness checks passing. The integrated driver's
final standalone smoke was retested after that fix. Timer exhaustion leaves
diagnostics missing and estimation available.
See the [repair record and deferred proposals](docs/REVIEW_FIXES_2026-09-29.md).
No distributed binary or prior qualification/adoption record is changed.

## Control-span checkpoint — 2026-09-28

A control-span reliability candidate is qualified in the current worktree
against baseline `dc9b9d48beb04e83804527321934b269f4bf0a23`. No installed or
frozen application package is changed. It replaces generic JLA coordinate-error
propagation with a weighted span perturbation certificate, exposes failure
diagnostics, and adds allocation-bounded `nativethreads()`. See the
[prospective contract and qualification](docs/CONTROL_SPAN_REPAIR_2026-09-28.md).
The canceled Separations campaigns remain canceled.
Final candidate B passed thirteen retained-design checks in SCC job 7767513,
including all three former refusals and both target-weight systems. The
50-cell A8 study and explicit B compatibility review, independent small
four-target oracles, Mac/Linux interfaces, thread routing and probe precision
are recorded in the report. Owner adoption is pending; Intel/Rosetta, universal
Mac and Windows artifacts are not qualified for this candidate.

## Accepted checkpoint — 2026-09-27

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
