# Release-hardening impact and compatibility review

## Source identity and scope

The release-hardening pass starts from
`acead96e6032f116bc192c5229d8446855c8f74e`. The candidate is the commit that
contains this review; its full hash is recorded in the final handoff and can be
resolved with `git rev-parse HEAD` from that clean checkout.

Changed surfaces are limited to the active plan and documentation, a
manifest-driven portable artifact builder and its tests, integrated static
gate selection, clean-install assertions, and ignored-artifact cleanup
classification. The pass does not modify:

- `fevc/fevc.ado`, any installed Mata runtime, the package manifest, or the
  package catalog;
- estimator, sample, target, deletion, weighting, nuisance, inference, RNG,
  tolerance, routing, fallback, failure, or numerical acceptance semantics;
- Rust estimator/native source, C ABI, build manifests, native helpers, plugin
  packaging, or qualified binary inputs; or
- benchmark generators, comparator code, production data, acceptance policy,
  immutable receipts, reviews, reports, or exact-SHA manifests.

## Evidence carried forward

The FEVC rename-equivalence receipt compares predecessor source
`a5805145b93961a98068de3452ff793013847942` with FEVC candidate source
`672ea448a8b80a353cdce3ba7122d820d6e5c5d4` under Stata 19 and records PASS.
The subsequent baseline through `acead96` preserved deliberate private
protocol/build identities and added the immutable rename evidence. The prompt
baseline additionally records passing rename quick/full, clean-install,
benchmark, CMG/B1, separation, MATLAB-bridge, and predecessor-equivalence
checks.

Those rename and runtime claims, the source-bound scientific/performance
results, and the macOS arm64/Rosetta and SCC Linux x86-64 platform claims carry
forward because this pass leaves every relevant production, ABI, build,
binary, package-manifest, input, comparator, and acceptance identity
unchanged. The human package-boundary, corresponding-source, notice,
provenance, and data-exclusion review also carries forward; the current license
and source-supply audits are rerun as focused confirmation.

No carried evidence is broadened. In particular, this review adds no Windows,
native-Intel, automatic-projection, unsupported-inference, 480,000-row
convergence, performance, binary-distribution, public-release, or version
claim. No large SCC or plugin-qualification rerun is justified by these
documentation, test, and non-native packaging-tool changes.

## Focused validation record

The final handoff records the exact command outputs. The required affected-
surface gates are:

- the full Python suite and deterministic generated-CMG check;
- public-identity, legacy-name, preserved-history, package-layout, parity,
  license/provenance, and source-supply audits;
- deterministic portable artifact check and construction from a clean commit;
- integrated Stata quick/full, CMG component, clean-install, benchmark, and
  separation checks with their explicit PASS markers;
- `git diff --check`, active-document link/path inspection, complete diff
  review, and a final clean worktree.

Plugin qualification is not selected because no native source, ABI, build,
native helper, plugin artifact rule, or native package payload changed. The
portable archive intentionally excludes plugin binaries and contains only
`stata.toc`, `fevc.pkg`, and the files listed by `fevc.pkg`.
