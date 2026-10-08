# Prerelease preparation and Windows runtime — October 8, 2026

The owner requests repository cleanup, a reviewable `0.5.0-rc.1` candidate,
and working Windows runtime checks. Work stays on `main`. Local source
checkpoints, candidate builds and private platform tests are in scope;
publication and tagging remain separate owner decisions.

## Current checkpoint

- Runtime/build source is frozen at `24754269`; packaging is committed at
  `623b156d`. The two MC harnesses are tracked and their
  68 focused Python tests pass. Integrated source validation passes: 886
  Python tests, CMG checks, Stata quick/full, portable clean installation,
  migration and harness checks. The pinned supply-chain audit also passes.
- Removed 191 ignored compiler-cache/test-fixture paths (22.8 GiB of allocated
  entries; APFS sharing prevents a physical-space claim). All 6,530 protected
  evidence files retain their hashes. Results, reports, source snapshots,
  accepted receipts and native manifests remain intact.
- Mean remains the exact/JLA point and projection default. Projection uses
  the retained physical-frequency mean of the working outcome. None remains
  explicit; Corrected projection and centered component inference remain
  unsupported. Statistical and numerical contracts are unchanged.
- Clean Mac qualification passes at `24754269` for arm64, Rosetta x86-64 and
  universal. A separate follow-on passes 24 installed-capability, explicit
  point-centering and Mean-projection checks across thin/universal aliases.
  Linux x86-64 also passes full qualification and installed point-centering
  and Mean-projection checks at the same source (SCC job `7962808`).
- Windows smoke `win-20261008T153726Z-59a2355d` fails with the aggregate
  `STATA_DRIVER_FAILED` status. The project harness preserves all full-profile
  assertions and writes bounded stage/return-code diagnostics, but the accepted
  collector cannot retrieve them. No failing assertion is known. The reviewed
  collector proposal awaits owner approval; no shared changes are deployed.
  The existing Windows manual-test payload retains its original status.
- Fresh and replacement local HTTP `net install` pass on macOS, each verifying
  all 61 installed files. The final portable archive (58 members) and complete
  corresponding-source archive (1,443 files) pass independent hash review.
  [Package validation](../native/prerelease-20261008/package-validation.json)
  records the exact artifacts and source compatibility. Windows runtime is
  outside these installation claims.
- The [candidate manifest](../native/prerelease-20261008/manifest.json) records
  the local Mac/Linux candidate and pending Windows status. Historical evidence
  remains immutable. This candidate has not been published or tagged.

## Remaining candidate gates

Resolve the Windows collection boundary after owner approval, then
obtain bounded diagnostics and complete runtime qualification of the exact
candidate. After Windows succeeds, assemble and test the complete native
package and review its exact corresponding source and notices. Recheck any
package bytes changed by that work. An incomplete five-platform set
must not be labelled a qualified complete candidate. Publication and tagging
require a separate owner decision.

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
