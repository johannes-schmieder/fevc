# Frozen subsample repair — 2026-09-26

## Diagnosis and scope

Baseline: `b7216f72e9dfdef003de9e21e0822c8d2f79f5ba` on `main`.
The two Ado dispatch paths generated an eligible-stayer indicator only inside
the frozen requested sample, leaving missing values elsewhere. Stata treats
missing as true in logical expressions, so combining that indicator with the
mover mask included excluded rows. Direct stayer-mask consumers also counted
or exported excluded rows. This predates the deletion-unit mover repair.

A 32-call diagnostic produced 15 affected failures, one affected success,
12 passing controls and four intentional missing-requested-input failures.
The successful Mata JLA call selected 72 rows but marked 76 in `e(sample)`;
its estimates agreed with the compact counterpart to relative `7.389e-17`.
Thus silent sample/accounting corruption was demonstrated, but this diagnostic
did not establish wrong point estimates. Exact Mata reported nonfinite input,
generic Mata JLA a resource-model error, and Rust an internal-invariant error.
The new regression failed on the original source with 14 excluded rows in
`e(sample)`.

## Repair

- `fevc__hybrid_sample.ado` constructs the shared eligible-stayer and combined
  sample masks as complete zero/one indicators. Excluded rows have zero
  retained-firm membership; stayer physical totals must be finite. The combined
  sample is explicitly bounded by the frozen complete-input mask.
- Both paths call this helper. Original-stayer indicators are also binary
  throughout the dataset, including the implicit-match preparation path.
  Audited downstream consumers include group maps, worker/row counts,
  singleton/unattached counts, physical/target mass, native augmentation,
  solver inputs and `e(sample)`.
- Paired weighted JLA tests additionally exposed draw changes caused by
  incidental Stata sort order. Mata now gives stayer rows one ordering label
  per worker before applying the normal row-content key; its mixed kernel
  still deletes individual physical observations. Rust receives stayer rows
  sorted by worker, firm, per-copy target mass, outcome, controls and frequency.
  Its projection attachment uses the same recorded row map. This changes
  seeded JLA draws where old incidental ordering differed, without changing
  the estimator, deletion partition or numerical acceptance tolerances.
- Mover eligibility still counts original deletion units. Graph-dropped
  movers cannot become original stayers. Required inputs missing **inside**
  requested matches still produce `MATCH_INPUT_MISSING`; missing inputs
  outside `if`/`in` do not enter the fit.
- Both package manifests, the source-bundle allowlist and native qualifier
  inventory include the helper. Source and installed-native runners include
  the new regression. The native install inventory gains one file (53 total).

The first inline mask repair exceeded Stata's compiled-program size limit.
Factoring the duplicated construction into a small helper resolved
that limit. Development test corrections removed unsupported Rust `probeorder`
and projection tuples; supported requests retain the same acceptance gates.

## Validation

Implementation source: `9681684a3f89e5883406fc27ca5e0eceb452d050`.
Integrated source: `16d5564abfbf7c9db0f36644a7ad4bf96af3fd81`. The latter
restores the checkpoint's policy link and guards an already unsupported
Windows native projection test; its installed payload is identical.

| Check | Result |
| --- | --- |
| `./.venv/bin/python fevc/tools/run_checks.py` | PASS: 829 Python tests, CMG generation/core gates, Stata quick/full, portable install, both helper migrations, benchmark and sample-audit smokes |
| `./ci/run_ci_profile.sh plugin-build` at `9681684a` | PASS: locked Rust format/Clippy/tests, C/SPI gates, thin and universal Mac candidates on arm64 and Rosetta, isolated installs |
| New subsample regression on Stata/MP 18 and 19 | PASS: 116 paired scenarios on each |
| Shipped Intel plugin (Rosetta) and shipped universal plugin (arm64/Rosetta) | PASS: 116 paired scenarios for each artifact/architecture |
| Frozen native catalog via `verify_native_installers.py` | PASS: fresh and replacement `net install`, 53 installed hashes each, installed inference/mover/subsample tests |
| Original 32-call diagnostic | PASS: all 28 valid calls; four expected `MATCH_INPUT_MISSING` failures remain |

The portable variant runs 58 pairs when no native plugin is available. The
integrated gate ran on `16d5564a` with the original shipped plugins; the native
profile ran on `9681684a` with new local candidates. The candidate binaries
were not substituted for the shipped artifacts. See the
[qualification](../../native/subsample-repair-20260926/macos-qualification.txt),
[source manifest](../../native/subsample-repair-20260926/macos-source.sha256),
[install receipt](../../native/subsample-repair-20260926/installed-verification.json),
and [compatibility record](../../native/subsample-repair-20260926/compatibility.json)
for exact commands, versions, hashes, sources and scope. Mac qualification used
Stata/MP 19, Rust/Cargo 1.85.1, Apple Clang 21.0.0 and macOS 26.6.2.

Useful diagnostic logs remain in ignored `.local/subsample-fix-20260926/`.
The first Python run caught the helper count and sorted bundle-allowlist
updates; the first integrated run caught a missing policy link in `PLAN.md`.
All were repaired before the passing final integrated run. Existing protected
pytest temporary directories emitted cleanup warnings; no final test failed.

The compact-data oracle is independent of the production sample helper. It
compares retained row identities, all-row exclusion, sample/worker/firm/deletion
counts, stayer exclusions and physical/target mass, results and numerical MCSE,
caller data/order/RNG restoration, and native registry cleanup. Exact and JLA
cover `if`, `in`, excluded missing inputs (including extended missings), an
excluded mover observation, and a selected population containing only movers.
Designs cover unpooled and pooled firms, original deletion IDs, controls,
frequency/target weights and both nuisance conventions. Controls cover
mover-only and observation populations, automatic engines and supported
projection routes. JLA uses seed 9262026, 33 probes and batch 8; paired matrix
relative differences must be below `1e-8`, count/mass differences within
`1e-10*max(1,abs(reference))`. No scientific or solver gate is relaxed.

## Compatibility and limits

No Rust source, C shim, ABI, dependency lock, generated CMG or shipped plugin
is changed. Prior artifact identities and compiled-code qualifications remain
bound to their original source receipts under
`native/deletion-unit-movers-20260926/`; those receipts are not rewritten.
New Ado-level sample behavior is supported by the checks recorded above,
not inferred from old qualification. No Linux or Windows runtime campaign was
rerun for this repair. Previous affected subsample behavior is not
reused as evidence. This is not a new Monte Carlo, scaling or statistical
coverage claim. No downstream temporary-frame adapter is removed by this
repository change.
