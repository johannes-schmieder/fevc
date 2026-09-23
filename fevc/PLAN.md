# Current checkpoint — 2026-09-23

## Main-only installation

The redundant `master` compatibility branch was removed on September 23;
`main` is the sole development and installation branch. Earlier references
below to publishing both refs describe the historical installation checks.

Fresh and replacement installs through both advertised commands passed on
Stata/MP 19, macOS Apple Silicon, after branch deletion. The installer commands
and package payload required no changes. The public installer verifier ran with:
`./.venv/bin/python fevc/tools/verify_native_installers.py --catalog . --test-root . --output .local/main-only-install-20260923 --public`.
It uses isolated Stata libraries and checks all 52 installed-file hashes,
native reporting, the README example, caller-data restoration, and match q0/q1
inference. The Python source suite passed all 829 tests, and
`./.venv/bin/python fevc/cmg/tools/assemble.py --all --check` passed.
Pytest reported temporary-directory cleanup permission warnings after passing.

## Binary installer refresh

The help/helper source is committed at `991597f7`. The Windows candidate was
rebuilt from that source on GitHub's hosted Windows runner and included in the
standard repository installer at `41ad5240`, on both `main` and `master`.
Compilation, 14 Rust tests, 129 exports, system-only dependencies, and all
Windows package hashes passed. Windows Stata testing is left to the owner;
see [installation instructions](../INSTALLATION.md).

The Mac plugins already match the current compiled source. Current-package
isolated tests pass for thin arm64, thin Rosetta x86-64, and the universal
binary on both architectures, including exact/JLA examples and the progress
regression. All 828 Python tests and generated CMG checks pass. See the
[new refresh record](../native/refresh-20260922.json) for source compatibility,
binary identities, and installer evidence. Older qualification records remain
unchanged.

Linux is still blocked by SCC authentication. Its committed binary remains
from September 21 and lacks the subsequent progress-summary cleanup. After
`ssh scc` authentication is renewed, use the existing source-bound deployment
and scheduled qualifier, collect its exact tested binary and scheduler receipt,
and repeat both public installer checks on Linux before updating the package.
No current Linux rebuild or new runtime test is claimed.

## Applied help and example data

The help now starts with a runnable example, explains the leave-out sample
and main user choices, and groups advanced options separately. It is about
69% shorter by word count. Methodological detail refers to the companion
paper and the existing technical guides.

`fevc, simulate_data(ex1)` through `ex5` generate the example datasets with
sample counts and true realized, target-weighted components. The installed
helper is readable from the help; seeds are configurable for random examples.
Generation requires explicit replacement of existing data, restores caller RNG
state, and restores old data on failure. All five clickable examples remain.
The estimation and native numerical implementations are unchanged.

Focused simulation checks pass, including independent truth calculations,
seed repeatability, alternate caller RNGs, replacement safeguards, nested
preservation, and injected-failure rollback. The 827 Python tests and generated
CMG check pass after adding the helper to the current source-bundle allowlist.
The integrated `./.venv/bin/python fevc/tools/run_checks.py` passed on Stata 19,
including quick/full suites, all five help examples, clean installation, both
helper-migration layouts, and harness smokes. Stata-rendered help has no raw
markup or wrapped code lines. Diagnostic logs and command/version details are
in ignored `.local/help-refresh/`. No new platform or native qualification
claim is made by these help/example changes.

## Progress-display cleanup

Repeated identical native method/settings and allocation summaries are now
suppressed within each reporting scope; elapsed-time updates continue and
changed settings remain visible. The help examples now put JLA first and exact
second, with all five runnable example bodies unchanged.

Mac arm64, Rosetta x86-64, and universal plugins were rebuilt and qualified from
`0614534c`. The strengthened Stata progress regression passed on all four
artifact/architecture combinations. Running the exact help example independently
confirmed one method summary and one allocation summary. See the new
[`native/` records](../native/README.md). Linux still carries the September 21
binary; rebuilding its progress fix requires renewed SCC login access.

Source archives omit repository plugin binaries through `.gitattributes`,
preserving the frozen SCC source-bundle builder and its no-old-binaries gate.
Direct repository installation downloads the committed plugins; the current
Windows candidate and its pending Stata check are documented in `native/README.md`.

The integrated local checks passed: 827 Python tests, CMG gates, the quick and
full Stata suites, help examples, installation and migration checks, and harness
smokes. Both public installers passed fresh and replacement installs on Mac,
including all 50 file hashes and the exact example's single-summary output.
See [`progress validation`](../native/progress-20260922-validation.json).
Both public refs carry the updated package. Source CI passed on `f7d13d1c`;
the existing broader Rust CI matrix is separate from native Stata qualification.

## Public package baseline

The existing repository is public with working `net install fevc` and
`github install johannes-schmieder/fevc` routes. `main` is the development
and installation branch; the former `master` compatibility ref was removed
on September 23.
The owner accepted the retained historical GitHub PR reviews. Do not restore
review files into the cleaned checkout. The Windows binary is now included as a build-tested candidate; the owner
will test it in Windows Stata. No tag or GitHub release has been created.

## September 21 qualification

Native build source: `f2a15dea9fe5715d654a00976e97c8d706efe63f`, including the
latest runtime reporting and total-elapsed updates. Mac arm64, Rosetta x86-64,
universal, and Linux x86-64 qualification passed. Linux job `7674352` passed
the full Stata suite and isolated native install with scheduler `failed=0`
and `exit_status=0`. All 825 original local Python tests and generated-source
checks passed. Public CI after the two verifier regressions were added passed
818 tests, with 9 licensed/local Stata checks skipped on GitHub.

Root catalogs and the four binary files are committed. Dependency notices,
actual build identities, hashes, compatibility review, and compact qualification
receipts live in [`native/`](../native/README.md). Packaging and later evidence
commits do not redefine the binaries' original tested source.

## September 21 installer verification

Both advertised public commands passed fresh and replacement installation
checks on Mac and Linux under Stata/MP 19. Checks cover all 50 installed-file
hashes, native reporting API 2, the README example and caller-data restoration,
help, decomposition, match q0/q1 regression, and an idle native registry.
See [`native/installation.json`](../native/installation.json).

The first Linux HTTP attempt passed all Stata application checks but failed a
case-sensitive Python lookup of LICENSE; Stata installs lowercase filenames.
The verifier fix preserves duplicate rejection and exact byte comparisons.
Its bounded public retest is recorded separately from the failed first attempt.

The final publication tree, rewritten local history, and all remote refs,
including the accepted historical PR refs, passed redacted Gitleaks scans.
Detailed logs remain in ignored `.local/public-install-20260921/`.

Preserve scientific contracts, routes, thresholds, and limitations. Native
build evidence does not create new statistical coverage, native Intel hardware,
Windows, or representative-scale claims. Candidate promotion and evidence reuse
follow [`development_acceptance_v1.json`](docs/development_acceptance_v1.json).
