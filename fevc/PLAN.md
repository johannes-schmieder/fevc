# Current checkpoint — 2026-09-21

Prepare and publish the existing repository with working `net install fevc`
and `github install johannes-schmieder/fevc` routes. The owner accepted that
old GitHub pull-request refs retain the deleted reviews and explicitly deferred
Windows binaries and testing. Do not restore review files into the checkout.

## Current source

The committed implementation includes the September 18 helper rename to
`fevc__*.ado` and the September 19 Rust runtime reporting and total-elapsed
updates. Qualify current Mac/Linux builds for the public installation package;
September 18 binaries do not represent these later native changes.

Preserve existing scientific contracts, routes, thresholds, and limitations.
Previous optimization and reporting evidence remains in ignored `.local/`
directories; do not overwrite or relabel historical receipts.

## Qualified package

Native build source: `f2a15dea9fe5715d654a00976e97c8d706efe63f`.
Mac arm64, Rosetta x86-64, universal, and Linux x86-64 qualification passed.
Linux job `7674352` passed the full Stata suite and isolated native install,
with scheduler `failed=0` and `exit_status=0`. All 825 Python source tests and
generated-source checks passed. Packaging source `055d7b2b` adds dependency
notices and installer verification without changing native/runtime sources.
The exact binaries and evidence are indexed in [`native/`](../native/README.md).

## Publication work

- Explicit `macos-linux` packaging profile with all four required binaries;
  the default complete profile still requires Windows.
- Clean-source macOS arm64/Rosetta/universal and Linux x86-64 qualification.
- Root installation catalogs, corresponding native source, licensing notices,
  and reviewed binary hashes/evidence; keep both installer refs synchronized.
- Fresh and replacement installation checks using both advertised commands,
  native execution, help, the README example, and installed-file hashes.
- Scan the final publication tree and history before changing visibility.

Run manifests and detailed installation evidence belong in ignored
`.local/public-install-20260921/`. Keep the public README focused on use and
installation. Candidate promotion and evidence reuse follow
[`development_acceptance_v1.json`](docs/development_acceptance_v1.json).
