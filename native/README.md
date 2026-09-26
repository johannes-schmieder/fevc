# Native package provenance

The repository installs qualified macOS arm64, macOS x86-64, macOS universal,
Linux x86-64, and Windows x86-64 plugins with deletion-unit mover support.
Qualification covers the existing supported routes; it does not add statistical
coverage, unsupported Windows routes, native Intel hardware evidence, or a
public release tag. Intel Mac execution uses Rosetta.

The [current input manifest](deletion-unit-movers-20260926/manifest.json) and
[package receipt](deletion-unit-movers-20260926/package.receipt.json) bind all
five artifacts. Mac and Windows were built from `d6f5571d`; Linux was built
from `31cf2ea7`, which corrects a progress-display test without changing the
estimator. Packaging source `c3ab9e52` adds the bounded private Windows harness.
The [Mac/Windows compatibility review](deletion-unit-movers-20260926/evidence/compatibility-build-to-package.json)
and [Linux compatibility review](deletion-unit-movers-20260926/evidence/compatibility-linux-to-package.json)
record the unchanged production inputs and the precise test/harness changes.

- [Mac qualification](deletion-unit-movers-20260926/evidence/macos-qualification.txt):
  thin arm64, thin Rosetta x86-64, universal under both architectures, and
  isolated installs. The [222-file manifest](deletion-unit-movers-20260926/evidence/macos-source.sha256)
  identifies the qualified inputs.
- [Linux qualification](deletion-unit-movers-20260926/evidence/linux-qualification.txt):
  full Stata/MP 19 suite and isolated install, SCC job `7745342`.
  [Scheduler accounting](deletion-unit-movers-20260926/evidence/linux-scheduler.json)
  requires `failed=0` and `exit_status=0`.
- [Windows qualification](deletion-unit-movers-20260926/evidence/windows-qualification.json):
  exact [hosted artifact](https://github.com/johannes-schmieder/fevc/actions/runs/36237445599),
  129-export PE/system-dependency audit, isolated install, lifecycle, component
  and match q0/q1 inference, deletion-unit oracle, positive projection, installed
  binary hash, and idle registry on private licensed Stata/MP 19. The fixed
  controller returns an aggregate source-bound PASS; raw Stata logs and
  individual project-check JSON are not available. Cleanup and stopped-instance
  checks passed. The original build-only receipt keeps its historical status.

At package commit `72da5542`, fresh and replacement public installs through both `net install` and
`github install` pass on Mac arm64 and Linux x86-64. Each case checks all
52 installed-file hashes and the native mover/inference regressions.
[Publication evidence](deletion-unit-movers-20260926/publication.json) binds
commit `72da5542`, the two platform receipts, and Linux job `7745408`.
Windows isolated installation was qualified separately on the exact artifact.
The later [subsample repair](../fevc/docs/SUBSAMPLE_REPAIR_2026-09-26.md)
adds one Ado helper (53 installed files) and changes sample/order preparation;
its [new qualification and compatibility record](subsample-repair-20260926/compatibility.json)
separates local rebuilt-candidate tests from tests of the unchanged shipped
artifacts. All five plugin files retain the artifact identities above.

The [integration record](../fevc/docs/DELETION_UNIT_MOVERS_2026-09-26.md)
records tests, failures, retained limitations and final public installer status.
Detailed diagnostic logs remain outside the tracked checkout. Earlier receipts
in this directory remain immutable and retain their original sources and limits.

Corresponding source, C shim, build scripts, and locked dependencies are in
`rust/` at each recorded build commit. See the
[build guide](../rust/stata_backend/README.md). [Pinned dependencies](dependencies.json)
record source downloads, licenses, and checksums; runtime license texts ship in
[`THIRD_PARTY_NOTICES.txt`](../fevc/THIRD_PARTY_NOTICES.txt).

Root `fevc.pkg` selects the native package; `fevc/fevc.pkg` is the portable
source manifest. Test a staged catalog in isolated Stata directories with:

```bash
./.venv/bin/python fevc/tools/verify_native_installers.py \
    --catalog /path/to/staged/package --test-root "$PWD" \
    --output /path/to/new/evidence-directory
```

Add `--public` to test fresh and replacement installs through both advertised
commands. The verifier checks every installed byte, reporting, the README
example, caller restoration, help, decomposition, match q0/q1, the deletion-unit
mover regression, and the idle registry. Licensed execution stays local or on
private infrastructure. Publish to `main`; no `master` branch is maintained.
This repository installation does not create a release tag or release archive.
