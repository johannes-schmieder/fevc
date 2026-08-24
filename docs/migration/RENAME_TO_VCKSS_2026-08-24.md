# Hard-cut rename to `vckss` — 2026-08-24

## Boundary

The source boundary is
`7fcf1b20105a546e993e0b3186c842f05f1f79a8`; the clean local checkout was
fast-forwarded over receipt-only commit
`f9fb00dc6254116a51853b1c2be7162b0a48368e` before any edit.

This migration makes a hard cut from `varcomp_kss` 0.3.0-dev to `vckss`
0.4.0-dev. It does not install a `varcomp_kss` wrapper or preserve the old
GitHub Actions slug as a callable identity.

| Previous active identity | Current identity |
|---|---|
| repository `johannes-schmieder/varcomp_kss` | `johannes-schmieder/vckss` |
| installable tree `varcomp_kss/` | `vckss/` |
| Stata command/package/help `varcomp_kss` | `vckss` |
| developer entrypoint `varcomp_kss_rust` | `vckss_rust` |
| plugin artifacts `varcomp_kss_rust_*` | `vckss_rust_*` |
| build-ID prefix `varcomp-kss-*` | `vckss-*` |
| active environment prefix `VARCOMP_KSS_*` | `VCKSS_*` |
| package version `0.3.0-dev` | `0.4.0-dev` |

Established private `_vckss_*`, `vckss__*`, and `VCKSS_*` symbols, Rust
crate names, C ABI symbols, KSS terminology, result contracts, result shapes,
statuses, and Mata/Rust interface API numbers are unchanged. CMG ownership API
8 and generator API 5 record only the generated target/package identity
change.

## Historical evidence

`frozen_legacy_inventory.json` remains byte-identical. The version-2
`relocation_inventory_v2.json` records original path, relocated path, size,
SHA-256, and byte-identity policy for every receipt-tip file. The active audit
verifies archived reports, receipts, logs, reviews, manifests, Rust progress
snapshots, and completed qualification evidence at their relocated paths.

Historical result fields and persisted schemas keep the identity under which
they were produced. New receipts and active consumers use `vckss`.

## Qualification boundary

The identity commit does not repair or qualify the known exact-V7 mismatch.
The untouched baseline failure is recorded in
`../../vckss/docs/EXACT_V7_BASELINE_FAILURE_2026-08-24.md`. Numerical
equivalence is a separate fresh-process qualification against source
`7fcf1b20105a546e993e0b3186c842f05f1f79a8`; its generated receipt must be
committed separately.

Public release remains disabled pending the existing human mathematical and
license/provenance review gates.
