# Native package provenance

The default `mcse(all)` interface is qualified on all five repository plugin
payloads. The [current adoption manifest](mcse-default-20260930/manifest.json)
binds the exact artifacts and the source compatibility review. Mac thin and
universal plugins pass under arm64 and Rosetta; SCC Linux job `7803612` passes
the full suite and isolated install. Windows build
[36771390189](https://github.com/johannes-schmieder/fevc/actions/runs/36771390189)
passes static-CRT compilation, 14 standalone backend tests and the 142-export
PE audit. Its exact DLL passes private Stata/MP 19 runtime, fresh installation,
MCSE mode, projection/inference attachment and installed-hash checks; the
[Windows receipt](mcse-default-20260930/evidence/windows-qualification.json)
records cleanup and the stopped machine.

Source `eee457b3` passes all six hosted Rust platform/toolchain jobs. A first
hosted attempt exposed a test-only cache oracle invoking a local Python venv
from an ordinary Rust regression. The narrow fix retains that regression and
the explicit independent oracle; it changes no estimator or CI policy.
Calibration and performance claims retain the scope of the
[development receipt](../fevc/docs/mcse_default_development_20260930.json).
Native Intel hardware, representative scale, broader statistical coverage,
and a release remain outside these claims. The original platform receipts
retain their dirty-source classification.

## Previous control-basis repair

The September 27 control-basis repair was qualified for macOS arm64, macOS x86-64
under Rosetta, universal Mac payloads, and SCC Linux x86-64. The
[adoption manifest](control-certificate-20260927/manifest.json) binds the exact
four installed artifacts. Linux source `4064febe` passed its full Stata/MP 19
suite and isolated install in SCC job `7761676`, with successful scheduler
accounting. Mac artifacts were built at `6fc08fee`; the
[compatibility review](control-certificate-20260927/evidence/compatibility.json)
records the test-only changes and preserves the original dirty-end receipt.

The Windows binary retains its previous bytes and is not qualified for this
repair. Native Intel hardware, public release, and representative-scale
performance remain outside these claims. The statistical model, retained
populations and numerical acceptance gates are unchanged. See the
[repair record](../fevc/docs/CONTROL_BASIS_REPAIR_2026-09-27.md).

## Previous payload qualification


The previous checkpoint installed qualified macOS arm64, macOS x86-64, macOS universal,
Linux x86-64, and Windows x86-64 plugins with deletion-unit mover support.
Qualification covers the existing supported routes; it does not add statistical
coverage, unsupported Windows routes, native Intel hardware evidence, or a
public release tag. Intel Mac execution uses Rosetta.

The [previous input manifest](deletion-unit-movers-20260926/manifest.json) and
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
