# Third-party notices

FEVC is distributed under MIT subject to the file-level notices and
provenance records described in `CODE_LICENSE.md`.

## CMG

The optional Rust backend contains an unmodified copy of the MIT CMG
implementation. Its upstream notices, exact source
identity, file manifest, and license are preserved in:

- `rust/vendor/cmg/LICENSE`
- `rust/vendor/CMG-SOURCE.json`
- `rust/vendor/cmg/docs/UPSTREAM.md`

The package-owned Mata implementation records its source-informed provenance
in `fevc/cmg/docs/SOURCE_PROVENANCE.md` and its upstream file identities in
`fevc/cmg/docs/UPSTREAM_SOURCE_MANIFEST.yaml`.

On 2026-11-03 Johannes Schmieder confirmed that CMG's developers had granted
official permission, communicated through Ioannis Koutis, to release CMG under
the MIT license. Johannes selected MIT for CMG, fevc, and fereg on that basis.

The MIT grant is in `CODE_LICENSE.md` and the CMG license above.

## KSS Matlab comparator

The maintained `LeaveOutTwoWay` MATLAB repository is an external behavioral
and scientific comparator. Its inspected source tree has no root software
license. No MATLAB source, binary data file, or critical-value table is copied
into FEVC. Implementations based on the published KSS formulas are
repository-authored MIT code.

## Dependencies and excluded material

Rust, Stata, Mata, and test dependencies remain governed by their own terms.
The MIT grant does not cover manuscripts, restricted or confidential data,
review packets, archived third-party materials, or other exclusions listed in
`CODE_LICENSE.md`.
