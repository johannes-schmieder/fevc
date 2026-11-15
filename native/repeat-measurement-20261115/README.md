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

The adoption checkpoint precedes publication. Final package checks, release
assets and eight actual published installation cases will be recorded here.
