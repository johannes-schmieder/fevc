# Active package plan

## Objective

Maintain `varcomp_kss` 0.3.0-dev as the sole Stata command and package
identity while preserving the estimator and numerical behavior qualified
under its predecessor name.

## Current boundaries

- Public command and installed files use `varcomp_kss`; no predecessor alias
  is shipped.
- Private Ado/Mata symbols use `vckss`, `_vckss_`, and `VCKSS` prefixes.
- CMG is an internal package component under `cmg/`. API 7 and generator API 4
  are ownership/interface changes over the API-6-qualified numerical core.
- Stable scientific vocabulary includes KSS method labels, `e(kss)`, result
  statuses, estimator options, tolerances, RNG contracts, and return shapes.
- Historical reports, receipts, reviews, plans, and source hashes remain
  byte-identical evidence and are not current instructions.

## Completion gates

1. Generated CMG sources pass deterministic drift checks and expose only the
   package and test targets.
2. All package, CMG, and retained MATLAB-harness Python tests pass.
3. Stata quick/full, clean-install, namespace, forced/automatic CMG, and
   benchmark smoke gates pass when Stata/MP is available.
4. Deterministic source bundles close over tracked package-owned source.
5. The active-name audit rejects predecessor package, shared-library, and
   removed non-KSS CMG surfaces outside hash-bound historical records.
6. Rename equivalence compares isolated predecessor and successor processes;
   scientific, structural, return-shape, caller-state, and RNG records match
   exactly after registered identity transitions. Timing and measured
   code-footprint fields remain in raw evidence and may differ only under the
   explicit finite/nonnegative normalization registry.

Public release remains disabled until human license/provenance review is
complete.
