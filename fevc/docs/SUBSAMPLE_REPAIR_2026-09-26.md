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
Factoring the genuinely duplicated construction into a small helper resolved
that limit. Development test corrections removed unsupported Rust `probeorder`
and projection tuples; supported requests retain the same acceptance gates.

## Validation

Validation is in progress. Diagnostic logs are retained under ignored
`.local/subsample-fix-20260926/`; final commands, source identity, versions,
tolerances and limitations will be recorded here after the gates finish.

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
New Ado-level sample behavior needs the checks recorded above and must not be
inferred from old qualification. Previous affected subsample behavior is not
reused as evidence. This is not a new Monte Carlo, scaling or statistical
coverage claim. No downstream temporary-frame adapter is removed by this
repository change.
