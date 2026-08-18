# Current decisions

## Package identity

- `varcomp_kss` is the only public command, help topic, package manifest, and
  installed file prefix.
- The rename is a hard cut because the predecessor command was not publicly
  released. No compatibility wrapper is installed.
- Version 0.3.0-dev records the package-identity boundary.

## Private namespace

- Active private Mata functions and structures use `vckss*`.
- Active Ado helpers use `_vckss_*`; package globals use `VCKSS_*`.
- Runtime build identifiers changed with the namespace so stale predecessor
  runtimes fail closed.

## CMG ownership

- CMG is a component of this package, not a shared library.
- Its deterministic generator produces only the shipped `vckss_cmg` runtime
  and the checked-in `cmgtest` target.
- CMG API 7 and generator API 4 remove the unused non-KSS surface without
  changing the API-6-qualified numerical core.

## Evidence

- Scientific KSS terminology, estimator results, numerical contracts, and
  historical milestone identifiers are not renamed.
- Predecessor reports, receipts, reviews, plans, logs, manifests, and prior
  changelog are immutable historical evidence.
- A candidate qualification or equivalence receipt must bind a committed
  successor source revision; it is not fabricated from an uncommitted tree.
