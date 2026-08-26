# Code license

## Selected license

The owner selected the GNU General Public License, Version 3 only
(`GPL-3.0-only`) on 2026-08-18 for the CMG implementation and for any
distributed package that forms one program with that implementation. The full
license text is in [`LICENSES/GPL-3.0-only.txt`](LICENSES/GPL-3.0-only.txt).

## Covered code

Subject to the file-level third-party notices and exclusions below, the grant
covers:

- the installable package and repository-authored implementation code under
  `vckss/**`, including the internal `vckss/cmg/**` component,
  tests, build tools, and generated CMG artifacts; and
- the Rust backend under `rust/**`, including the GPL-3.0-only full-CMG source
  vendored with its original copyright, license, and provenance notices.

Repository-authored files in those covered distributions are offered under
GPL-3.0-only. Adapted CMG files retain the copyright and GPL notices of their
original authors. A file-level provenance manifest must identify copied and
modified third-party source.

## Not covered by this grant

This is not a license for the whole repository. It does not grant rights to:

- manuscripts, releases, source notes, proofs, reviews, or archived material;
- restricted, licensed, confidential, or excluded data; or
- third-party dependencies except under their own licenses.

Those materials may appear in the same repository as a collection or
aggregate without becoming GPL-covered. Their existing notices and use terms
remain controlling.

## Distribution condition

Any conveyed object code must satisfy GPLv3's corresponding-source and notice
requirements. `CMG-MATA-1` is implemented entirely in Mata and includes no
compiled CMG helper, MEX file, Stata plugin, subprocess, or binary interchange
layer. The optional Rust backend includes a compiled full-CMG implementation
whose complete corresponding source and notices are retained under
`rust/vendor/cmg/`. Public distribution still requires a human review of the exact package
boundary, corresponding-source bundle, upstream notices, and third-party/data
exclusions. This document records the owner's license selection; it is not
legal advice.
