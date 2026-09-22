# Current checkpoint — 2026-09-22

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
Direct repository installation still downloads all four committed plugins.

## Public package baseline

The existing repository is public with working `net install fevc` and
`github install johannes-schmieder/fevc` routes. `main` is the development
branch; `master` is the matching compatibility ref for the community installer.
The owner accepted the retained historical GitHub PR reviews. Do not restore
review files into the cleaned checkout. Windows binaries and testing remain
deferred. No tag or GitHub release has been created.

## Qualified package

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

## Installer verification

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
