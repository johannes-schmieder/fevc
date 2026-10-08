# Prerelease preparation and Windows runtime — October 8, 2026

The owner requests repository cleanup, a reviewable `0.5.0-rc.1` candidate,
and working Windows runtime checks. Work stays on `main`. Local source
checkpoints, candidate builds and private platform tests are in scope;
publication and tagging remain separate owner decisions.

## Current checkpoint

- Removed 191 ignored compiler-cache/test-fixture paths (22.8 GiB of allocated
  cache entries; APFS sharing means this is not a physical-space claim).
  All 6,530 protected evidence files retain their hashes. MC results, the PDF
  report, source snapshots, accepted receipts and native manifests remain.
  The local inventory and receipt are in `.local/prerelease-20261008/`.
- Reviewed the pending source and both MC harnesses. Their 68 focused Python
  tests pass. The MC source is now tracked, resolving the conservative
  dirty-bundle inventory issue without weakening its allowlist.
- Mean is the point-estimate and projection default. Projection uses the
  retained physical-frequency mean of the working outcome. Explicit None
  remains available; Corrected projection and centered component inference
  remain unsupported. Help and active documentation reflect this contract.
- The project Windows driver now distinguishes a small smoke from the full
  profile, preserves all full-profile assertions, and writes bounded failure
  diagnostics plus a terminal FAIL status. Local harness checks pass; remote
  PowerShell execution and Windows runtime remain unqualified. See
  [the Windows harness guide](../rust/stata_backend/WINDOWS_CI.md).
- The accepted Windows collector currently returns only its aggregate receipt.
  A separately reviewed collector change would be needed to retrieve bounded
  project failure diagnostics or privately built candidate bytes. Do not
  bypass that boundary or infer a failing assertion from the aggregate error.

## Remaining candidate gates

Freeze the reviewed source, run the mandatory Python/CMG/Stata checks, and
qualify the affected native platforms against that source. Start Windows with
its bounded smoke. Linux needs the new Mean-projection capability and explicit
installed-candidate checks. Bind each adopted payload to its actual build
source, test scope and hash, then verify fresh/replacement installation of the
final candidate bytes. Do not label an incomplete five-platform manifest as a
qualified complete candidate.

## Preserved development evidence

The Mean-projection implementation passed the 48-cell independent exact
oracle, eight-cell native projection oracle, full Stata quick suite, Rust
workspace tests, formatting and strict Clippy. Mac arm64/Rosetta thin and
universal build/runtime/isolated-install checks also passed internally; their
outer CI receipt failed the clean-source gate because the development tree
was dirty. This is development evidence, not a clean-source qualification.
The three tested Mac candidates are preserved separately under
`.local/prerelease-20261008/macos-development-candidates/`; the tracked payload
was reset to the previously published bytes for the clean source checkpoint.
No candidate bytes were discarded.

The detailed development record is
`.local/projection-mean-20261007/development-validation.json`; the subsequent
documentation-only compatibility audit is in
`.local/projection-mean-docs-20261007/validation.json`. Earlier Mean-default and
MC evidence retain their original scope. The prior public `net install`
check verified all 61 published payload hashes and fresh/replacement installs
at `283d2524`; it does not qualify this unpublished candidate.

Current scientific contracts live in [CENTERING.md](docs/CENTERING.md),
[INFERENCE.md](docs/INFERENCE.md) and [DECISIONS.md](docs/DECISIONS.md).
Validation follows [development_acceptance_v1.json](docs/development_acceptance_v1.json).
