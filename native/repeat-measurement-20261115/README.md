# Qualified 0.6.0 adoption

All five 0.6.0 plugins are built from `2082f209` with unmodified common
CMG `e5613e81` and the explicit 128-level limit. They include repeat-measurement
component standard errors and normal/q=1 intervals on the supported routes.
Mac arm64 and Rosetta thin/universal/installed, SCC Linux full/native/installed,
and all 18 private Windows groups pass. Stata 15.1 also passes the full suite
and installed-native tests on Intel/Rosetta; focused Stata 18/19 tests pass.
Windows qualification uses Stata 19 and one/two workers. Native Intel hardware
and Stata 16/17 runtime execution are not claimed.
The [manifest](manifest.json) binds the exact five binaries and corresponding
source. Windows retains all 18 groups, 144 paired commands and twelve separate
default-SE timing observations. Earlier failed development attempts remain in
the private release archive; the source compatibility review states the limits
of reused simulation evidence. No general coverage claim is made.

Version [0.6.0](https://github.com/johannes-schmieder/fevc/releases/tag/v0.6.0)
is published at adoption commit `b861a368`. The [publication audit](final-audit.json)
verifies all eight public tag/main/GitHub/downloaded-ZIP fresh and replacement
installations, with 66 exact installed files and all five plugin hashes in each.
Actual-default/native inference and caller-state checks pass on Mac arm64;
other platform runtime claims retain the separate common-build receipts.
The initial unstaged-evidence packaging failure remains recorded alongside
the subsequent 1,033-test pass. The adoption manifest remains unchanged.
