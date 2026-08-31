# Third-party notices

FEVC is distributed under GPL-3.0-only subject to the file-level notices and
provenance records described in `CODE_LICENSE.md`.

## CMG

The optional Rust backend contains an adapted copy of the GPL-3.0-only CMG
implementation. Its original and modified-source notices, exact upstream
identity, file manifest, and license are preserved in:

- `rust/vendor/cmg/LICENSE`
- `rust/vendor/cmg/VENDOR.md`
- `rust/vendor/cmg/UPSTREAM_MANIFEST.sha256`
- `rust/vendor/cmg/docs/UPSTREAM.md`

The package-owned Mata implementation records its source-informed provenance
in `fevc/cmg/docs/SOURCE_PROVENANCE.md` and its upstream file identities in
`fevc/cmg/docs/UPSTREAM_SOURCE_MANIFEST.yaml`.

## Maintained MATLAB comparator

The maintained `LeaveOutTwoWay` MATLAB repository is an external behavioral
and scientific comparator. Its inspected source tree has no root software
license. No MATLAB source, binary data file, or critical-value table is copied
into FEVC. Implementations based on the published KSS formulas are
repository-authored GPL-3.0-only code.

## Dependencies and excluded material

Rust, Stata, Mata, and test dependencies remain governed by their own terms.
The GPL grant does not cover manuscripts, restricted or confidential data,
review packets, archived third-party materials, or other exclusions listed in
`CODE_LICENSE.md`.
